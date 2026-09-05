// 使用者用詞頻次服務 — 紀錄每個 displayed word 被選用的次數,
// 餵給 Rust ranking pipeline(RustEngineBridge.processCandidates)做頻率加權。
// 屬 user_data SQLite,平台側保留(設計如此,不進 Rust;見 feedback_user_data_sqlite_stays_native)。
// 由 CompositionRoot 持有。

package com.siansiansu.taigikeyboard.ime.text.composing

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import android.database.sqlite.SQLiteStatement
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.dictionary.FrequencyData
import com.siansiansu.taigikeyboard.ime.dictionary.FrequencyRow
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
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
 * User-word frequency service. Records how often each displayed word is
 * picked so the engine ranking pipeline (`RustEngineBridge.processCandidates`)
 * can boost them. Owned by `CompositionRoot`.
 *
 * File layout: Constants · Schema · Properties · Init ·
 * Public API (Recording / Queries / Mutations) · Pruning · DatabaseHelper.
 */
class UserFrequencyService(
    appContext: Context,
    private val logger: LoggerBackend,
) {
    private val appContext: Context = appContext.applicationContext

    // ------------------------------------------------------------------ //
    // Constants
    // ------------------------------------------------------------------ //

    companion object {
        private const val TAG = "UserFrequencyService"
        private const val DATABASE_NAME = "user_frequency.db"
        // v3.6.1 R5: bumped 1 → 2 for the `(word, tl)` pair-key migration
        // (Core Principle #7). `onUpgrade` rebuilds the table.
        private const val DATABASE_VERSION = 2
        private const val MAX_ENTRIES = 20_000
        private const val PRUNE_CHECK_INTERVAL = 100
        private const val PRUNE_BATCH_SIZE = 2_000
    }

    // ------------------------------------------------------------------ //
    // Schema — table & column names
    // ------------------------------------------------------------------ //

    private object Table {
        const val NAME = "user_frequency"
        const val ID = "id"
        const val WORD = "word"
        // R5 (#7): canonical-TL reading. The identity is `(word, tl)`, so
        // 一字多音 (重/tîng vs 重/tāng) keep separate rows. `tl == ''` is the
        // legacy fallback bucket (pre-R5 rows / old-backup import).
        const val TL = "tl"
        const val COUNT = "count"
        const val LAST_USED = "last_used"
        const val CREATED_AT = "created_at"
    }

    private object MetadataTable {
        const val NAME = "metadata"
        const val KEY = "key"
        const val VALUE = "value"
    }

    // ------------------------------------------------------------------ //
    // Properties
    // ------------------------------------------------------------------ //

    @Volatile
    private var dbHelper: DatabaseHelper? = null
    private val initMutex = Mutex()
    private val recordCounter = AtomicInteger(0)

    // ------------------------------------------------------------------ //
    // Init
    // ------------------------------------------------------------------ //

    /**
     * Open the underlying SQLite connection and create the schema if absent.
     *
     * Promoted to public so the IME Application can warm the DB at boot
     * (mirrors iOS `setupCoreServices`'s fire-and-forget
     * `userFrequencyService.ensureInitialized()` call). Without this
     * warmup, the Continuous-input fetch path never lazy-inits the freq
     * DB itself — so `user_frequency.db` would stay closed until the
     * user committed something, and persisted boost would be ignored
     * for the entire first burst of compositions.
     *
     * Construction of `DatabaseHelper` is cheap (no DB I/O); the actual
     * `onCreate` schema run is deferred until first `readableDatabase`
     * access, so this method explicitly touches `readableDatabase` to force
     * the schema creation. Idempotent — re-entry returns immediately once
     * the connection is open.
     *
     * Failures propagate; the warmup site in `TaigiKeyboardApplication`
     * catches + logs them so a transient DB I/O error never aborts the IME
     * boot path.
     */
    // 強制打開 SQLite 連線並執行 schema 建立 — 公開後,IME Application onCreate 可提前 warm-up,
    // 鏡射 iOS setupCoreServices 的 fire-and-forget pattern。
    // 純 DatabaseHelper() 是 cheap 的,實際 DDL 必須觸發 readableDatabase 才會跑,所以這裡主動讀一次。
    suspend fun ensureInitialized() {
        if (dbHelper != null) return

        initMutex.withLock {
            if (dbHelper != null) return

            withContext(Dispatchers.IO) {
                val helper = DatabaseHelper(appContext, logger)
                // Force `SQLiteOpenHelper.onCreate` to run on this thread
                // before we publish the reference — any reader that sees
                // `dbHelper != null` is guaranteed an open + schema'd
                // connection via the `@Volatile` happens-before edge.
                helper.readableDatabase
                dbHelper = helper

                if (BuildConfig.DEBUG) {
                    logDatabaseInfo()
                }
            }
        }
    }

    /**
     * Synchronous probe — has [ensureInitialized] (or any DB-touching entry
     * point) completed at least once in this process? Cheap (`@Volatile`
     * read), no I/O, no locks. The Continuous-input fetch path uses this
     * as a cold-start gate to skip the user-frequency phase-2 query before
     * any DB connection is open. Mirrors iOS
     * `UserFrequencyService.isConnected()`.
     */
    // 同步探測 DB 是否已打開過(cold-start gate)。@Volatile 單讀,免鎖無 I/O。
    fun isConnected(): Boolean = dbHelper != null

    private fun logDatabaseInfo() {
        try {
            val db = dbHelper?.readableDatabase ?: return

            val countCursor =
                db.rawQuery(
                    "SELECT COUNT(*) FROM ${Table.NAME}",
                    null,
                )
            val recordCount =
                countCursor.use {
                    if (it.moveToFirst()) it.getInt(0) else 0
                }

            val metadataCursor =
                db.rawQuery(
                    "SELECT ${MetadataTable.KEY}, ${MetadataTable.VALUE} FROM ${MetadataTable.NAME}",
                    null,
                )
            val metadata = mutableMapOf<String, String>()
            metadataCursor.use {
                while (it.moveToNext()) {
                    val key = it.getString(0)
                    val value = it.getString(1)
                    metadata[key] = value
                }
            }

            logger.i(TAG, "[INIT] Database initialized successfully")
            logger.i(TAG, "[INIT] User frequency records: $recordCount")
            logger.i(TAG, "[INIT] Schema version: ${metadata["schema_version"]}")
            logger.i(TAG, "[INIT] App version: ${metadata["app_version"]}")
        } catch (e: Exception) {
            logger.w(TAG, "[INIT] Failed to log database info", e)
        }
    }

    // ------------------------------------------------------------------ //
    // Public API — Recording
    // ------------------------------------------------------------------ //

    /**
     * Record a usage of [word] with its canonical-TL reading [tl]. R5 (#7):
     * `(word, tl)` is the identity, so 一字多音 increment separate buckets.
     * Pass `""` only when the candidate has no canonical TL (wire skew /
     * TPS-OOV) → the legacy fallback bucket.
     */
    @Suppress("SqlResolve")
    suspend fun recordUsage(
        word: String,
        tl: String,
    ) = withContext(Dispatchers.IO) {
        try {
            ensureInitialized()
            val db = dbHelper?.writableDatabase ?: return@withContext

            val sql =
                """
                INSERT INTO ${Table.NAME} (${Table.WORD}, ${Table.TL}, ${Table.COUNT}, ${Table.LAST_USED})
                VALUES (?, ?, 1, CURRENT_TIMESTAMP)
                ON CONFLICT(${Table.WORD}, ${Table.TL}) DO UPDATE SET
                    ${Table.COUNT} = ${Table.COUNT} + 1,
                    ${Table.LAST_USED} = CURRENT_TIMESTAMP
                """.trimIndent()

            db.execSQL(sql, arrayOf(word, tl))

            logger.debug(TAG) { "[RECORD] Recorded usage for: $word / $tl" }

            if (recordCounter.incrementAndGet() >= PRUNE_CHECK_INTERVAL) {
                recordCounter.set(0)
                pruneOldEntries()
            }
        } catch (e: Exception) {
            // Fire-and-forget: a failed frequency write must never block
            // typing. Single boundary — execSQL throws the real
            // SQLiteException, logged once here. LoggerBackend gates all levels
            // on BuildConfig.DEBUG (release no-op).
            logger.e(TAG, "frequency.record.failed word=$word tl=$tl", e)
        }
    }

    // ------------------------------------------------------------------ //
    // Public API — Queries
    // ------------------------------------------------------------------ //

    /** Current count for [word], or 0 if unknown. */
    suspend fun frequency(word: String): Int =
        withContext(Dispatchers.IO) {
            frequencyData(word).count
        }

    /** Frequency + last-used for [word]. Returns [FrequencyData.EMPTY] on error / missing row. */
    suspend fun frequencyData(word: String): FrequencyData =
        withContext(Dispatchers.IO) {
            try {
                ensureInitialized()
                val db = dbHelper?.readableDatabase ?: return@withContext FrequencyData.EMPTY

                // R5: a word may span several `(word, tl)` rows. This
                // single-word accessor (UI count / compat) aggregates them:
                // total count + most-recent last_used. The pair-keyed
                // ranking path uses `frequencyDataBatch` (per-reading).
                val cursor =
                    db.rawQuery(
                        """
                        SELECT SUM(${Table.COUNT}), MAX(strftime('%s', ${Table.LAST_USED}) * 1000)
                        FROM ${Table.NAME}
                        WHERE ${Table.WORD} = ?
                        """.trimIndent(),
                        arrayOf(word),
                    )

                cursor.use {
                    // SUM/MAX over zero rows yields one all-NULL row → treat
                    // as EMPTY; a real hit has a non-null count.
                    if (it.moveToFirst() && !it.isNull(0)) {
                        val count = it.getInt(0)
                        val lastUsedMillis = it.getLong(1)
                        return@withContext FrequencyData(count, lastUsedMillis)
                    }
                }

                FrequencyData.EMPTY
            } catch (e: Exception) {
                logger.e(TAG, "[QUERY] Failed to get frequency data for: $word", e)
                FrequencyData.EMPTY
            }
        }

    /**
     * Batch lookup — R5 returns one ROW per `(word, tl)` reading for every
     * [words] entry in the DB (a word may yield several: each learned
     * reading + the legacy `tl == ''` bucket), so the engine can build its
     * `(display_text, canonical_tl)` pair-keyed `FrequencyMap`. Keyed on
     * `word` only (`WHERE word IN`); the caller dedupes the query keys by
     * display text.
     */
    suspend fun frequencyDataBatch(words: List<String>): List<FrequencyRow> =
        withContext(Dispatchers.IO) {
            if (words.isEmpty()) return@withContext emptyList()

            try {
                ensureInitialized()
                val db = dbHelper?.readableDatabase ?: return@withContext emptyList()

                val rows = mutableListOf<FrequencyRow>()
                val placeholders = words.joinToString(",") { "?" }

                val cursor =
                    db.rawQuery(
                        """
                        SELECT ${Table.WORD}, ${Table.TL}, ${Table.COUNT}, strftime('%s', ${Table.LAST_USED}) * 1000
                        FROM ${Table.NAME}
                        WHERE ${Table.WORD} IN ($placeholders)
                        """.trimIndent(),
                        words.toTypedArray(),
                    )

                cursor.use {
                    while (it.moveToNext()) {
                        val word = it.getString(0)
                        val tl = it.getString(1) ?: ""
                        val count = it.getInt(2)
                        val lastUsedMillis = it.getLong(3)
                        rows.add(FrequencyRow(word, tl, FrequencyData(count, lastUsedMillis)))
                    }
                }

                rows
            } catch (e: Exception) {
                logger.e(TAG, "[QUERY] Failed to get frequency data batch", e)
                emptyList()
            }
        }

    /**
     * All `(word, tl, count)` rows, one per learned reading + any legacy
     * `tl == ''` row. Preserves the R5 per-reading identity (#7). Shared by all
     * three consumers — the `.taigi` backup export, the 詞頻 management viewer
     * (which lists + deletes per `(word, tl)`), AND the hand-editable CSV
     * export — all per-reading. Do NOT add viewer-only SQL (limit / filter)
     * here — it would leak into backup; split a wrapper if their needs diverge.
     * Deterministic tie-break `(word, tl)` keeps equal count/time rows stable.
     */
    suspend fun getAllFrequencyRows(): List<Triple<String, String, Int>> =
        withContext(Dispatchers.IO) {
            try {
                ensureInitialized()
                val db = dbHelper?.readableDatabase ?: return@withContext emptyList()

                val cursor =
                    db.rawQuery(
                        """
                        SELECT ${Table.WORD}, ${Table.TL}, ${Table.COUNT}
                        FROM ${Table.NAME}
                        ORDER BY ${Table.COUNT} DESC, ${Table.LAST_USED} DESC, ${Table.WORD} ASC, ${Table.TL} ASC
                        """.trimIndent(),
                        null,
                    )

                val rows = mutableListOf<Triple<String, String, Int>>()
                cursor.use {
                    while (it.moveToNext()) {
                        rows.add(Triple(it.getString(0), it.getString(1) ?: "", it.getInt(2)))
                    }
                }
                rows
            } catch (e: Exception) {
                logger.e(TAG, "[QUERY] Failed to get all frequency rows", e)
                emptyList()
            }
        }

    /** Returns total entry count, or -1 if DB is not open. */
    fun totalCount(): Int {
        return try {
            val db = dbHelper?.readableDatabase ?: return -1
            val cursor = db.rawQuery("SELECT COUNT(*) FROM ${Table.NAME}", null)
            cursor.use { if (it.moveToFirst()) it.getInt(0) else -1 }
        } catch (_: Exception) {
            -1
        }
    }

    // ------------------------------------------------------------------ //
    // Public API — Mutations
    // ------------------------------------------------------------------ //

    /**
     * Batch-import frequency entries `(word, tl, count)`, merging by
     * max(existing, incoming). R5: keyed on the `(word, tl)` pair so each
     * reading merges independently; a pre-R5 backup row imports with
     * `tl == ""` (the legacy fallback bucket). Returns imported count.
     */
    @Suppress("SqlResolve")
    suspend fun batchImportMerge(entries: List<Triple<String, String, Int>>): Int =
        withContext(Dispatchers.IO) {
            try {
                ensureInitialized()
                val db = dbHelper?.writableDatabase ?: return@withContext 0

                val sql =
                    """
                    INSERT INTO ${Table.NAME} (${Table.WORD}, ${Table.TL}, ${Table.COUNT}, ${Table.LAST_USED})
                    VALUES (?, ?, ?, datetime('now'))
                    ON CONFLICT(${Table.WORD}, ${Table.TL}) DO UPDATE SET
                        ${Table.COUNT} = MAX(${Table.COUNT}, excluded.${Table.COUNT}),
                        ${Table.LAST_USED} = datetime('now')
                    """.trimIndent()

                db.beginTransaction()
                var imported = 0
                try {
                    val stmt = db.compileStatement(sql)
                    for ((word, tl, count) in entries) {
                        stmt.bindArgs(word, tl, count.toLong())
                        stmt.executeInsert()
                        imported++
                    }
                    db.setTransactionSuccessful()
                } finally {
                    db.endTransaction()
                }
                imported
            } catch (e: Exception) {
                logger.e(TAG, "[BATCH_IMPORT] Failed", e)
                0
            }
        }

    /**
     * Delete a single `(word, tl)` reading. R5 (#7): identity is the pair, so
     * 一字多音 (重/tāng vs 重/tîng) delete independently. Deleting the legacy
     * `tl == ''` row removes only the fallback bucket; re-learned exact rows
     * survive.
     */
    // 刪除單一 (word, tl) 讀音 (#7);一字多音各自獨立刪。legacy '' 列只移除 fallback 桶。
    suspend fun deleteWord(
        word: String,
        tl: String,
    ) = withContext(Dispatchers.IO) {
        ensureInitialized()
        val db = dbHelper?.writableDatabase ?: return@withContext
        db.delete(Table.NAME, "${Table.WORD} = ? AND ${Table.TL} = ?", arrayOf(word, tl))
    }

    /** Close handle and delete database file (debug / reset). */
    suspend fun deleteDatabase() =
        withContext(Dispatchers.IO) {
            try {
                dbHelper?.close()
                dbHelper = null

                val dbFile = appContext.getDatabasePath(DATABASE_NAME)
                if (dbFile.exists()) {
                    dbFile.delete()
                    logger.i(TAG, "[DELETE] Database deleted successfully")
                }
            } catch (e: Exception) {
                logger.e(TAG, "[DELETE] Failed to delete database", e)
            }
        }

    // ------------------------------------------------------------------ //
    // Pruning
    // ------------------------------------------------------------------ //

    private fun pruneOldEntries() {
        try {
            val db = dbHelper?.writableDatabase ?: return

            val cursor = db.rawQuery("SELECT COUNT(*) FROM ${Table.NAME}", null)
            val currentCount =
                cursor.use {
                    if (it.moveToFirst()) it.getInt(0) else 0
                }

            if (currentCount <= MAX_ENTRIES) return

            val deleteCount = minOf(PRUNE_BATCH_SIZE, currentCount - MAX_ENTRIES + PRUNE_BATCH_SIZE)

            db.execSQL(
                """
                DELETE FROM ${Table.NAME}
                WHERE ${Table.ID} IN (
                    SELECT ${Table.ID} FROM ${Table.NAME}
                    ORDER BY ${Table.COUNT} ASC, ${Table.LAST_USED} ASC
                    LIMIT ?
                )
                """.trimIndent(),
                arrayOf(deleteCount.toString()),
            )

            logger.i(TAG, "[PRUNE] Deleted $deleteCount frequency entries (was $currentCount)")
        } catch (e: Exception) {
            logger.e(TAG, "[PRUNE] Failed to prune frequency entries", e)
        }
    }

    // ------------------------------------------------------------------ //
    // DatabaseHelper
    // ------------------------------------------------------------------ //

    private class DatabaseHelper(
        private val context: Context,
        private val logger: LoggerBackend,
    ) : SQLiteOpenHelper(
            context,
            DATABASE_NAME,
            null,
            DATABASE_VERSION,
        ) {
        override fun onCreate(db: SQLiteDatabase) {
            createUserFrequencyTable(db)
            createUserFrequencyIndexes(db)
            createMetadataTable(db)
            insertMetadata(db, context)

            logger.i(TAG, "[CREATE] Database tables created successfully")
        }

        private fun createUserFrequencyTable(db: SQLiteDatabase) {
            // R5 (#7): identity is `UNIQUE(word, tl)` — 一字多音 keep separate
            // rows. `tl` defaults to '' (the legacy fallback bucket).
            db.execSQL(
                """
                CREATE TABLE ${Table.NAME} (
                    ${Table.ID} INTEGER PRIMARY KEY AUTOINCREMENT,
                    ${Table.WORD} TEXT NOT NULL,
                    ${Table.TL} TEXT NOT NULL DEFAULT '',
                    ${Table.COUNT} INTEGER DEFAULT 1,
                    ${Table.LAST_USED} TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    ${Table.CREATED_AT} TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE(${Table.WORD}, ${Table.TL})
                );
                """.trimIndent(),
            )
        }

        private fun createUserFrequencyIndexes(db: SQLiteDatabase) {
            // No standalone idx_word: the UNIQUE(word, tl) autoindex has
            // `word` leftmost, so it serves `WHERE word = ?` / `IN (...)`.
            db.execSQL("CREATE INDEX IF NOT EXISTS idx_count ON ${Table.NAME}(${Table.COUNT} DESC);")
            db.execSQL("CREATE INDEX IF NOT EXISTS idx_last_used ON ${Table.NAME}(${Table.LAST_USED} DESC);")
        }

        private fun createMetadataTable(db: SQLiteDatabase) {
            db.execSQL(
                """
                CREATE TABLE IF NOT EXISTS ${MetadataTable.NAME} (
                    ${MetadataTable.KEY} TEXT PRIMARY KEY,
                    ${MetadataTable.VALUE} TEXT
                );
                """.trimIndent(),
            )
        }

        private fun insertMetadata(
            db: SQLiteDatabase,
            context: Context,
        ) {
            try {
                val appVersion =
                    context.packageManager
                        .getPackageInfo(
                            context.packageName,
                            0,
                        ).versionName

                db.execSQL(
                    "INSERT OR REPLACE INTO ${MetadataTable.NAME} (${MetadataTable.KEY}, ${MetadataTable.VALUE}) VALUES (?, ?)",
                    arrayOf("schema_version", DATABASE_VERSION.toString()),
                )
                db.execSQL(
                    "INSERT OR REPLACE INTO ${MetadataTable.NAME} (${MetadataTable.KEY}, ${MetadataTable.VALUE}) VALUES (?, ?)",
                    arrayOf("app_version", appVersion),
                )
                db.execSQL(
                    "INSERT OR REPLACE INTO ${MetadataTable.NAME} (${MetadataTable.KEY}, ${MetadataTable.VALUE}) VALUES (?, datetime('now'))",
                    arrayOf("created_date"),
                )
                db.execSQL(
                    "INSERT OR REPLACE INTO ${MetadataTable.NAME} (${MetadataTable.KEY}, ${MetadataTable.VALUE}) VALUES (?, datetime('now'))",
                    arrayOf("last_modified"),
                )
            } catch (e: Exception) {
                logger.w(TAG, "[METADATA] Failed to insert metadata", e)
            }
        }

        override fun onUpgrade(
            db: SQLiteDatabase,
            oldVersion: Int,
            newVersion: Int,
        ) {
            if (oldVersion < 2) {
                migrateToPairKey(db)
            }
            logger.i(TAG, "[UPGRADE] Database upgraded from $oldVersion to $newVersion (data preserved)")
        }

        /**
         * R5 (#7): rebuild the pre-R5 table (inline `word UNIQUE`, no `tl`)
         * into the `(word, tl)` pair-key shape. SQLite cannot drop an inline
         * column UNIQUE via `ALTER`, so this does create-new / copy / drop /
         * rename. `SQLiteOpenHelper` already wraps `onUpgrade` in a
         * transaction, so a throw here rolls the whole rebuild back. Existing
         * rows backfill `tl = ''` (the legacy fallback bucket); `id` /
         * `count` / `last_used` / `created_at` are preserved exactly. Mirrors
         * iOS `UserFrequencySchema.migrateToPairKeyIfNeeded`.
         */
        // R5 — 把舊表(inline word UNIQUE、無 tl)重建成 (word, tl) pair-key。
        // inline UNIQUE 無法 ALTER 掉 → create-new/copy/drop/rename;onUpgrade 已在 transaction 內,throw 會回滾。
        private fun migrateToPairKey(db: SQLiteDatabase) {
            val newTable = "${Table.NAME}_pairkey_migrate"
            db.execSQL(
                """
                CREATE TABLE $newTable (
                    ${Table.ID} INTEGER PRIMARY KEY AUTOINCREMENT,
                    ${Table.WORD} TEXT NOT NULL,
                    ${Table.TL} TEXT NOT NULL DEFAULT '',
                    ${Table.COUNT} INTEGER DEFAULT 1,
                    ${Table.LAST_USED} TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    ${Table.CREATED_AT} TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE(${Table.WORD}, ${Table.TL})
                );
                """.trimIndent(),
            )
            db.execSQL(
                """
                INSERT INTO $newTable (${Table.ID}, ${Table.WORD}, ${Table.TL}, ${Table.COUNT}, ${Table.LAST_USED}, ${Table.CREATED_AT})
                SELECT ${Table.ID}, ${Table.WORD}, '', ${Table.COUNT}, ${Table.LAST_USED}, ${Table.CREATED_AT} FROM ${Table.NAME};
                """.trimIndent(),
            )
            db.execSQL("DROP TABLE ${Table.NAME};")
            db.execSQL("ALTER TABLE $newTable RENAME TO ${Table.NAME};")
            // Old installs created idx_word; drop it so the schema converges
            // to one shape across fresh + upgraded DBs.
            db.execSQL("DROP INDEX IF EXISTS idx_word;")
            createUserFrequencyIndexes(db)
        }
    }
}
