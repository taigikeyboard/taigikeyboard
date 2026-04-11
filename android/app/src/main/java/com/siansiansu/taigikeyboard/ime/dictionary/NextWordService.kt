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
import java.util.concurrent.atomic.AtomicInteger
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
    private const val DATABASE_VERSION = 4 // v4: added prev_tl column
    private const val DEFAULT_LIMIT = 30

    // 使用者來源權重（相對於字典來源）
    // USER_WEIGHT 較高，讓使用者學習的關聯更有競爭力
    private const val USER_WEIGHT = 50
    private const val DICT_WEIGHT = 1

    // 時間衰減參數（參考 RIME 的指數衰減公式）
    // 半衰期：168 小時（一週），超過一週的關聯權重減半
    private const val DECAY_HALF_LIFE_HOURS = 168.0

    // Memory strength: ensures user entries rank above dict entries
    // Matches iOS NextWordService scoring constants
    private const val LEARNING_BONUS = 300.0
    private const val HIGH_USAGE_DECAY_FLOOR = 0.95 // count >= 3: near-permanent retention
    private const val LOW_USAGE_DECAY_FLOOR = 0.3 // count < 3: prevents full decay (~1 month visible)
    private const val HIGH_USAGE_THRESHOLD = 3

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
    private const val COL_STTI = "stti"
    private const val COL_KHPOO = "khpoo"

    private var dictDatabase: SQLiteDatabase? = null
    private var userDatabase: SQLiteDatabase? = null
    private var isInitialized = false
    private val initMutex = Mutex()

    // 記錄計數器（用於觸發清理檢查，AtomicInteger for thread safety）
    private val recordCounter = AtomicInteger(0)

    /**
     * NextWord 預測結果
     */
    data class Prediction(
        val hanzi: String, // 預測的下一個字
        val tl: String, // TL 羅馬字
        val score: Double, // 排序分數（Double 以支援時間衰減）
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
     * Calculate user-layer score with decay floor + learning bonus
     *
     * Ensures user entries always rank above dict entries (learningBonus = 300).
     * High-usage entries (count >= 3) get near-permanent retention via decay floor.
     * Matches iOS NextWordService.calculateUserScore().
     *
     * @param count usage count
     * @param lastUsedMs last used time in milliseconds
     * @return weighted score
     */
    private fun calculateUserScore(
        count: Int,
        lastUsedMs: Long,
    ): Double {
        val decay = calculateDecay(lastUsedMs)
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
        roman: String = "",
        limit: Int = DEFAULT_LIMIT,
        context: Context,
        prefs: PrefHelper? = null,
    ): List<Prediction> =
        withContext(Dispatchers.IO) {
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
                    // 建立詞庫過濾條件（atomic snapshot to avoid torn reads）
                    val prefHelper = prefs ?: PrefHelper(context)
                    val dictWhereCondition = buildDictWhereCondition(prefHelper.snapshotEnabledDictionaries())

                    val sql =
                        """
                        SELECT next_word, next_tl, count
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
                            val count = it.getInt(2)

                            val key = "${nextWord}\t$nextTl"
                            results[key] =
                                Prediction(
                                    hanzi = nextWord,
                                    tl = nextTl,
                                    score = count.toDouble() * DICT_WEIGHT,
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
                    val sql =
                        """
                        SELECT next_word, next_tl, count,
                               strftime('%s', last_used) * 1000 AS last_used_ms
                        FROM user_association
                        WHERE prev_word = ? AND (prev_tl = ? OR prev_tl = '')
                        ORDER BY count DESC
                        LIMIT ?
                        """.trimIndent()

                    if (BuildConfig.DEBUG) {
                        Log.d(TAG, "[PREDICT] User query: prev_word='$word', prev_tl='$roman'")
                    }

                    val cursor = db.rawQuery(sql, arrayOf(word, roman, (limit * 2).toString()))
                    var userCount = 0
                    cursor.use {
                        while (it.moveToNext()) {
                            val nextWord = it.getString(0) ?: continue
                            val nextTl = it.getString(1) ?: ""
                            val count = it.getInt(2)
                            val lastUsedMs = it.getLong(3)
                            userCount++

                            // Calculate score with learning bonus and decay floors
                            val userScore = calculateUserScore(count, lastUsedMs)

                            if (BuildConfig.DEBUG) {
                                val decay = calculateDecay(lastUsedMs)
                                Log.d(
                                    TAG,
                                    "[PREDICT] User found: '$word' -> '$nextWord' (count=$count, decay=%.3f, score=%.1f)".format(
                                        decay,
                                        userScore,
                                    ),
                                )
                            }

                            val key = "${nextWord}\t$nextTl"
                            val existing = results[key]

                            if (existing != null) {
                                // Merge scores, prefer user's TL if available
                                results[key] =
                                    existing.copy(
                                        tl = if (nextTl.isNotEmpty()) nextTl else existing.tl,
                                        score = existing.score + userScore,
                                    )
                            } else {
                                results[key] =
                                    Prediction(
                                        hanzi = nextWord,
                                        tl = nextTl,
                                        score = userScore,
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
            val sortedResults =
                results.values
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
     * @param context Android context
     */
    @Suppress("SqlResolve")
    suspend fun recordAssociation(
        prev: String,
        prevTl: String = "",
        nextHanzi: String,
        nextTl: String = "",
        context: Context,
    ) = withContext(Dispatchers.IO) {
        if (prev.isEmpty() || nextHanzi.isEmpty()) {
            return@withContext
        }

        ensureInitialized(context)

        val db = userDatabase ?: return@withContext

        try {
            // INSERT OR UPDATE
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

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[RECORD] '$prev' (tl='$prevTl') -> '$nextHanzi' (tl='$nextTl')")
            }

            // 定期檢查是否需要清理舊關聯
            if (recordCounter.incrementAndGet() >= PRUNE_CHECK_INTERVAL) {
                recordCounter.set(0)
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

        dictDatabase =
            SQLiteDatabase.openDatabase(
                dbFile.absolutePath,
                null,
                SQLiteDatabase.OPEN_READONLY,
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

        val db = userDatabase ?: return

        // Check and apply schema migrations
        migrateUserDb(db)

        // 建立表格（如果不存在）
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

        db.execSQL(
            "CREATE INDEX IF NOT EXISTS idx_user_prev_word ON user_association(prev_word)",
        )
        db.execSQL(
            "CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl ON user_association(prev_word, prev_tl)",
        )

        // 一次性遷移：WAL → DELETE（v3.4.8）
        // WAL 在 IME 場景無顯著優勢，DELETE mode 更簡單且不會產生 WAL 檔膨脹
        migrateFromWAL(db)
    }

    /**
     * 一次性 WAL → DELETE 遷移
     * 若資料庫仍在 WAL 模式，執行 checkpoint 後切回 DELETE
     */
    private fun migrateFromWAL(db: SQLiteDatabase) {
        val journalMode =
            db.rawQuery("PRAGMA journal_mode;", null)?.use { cursor ->
                if (cursor.moveToFirst()) cursor.getString(0) else null
            } ?: return

        if (!journalMode.equals("wal", ignoreCase = true)) {
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[MIGRATE] No WAL detected (journal_mode=$journalMode), skipping migration")
            }
            return
        }

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[MIGRATE] WAL detected, performing checkpoint and switching to DELETE")
        }
        db.rawQuery("PRAGMA wal_checkpoint(TRUNCATE);", null)?.use { cursor ->
            if (BuildConfig.DEBUG && cursor.moveToFirst()) {
                val busy = cursor.getInt(0)
                val log = cursor.getInt(1)
                val checkpointed = cursor.getInt(2)
                if (busy != 0 || log != checkpointed) {
                    Log.w(TAG, "[MIGRATE] WAL checkpoint incomplete: busy=$busy, log=$log, checkpointed=$checkpointed")
                }
            }
        }
        db.rawQuery("PRAGMA journal_mode=DELETE;", null)?.close()
    }

    /**
     * Migrate user_association.db schema to current DATABASE_VERSION.
     *
     * v2: Removed next_poj and delimiter columns (align with iOS schema).
     * SQLite < 3.35 does not support DROP COLUMN, so we recreate the table
     * and copy existing data.
     */
    private fun migrateUserDb(db: SQLiteDatabase) {
        val cursor = db.rawQuery("PRAGMA user_version;", null)
        val currentVersion =
            cursor.use {
                if (it.moveToFirst()) it.getInt(0) else 0
            }

        if (currentVersion >= DATABASE_VERSION) return

        if (BuildConfig.DEBUG) {
            Log.i(TAG, "[MIGRATE] user_association.db v$currentVersion -> v$DATABASE_VERSION")
        }

        // v0/v1 -> v2: remove next_poj and delimiter columns
        if (currentVersion < 2) {
            // Only migrate if old table actually exists with the old columns
            val hasOldTable =
                try {
                    val ti =
                        db.rawQuery(
                            "SELECT name FROM sqlite_master WHERE type='table' AND name='user_association'",
                            null,
                        )
                    val exists = ti.use { it.moveToFirst() }
                    exists
                } catch (_: Exception) {
                    false
                }

            if (hasOldTable) {
                try {
                    db.beginTransaction()
                    // Recreate table without next_poj and delimiter
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
                    if (BuildConfig.DEBUG) {
                        Log.e(TAG, "[MIGRATE] Failed to migrate user_association", e)
                    }
                } finally {
                    db.endTransaction()
                }
            }
        }

        // v2 -> v3: UNIQUE(prev_word, next_word) -> UNIQUE(prev_word, next_word, next_tl) (drop+recreate)
        if (currentVersion in 2 until 3) {
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

            if (hasTable) {
                try {
                    db.beginTransaction()
                    db.execSQL("DROP TABLE user_association")
                    db.execSQL("DROP INDEX IF EXISTS idx_user_prev_word")
                    db.setTransactionSuccessful()
                    if (BuildConfig.DEBUG) {
                        Log.i(TAG, "[MIGRATE] Recreated user_association with UNIQUE(prev_word, next_word, next_tl)")
                    }
                } catch (e: Exception) {
                    if (BuildConfig.DEBUG) {
                        Log.e(TAG, "[MIGRATE] v2->v3 migration failed", e)
                    }
                } finally {
                    db.endTransaction()
                }
            }
        }

        // v3 -> v4: add prev_tl column (ALTER TABLE, no data loss)
        if (currentVersion in 3 until 4) {
            try {
                db.beginTransaction()
                db.execSQL("ALTER TABLE user_association ADD COLUMN prev_tl TEXT DEFAULT ''")
                db.execSQL("CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl ON user_association(prev_word, prev_tl)")
                db.setTransactionSuccessful()
                if (BuildConfig.DEBUG) {
                    Log.i(TAG, "[MIGRATE] Added prev_tl column to user_association")
                }
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) {
                    Log.e(TAG, "[MIGRATE] v3->v4 migration failed", e)
                }
            } finally {
                db.endTransaction()
            }
        }

        // Stamp the new version
        db.execSQL("PRAGMA user_version = $DATABASE_VERSION;")
    }

    /**
     * 使用者關聯資料（Debug 用）
     */
    data class AssociationEntry(
        val prevWord: String,
        val prevTl: String = "",
        val nextWord: String,
        val nextTl: String,
        val count: Int,
    )

    /**
     * 取得所有使用者關聯資料（Debug 用）
     */
    suspend fun allAssociations(context: Context): List<AssociationEntry> =
        withContext(Dispatchers.IO) {
            try {
                ensureInitialized(context)
                val db = userDatabase ?: return@withContext emptyList()

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

                results
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) {
                    Log.e(TAG, "[QUERY] Failed to get all associations", e)
                }
                emptyList()
            }
        }

    /**
     * Batch import association entries with merge strategy: keep higher count.
     */
    @Suppress("SqlResolve")
    suspend fun batchImportAssociations(
        context: Context,
        entries: List<AssociationEntry>,
    ): Int =
        withContext(Dispatchers.IO) {
            try {
                ensureInitialized(context)
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
                        stmt.clearBindings()
                        stmt.bindString(1, entry.prevWord)
                        stmt.bindString(2, entry.prevTl)
                        stmt.bindString(3, entry.nextWord)
                        stmt.bindString(4, entry.nextTl)
                        stmt.bindLong(5, entry.count.toLong())
                        stmt.executeInsert()
                        imported++
                    }
                    db.setTransactionSuccessful()
                } finally {
                    db.endTransaction()
                }
                imported
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) {
                    Log.e(TAG, "[BATCH_IMPORT] Association import failed", e)
                }
                0
            }
        }

    /**
     * Delete a single user association entry
     */
    suspend fun deleteAssociation(
        context: Context,
        entry: AssociationEntry,
    ) = withContext(Dispatchers.IO) {
        try {
            ensureInitialized(context)
            val db = userDatabase ?: return@withContext

            db.execSQL(
                "DELETE FROM user_association WHERE prev_word = ? AND prev_tl = ? AND next_word = ? AND next_tl = ?",
                arrayOf(entry.prevWord, entry.prevTl, entry.nextWord, entry.nextTl),
            )
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[DELETE] Failed to delete association", e)
            }
        }
    }

    /**
     * 清除所有使用者關聯資料
     */
    suspend fun clearAllAssociations(context: Context) =
        withContext(Dispatchers.IO) {
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
    private suspend fun pruneOldAssociations() =
        withContext(Dispatchers.IO) {
            val db = userDatabase ?: return@withContext

            try {
                // 1. 計算目前的關聯數量
                val countCursor = db.rawQuery("SELECT COUNT(*) FROM user_association", null)
                val currentCount =
                    countCursor.use {
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
     * 建立詞庫過濾 WHERE 條件
     *
     * 使用 OR 邏輯：只要 Bigram 來自任一開啟的詞庫即可
     * 全部開啟時返回空字串（不加過濾）
     *
     * @param snapshot atomic snapshot of dictionary enabled flags
     */
    private fun buildDictWhereCondition(snapshot: PrefHelper.DictEnabledSnapshot): String {
        val conditions = mutableListOf<String>()

        if (snapshot.moe) conditions.add("$COL_KAUTIAN = 1")
        if (snapshot.newword) conditions.add("$COL_TAIGITV = 1")
        if (snapshot.itaigi) conditions.add("$COL_ITAIGI = 1")
        if (snapshot.taiwanPlant) conditions.add("$COL_SITBUT = 1")
        if (snapshot.taiHua) conditions.add("$COL_TAIHOA = 1")
        if (snapshot.taiwanJapan) conditions.add("$COL_TAIJIT = 1")
        if (snapshot.kungge) conditions.add("$COL_KUNGGE = 1")
        if (snapshot.stti) conditions.add("$COL_STTI = 1")
        if (snapshot.khpoo) conditions.add("$COL_KHPOO = 1")

        // 全部開啟時不加過濾條件
        val allEnabled =
            snapshot.moe && snapshot.newword &&
                snapshot.itaigi && snapshot.taiwanPlant &&
                snapshot.taiHua && snapshot.taiwanJapan &&
                snapshot.kungge && snapshot.stti &&
                snapshot.khpoo

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
