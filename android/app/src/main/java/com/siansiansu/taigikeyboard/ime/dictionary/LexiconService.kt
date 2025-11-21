package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import android.database.Cursor
import android.database.sqlite.SQLiteDatabase
import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream

/**
 * Service for querying Taigi dictionary database
 * Thread-safe singleton with lazy initialization
 */
object LexiconService {
    private const val TAG = "LexiconService"

    private var database: SQLiteDatabase? = null
    private var isInitialized = false
    private val initMutex = Mutex()

    // Column names
    private object Column {
        const val HANZI = "hanzi"
        const val POJ = "poj"
        const val POJ_NO_TONE = "poj_no_tone"
        const val TL = "tl"
        const val TL_NO_TONE = "tl_no_tone"
    }

    /**
     * Search for words in the dictionary
     * @param input Search query string (preprocessed, may be lowercased for search)
     * @param originalInput Original user input (preserves case for capitalization)
     * @param inputType Type of input (hanzi, roman with/without tone)
     * @param inputMode POJ or TL mode
     * @param limit Maximum number of results
     * @param context Android context for accessing assets
     * @return List of matching TaigiWord entries
     */
    suspend fun search(
        input: String,
        originalInput: String = input,
        inputType: InputType,
        inputMode: InputMode = InputMode.POJ,
        limit: Int = DictionaryConstants.DEFAULT_SEARCH_LIMIT,
        context: Context
    ): List<TaigiWord> = withContext(Dispatchers.IO) {
        if (input.isEmpty()) {
            return@withContext emptyList()
        }

        ensureInitialized(context)

        val db = database ?: throw DictionaryError.DatabaseNotAvailable

        val column = getColumn(inputType, inputMode)
        val searchText = input.lowercase()

        try {
            val words = query(db, column, searchText, inputMode, limit)
            val processedWords = words.map { word ->
                val processedHanzi = if (word.hanzi != null && startsWithRomanLetter(word.hanzi)) {
                    capitalize(word.hanzi, originalInput, inputMode)
                } else {
                    word.hanzi
                }

                word.copy(
                    roman = capitalize(word.roman, originalInput, inputMode),
                    hanzi = processedHanzi
                )
            }

            val uniqueWords = removeDuplicates(processedWords)
            applyUserFrequencySort(uniqueWords)
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[SEARCH] Query failed", e)
            }
            throw DictionaryError.QueryExecutionFailed(e.message ?: "Unknown error")
        }
    }

    /**
     * Ensure database is initialized (lazy initialization with concurrency safety)
     */
    private suspend fun ensureInitialized(context: Context) {
        if (isInitialized) return

        initMutex.withLock {
            if (isInitialized) return

            try {
                connect(context)
                isInitialized = true
                if (BuildConfig.DEBUG) {
                    Log.i(TAG, "[INIT] Database initialized successfully")
                }
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) {
                    Log.e(TAG, "[INIT] Database initialization failed", e)
                }
                throw e
            }
        }
    }

    /**
     * Connect to the dictionary database
     */
    private fun connect(context: Context) {
        val dbPath = getDatabasePath(context)

        database = SQLiteDatabase.openDatabase(
            dbPath,
            null,
            SQLiteDatabase.OPEN_READONLY
        )

        configure()
    }

    /**
     * Get database file path, copying from assets if necessary
     * 每次 app 更新時都會重新複製 dictionary.db，確保字典內容是最新版本
     */
    private fun getDatabasePath(context: Context): String {
        val dbFile = File(context.filesDir, DictionaryConstants.DATABASE_NAME)
        val versionFile = File(context.filesDir, "dictionary_app_version.txt")

        val currentAppVersion = BuildConfig.VERSION_CODE
        val lastCopiedVersion = if (versionFile.exists()) {
            versionFile.readText().trim().toIntOrNull() ?: 0
        } else {
            0
        }

        // App 版本更新時重新複製字典（確保使用者獲得最新字典）
        if (currentAppVersion > lastCopiedVersion || !dbFile.exists()) {
            try {
                context.assets.open(DictionaryConstants.DATABASE_NAME).use { input ->
                    FileOutputStream(dbFile).use { output ->
                        input.copyTo(output)
                    }
                }
                versionFile.writeText(currentAppVersion.toString())

                if (BuildConfig.DEBUG) {
                    Log.i(TAG, "[UPDATE] Dictionary updated from v$lastCopiedVersion to v$currentAppVersion")
                    Log.i(TAG, "[PATH] Database path: ${dbFile.absolutePath}")
                }
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) {
                    Log.e(TAG, "[ERROR] Failed to copy dictionary from assets", e)
                }
                throw DictionaryError.DatabaseConnectionFailed("Failed to copy dictionary: ${e.message}")
            }
        }

        return dbFile.absolutePath
    }

    /**
     * Configure database for optimal performance
     */
    private fun configure() {
        val db = database ?: return

        val configurations = listOf(
            "PRAGMA journal_mode=DELETE;",
            "PRAGMA cache_size=10000;",
            "PRAGMA temp_store=MEMORY;",
            "PRAGMA mmap_size=0;",
            "PRAGMA synchronous=NORMAL;"
        )

        configurations.forEach { config ->
            try {
                db.execSQL(config)
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) {
                    Log.w(TAG, "[CONFIG] Failed to execute: $config", e)
                }
            }
        }
    }

    /**
     * Execute query on database
     */
    private fun query(
        db: SQLiteDatabase,
        column: String,
        input: String,
        inputMode: InputMode,
        limit: Int
    ): List<TaigiWord> {
        val sql = buildSQL(column, inputMode)
        val normalizedInput = input.replace("-", "")
        val normalizedPattern = "$normalizedInput%"
        val originalPattern = "$input%"

        val cursor = db.rawQuery(
            sql,
            arrayOf(normalizedPattern, originalPattern, input, originalPattern, limit.toString())
        )

        return cursor.use { extract(it, limit) }
    }

    /**
     * Build SQL query string
     */
    private fun buildSQL(column: String, inputMode: InputMode): String {
        val romanColumn = if (inputMode == InputMode.POJ) Column.POJ else Column.TL
        val maxSyllableCount = 3

        return """
            SELECT id, $romanColumn, ${Column.HANZI}, syllable_count
            FROM dictionary
            WHERE (REPLACE($column, '-', '') LIKE ?
               OR $column LIKE ?)
               AND syllable_count <= $maxSyllableCount
            ORDER BY
                CASE
                    WHEN $column = ? THEN 0
                    WHEN $column LIKE ? THEN 1
                    ELSE 2
                END,
                LENGTH($romanColumn) ASC,
                $romanColumn ASC
            LIMIT ?;
        """.trimIndent()
    }

    /**
     * Extract results from cursor
     */
    private fun extract(cursor: Cursor, limit: Int): List<TaigiWord> {
        val results = mutableListOf<TaigiWord>()

        while (cursor.moveToNext() && results.size < limit) {
            val id = cursor.getInt(0)
            val roman = cursor.getString(1) ?: ""
            val hanziText = cursor.getString(2)
            val hanzi = if (hanziText.isNullOrEmpty()) null else hanziText
            val lengthScore = cursor.getInt(3)

            results.add(
                TaigiWord(
                    id = id,
                    roman = roman,
                    hanzi = hanzi,
                    lengthScore = lengthScore
                )
            )
        }

        return results
    }

    /**
     * Get column name based on input type and mode
     */
    private fun getColumn(inputType: InputType, inputMode: InputMode): String {
        return when (inputType) {
            is InputType.Hanzi -> Column.HANZI
            is InputType.RomanWithTone -> if (inputMode == InputMode.POJ) Column.POJ else Column.TL
            is InputType.RomanWithoutTone -> if (inputMode == InputMode.POJ) Column.POJ_NO_TONE else Column.TL_NO_TONE
        }
    }

    /**
     * 根據原始輸入的大小寫，調整候選詞的首字元大小寫
     *
     * 參考 iOS 實作：taigi-keyboard-ios/Sources/Extension/LexiconService.swift:147-176
     *
     * @param text 候選詞文字
     * @param originalInput 原始使用者輸入（保留大小寫）
     * @param inputMode POJ 或 TL 模式
     * @return 調整大小寫後的候選詞
     */
    private fun capitalize(text: String, originalInput: String, inputMode: InputMode): String {
        // TODO: 未來可以整合自動大寫設定檢查

        if (originalInput.isEmpty() || text.isEmpty()) return text

        // 檢查原始輸入的首字元是否大寫
        val firstChar = originalInput.first()
        val isInputUpperCase = firstChar.isUpperCase()

        if (!isInputUpperCase) {
            return text
        }

        // 檢查候選詞首字元是否為字母
        val textFirst = text.first()
        if (!textFirst.isLetter()) return text

        // 使用聲調字母大寫轉換
        val first = ToneConverterModels.uppercaseToneLetter(textFirst.toString(), inputMode)
        val rest = text.drop(1)

        val result = first + rest

        return result
    }

    /**
     * Check if text starts with a roman letter
     */
    private fun startsWithRomanLetter(text: String): Boolean {
        return text.firstOrNull()?.isLetter() == true
    }

    /**
     * 移除重複的候選詞
     *
     * 使用 roman + hanzi 組合作為唯一性判斷依據，
     * 保留第一次出現的候選詞
     *
     * @param words 原始候選詞列表
     * @return 去重後的候選詞列表
     */
    private fun removeDuplicates(words: List<TaigiWord>): List<TaigiWord> {
        val seen = mutableSetOf<String>()
        val result = mutableListOf<TaigiWord>()

        for (word in words) {
            val key = "${word.roman}|${word.hanzi ?: ""}"
            if (!seen.contains(key)) {
                seen.add(key)
                result.add(word)
            }
        }

        return result
    }

    /**
     * 根據使用者頻率重新排序候選詞
     *
     * 排序規則（依優先順序）：
     * 1. 使用頻率（降序）：常用詞優先
     * 2. 詞彙長度（升序）：短詞優先
     * 3. 羅馬字（升序）：字母順序
     *
     * @param words 原始候選詞列表
     * @return 排序後的候選詞列表
     */
    private suspend fun applyUserFrequencySort(words: List<TaigiWord>): List<TaigiWord> {
        return withContext(Dispatchers.IO) {
            try {
                // 收集所有候選詞的 displayText 並查詢頻率
                val frequencies = mutableMapOf<String, Int>()
                words.forEach { word ->
                    val text = word.displayText
                    if (!frequencies.containsKey(text)) {
                        frequencies[text] = UserFrequencyService.getFrequency(text)
                    }
                }

                // 多條件排序
                words.sortedWith(
                    compareByDescending<TaigiWord> { frequencies[it.displayText] ?: 0 } // 1. 頻率降序
                        .thenBy { it.roman.length }                                      // 2. 長度升序
                        .thenBy { it.roman }                                             // 3. 羅馬字升序
                )
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) {
                    Log.w(TAG, "[SORT] User frequency sort failed, using original order", e)
                }
                words // 發生錯誤時返回原始順序
            }
        }
    }

    /**
     * Check if database is connected
     */
    fun isConnected(): Boolean {
        return database != null && database?.isOpen == true
    }

    /**
     * Close database connection (call in service cleanup)
     */
    fun close() {
        database?.close()
        database = null
        isInitialized = false
        if (BuildConfig.DEBUG) {
            Log.i(TAG, "[CLOSE] Database connection closed")
        }
    }
}
