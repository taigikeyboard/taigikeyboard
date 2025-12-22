package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File
import kotlin.math.exp

/**
 * NextWord 下一詞預測服務
 *
 * 使用「相鄰字 Bigram」模型預測下一個字：
 * - 選擇「早安」→ 用「安」查詢 → 預測下一個字
 *
 * 資料來源：
 * - dictionary.db (word_association 表): 字典關聯（由 LexiconService 複製）
 * - user_association.db: 使用者學習（永久保留）
 *
 * Thread-safe singleton with lazy initialization
 */
object NextWordService {
    private const val TAG = "NextWordService"
    private const val DICT_DB_NAME = "dictionary.db"
    private const val USER_DB_NAME = "user_association.db"
    private const val DEFAULT_LIMIT = 30

    // 使用者來源權重（相對於字典來源）
    // USER_WEIGHT 較高，讓使用者學習的關聯更有競爭力
    private const val USER_WEIGHT = 50
    private const val DICT_WEIGHT = 1

    // 時間衰減參數（參考 RIME 的指數衰減公式）
    // 半衰期：168 小時（一週），超過一週的關聯權重減半
    private const val DECAY_HALF_LIFE_HOURS = 168.0

    // 使用者關聯數量上限（防止資料庫無限增長）
    private const val MAX_USER_ASSOCIATIONS = 50_000
    // 每 N 次記錄後檢查是否需要清理
    private const val PRUNE_CHECK_INTERVAL = 100
    // 超過上限時，刪除最低分的 N 筆
    private const val PRUNE_BATCH_SIZE = 5_000

    // 詞庫來源欄位名稱（對應 word_association 表）
    private const val COL_KAUTIAN = "kautian"
    private const val COL_TAIGITV = "taigitv"
    private const val COL_ITAIGI = "itaigi"
    private const val COL_SITBUT = "sitbut"
    private const val COL_TAIHOA = "taihoa"
    private const val COL_TAIJIT = "taijit"
    private const val COL_KUNGGE = "kungge"

    private var dictDatabase: SQLiteDatabase? = null
    private var userDatabase: SQLiteDatabase? = null
    private var isInitialized = false
    private val initMutex = Mutex()

    // 記錄計數器（用於觸發清理檢查）
    private var recordCounter = 0

    /**
     * NextWord 預測結果
     */
    data class Prediction(
        val hanzi: String,      // 預測的下一個字
        val tl: String,         // TL 羅馬字
        val poj: String,        // POJ 羅馬字
        val delimiter: String,  // 分隔符（羅馬字模式用："-" 或 " "）
        val score: Double       // 排序分數（改為 Double 以支援時間衰減）
    )

    /**
     * 計算時間衰減因子
     *
     * 使用指數衰減公式（參考 RIME）：
     * decay = exp(-ageHours / halfLifeHours * ln(2))
     *
     * 效果：
     * - 剛使用：decay ≈ 1.0
     * - 1 週後：decay ≈ 0.5
     * - 2 週後：decay ≈ 0.25
     * - 1 月後：decay ≈ 0.06
     *
     * @param lastUsedMs 上次使用時間（毫秒）
     * @return 衰減因子（0.0 ~ 1.0）
     */
    private fun calculateDecay(lastUsedMs: Long): Double {
        val now = System.currentTimeMillis()
        val ageHours = (now - lastUsedMs) / 3600000.0
        // ln(2) ≈ 0.693，用於半衰期計算
        return exp(-ageHours / DECAY_HALF_LIFE_HOURS * 0.693)
    }

    /**
     * 預測下一個字
     *
     * 使用 Bigram 模型：取選中詞的「最後一字」查詢下一個字
     * - 選擇「早安」→ 用「安」查詢 → 預測「安」後面常接的字
     *
     * @param word 當前選中的詞
     * @param limit 最大結果數
     * @param context Android context
     * @return 預測結果列表
     */
    suspend fun predict(
        word: String,
        limit: Int = DEFAULT_LIMIT,
        context: Context
    ): List<Prediction> = withContext(Dispatchers.IO) {
        if (word.isEmpty()) {
            return@withContext emptyList()
        }

        // Bigram 模型：使用最後一字作為查詢 key
        val lastChar = word.last().toString()

        ensureInitialized(context)

        val results = mutableMapOf<String, Prediction>()

        // 1. 查詢字典關聯（Bigram：用最後一字查詢）
        dictDatabase?.let { db ->
            try {
                // 建立詞庫過濾條件
                val prefs = PrefHelper(context)
                val dictWhereCondition = buildDictWhereCondition(prefs)

                val sql = """
                    SELECT next_word, next_tl, next_poj, delimiter, count
                    FROM word_association
                    WHERE prev_word = ?
                    $dictWhereCondition
                    ORDER BY count DESC
                    LIMIT ?
                """.trimIndent()

                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[PREDICT] Dict query: prev_word='$lastChar', filter='$dictWhereCondition'")
                }

                val cursor = db.rawQuery(sql, arrayOf(lastChar, (limit * 2).toString()))
                cursor.use {
                    while (it.moveToNext()) {
                        val nextWord = it.getString(0) ?: continue
                        val nextTl = it.getString(1) ?: ""
                        val nextPoj = it.getString(2) ?: ""
                        val delimiter = it.getString(3) ?: "-"
                        val count = it.getInt(4)

                        results[nextWord] = Prediction(
                            hanzi = nextWord,
                            tl = nextTl,
                            poj = nextPoj,
                            delimiter = delimiter,
                            score = count.toDouble() * DICT_WEIGHT
                        )
                    }
                }
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) {
                    Log.e(TAG, "[PREDICT] Dict query failed", e)
                }
            }
        }

        // 2. 查詢使用者關聯（合併到結果中）
        // 使用者學習也是 Bigram 模型，用「完整詞」查詢
        // 加入 last_used 欄位以計算時間衰減
        userDatabase?.let { db ->
            try {
                val sql = """
                    SELECT next_word, next_tl, next_poj, delimiter, count,
                           strftime('%s', last_used) * 1000 AS last_used_ms
                    FROM user_association
                    WHERE prev_word = ?
                    ORDER BY count DESC
                    LIMIT ?
                """.trimIndent()

                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[PREDICT] User query: prev_word='$word'")
                }

                val cursor = db.rawQuery(sql, arrayOf(word, (limit * 2).toString()))
                var userCount = 0
                cursor.use {
                    while (it.moveToNext()) {
                        val nextWord = it.getString(0) ?: continue
                        val nextTl = it.getString(1) ?: ""
                        val nextPoj = it.getString(2) ?: ""
                        val delimiter = it.getString(3) ?: " "
                        val count = it.getInt(4)
                        val lastUsedMs = it.getLong(5)
                        userCount++

                        // 計算時間衰減
                        val decay = calculateDecay(lastUsedMs)
                        val userScore = count.toDouble() * USER_WEIGHT * decay

                        if (BuildConfig.DEBUG) {
                            Log.d(TAG, "[PREDICT] User found: '$word' -> '$nextWord' (count=$count, decay=%.3f, score=%.1f)".format(decay, userScore))
                        }

                        val existing = results[nextWord]

                        if (existing != null) {
                            // 合併分數，使用使用者的羅馬字和分隔符（如果有）
                            results[nextWord] = existing.copy(
                                tl = if (nextTl.isNotEmpty()) nextTl else existing.tl,
                                poj = if (nextPoj.isNotEmpty()) nextPoj else existing.poj,
                                delimiter = delimiter,  // 使用者的 delimiter 優先
                                score = existing.score + userScore
                            )
                        } else {
                            results[nextWord] = Prediction(
                                hanzi = nextWord,
                                tl = nextTl,
                                poj = nextPoj,
                                delimiter = delimiter,
                                score = userScore
                            )
                        }
                    }
                }
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[PREDICT] User query returned $userCount results")
                }
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) {
                    Log.e(TAG, "[PREDICT] User query failed", e)
                }
            }
        }

        // 3. 按分數排序，返回結果
        val sortedResults = results.values
            .sortedByDescending { it.score }
            .take(limit)

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[PREDICT] '$word' -> ${sortedResults.size} total results (dict+user)")
        }

        sortedResults
    }

    /**
     * 記錄使用者選詞關聯（Bigram）
     *
     * @param prev 前一個選中的詞
     * @param nextHanzi 當前選中的詞（漢字）
     * @param nextTl 當前選中的詞（TL）
     * @param nextPoj 當前選中的詞（POJ）
     * @param delimiter 分隔符（"-" 或 " "，羅馬字模式用）
     * @param context Android context
     */
    suspend fun recordAssociation(
        prev: String,
        nextHanzi: String,
        nextTl: String = "",
        nextPoj: String = "",
        delimiter: String = " ",
        context: Context
    ) = withContext(Dispatchers.IO) {
        if (prev.isEmpty() || nextHanzi.isEmpty()) {
            return@withContext
        }

        ensureInitialized(context)

        val db = userDatabase ?: return@withContext

        try {
            // INSERT OR UPDATE
            val sql = """
                INSERT INTO user_association (prev_word, next_word, next_tl, next_poj, delimiter, count, last_used)
                VALUES (?, ?, ?, ?, ?, 1, CURRENT_TIMESTAMP)
                ON CONFLICT(prev_word, next_word) DO UPDATE SET
                    count = count + 1,
                    next_tl = CASE WHEN ? != '' THEN ? ELSE next_tl END,
                    next_poj = CASE WHEN ? != '' THEN ? ELSE next_poj END,
                    delimiter = ?,
                    last_used = CURRENT_TIMESTAMP
            """.trimIndent()

            db.execSQL(sql, arrayOf(prev, nextHanzi, nextTl, nextPoj, delimiter, nextTl, nextTl, nextPoj, nextPoj, delimiter))

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[RECORD] '$prev' -> '$nextHanzi' ($nextTl/$nextPoj, delimiter='$delimiter')")
            }

            // 定期檢查是否需要清理舊關聯
            recordCounter++
            if (recordCounter >= PRUNE_CHECK_INTERVAL) {
                recordCounter = 0
                pruneOldAssociations()
            }
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[RECORD] Insert failed", e)
            }
        }
    }

    /**
     * Ensure databases are initialized
     */
    private suspend fun ensureInitialized(context: Context) {
        if (isInitialized) return

        initMutex.withLock {
            if (isInitialized) return

            try {
                // 初始化字典關聯 db（從 assets 複製）
                connectDictDb(context)

                // 初始化使用者關聯 db（本地建立）
                connectUserDb(context)

                isInitialized = true

                if (BuildConfig.DEBUG) {
                    Log.i(TAG, "[INIT] NextWord databases initialized")
                }
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) {
                    Log.e(TAG, "[INIT] Initialization failed", e)
                }
            }
        }
    }

    /**
     * 連接字典關聯 db（使用 LexiconService 已複製的 dictionary.db）
     */
    private fun connectDictDb(context: Context) {
        val dbFile = File(context.filesDir, DICT_DB_NAME)

        if (!dbFile.exists()) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[INIT] dictionary.db not found (LexiconService not initialized?)")
            }
            return
        }

        dictDatabase = SQLiteDatabase.openDatabase(
            dbFile.absolutePath,
            null,
            SQLiteDatabase.OPEN_READONLY
        )

        if (BuildConfig.DEBUG) {
            Log.i(TAG, "[INIT] Connected to dictionary.db for word_association")
        }
    }

    /**
     * 連接使用者關聯 db（本地建立，永久保留）
     */
    private fun connectUserDb(context: Context) {
        val dbFile = File(context.filesDir, USER_DB_NAME)

        userDatabase = SQLiteDatabase.openOrCreateDatabase(dbFile, null)

        // 建立表格（如果不存在）
        userDatabase?.execSQL("""
            CREATE TABLE IF NOT EXISTS user_association (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prev_word TEXT NOT NULL,
                next_word TEXT NOT NULL,
                next_tl TEXT,
                next_poj TEXT,
                delimiter TEXT DEFAULT ' ',
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(prev_word, next_word)
            )
        """)

        userDatabase?.execSQL(
            "CREATE INDEX IF NOT EXISTS idx_user_prev_word ON user_association(prev_word)"
        )

        // 遷移：為舊表加入 delimiter 欄位（如果不存在）
        try {
            userDatabase?.execSQL("ALTER TABLE user_association ADD COLUMN delimiter TEXT DEFAULT ' '")
        } catch (e: Exception) {
            // 欄位已存在，忽略錯誤
        }

        // 設定 WAL 模式（支援讀寫併發）
        // PRAGMA 需要用 rawQuery 執行
        userDatabase?.rawQuery("PRAGMA journal_mode=WAL;", null)?.close()
    }

    /**
     * 使用者關聯資料（Debug 用）
     */
    data class Association(
        val prevWord: String,
        val nextWord: String,
        val nextTl: String,
        val nextPoj: String,
        val delimiter: String,
        val count: Int
    )

    /**
     * 取得所有使用者關聯資料（Debug 用）
     */
    suspend fun getAllAssociations(context: Context): List<Association> = withContext(Dispatchers.IO) {
        try {
            ensureInitialized(context)
            val db = userDatabase ?: return@withContext emptyList()

            val cursor = db.rawQuery(
                """
                SELECT prev_word, next_word, next_tl, next_poj, delimiter, count
                FROM user_association
                ORDER BY count DESC, last_used DESC
                """.trimIndent(),
                null
            )

            val results = mutableListOf<Association>()
            cursor.use {
                while (it.moveToNext()) {
                    results.add(
                        Association(
                            prevWord = it.getString(0) ?: "",
                            nextWord = it.getString(1) ?: "",
                            nextTl = it.getString(2) ?: "",
                            nextPoj = it.getString(3) ?: "",
                            delimiter = it.getString(4) ?: " ",
                            count = it.getInt(5)
                        )
                    )
                }
            }

            results
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[QUERY] Failed to get all associations", e)
            }
            emptyList()
        }
    }

    /**
     * 清除所有使用者關聯資料（Debug 用）
     */
    suspend fun clearAllAssociations(context: Context) = withContext(Dispatchers.IO) {
        try {
            ensureInitialized(context)
            val db = userDatabase ?: return@withContext

            db.execSQL("DELETE FROM user_association")

            if (BuildConfig.DEBUG) {
                Log.i(TAG, "[CLEAR] All user associations cleared")
            }
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[CLEAR] Failed to clear associations", e)
            }
        }
    }

    /**
     * 清理舊的使用者關聯（當超過上限時）
     *
     * 刪除策略：
     * 1. 計算每筆關聯的「有效分數」= count × decay
     * 2. 刪除分數最低的 PRUNE_BATCH_SIZE 筆
     *
     * 這確保保留：
     * - 最近使用的關聯（decay 高）
     * - 常用的關聯（count 高）
     */
    private suspend fun pruneOldAssociations() = withContext(Dispatchers.IO) {
        val db = userDatabase ?: return@withContext

        try {
            // 1. 計算目前的關聯數量
            val countCursor = db.rawQuery("SELECT COUNT(*) FROM user_association", null)
            val currentCount = countCursor.use {
                if (it.moveToFirst()) it.getInt(0) else 0
            }

            if (currentCount <= MAX_USER_ASSOCIATIONS) {
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[PRUNE] No pruning needed: $currentCount <= $MAX_USER_ASSOCIATIONS")
                }
                return@withContext
            }

            // 2. 刪除分數最低的關聯
            // 使用 SQLite 計算有效分數：count * exp(-age_hours / half_life * 0.693)
            // SQLite 沒有 exp()，改用近似方法：直接按 count 和 last_used 排序
            // 優先刪除：count 低 且 last_used 舊 的關聯
            val deleteCount = minOf(PRUNE_BATCH_SIZE, currentCount - MAX_USER_ASSOCIATIONS + PRUNE_BATCH_SIZE)

            val deleteSql = """
                DELETE FROM user_association
                WHERE id IN (
                    SELECT id FROM user_association
                    ORDER BY count ASC, last_used ASC
                    LIMIT ?
                )
            """.trimIndent()

            db.execSQL(deleteSql, arrayOf(deleteCount.toString()))

            if (BuildConfig.DEBUG) {
                Log.i(TAG, "[PRUNE] Deleted $deleteCount associations (was $currentCount, target <= $MAX_USER_ASSOCIATIONS)")
            }
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[PRUNE] Failed to prune associations", e)
            }
        }
    }

    /**
     * 取得目前使用者關聯數量
     */
    suspend fun getAssociationCount(context: Context): Int = withContext(Dispatchers.IO) {
        try {
            ensureInitialized(context)
            val db = userDatabase ?: return@withContext 0

            val cursor = db.rawQuery("SELECT COUNT(*) FROM user_association", null)
            cursor.use {
                if (it.moveToFirst()) it.getInt(0) else 0
            }
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[COUNT] Failed to get association count", e)
            }
            0
        }
    }

    /**
     * 建立詞庫過濾 WHERE 條件
     *
     * 使用 OR 邏輯：只要 Bigram 來自任一開啟的詞庫即可
     * 全部開啟時返回空字串（不加過濾）
     */
    private fun buildDictWhereCondition(prefs: PrefHelper): String {
        val conditions = mutableListOf<String>()

        if (prefs.moeDictEnabled) conditions.add("$COL_KAUTIAN = 1")
        if (prefs.newwordDictEnabled) conditions.add("$COL_TAIGITV = 1")
        if (prefs.itaigiDictEnabled) conditions.add("$COL_ITAIGI = 1")
        if (prefs.sitbutDictEnabled) conditions.add("$COL_SITBUT = 1")
        if (prefs.taihoaDictEnabled) conditions.add("$COL_TAIHOA = 1")
        if (prefs.taijitDictEnabled) conditions.add("$COL_TAIJIT = 1")
        if (prefs.kunggeDictEnabled) conditions.add("$COL_KUNGGE = 1")

        // 全部開啟時不加過濾條件
        val allEnabled = prefs.moeDictEnabled && prefs.newwordDictEnabled &&
            prefs.itaigiDictEnabled && prefs.sitbutDictEnabled &&
            prefs.taihoaDictEnabled && prefs.taijitDictEnabled && prefs.kunggeDictEnabled

        if (allEnabled) {
            return ""
        }

        // 全部關閉時返回不可能的條件（不顯示任何結果）
        if (conditions.isEmpty()) {
            return "AND 0"
        }

        return "AND (${conditions.joinToString(" OR ")})"
    }

    /**
     * Close database connections
     */
    fun close() {
        dictDatabase?.close()
        dictDatabase = null
        userDatabase?.close()
        userDatabase = null
        isInitialized = false

        if (BuildConfig.DEBUG) {
            Log.i(TAG, "[CLOSE] Database connections closed")
        }
    }
}
