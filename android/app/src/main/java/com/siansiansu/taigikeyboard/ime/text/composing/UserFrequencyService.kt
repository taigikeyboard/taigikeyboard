package com.siansiansu.taigikeyboard.ime.text.composing

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import android.database.sqlite.SQLiteStatement
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.dictionary.FrequencyData
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
        private const val DATABASE_VERSION = 1
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

    private var dbHelper: DatabaseHelper? = null
    private val initMutex = Mutex()
    private var isInitialized = false
    private val recordCounter = AtomicInteger(0)

    // ------------------------------------------------------------------ //
    // Init
    // ------------------------------------------------------------------ //

    /** Lazy database initialization. Called internally by every DB-touching entry point. */
    private suspend fun initialize() {
        if (isInitialized) return

        initMutex.withLock {
            if (isInitialized) return

            dbHelper = DatabaseHelper(appContext, logger)
            isInitialized = true

            if (BuildConfig.DEBUG) {
                logDatabaseInfo()
            }
        }
    }

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

    /** Record a usage of [word]. Increments count and refreshes last-used timestamp. */
    @Suppress("SqlResolve")
    suspend fun recordUsage(word: String) =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.writableDatabase ?: return@withContext

                val sql =
                    """
                    INSERT INTO ${Table.NAME} (${Table.WORD}, ${Table.COUNT}, ${Table.LAST_USED})
                    VALUES (?, 1, CURRENT_TIMESTAMP)
                    ON CONFLICT(${Table.WORD}) DO UPDATE SET
                        ${Table.COUNT} = ${Table.COUNT} + 1,
                        ${Table.LAST_USED} = CURRENT_TIMESTAMP
                    """.trimIndent()

                db.execSQL(sql, arrayOf(word))

                logger.debug(TAG) { "[RECORD] Recorded usage for: $word" }

                if (recordCounter.incrementAndGet() >= PRUNE_CHECK_INTERVAL) {
                    recordCounter.set(0)
                    pruneOldEntries()
                }
            } catch (e: Exception) {
                logger.e(TAG, "[RECORD] Failed to record usage for: $word", e)
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
                initialize()
                val db = dbHelper?.readableDatabase ?: return@withContext FrequencyData.EMPTY

                val cursor =
                    db.rawQuery(
                        """
                        SELECT ${Table.COUNT}, strftime('%s', ${Table.LAST_USED}) * 1000
                        FROM ${Table.NAME}
                        WHERE ${Table.WORD} = ?
                        """.trimIndent(),
                        arrayOf(word),
                    )

                cursor.use {
                    if (it.moveToFirst()) {
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

    /** Batch lookup: returns a map for every [words] entry that exists in the DB. */
    suspend fun frequencyDataBatch(words: List<String>): Map<String, FrequencyData> =
        withContext(Dispatchers.IO) {
            if (words.isEmpty()) return@withContext emptyMap()

            try {
                initialize()
                val db = dbHelper?.readableDatabase ?: return@withContext emptyMap()

                val result = mutableMapOf<String, FrequencyData>()
                val placeholders = words.joinToString(",") { "?" }

                val cursor =
                    db.rawQuery(
                        """
                        SELECT ${Table.WORD}, ${Table.COUNT}, strftime('%s', ${Table.LAST_USED}) * 1000
                        FROM ${Table.NAME}
                        WHERE ${Table.WORD} IN ($placeholders)
                        """.trimIndent(),
                        words.toTypedArray(),
                    )

                cursor.use {
                    while (it.moveToNext()) {
                        val word = it.getString(0)
                        val count = it.getInt(1)
                        val lastUsedMillis = it.getLong(2)
                        result[word] = FrequencyData(count, lastUsedMillis)
                    }
                }

                result
            } catch (e: Exception) {
                logger.e(TAG, "[QUERY] Failed to get frequency data batch", e)
                emptyMap()
            }
        }

    /** Top-[limit] most frequent words, descending by count then recency. */
    suspend fun topWords(limit: Int = 100): List<Pair<String, Int>> =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.readableDatabase ?: return@withContext emptyList()

                val cursor =
                    db.rawQuery(
                        """
                        SELECT ${Table.WORD}, ${Table.COUNT}
                        FROM ${Table.NAME}
                        ORDER BY ${Table.COUNT} DESC, ${Table.LAST_USED} DESC
                        LIMIT ?
                        """.trimIndent(),
                        arrayOf(limit.toString()),
                    )

                collectWordCountPairs(cursor)
            } catch (e: Exception) {
                logger.e(TAG, "[QUERY] Failed to get top words", e)
                emptyList()
            }
        }

    /** All frequency rows (ordered by count DESC, then last-used DESC). */
    suspend fun getAllFrequencies(): List<Pair<String, Int>> =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.readableDatabase ?: return@withContext emptyList()

                val cursor =
                    db.rawQuery(
                        """
                        SELECT ${Table.WORD}, ${Table.COUNT}
                        FROM ${Table.NAME}
                        ORDER BY ${Table.COUNT} DESC, ${Table.LAST_USED} DESC
                        """.trimIndent(),
                        null,
                    )

                collectWordCountPairs(cursor)
            } catch (e: Exception) {
                logger.e(TAG, "[QUERY] Failed to get all frequencies", e)
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

    private fun collectWordCountPairs(cursor: android.database.Cursor): List<Pair<String, Int>> {
        val results = mutableListOf<Pair<String, Int>>()
        cursor.use {
            while (it.moveToNext()) {
                val word = it.getString(0)
                val count = it.getInt(1)
                results.add(word to count)
            }
        }
        return results
    }

    // ------------------------------------------------------------------ //
    // Public API — Mutations
    // ------------------------------------------------------------------ //

    /** Batch-import frequency entries, merging by max(existing, incoming). Returns imported count. */
    @Suppress("SqlResolve")
    suspend fun batchImportMerge(entries: List<Pair<String, Int>>): Int =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.writableDatabase ?: return@withContext 0

                val sql =
                    """
                    INSERT INTO ${Table.NAME} (${Table.WORD}, ${Table.COUNT}, ${Table.LAST_USED})
                    VALUES (?, ?, datetime('now'))
                    ON CONFLICT(${Table.WORD}) DO UPDATE SET
                        ${Table.COUNT} = MAX(${Table.COUNT}, excluded.${Table.COUNT}),
                        ${Table.LAST_USED} = datetime('now')
                    """.trimIndent()

                db.beginTransaction()
                var imported = 0
                try {
                    val stmt = db.compileStatement(sql)
                    for ((word, count) in entries) {
                        stmt.bindArgs(word, count.toLong())
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

    /** Delete a single word from frequency data. */
    suspend fun deleteWord(word: String) =
        withContext(Dispatchers.IO) {
            initialize()
            val db = dbHelper?.writableDatabase ?: return@withContext
            db.delete(Table.NAME, "${Table.WORD} = ?", arrayOf(word))
        }

    /** Clear all frequency rows (debug / reset). */
    suspend fun clearAllFrequencies() =
        withContext(Dispatchers.IO) {
            try {
                initialize()
                val db = dbHelper?.writableDatabase ?: return@withContext

                db.execSQL("DELETE FROM ${Table.NAME}")

                logger.i(TAG, "[CLEAR] All frequencies cleared")
            } catch (e: Exception) {
                logger.e(TAG, "[CLEAR] Failed to clear frequencies", e)
            }
        }

    /** Close handle and delete database file (debug / reset). */
    suspend fun deleteDatabase() =
        withContext(Dispatchers.IO) {
            try {
                dbHelper?.close()
                dbHelper = null
                isInitialized = false

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
            db.execSQL(
                """
                CREATE TABLE ${Table.NAME} (
                    ${Table.ID} INTEGER PRIMARY KEY AUTOINCREMENT,
                    ${Table.WORD} TEXT NOT NULL UNIQUE,
                    ${Table.COUNT} INTEGER DEFAULT 1,
                    ${Table.LAST_USED} TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    ${Table.CREATED_AT} TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );
                """.trimIndent(),
            )
        }

        private fun createUserFrequencyIndexes(db: SQLiteDatabase) {
            db.execSQL("CREATE INDEX idx_word ON ${Table.NAME}(${Table.WORD});")
            db.execSQL("CREATE INDEX idx_count ON ${Table.NAME}(${Table.COUNT} DESC);")
            db.execSQL("CREATE INDEX idx_last_used ON ${Table.NAME}(${Table.LAST_USED} DESC);")
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
                    arrayOf("schema_version", "1"),
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
            logger.i(TAG, "[UPGRADE] Database upgraded from $oldVersion to $newVersion (data preserved)")
        }
    }
}
