package com.siansiansu.taigikeyboard.ime.text.composing

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File

/**
 * 使用者詞彙頻率服務
 *
 * 負責：
 * - 記錄使用者選擇詞彙的頻率
 * - 查詢詞彙使用頻率用於候選詞排序
 * - SQLite 持久化儲存
 */
object UserFrequencyService {
    private const val TAG = "UserFrequencyService"
    private const val DATABASE_NAME = "user_frequency.db"
    private const val DATABASE_VERSION = 1
    private const val MAX_ENTRIES = 20_000
    private const val PRUNE_CHECK_INTERVAL = 100
    private const val PRUNE_BATCH_SIZE = 2_000

    // 表名稱與欄位
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

    private var appContext: Context? = null
    private var dbHelper: DatabaseHelper? = null
    private val initMutex = Mutex()
    private var isInitialized = false
    @Volatile
    private var recordCounter = 0

    /**
     * 初始化服務（建議在 Application.onCreate 中呼叫）
     */
    fun init(context: Context) {
        appContext = context.applicationContext
    }

    /**
     * 初始化資料庫（延遲初始化）
     */
    private suspend fun initialize() {
        if (isInitialized) return

        initMutex.withLock {
            if (isInitialized) return

            val context = appContext ?: throw IllegalStateException(
                "UserFrequencyService 尚未初始化，請在 Application.onCreate 中呼叫 init(context)"
            )

            dbHelper = DatabaseHelper(context)
            isInitialized = true

            if (BuildConfig.DEBUG) {
                logDatabaseInfo()
            }
        }
    }

    /**
     * 記錄資料庫資訊（僅 DEBUG 模式）
     */
    private fun logDatabaseInfo() {
        try {
            val db = dbHelper?.readableDatabase ?: return

            // 檢查使用者頻率資料筆數
            val countCursor = db.rawQuery(
                "SELECT COUNT(*) FROM ${Table.NAME}",
                null
            )
            val recordCount = countCursor.use {
                if (it.moveToFirst()) it.getInt(0) else 0
            }

            // 讀取 metadata
            val metadataCursor = db.rawQuery(
                "SELECT ${MetadataTable.KEY}, ${MetadataTable.VALUE} FROM ${MetadataTable.NAME}",
                null
            )
            val metadata = mutableMapOf<String, String>()
            metadataCursor.use {
                while (it.moveToNext()) {
                    val key = it.getString(0)
                    val value = it.getString(1)
                    metadata[key] = value
                }
            }

            Log.i(TAG, "[INIT] Database initialized successfully")
            Log.i(TAG, "[INIT] User frequency records: $recordCount")
            Log.i(TAG, "[INIT] Schema version: ${metadata["schema_version"]}")
            Log.i(TAG, "[INIT] App version: ${metadata["app_version"]}")
        } catch (e: Exception) {
            Log.w(TAG, "[INIT] Failed to log database info", e)
        }
    }

    /**
     * 記錄詞彙使用
     */
    suspend fun recordUsage(word: String) = withContext(Dispatchers.IO) {
        try {
            initialize()
            val db = dbHelper?.writableDatabase ?: return@withContext

            val sql = """
                INSERT INTO ${Table.NAME} (${Table.WORD}, ${Table.COUNT}, ${Table.LAST_USED})
                VALUES (?, 1, CURRENT_TIMESTAMP)
                ON CONFLICT(${Table.WORD}) DO UPDATE SET
                    ${Table.COUNT} = ${Table.COUNT} + 1,
                    ${Table.LAST_USED} = CURRENT_TIMESTAMP;
            """.trimIndent()

            db.execSQL(sql, arrayOf(word))

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[RECORD] Recorded usage for: $word")
            }

            recordCounter++
            if (recordCounter >= PRUNE_CHECK_INTERVAL) {
                recordCounter = 0
                pruneOldEntries()
            }
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[RECORD] Failed to record usage for: $word", e)
            }
        }
    }

    /**
     * Prune least-used entries when exceeding capacity
     */
    private fun pruneOldEntries() {
        try {
            val db = dbHelper?.writableDatabase ?: return

            val cursor = db.rawQuery("SELECT COUNT(*) FROM ${Table.NAME}", null)
            val currentCount = cursor.use {
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
                arrayOf(deleteCount.toString())
            )

            if (BuildConfig.DEBUG) {
                Log.i(TAG, "[PRUNE] Deleted $deleteCount frequency entries (was $currentCount)")
            }
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[PRUNE] Failed to prune frequency entries", e)
            }
        }
    }

    /**
     * 使用者頻率資料（包含頻率和最後使用時間）
     */
    data class FrequencyData(
        val count: Int,
        val lastUsedMillis: Long  // Unix timestamp in milliseconds
    )

    /**
     * 取得詞彙使用頻率
     */
    suspend fun frequency(word: String): Int = withContext(Dispatchers.IO) {
        frequencyData(word).count
    }

    /**
     * 取得詞彙使用頻率資料（包含頻率和最後使用時間）
     */
    suspend fun frequencyData(word: String): FrequencyData = withContext(Dispatchers.IO) {
        try {
            initialize()
            val db = dbHelper?.readableDatabase ?: return@withContext FrequencyData(0, 0)

            val cursor = db.rawQuery(
                """
                SELECT ${Table.COUNT}, strftime('%s', ${Table.LAST_USED}) * 1000
                FROM ${Table.NAME}
                WHERE ${Table.WORD} = ?
                """.trimIndent(),
                arrayOf(word)
            )

            cursor.use {
                if (it.moveToFirst()) {
                    val count = it.getInt(0)
                    val lastUsedMillis = it.getLong(1)
                    return@withContext FrequencyData(count, lastUsedMillis)
                }
            }

            FrequencyData(0, 0)
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[QUERY] Failed to get frequency data for: $word", e)
            }
            FrequencyData(0, 0)
        }
    }

    /**
     * 批次取得多個詞彙的頻率資料
     */
    suspend fun frequencyDataBatch(words: List<String>): Map<String, FrequencyData> = withContext(Dispatchers.IO) {
        if (words.isEmpty()) return@withContext emptyMap()

        try {
            initialize()
            val db = dbHelper?.readableDatabase ?: return@withContext emptyMap()

            val result = mutableMapOf<String, FrequencyData>()
            val placeholders = words.joinToString(",") { "?" }

            val cursor = db.rawQuery(
                """
                SELECT ${Table.WORD}, ${Table.COUNT}, strftime('%s', ${Table.LAST_USED}) * 1000
                FROM ${Table.NAME}
                WHERE ${Table.WORD} IN ($placeholders)
                """.trimIndent(),
                words.toTypedArray()
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
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[QUERY] Failed to get frequency data batch", e)
            }
            emptyMap()
        }
    }

    /**
     * 取得最常用的詞彙
     */
    suspend fun topWords(limit: Int = 100): List<Pair<String, Int>> = withContext(Dispatchers.IO) {
        try {
            initialize()
            val db = dbHelper?.readableDatabase ?: return@withContext emptyList()

            val cursor = db.rawQuery(
                """
                SELECT ${Table.WORD}, ${Table.COUNT}
                FROM ${Table.NAME}
                ORDER BY ${Table.COUNT} DESC, ${Table.LAST_USED} DESC
                LIMIT ?
                """.trimIndent(),
                arrayOf(limit.toString())
            )

            val results = mutableListOf<Pair<String, Int>>()
            cursor.use {
                while (it.moveToNext()) {
                    val word = it.getString(0)
                    val count = it.getInt(1)
                    results.add(word to count)
                }
            }

            results
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[QUERY] Failed to get top words", e)
            }
            emptyList()
        }
    }

    /**
     * 取得所有詞彙頻率（Debug 用）
     */
    suspend fun getAllFrequencies(context: Context): List<Pair<String, Int>> = withContext(Dispatchers.IO) {
        try {
            if (appContext == null) {
                appContext = context.applicationContext
            }
            initialize()
            val db = dbHelper?.readableDatabase ?: return@withContext emptyList()

            val cursor = db.rawQuery(
                """
                SELECT ${Table.WORD}, ${Table.COUNT}
                FROM ${Table.NAME}
                ORDER BY ${Table.COUNT} DESC, ${Table.LAST_USED} DESC
                """.trimIndent(),
                null
            )

            val results = mutableListOf<Pair<String, Int>>()
            cursor.use {
                while (it.moveToNext()) {
                    val word = it.getString(0)
                    val count = it.getInt(1)
                    results.add(word to count)
                }
            }

            results
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[QUERY] Failed to get all frequencies", e)
            }
            emptyList()
        }
    }

    /**
     * 清除所有頻率資料（Debug 用）
     */
    suspend fun clearAllFrequencies(context: Context) = withContext(Dispatchers.IO) {
        try {
            if (appContext == null) {
                appContext = context.applicationContext
            }
            initialize()
            val db = dbHelper?.writableDatabase ?: return@withContext

            db.execSQL("DELETE FROM ${Table.NAME}")

            if (BuildConfig.DEBUG) {
                Log.i(TAG, "[CLEAR] All frequencies cleared")
            }
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[CLEAR] Failed to clear frequencies", e)
            }
        }
    }

    /**
     * 刪除資料庫
     */
    suspend fun deleteDatabase() = withContext(Dispatchers.IO) {
        try {
            dbHelper?.close()
            dbHelper = null
            isInitialized = false

            val context = appContext ?: return@withContext
            val dbFile = context.getDatabasePath(DATABASE_NAME)
            if (dbFile.exists()) {
                dbFile.delete()
                if (BuildConfig.DEBUG) {
                    Log.i(TAG, "[DELETE] Database deleted successfully")
                }
            }
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[DELETE] Failed to delete database", e)
            }
        }
    }

    /**
     * 資料庫輔助類別
     */
    private class DatabaseHelper(private val context: Context) : SQLiteOpenHelper(
        context,
        DATABASE_NAME,
        null,
        DATABASE_VERSION
    ) {
        override fun onCreate(db: SQLiteDatabase) {
            // 建立使用者頻率表
            db.execSQL(
                """
                CREATE TABLE ${Table.NAME} (
                    ${Table.ID} INTEGER PRIMARY KEY AUTOINCREMENT,
                    ${Table.WORD} TEXT NOT NULL UNIQUE,
                    ${Table.COUNT} INTEGER DEFAULT 1,
                    ${Table.LAST_USED} TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    ${Table.CREATED_AT} TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );
                """.trimIndent()
            )

            // 建立索引
            db.execSQL("CREATE INDEX idx_word ON ${Table.NAME}(${Table.WORD});")
            db.execSQL("CREATE INDEX idx_count ON ${Table.NAME}(${Table.COUNT} DESC);")
            db.execSQL("CREATE INDEX idx_last_used ON ${Table.NAME}(${Table.LAST_USED} DESC);")

            // 建立 metadata 表
            db.execSQL(
                """
                CREATE TABLE IF NOT EXISTS ${MetadataTable.NAME} (
                    ${MetadataTable.KEY} TEXT PRIMARY KEY,
                    ${MetadataTable.VALUE} TEXT
                );
                """.trimIndent()
            )

            // 寫入 metadata
            insertMetadata(db, context)

            if (BuildConfig.DEBUG) {
                Log.i(TAG, "[CREATE] Database tables created successfully")
            }
        }

        /**
         * 寫入 metadata 資訊
         */
        private fun insertMetadata(db: SQLiteDatabase, context: Context) {
            try {
                val appVersion = context.packageManager.getPackageInfo(
                    context.packageName,
                    0
                ).versionName

                db.execSQL(
                    "INSERT OR REPLACE INTO ${MetadataTable.NAME} (${MetadataTable.KEY}, ${MetadataTable.VALUE}) VALUES (?, ?)",
                    arrayOf("schema_version", "1")
                )
                db.execSQL(
                    "INSERT OR REPLACE INTO ${MetadataTable.NAME} (${MetadataTable.KEY}, ${MetadataTable.VALUE}) VALUES (?, ?)",
                    arrayOf("app_version", appVersion)
                )
                db.execSQL(
                    "INSERT OR REPLACE INTO ${MetadataTable.NAME} (${MetadataTable.KEY}, ${MetadataTable.VALUE}) VALUES (?, datetime('now'))",
                    arrayOf("created_date")
                )
                db.execSQL(
                    "INSERT OR REPLACE INTO ${MetadataTable.NAME} (${MetadataTable.KEY}, ${MetadataTable.VALUE}) VALUES (?, datetime('now'))",
                    arrayOf("last_modified")
                )
            } catch (e: Exception) {
                // metadata 寫入失敗不影響主要功能
                if (BuildConfig.DEBUG) {
                    Log.w(TAG, "[METADATA] Failed to insert metadata", e)
                }
            }
        }

        override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) {
            // 漸進式升級策略：保留使用者資料
            // 未來如需 schema 變更，在此處新增對應版本的 ALTER TABLE 語句

            // 範例：
            // var currentVersion = oldVersion
            // if (currentVersion == 1) {
            //     // 執行 v1 -> v2 的 schema 變更
            //     // db.execSQL("ALTER TABLE ${Table.NAME} ADD COLUMN new_field TEXT")
            //     currentVersion = 2
            // }

            if (BuildConfig.DEBUG) {
                Log.i(TAG, "[UPGRADE] Database upgraded from $oldVersion to $newVersion (data preserved)")
            }
        }
    }
}
