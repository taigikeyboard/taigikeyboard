// NextWord 平台服務 — 管理 user_association.db SQLite(read+write)+ bundled association.bin 共用查詢。
// bigram lookup 已走 Rust lexicon::assoc_lookup;此處只剩 SQLite 寫路徑與啟動 prep 等平台粘合。
// 對應 iOS NextWord/Services/NextWordService.swift。

package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteStatement
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.db.vacuumBestEffort
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File
import java.util.concurrent.atomic.AtomicInteger

private fun SQLiteStatement.bindArgs(vararg args: Any?) {
    clearBindings()
    args.forEachIndexed { index, arg ->
        val i = index + 1
        when (arg) {
            null -> bindNull(i)
            is String -> bindString(i, arg)
            is Long -> bindLong(i, arg)
            is Int -> bindLong(i, arg.toLong())
            else -> throw IllegalArgumentException("Unsupported bind type: ${arg::class}")
        }
    }
}

/**
 * NextWord bigram prediction service.
 *
 * Uses a bigram model keyed on the last character of the selected word:
 * - 選「早安」→ 用「安」查詢 → 預測下一個字
 *
 * Data sources:
 * - association.bin (binary mmap): read-only dictionary associations
 * - user_association.db: user-learned associations (persisted forever)
 *
 * Owned by `CompositionRoot`; lazy initialization guarded by a mutex.
 *
 * File layout: Constants · Types · Properties · Init · Public API
 * (Prediction / Recording / Queries) · Lifecycle · User DB Schema ·
 * Scoring · Pruning.
 */
class NextWordService(
    appContext: Context,
    private val logger: LoggerBackend,
) {
    private val appContext: Context = appContext.applicationContext

    // ------------------------------------------------------------------ //
    // Constants
    // ------------------------------------------------------------------ //

    companion object {
        private const val TAG = "NextWordService"
        private const val USER_DB_NAME = "user_association.db"
        // CROSS-PLATFORM INVARIANT — mirrors iOS `NextWordSchema.schemaVersion`
        // and macOS `UserAssociationStore.schemaVersion`. Drift causes silent
        // divergence. v6: UNIQUE widened to carry `prev_tl`.
        private const val DATABASE_VERSION = 6

        /**
         * Default candidate limit when callers don't specify one.
         * Matches iOS `NextWordService.Constants.defaultLimit`.
         */
        const val DEFAULT_LIMIT: Int = 30
        //
        // Scoring + merge-by-(hanzi, tl) live in `engine/nextword/src/{scorer,filter}.rs`
        // post-v3.5.5. The platform `predict()` returns un-scored rows tagged
        // by `Source`; the caller routes them through
        // `RustEngineBridge.nextwordFilter` for the score+merge+sort+limit step.

        // User-association capacity (prevents unbounded DB growth)
        private const val MAX_USER_ASSOCIATIONS = 50_000

        // After every N records, check whether pruning is needed
        private const val PRUNE_CHECK_INTERVAL = 100

        // When over capacity, delete the N lowest-scoring entries
        private const val PRUNE_BATCH_SIZE = 5_000

        // Built once rather than re-`trimIndent()`-ed on every commit: these
        // three run on the IME's per-committed-word path.
        private val RECORD_ASSOCIATION_SQL =
            """
            INSERT INTO user_association (prev_word, prev_tl, next_word, next_tl, count, last_used)
            VALUES (?, ?, ?, ?, 1, CURRENT_TIMESTAMP)
            ON CONFLICT(prev_word, prev_tl, next_word, next_tl) DO UPDATE SET
                count = count + 1,
                last_used = CURRENT_TIMESTAMP
            """.trimIndent()

        private val USER_PREDICT_SQL =
            """
            SELECT next_word, next_tl, count,
                   strftime('%s', last_used) * 1000 AS last_used_ms
            FROM user_association
            WHERE prev_word = ?
            ORDER BY
                CASE WHEN prev_tl = ? THEN 0 WHEN prev_tl = '' THEN 1 ELSE 2 END,
                count DESC, last_used DESC, id ASC
            LIMIT ?
            """.trimIndent()

        private val BATCH_IMPORT_ASSOCIATION_SQL =
            """
            INSERT INTO user_association (prev_word, prev_tl, next_word, next_tl, count, last_used)
            VALUES (?, ?, ?, ?, ?, datetime('now'))
            ON CONFLICT(prev_word, prev_tl, next_word, next_tl) DO UPDATE SET
                count = MAX(count, excluded.count),
                last_used = datetime('now')
            """.trimIndent()
    }

    // ------------------------------------------------------------------ //
    // Public types
    // ------------------------------------------------------------------ //

    /** User-learned association row (debug / export). */
    data class AssociationEntry(
        val prevWord: String,
        val prevTl: String = "",
        val nextWord: String,
        val nextTl: String,
        val count: Int,
    )

    // ------------------------------------------------------------------ //
    // Properties
    // ------------------------------------------------------------------ //

    @Volatile private var userDatabase: SQLiteDatabase? = null

    @Volatile private var isInitialized = false
    private val initMutex = Mutex()

    private val recordCounter = AtomicInteger(0)

    // ------------------------------------------------------------------ //
    // Init
    // ------------------------------------------------------------------ //

    /**
     * Ensure user database is initialized. Bundled-bigram readers live in
     * the Rust shared-core engine (installed once at app startup via
     * `LexiconBridge.install` in `AppInitializer`), so this method only
     * gates the SQLite user_association.db.
     */
    private suspend fun ensureInitialized() {
        if (isInitialized) return

        initMutex.withLock {
            if (isInitialized) return

            try {
                connectUserDb()
                isInitialized = userDatabase != null
                logger.i(TAG, "[INIT] NextWord initialized: userDb=${userDatabase != null}")
            } catch (e: Exception) {
                logger.e(TAG, "[INIT] Initialization failed", e)
            }
        }
    }

    /**
     * Open (or create) `user_association.db` and run pending migrations.
     *
     * The handle is published to [userDatabase] only after the schema is
     * ready. A migration that throws must leave NO usable handle behind:
     * `ensureInitialized` catches and leaves `isInitialized` false, and a
     * half-migrated database reachable through the property would be written
     * to anyway by the paths that only null-check it.
     */
    private fun connectUserDb() {
        val dbFile = File(appContext.filesDir, USER_DB_NAME)
        val db = SQLiteDatabase.openOrCreateDatabase(dbFile, null)

        try {
            val didMigrate = ensureUserAssocSchema(db)
            if (didMigrate) {
                // R6: refresh the query planner's stats once, AFTER all DDL
                // (CREATE INDEX included). Gated on an actual migration so it
                // never runs on a steady-state open.
                db.execSQL("PRAGMA optimize;")
            }
            // One-shot migration: WAL → DELETE journal mode (v3.4.8).
            migrateFromWAL(db)
        } catch (e: Exception) {
            db.close()
            throw e
        }

        userDatabase = db
    }

    // ------------------------------------------------------------------ //
    // Public API — Prediction
    // ------------------------------------------------------------------ //

    /**
     * Predict raw rows for the next-word bigram bridge.
     *
     * Post-v3.5.5: returns un-merged un-scored rows tagged by source
     * (`SOURCE_DICT` from `association.bin` mmap, `SOURCE_USER` from
     * `user_association.db`). The caller passes them to
     * [RustEngineBridge.nextwordFilter] which scores (dict via
     * `DICT_WEIGHT`; user via decay+learning math), merges by
     * `(hanzi, tl)`, sorts desc by score, applies limit, and shapes per
     * display rules.
     *
     * [nowMs] still threads through for log-decay parity (the caller
     * forwards it on to `nextwordFilter`'s `now_ms` so the
     * `shouldRecordAssociation` clock and the user-row decay scoring see
     * ONE consistent "now" per intent — `nextword-engine-boundary.md` §13.3).
     */
    suspend fun predict(
        word: String,
        roman: String = "",
        limit: Int = DEFAULT_LIMIT,
        settings: EngineSettings,
        @Suppress("UNUSED_PARAMETER") nowMs: Long,
    ): List<RustEngineBridge.NextWordRawRow> =
        withContext(Dispatchers.IO) {
            if (word.isEmpty()) {
                return@withContext emptyList()
            }

            val lastChar = word.last().toString()

            ensureInitialized()

            val rows = mutableListOf<RustEngineBridge.NextWordRawRow>()

            // 1. Dictionary associations — un-scored rows tagged SOURCE_DICT.
            // Over-fetch limit * 2 so the Rust filter has slack to merge
            // (hanzi, tl) collisions across dict + user without dropping
            // below the caller's requested limit (Codex post-impl P2-1).
            // Cold-start gate around assocLookup only — user-DB paths below
            // don't need the lexicon engine (Codex r3173440132). If install
            // hasn't completed (or failed), skip dict rows and let user
            // associations still surface.
            val lexiconReady = com.siansiansu.taigikeyboard.ime.core.CompositionRoot
                .shared(appContext)
                .awaitLexiconReady()
            if (lexiconReady) {
                try {
                    logger.debug(TAG) { "[PREDICT] Dict query: prev_word='$lastChar'" }
                    // Engine applies the 1-layer source filter (low 9 bits of
                    // bitmask) per audit §4. Over-fetch limit * 2 so the Rust
                    // filter step has slack to merge (hanzi, tl) collisions
                    // across dict + user without dropping below the caller's
                    // requested limit. `assocLookupBitmask` is engine-resolved
                    // (`UInt.MAX_VALUE` sentinel when all 9 sources on, else
                    // exact mask) — pre-v3.5.8 the platform branched on
                    // `allAssociationSourcesEnabled`.
                    val toggles = com.siansiansu.taigikeyboard.engine.LexiconBridge
                        .DictionaryToggles
                        .from(settings)
                    val bitmask = com.siansiansu.taigikeyboard.engine.LexiconBridge
                        .dictionaryFilters(toggles)
                        .assocLookupBitmask
                    val entries = com.siansiansu.taigikeyboard.engine.LexiconBridge.assocLookup(
                        previousWord = lastChar,
                        limit = (limit * 2).toUInt(),
                        enabledSourcesBitmask = bitmask,
                    )
                    for (entry in entries) {
                        rows.add(
                            RustEngineBridge.NextWordRawRow(
                                hanzi = entry.candidateWord,
                                tl = entry.candidateTl,
                                count = entry.count.toLong(),
                                lastUsedMs = 0L,
                                source = RustEngineBridge.NextWordRawRow.Source.DICT,
                            ),
                        )
                    }
                    logger.debug(TAG) { "[PREDICT] Dict: ${entries.size} entries (bridge)" }
                } catch (e: Exception) {
                    logger.e(TAG, "[PREDICT] Dict query failed", e)
                }
            } else {
                logger.debug(TAG) { "[PREDICT] Dict skipped — lexicon not ready" }
            }

            // 2. User associations — un-scored rows tagged SOURCE_USER.
            val dictCount = rows.size
            userDatabase?.let { db ->
                try {
                    // CROSS-PLATFORM INVARIANT — mirrors
                    // ios/Sources/TaigiKeyboard/NextWord/Repository/NextWordRepository.swift
                    // fetchUserRows. prev_word (Hanji) is the only lookup key;
                    // prev_tl is a ranking signal (exact > empty > mismatch),
                    // NOT a hard filter, so a mismatched non-empty prev_tl
                    // (the other reading of a 一字多音 Hanji, or a pre-v3.6.1
                    // raw form) is still recalled. Drift causes silent
                    // divergence. Pins behavioral-invariants.md §24.
                    //
                    // ROW ORDER IS LOAD-BEARING. v6 stores 重/tîng → 複 and
                    // 重/tāng → 複 separately, so a Hanji-only lookup can return
                    // several rows predicting the SAME word. The engine keeps
                    // only the FIRST user row per predicted (hanzi, tl)
                    // (engine/nextword/src/filter.rs) rather than summing their
                    // scores, so this ORDER BY is what decides which reading's
                    // evidence is used. Nothing between this cursor and the
                    // engine may reorder these rows.
                    logger.debug(TAG) { "[PREDICT] User query: prev_word='$word', prev_tl='$roman'" }
                    // Over-fetch limit * 2 — same merge-slack reason as the
                    // dict path above (Codex post-impl P2-1).
                    val cursor = db.rawQuery(USER_PREDICT_SQL, arrayOf(word, roman, (limit * 2).toString()))
                    cursor.use {
                        while (it.moveToNext()) {
                            val nextWord = it.getString(0) ?: continue
                            val nextTl = it.getString(1) ?: ""
                            val count = it.getInt(2)
                            val lastUsedMs = it.getLong(3)
                            rows.add(
                                RustEngineBridge.NextWordRawRow(
                                    hanzi = nextWord,
                                    tl = nextTl,
                                    count = count.toLong(),
                                    lastUsedMs = lastUsedMs,
                                    source = RustEngineBridge.NextWordRawRow.Source.USER,
                                ),
                            )
                        }
                    }
                    logger.debug(TAG) { "[PREDICT] User: ${rows.size - dictCount} new rows (total ${rows.size})" }
                } catch (e: Exception) {
                    logger.e(TAG, "[PREDICT] User query failed", e)
                }
            }

            rows
        }

    // ------------------------------------------------------------------ //
    // Public API — Recording
    // ------------------------------------------------------------------ //

    /** Record a bigram transition in `user_association` and prune periodically. */
    @Suppress("SqlResolve")
    suspend fun recordAssociation(
        prev: String,
        prevTl: String = "",
        nextHanzi: String,
        nextTl: String = "",
    ): Unit =
        withContext(Dispatchers.IO) {
            if (prev.isEmpty() || nextHanzi.isEmpty()) {
                return@withContext
            }

            ensureInitialized()

            val db = userDatabase ?: return@withContext

            try {
                db.execSQL(RECORD_ASSOCIATION_SQL, arrayOf(prev, prevTl, nextHanzi, nextTl))

                logger.debug(TAG) { "[RECORD] '$prev' (tl='$prevTl') -> '$nextHanzi' (tl='$nextTl')" }

                if (recordCounter.incrementAndGet() >= PRUNE_CHECK_INTERVAL) {
                    recordCounter.set(0)
                    pruneOldAssociations()
                }
            } catch (e: Exception) {
                // Fire-and-forget: a failed association write must never block
                // typing. Single boundary — execSQL throws the real
                // SQLiteException, logged once here. LoggerBackend gates all
                // levels on BuildConfig.DEBUG (release no-op).
                logger.e(TAG, "association.record.failed prev=$prev next=$nextHanzi", e)
            }
        }

    /** Batch-import association rows, merging by max(existing, incoming) count. */
    @Suppress("SqlResolve")
    suspend fun batchImportAssociations(entries: List<AssociationEntry>): Int =
        withContext(Dispatchers.IO) {
            try {
                ensureInitialized()
                val db = userDatabase ?: return@withContext 0

                db.beginTransaction()
                var imported = 0
                try {
                    val stmt = db.compileStatement(BATCH_IMPORT_ASSOCIATION_SQL)
                    for (entry in entries) {
                        stmt.bindArgs(
                            entry.prevWord,
                            entry.prevTl,
                            entry.nextWord,
                            entry.nextTl,
                            entry.count.toLong(),
                        )
                        stmt.executeInsert()
                        imported++
                    }
                    db.setTransactionSuccessful()
                } finally {
                    db.endTransaction()
                }
                imported
            } catch (e: Exception) {
                logger.e(TAG, "[BATCH_IMPORT] Association import failed", e)
                0
            }
        }

    /** Delete a single user association row. */
    suspend fun deleteAssociation(entry: AssociationEntry) =
        withContext(Dispatchers.IO) {
            try {
                ensureInitialized()
                val db = userDatabase ?: return@withContext

                db.execSQL(
                    "DELETE FROM user_association WHERE prev_word = ? AND prev_tl = ? AND next_word = ? AND next_tl = ?",
                    arrayOf(entry.prevWord, entry.prevTl, entry.nextWord, entry.nextTl),
                )
            } catch (e: Exception) {
                logger.e(TAG, "[DELETE] Failed to delete association", e)
            }
        }

    /** Clear every user association row. */
    suspend fun clearAllAssociations() =
        withContext(Dispatchers.IO) {
            try {
                ensureInitialized()
                val db = userDatabase ?: return@withContext

                db.execSQL("DELETE FROM user_association")

                logger.i(TAG, "[CLEAR] All user associations cleared")
                vacuumBestEffort(db, logger, TAG)
            } catch (e: Exception) {
                logger.e(TAG, "[CLEAR] Failed to clear associations", e)
            }
        }

    // ------------------------------------------------------------------ //
    // Public API — Queries
    // ------------------------------------------------------------------ //

    /** All user associations (debug / export). */
    suspend fun allAssociations(): List<AssociationEntry> =
        withContext(Dispatchers.IO) {
            try {
                ensureInitialized()
                val db = userDatabase ?: return@withContext emptyList()
                fetchAllAssociations(db)
            } catch (e: Exception) {
                logger.e(TAG, "[QUERY] Failed to get all associations", e)
                emptyList()
            }
        }

    private fun fetchAllAssociations(db: SQLiteDatabase): List<AssociationEntry> {
        val cursor =
            db.rawQuery(
                """
                SELECT prev_word, prev_tl, next_word, next_tl, count
                FROM user_association
                ORDER BY count DESC, last_used DESC
                """.trimIndent(),
                null,
            )
        val results = mutableListOf<AssociationEntry>()
        cursor.use {
            while (it.moveToNext()) {
                results.add(
                    AssociationEntry(
                        prevWord = it.getString(0) ?: "",
                        prevTl = it.getString(1) ?: "",
                        nextWord = it.getString(2) ?: "",
                        nextTl = it.getString(3) ?: "",
                        count = it.getInt(4),
                    ),
                )
            }
        }
        return results
    }

    // ------------------------------------------------------------------ //
    // Lifecycle
    // ------------------------------------------------------------------ //

    /** Release user database handle. Subsequent API calls re-open on demand. */
    fun close() {
        userDatabase?.close()
        userDatabase = null
        isInitialized = false

        logger.i(TAG, "[CLOSE] Resources released")
    }

    // ------------------------------------------------------------------ //
    // User DB — Schema, Indexes, Migrations
    // ------------------------------------------------------------------ //

    /// The v6 table. The UNIQUE key carries `prev_tl` because a Taiwanese word
    /// is the `(漢字, canonical TL)` pair (CLAUDE.md Core Principle #7) on the
    /// bigram's PREVIOUS side as well as its next: 重/tîng → 複 and 重/tāng → 複
    /// are two observations, not one. CROSS-PLATFORM INVARIANT — mirrors
    /// ios/…/NextWord/Repository/NextWordSchema.swift `createTables`.
    private fun createUserAssocTable(db: SQLiteDatabase) = db.execSQL(userAssocTableSql("user_association"))

    /** The v6 shape, one source for both the live table and the rebuild's. */
    private fun userAssocTableSql(name: String) =
        """
        CREATE TABLE IF NOT EXISTS $name (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            prev_word TEXT NOT NULL,
            prev_tl TEXT DEFAULT '',
            next_word TEXT NOT NULL,
            next_tl TEXT DEFAULT '',
            count INTEGER DEFAULT 1,
            last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(prev_word, prev_tl, next_word, next_tl)
        )
        """.trimIndent()

    private fun createUserAssocIndexes(db: SQLiteDatabase) {
        // One read index: (prev_word, prev_tl) serves the recall query's
        // `WHERE prev_word = ?` from its left prefix + the prev_tl tier
        // ordering. The single-column idx_user_prev_word is its subset;
        // dropped in v5.
        db.execSQL(
            "CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl ON user_association(prev_word, prev_tl)",
        )
    }

    /** One-shot WAL → DELETE journal-mode migration. */
    private fun migrateFromWAL(db: SQLiteDatabase) {
        val journalMode =
            db.rawQuery("PRAGMA journal_mode;", null)?.use { cursor ->
                if (cursor.moveToFirst()) cursor.getString(0) else null
            } ?: return

        if (!journalMode.equals("wal", ignoreCase = true)) {
            logger.debug(TAG) { "[MIGRATE] No WAL detected (journal_mode=$journalMode), skipping migration" }
            return
        }

        logger.d(TAG, "[MIGRATE] WAL detected, performing checkpoint and switching to DELETE")
        db.rawQuery("PRAGMA wal_checkpoint(TRUNCATE);", null)?.use { cursor ->
            if (BuildConfig.DEBUG && cursor.moveToFirst()) {
                val busy = cursor.getInt(0)
                val log = cursor.getInt(1)
                val checkpointed = cursor.getInt(2)
                if (busy != 0 || log != checkpointed) {
                    logger.w(
                        TAG,
                        "[MIGRATE] WAL checkpoint incomplete: busy=$busy, log=$log, checkpointed=$checkpointed",
                    )
                }
            }
        }
        db.rawQuery("PRAGMA journal_mode=DELETE;", null)?.close()
    }

    /**
     * Bring `user_association.db` to `DATABASE_VERSION`, then guarantee the
     * terminal table + indexes exist. Returns whether an upgrade ran.
     *
     * An upgrade runs as ONE transaction covering the rebuild, the terminal
     * DDL, and the version stamp together: a database that says v6 must
     * actually have the v6 table and both v6 indexes, so a failure anywhere
     * has to take the version stamp down with it. Any exception propagates —
     * a half-migrated database is never stamped v6, and never published.
     *
     * The historical ladder (v0→v2→v3→v4→v5) collapses into ONE convergent
     * rebuild. SQLite cannot ALTER a table-level UNIQUE, so widening the key
     * means rebuilding the table anyway, and a rebuild that reads the columns
     * it finds subsumes every intermediate step.
     *
     * It also repairs a real pre-existing defect: the old ladder's later gates
     * tested the ORIGINAL `currentVersion`, so a v0/v1 database ran only
     * `migrateV0ToV2` — whose new table had no `prev_tl` column — and then got
     * stamped "v5" without it, with `createUserAssocTable`'s `IF NOT EXISTS`
     * unable to repair it. `rebuildToV6` detects missing columns rather than
     * assuming them.
     *
     * CROSS-PLATFORM INVARIANT — mirrors ios/…/NextWordSchema.swift
     * `ensureTables`. Drift causes silent divergence.
     */
    private fun ensureUserAssocSchema(db: SQLiteDatabase): Boolean {
        val currentVersion =
            db.rawQuery("PRAGMA user_version;", null).use {
                if (it.moveToFirst()) it.getInt(0) else 0
            }

        if (currentVersion >= DATABASE_VERSION) {
            createUserAssocTable(db)
            createUserAssocIndexes(db)
            return false
        }

        logger.i(TAG, "[MIGRATE] user_association.db v$currentVersion -> v$DATABASE_VERSION")

        db.beginTransaction()
        try {
            migrateToV6(db, currentVersion)
            createUserAssocTable(db)
            createUserAssocIndexes(db)
            db.execSQL("PRAGMA user_version = $DATABASE_VERSION;")
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
        return true
    }

    /**
     * Bring a pre-v6 database to the v6 shape, inside the caller's transaction.
     *
     * The one version dropped rather than rebuilt is v2, matching what
     * `migrateV2ToV3` did. A v2 stamp is ambiguous — `migrateV0ToV2` created a
     * table with `next_tl` in the key and stamped it v2, while a table written
     * as v2 in the first place may not have — and a rebuild cannot tell those
     * apart, so it drops rather than guess. The v0/v1 shape, by contrast, is
     * known: `migrateV0ToV2` rebuild-copied its rows and dropped only the
     * unused `next_poj` / `delimiter` COLUMNS, so it is rebuilt here too.
     */
    private fun migrateToV6(
        db: SQLiteDatabase,
        currentVersion: Int,
    ) {
        if (currentVersion == 2) {
            db.execSQL("DROP TABLE IF EXISTS user_association")
        } else if (userAssocTableExists(db)) {
            rebuildToV6(db)
        }
        // The v4 single-column index: dropped with its table on either branch
        // above, so this only catches a DB whose table was already absent.
        db.execSQL("DROP INDEX IF EXISTS idx_user_prev_word")
    }

    private fun userAssocTableExists(db: SQLiteDatabase): Boolean =
        db
            .rawQuery(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='user_association' LIMIT 1",
                null,
            ).use { it.moveToFirst() }

    private fun userAssocHasColumn(
        db: SQLiteDatabase,
        column: String,
    ): Boolean =
        db
            .rawQuery(
                "SELECT COUNT(*) FROM pragma_table_info('user_association') WHERE name = ?",
                arrayOf(column),
            ).use { it.moveToFirst() && it.getInt(0) > 0 }

    /**
     * Rebuild the table under the v6 key, preserving every row.
     *
     * Widening a UNIQUE key can never conflict — the old key
     * `(prev_word, next_word, next_tl)` is a strict subset of the new one — so
     * the copy needs no dedupe or merge step.
     *
     * `id` is copied rather than reassigned: it is the final tiebreak of the
     * read query's per-prediction pick, so renumbering could silently change
     * which row wins for rows that tie on everything else.
     *
     * Order is SQLite's documented one — create new, copy, drop old, rename —
     * rather than rename-first, which can rewrite references inside triggers
     * and views. Runs inside the caller's transaction.
     *
     * CROSS-PLATFORM INVARIANT — mirrors ios/…/NextWordSchema.swift
     * `rebuildToV6`. Drift causes silent divergence.
     */
    private fun rebuildToV6(db: SQLiteDatabase) {
        // A v0/v1/v3 table predates the prev_tl column, and a DB that came up
        // the v0/v1 ladder can be stamped v5 without it. Read what is there
        // rather than assuming — mistaking a real column for an absent one
        // would blank every stored romanization.
        val prevTl = if (userAssocHasColumn(db, "prev_tl")) "COALESCE(prev_tl, '')" else "''"
        val nextTl = if (userAssocHasColumn(db, "next_tl")) "COALESCE(next_tl, '')" else "''"

        db.execSQL(userAssocTableSql("user_association_new"))
        db.execSQL(
            """
            INSERT INTO user_association_new
                (id, prev_word, prev_tl, next_word, next_tl, count, last_used)
            SELECT id, prev_word, $prevTl, next_word, $nextTl, count, last_used
            FROM user_association
            """.trimIndent(),
        )
        db.execSQL("DROP TABLE user_association")
        db.execSQL("ALTER TABLE user_association_new RENAME TO user_association")
        logger.i(TAG, "[MIGRATE] user_association rebuilt under the v6 key")
    }

    // ------------------------------------------------------------------ //
    // Pruning
    // ------------------------------------------------------------------ //

    /** Drop the lowest-score user associations when capacity is exceeded. */
    private suspend fun pruneOldAssociations() =
        withContext(Dispatchers.IO) {
            val db = userDatabase ?: return@withContext

            try {
                val countCursor = db.rawQuery("SELECT COUNT(*) FROM user_association", null)
                val currentCount =
                    countCursor.use {
                        if (it.moveToFirst()) it.getInt(0) else 0
                    }

                if (currentCount <= MAX_USER_ASSOCIATIONS) {
                    logger.debug(TAG) { "[PRUNE] No pruning needed: $currentCount <= $MAX_USER_ASSOCIATIONS" }
                    return@withContext
                }

                val deleteCount = minOf(PRUNE_BATCH_SIZE, currentCount - MAX_USER_ASSOCIATIONS + PRUNE_BATCH_SIZE)

                val deleteSql =
                    """
                    DELETE FROM user_association
                    WHERE id IN (
                        SELECT id FROM user_association
                        ORDER BY count ASC, last_used ASC
                        LIMIT ?
                    )
                    """.trimIndent()

                db.execSQL(deleteSql, arrayOf(deleteCount.toString()))

                logger.i(
                    TAG,
                    "[PRUNE] Deleted $deleteCount associations (was $currentCount, target <= $MAX_USER_ASSOCIATIONS)",
                )
            } catch (e: Exception) {
                logger.e(TAG, "[PRUNE] Failed to prune associations", e)
            }
        }
}
