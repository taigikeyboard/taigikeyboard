package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteStatement
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.nextword.RawNextWordPrediction
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File
import java.util.concurrent.atomic.AtomicInteger
import kotlin.math.exp

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
        private const val DATABASE_VERSION = 4 // v4: added prev_tl column
        private const val DEFAULT_LIMIT = 30

        // CROSS-PLATFORM INVARIANT — the scoring constants below
        // (USER_WEIGHT, DICT_WEIGHT, DECAY_HALF_LIFE_HOURS, LEARNING_BONUS,
        // *_DECAY_FLOOR, *_THRESHOLD) MUST mirror iOS NextWordService.swift.
        // Drift causes silent ranking divergence between platforms.

        // Source weights — USER_WEIGHT > DICT_WEIGHT so learned entries rank above dict
        private const val USER_WEIGHT = 50
        private const val DICT_WEIGHT = 1

        // Time decay (RIME-style exponential decay): half-life 168h (1 week)
        private const val DECAY_HALF_LIFE_HOURS = 168.0

        // Memory strength: ensures user entries rank above dict entries
        private const val LEARNING_BONUS = 300.0
        private const val HIGH_USAGE_DECAY_FLOOR = 0.95 // count >= 3: near-permanent retention
        private const val LOW_USAGE_DECAY_FLOOR = 0.3 // count < 3: prevents full decay (~1 month visible)
        private const val HIGH_USAGE_THRESHOLD = 3

        // User-association capacity (prevents unbounded DB growth)
        private const val MAX_USER_ASSOCIATIONS = 50_000

        // After every N records, check whether pruning is needed
        private const val PRUNE_CHECK_INTERVAL = 100

        // When over capacity, delete the N lowest-scoring entries
        private const val PRUNE_BATCH_SIZE = 5_000
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

    @Volatile private var associationReader: AssociationBinaryReader? = null

    @Volatile private var userDatabase: SQLiteDatabase? = null

    @Volatile private var isInitialized = false
    private val initMutex = Mutex()

    private val recordCounter = AtomicInteger(0)

    // ------------------------------------------------------------------ //
    // Init
    // ------------------------------------------------------------------ //

    /**
     * Ensure databases are initialized.
     *
     * If the association binary reader is null after initial setup
     * (LexiconService has not copied `association.bin` yet), subsequent
     * calls retry opening it. This avoids permanently losing dictionary
     * bigram predictions due to init ordering.
     */
    private suspend fun ensureInitialized() {
        if (isInitialized) {
            if (associationReader == null) {
                initAssociationReader()
            }
            return
        }

        initMutex.withLock {
            if (isInitialized) return

            try {
                initAssociationReader()
                connectUserDb()

                // Only mark initialized when the user DB is ready. Association
                // reader may still be null if LexiconService has not yet
                // copied the file; the lazy retry above will pick it up later.
                isInitialized = userDatabase != null

                logger.i(
                    TAG,
                    "[INIT] NextWord initialized: assocReader=${associationReader != null}, userDb=${userDatabase != null}",
                )
            } catch (e: Exception) {
                logger.e(TAG, "[INIT] Initialization failed", e)
            }
        }
    }

    /** Open the read-only association binary reader once `association.bin` is available. */
    private fun initAssociationReader() {
        val binFile = File(appContext.filesDir, DictionaryConstants.ASSOC_BIN_NAME)

        if (!binFile.exists()) {
            logger.e(TAG, "[INIT] association.bin not found (LexiconService not initialized?)")
            return
        }

        associationReader = AssociationBinaryReader.open(binFile)

        if (associationReader != null) {
            logger.i(TAG, "[INIT] Association binary reader loaded")
        } else {
            logger.e(TAG, "[INIT] Failed to open association.bin")
        }
    }

    /** Open (or create) `user_association.db` and run pending migrations. */
    private fun connectUserDb() {
        val dbFile = File(appContext.filesDir, USER_DB_NAME)

        userDatabase = SQLiteDatabase.openOrCreateDatabase(dbFile, null)

        val db = userDatabase ?: return

        migrateUserDb(db)
        createUserAssocTable(db)
        createUserAssocIndexes(db)

        // One-shot migration: WAL → DELETE journal mode (v3.4.8).
        migrateFromWAL(db)
    }

    // ------------------------------------------------------------------ //
    // Public API — Prediction
    // ------------------------------------------------------------------ //

    /**
     * Predict the next character given the last-committed [word]. Merges
     * dictionary bigrams with user-learned entries.
     *
     * [nowMs] is supplied by the caller (A5-impl clock-injection — the
     * executor holds the single `System.currentTimeMillis()` reader for
     * the whole intent, so the `shouldRecordAssociation` window and the
     * user-row decay score see the same "now"). Android walks one step
     * ahead of iOS here; iOS `NextWordService.predict` still reads the
     * clock internally. Documented in `nextword-engine-boundary.md` §13.3.
     */
    suspend fun predict(
        word: String,
        roman: String = "",
        limit: Int = DEFAULT_LIMIT,
        settings: EngineSettings,
        nowMs: Long,
    ): List<RawNextWordPrediction> =
        withContext(Dispatchers.IO) {
            if (word.isEmpty()) {
                return@withContext emptyList()
            }

            val lastChar = word.last().toString()

            ensureInitialized()

            val results = mutableMapOf<String, RawNextWordPrediction>()

            // 1. Dictionary associations — look up via `last char` in association.bin
            associationReader?.let { reader ->
                try {
                    val enabledDicts = EnabledDictionaries.fromSettings(settings)

                    logger.debug(TAG) { "[PREDICT] Dict query: prev_word='$lastChar'" }

                    // Over-fetch limit * 2 for dedup merging (matches iOS)
                    val entries = reader.lookup(lastChar, limit * 2)
                    for (entry in entries) {
                        if (!AssociationBinaryReader.passesFilter(entry.bitmask, enabledDicts)) continue

                        val key = "${entry.nextWord}\t${entry.nextTl}"
                        results[key] =
                            RawNextWordPrediction(
                                hanzi = entry.nextWord,
                                tl = entry.nextTl,
                                score = entry.count.toDouble() * DICT_WEIGHT,
                            )
                    }

                    logger.debug(TAG) { "[PREDICT] Dict: ${entries.size} raw -> ${results.size} after filter" }
                } catch (e: Exception) {
                    logger.e(TAG, "[PREDICT] Dict query failed", e)
                }
            }

            // 2. User associations — bigram keyed on the full word (with TL disambiguator)
            userDatabase?.let { db ->
                try {
                    val sql =
                        """
                        SELECT next_word, next_tl, count,
                               strftime('%s', last_used) * 1000 AS last_used_ms
                        FROM user_association
                        WHERE prev_word = ? AND (prev_tl = ? OR prev_tl = '')
                        ORDER BY count DESC
                        LIMIT ?
                        """.trimIndent()

                    logger.debug(TAG) { "[PREDICT] User query: prev_word='$word', prev_tl='$roman'" }

                    val cursor = db.rawQuery(sql, arrayOf(word, roman, (limit * 2).toString()))
                    var userCount = 0
                    cursor.use {
                        while (it.moveToNext()) {
                            val nextWord = it.getString(0) ?: continue
                            val nextTl = it.getString(1) ?: ""
                            val count = it.getInt(2)
                            val lastUsedMs = it.getLong(3)
                            userCount++

                            val userScore = calculateUserScore(count, lastUsedMs, nowMs)

                            logger.debug(TAG) {
                                val decay = calculateDecay(lastUsedMs, nowMs)
                                "[PREDICT] User found: '$word' -> '$nextWord' (count=$count, decay=%.3f, score=%.1f)".format(
                                    decay,
                                    userScore,
                                )
                            }

                            val key = "${nextWord}\t$nextTl"
                            val existing = results[key]

                            if (existing != null) {
                                results[key] =
                                    existing.copy(
                                        tl = if (nextTl.isNotEmpty()) nextTl else existing.tl,
                                        score = existing.score + userScore,
                                    )
                            } else {
                                results[key] =
                                    RawNextWordPrediction(
                                        hanzi = nextWord,
                                        tl = nextTl,
                                        score = userScore,
                                    )
                            }
                        }
                    }
                    logger.debug(TAG) { "[PREDICT] User query returned $userCount results" }
                } catch (e: Exception) {
                    logger.e(TAG, "[PREDICT] User query failed", e)
                }
            }

            val sortedResults =
                results.values
                    .sortedByDescending { it.score }
                    .take(limit)

            logger.debug(TAG) { "[PREDICT] '$word' -> ${sortedResults.size} total results (dict+user)" }

            sortedResults
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
    ) = withContext(Dispatchers.IO) {
        if (prev.isEmpty() || nextHanzi.isEmpty()) {
            return@withContext
        }

        ensureInitialized()

        val db = userDatabase ?: return@withContext

        try {
            val sql =
                """
                INSERT INTO user_association (prev_word, prev_tl, next_word, next_tl, count, last_used)
                VALUES (?, ?, ?, ?, 1, CURRENT_TIMESTAMP)
                ON CONFLICT(prev_word, next_word, next_tl) DO UPDATE SET
                    prev_tl = excluded.prev_tl,
                    count = count + 1,
                    last_used = CURRENT_TIMESTAMP
                """.trimIndent()

            db.execSQL(sql, arrayOf(prev, prevTl, nextHanzi, nextTl))

            logger.debug(TAG) { "[RECORD] '$prev' (tl='$prevTl') -> '$nextHanzi' (tl='$nextTl')" }

            if (recordCounter.incrementAndGet() >= PRUNE_CHECK_INTERVAL) {
                recordCounter.set(0)
                pruneOldAssociations()
            }
        } catch (e: Exception) {
            logger.e(TAG, "[RECORD] Insert failed", e)
        }
    }

    /** Batch-import association rows, merging by max(existing, incoming) count. */
    @Suppress("SqlResolve")
    suspend fun batchImportAssociations(entries: List<AssociationEntry>): Int =
        withContext(Dispatchers.IO) {
            try {
                ensureInitialized()
                val db = userDatabase ?: return@withContext 0

                val sql =
                    """
                    INSERT INTO user_association (prev_word, prev_tl, next_word, next_tl, count, last_used)
                    VALUES (?, ?, ?, ?, ?, datetime('now'))
                    ON CONFLICT(prev_word, next_word, next_tl) DO UPDATE SET
                        prev_tl = excluded.prev_tl,
                        count = MAX(count, excluded.count),
                        last_used = datetime('now')
                    """.trimIndent()

                db.beginTransaction()
                var imported = 0
                try {
                    val stmt = db.compileStatement(sql)
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

    /** Release database + reader handles. Subsequent API calls re-open on demand. */
    fun close() {
        associationReader = null
        userDatabase?.close()
        userDatabase = null
        isInitialized = false

        logger.i(TAG, "[CLOSE] Resources released")
    }

    // ------------------------------------------------------------------ //
    // User DB — Schema, Indexes, Migrations
    // ------------------------------------------------------------------ //

    private fun createUserAssocTable(db: SQLiteDatabase) {
        db.execSQL(
            """
            CREATE TABLE IF NOT EXISTS user_association (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prev_word TEXT NOT NULL,
                prev_tl TEXT DEFAULT '',
                next_word TEXT NOT NULL,
                next_tl TEXT DEFAULT '',
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(prev_word, next_word, next_tl)
            )
        """,
        )
    }

    private fun createUserAssocIndexes(db: SQLiteDatabase) {
        db.execSQL(
            "CREATE INDEX IF NOT EXISTS idx_user_prev_word ON user_association(prev_word)",
        )
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
                    logger.w(TAG, "[MIGRATE] WAL checkpoint incomplete: busy=$busy, log=$log, checkpointed=$checkpointed")
                }
            }
        }
        db.rawQuery("PRAGMA journal_mode=DELETE;", null)?.close()
    }

    /**
     * Migrate `user_association.db` to the current `DATABASE_VERSION`.
     * Each step delegates to a named migration function.
     */
    private fun migrateUserDb(db: SQLiteDatabase) {
        val cursor = db.rawQuery("PRAGMA user_version;", null)
        val currentVersion =
            cursor.use {
                if (it.moveToFirst()) it.getInt(0) else 0
            }

        if (currentVersion >= DATABASE_VERSION) return

        logger.i(TAG, "[MIGRATE] user_association.db v$currentVersion -> v$DATABASE_VERSION")

        if (currentVersion < 2) migrateV0ToV2(db)
        if (currentVersion in 2 until 3) migrateV2ToV3(db)
        if (currentVersion in 3 until 4) migrateV3ToV4(db)

        db.execSQL("PRAGMA user_version = $DATABASE_VERSION;")
    }

    /** v0/v1 → v2: drop next_poj and delimiter columns to align with iOS. */
    private fun migrateV0ToV2(db: SQLiteDatabase) {
        val hasOldTable =
            try {
                val ti =
                    db.rawQuery(
                        "SELECT name FROM sqlite_master WHERE type='table' AND name='user_association'",
                        null,
                    )
                ti.use { it.moveToFirst() }
            } catch (_: Exception) {
                false
            }

        if (!hasOldTable) return

        try {
            db.beginTransaction()
            db.execSQL("ALTER TABLE user_association RENAME TO user_association_old")
            db.execSQL(
                """
                CREATE TABLE user_association (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    prev_word TEXT NOT NULL,
                    next_word TEXT NOT NULL,
                    next_tl TEXT DEFAULT '',
                    count INTEGER DEFAULT 1,
                    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE(prev_word, next_word, next_tl)
                )
            """,
            )
            db.execSQL(
                """
                INSERT INTO user_association (prev_word, next_word, next_tl, count, last_used)
                SELECT prev_word, next_word, COALESCE(next_tl, ''), count, last_used
                FROM user_association_old
            """,
            )
            db.execSQL("DROP TABLE user_association_old")
            db.execSQL("CREATE INDEX IF NOT EXISTS idx_user_prev_word ON user_association(prev_word)")
            db.setTransactionSuccessful()
        } catch (e: Exception) {
            logger.e(TAG, "[MIGRATE] Failed to migrate user_association", e)
        } finally {
            db.endTransaction()
        }
    }

    /** v2 → v3: UNIQUE(prev_word, next_word) → UNIQUE(prev_word, next_word, next_tl). */
    private fun migrateV2ToV3(db: SQLiteDatabase) {
        val hasTable =
            try {
                val ti =
                    db.rawQuery(
                        "SELECT name FROM sqlite_master WHERE type='table' AND name='user_association'",
                        null,
                    )
                ti.use { it.moveToFirst() }
            } catch (_: Exception) {
                false
            }

        if (!hasTable) return

        try {
            db.beginTransaction()
            db.execSQL("DROP TABLE user_association")
            db.execSQL("DROP INDEX IF EXISTS idx_user_prev_word")
            db.setTransactionSuccessful()
            logger.i(TAG, "[MIGRATE] Recreated user_association with UNIQUE(prev_word, next_word, next_tl)")
        } catch (e: Exception) {
            logger.e(TAG, "[MIGRATE] v2->v3 migration failed", e)
        } finally {
            db.endTransaction()
        }
    }

    /** v3 → v4: add prev_tl column (ALTER TABLE, no data loss). */
    private fun migrateV3ToV4(db: SQLiteDatabase) {
        try {
            db.beginTransaction()
            db.execSQL("ALTER TABLE user_association ADD COLUMN prev_tl TEXT DEFAULT ''")
            db.execSQL("CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl ON user_association(prev_word, prev_tl)")
            db.setTransactionSuccessful()
            logger.i(TAG, "[MIGRATE] Added prev_tl column to user_association")
        } catch (e: Exception) {
            logger.e(TAG, "[MIGRATE] v3->v4 migration failed", e)
        } finally {
            db.endTransaction()
        }
    }

    // ------------------------------------------------------------------ //
    // Scoring
    // ------------------------------------------------------------------ //

    /**
     * RIME-style exponential decay factor. decay = exp(-ageHours / halfLifeHours * ln(2)).
     * Recent usage ≈ 1.0; one week out ≈ 0.5; one month out ≈ 0.06.
     */
    private fun calculateDecay(
        lastUsedMs: Long,
        nowMs: Long,
    ): Double {
        val ageHours = (nowMs - lastUsedMs) / 3600000.0
        return exp(-ageHours / DECAY_HALF_LIFE_HOURS * 0.693)
    }

    /**
     * User-layer score with decay floor + learning bonus. Ensures user
     * entries outrank dict entries by `LEARNING_BONUS`. Mirrors iOS
     * `NextWordService.calculateUserScore()`.
     */
    private fun calculateUserScore(
        count: Int,
        lastUsedMs: Long,
        nowMs: Long,
    ): Double {
        val decay = calculateDecay(lastUsedMs, nowMs)
        val rawScore = count.toDouble() * USER_WEIGHT
        val decayFloor =
            if (count >= HIGH_USAGE_THRESHOLD) {
                HIGH_USAGE_DECAY_FLOOR
            } else {
                LOW_USAGE_DECAY_FLOOR
            }
        val effectiveDecay = maxOf(decayFloor, decay)
        return rawScore * effectiveDecay + LEARNING_BONUS
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

                logger.i(TAG, "[PRUNE] Deleted $deleteCount associations (was $currentCount, target <= $MAX_USER_ASSOCIATIONS)")
            } catch (e: Exception) {
                logger.e(TAG, "[PRUNE] Failed to prune associations", e)
            }
        }
}
