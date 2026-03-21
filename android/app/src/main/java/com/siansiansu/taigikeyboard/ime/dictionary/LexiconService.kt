package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream
import java.text.Normalizer

/**
 * Service for querying Taigi dictionary database
 *
 * 查詢流程：
 * 1. 使用 Trie 前綴匹配取得候選 rowid
 * 2. 使用 SQLite 批次查詢完整資料
 * 3. 按 frequency 排序
 *
 * Thread-safe singleton with lazy initialization
 */
object LexiconService {
    private const val TAG = "LexiconService"

    @Volatile private var database: SQLiteDatabase? = null
    @Volatile private var isInitialized = false
    private val initMutex = Mutex()

    // Column names
    private object Column {
        const val ID = "id"
        const val HANZI = "hanzi"
        const val POJ = "poj"
        const val TL = "tl"
        const val FREQUENCY = "frequency"
        const val KAUTIAN = "kautian"      // 教育部臺灣台語常用詞辭典
        const val TAIGITV = "taigitv"      // 台語新詞辭庫
        const val ITAIGI = "itaigi"        // iTaigi 華台對照典
        const val SITBUT = "sitbut"        // 台灣植物名彙
        const val TAIHOA = "taihoa"        // 台華線頂對照典
        const val TAIJIT = "taijit"        // 台日大辭典
        const val KUNGGE = "kungge"        // 台語工藝詞庫
        const val STTI = "stti"            // 學科術語辭典
        const val KHPOO = "khpoo"          // 腔口補充資料
        const val LKK = "lkk"              // LKK漢羅合用建議用字
    }

    /**
     * Search for words in the dictionary
     *
     * 使用 Trie + SQLite 混合查詢：
     * 1. Trie 前綴匹配取得 rowid
     * 2. SQLite 批次查詢完整資料
     * 3. 按 frequency 排序
     *
     * @param input Search query string (preprocessed, may be lowercased for search)
     * @param inputType Type of input (hanzi, roman with/without tone)
     * @param inputMode POJ or TL mode
     * @param limit Maximum number of results
     * @param context Android context for accessing assets
     * @return List of matching TaigiWord entries
     */
    suspend fun search(
        input: String,
        inputType: InputType,
        inputMode: InputMode = InputMode.POJ,
        limit: Int = DictionaryConstants.DEFAULT_SEARCH_LIMIT,
        context: Context,
        prefs: PrefHelper? = null
    ): List<TaigiWord> = withContext(Dispatchers.IO) {
        val searchStart = System.currentTimeMillis()
        if (input.isEmpty()) {
            return@withContext emptyList()
        }

        // Hanzi input cannot be searched via trie (matching iOS guard)
        if (inputType is InputType.Hanzi) {
            return@withContext emptyList()
        }

        val initStart = System.currentTimeMillis()
        ensureInitialized(context)
        if (BuildConfig.DEBUG) Log.d("PERF", "[3a] ensureInitialized: ${System.currentTimeMillis() - initStart}ms")

        val db = database ?: throw DictionaryError.DatabaseNotAvailable

        // 讀取搜尋設定（atomic snapshot to avoid torn reads across multiple getters）
        val p = prefs ?: PrefHelper(context)
        val dictSnapshot = p.snapshotEnabledDictionaries()
        val enabledDicts = EnabledDictionaries(
            kautian = dictSnapshot.moe,
            taigitv = dictSnapshot.newword,
            itaigi = dictSnapshot.itaigi,
            sitbut = dictSnapshot.taiwanPlant,
            taihoa = dictSnapshot.taiHua,
            taijit = dictSnapshot.taiwanJapan,
            kungge = dictSnapshot.kungge,
            stti = dictSnapshot.stti,
            khpoo = dictSnapshot.khpoo,
            variant = dictSnapshot.variant,
            khiin = dictSnapshot.khiin,
            lkk = dictSnapshot.lkk
        )

        try {
            // Query custom dictionary by roman prefix (highest priority, matching iOS)
            // Normalize input for notone matching (strip tones, digits, hyphens, spaces)
            val customNotoneKey = CustomDictionaryService.generateNotone(input)
            val customWords = try {
                CustomDictionaryService.search(input, notonePrefix = customNotoneKey, limit = 20).also { entries ->
                    if (BuildConfig.DEBUG) Log.d(TAG, "[SEARCH] customDict key='$input' notoneKey='$customNotoneKey' results=${entries.size}")
                }.map { entry ->
                    TaigiWord(
                        id = -2,  // Custom dictionary marker (distinguishes from NextWord id < -2)
                        roman = entry.roman,
                        hanzi = entry.hanzi,
                        lengthScore = null
                    )
                }
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) Log.w(TAG, "[SEARCH] Custom dictionary query failed: ${e.message}", e)
                emptyList()
            }

            // 使用 Trie + SQLite 查詢
            val trieStart = System.currentTimeMillis()
            val words = searchWithTrie(
                db, input, inputMode, limit, enabledDicts
            )
            if (BuildConfig.DEBUG) Log.d("PERF", "[3b] searchWithTrie (${words.size} results): ${System.currentTimeMillis() - trieStart}ms")

            // TPS ㄜ expansion: also search "or" variant when toggle ON
            val allSystemWords = if (
                TPSConverter.containsTPS(input) && p.tpsOrMapsToER
            ) {
                val tlInput = TPSConverter.toTL(input)
                if (tlInput.contains("er")) {
                    val orVariant = tlInput.replace("er", "or")
                    val orWords = searchWithTrie(db, orVariant, inputMode, limit, enabledDicts)
                    val existingIds = words.map { it.id }.toSet()
                    words + orWords.filter { it.id !in existingIds }
                } else words
            } else words

            // Merge: custom words first, then system words (matching iOS)
            val mergedWords = customWords + allSystemWords

            // Capitalization deferred to SuggestionCaseTransformer (view layer, matching iOS)

            val sortStart = System.currentTimeMillis()
            val uniqueWords = removeDuplicates(mergedWords)
            val normalizedInput = InputNormalizer.normalize(input, inputMode)
            val result = sortByScore(uniqueWords, normalizedInput)
            if (BuildConfig.DEBUG) {
                Log.d("PERF", "[3d] sort: ${System.currentTimeMillis() - sortStart}ms")
                Log.d("PERF", "[3-TOTAL] LexiconService.search: ${System.currentTimeMillis() - searchStart}ms")
            }
            result
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[SEARCH] Query failed", e)
            }
            throw DictionaryError.QueryExecutionFailed(e.message ?: "Unknown error")
        }
    }

    /**
     * 辭典開關設定
     */
    private data class EnabledDictionaries(
        val kautian: Boolean,   // 教育部臺灣台語常用詞辭典
        val taigitv: Boolean,   // 台語新詞辭庫
        val itaigi: Boolean,    // iTaigi 華台對照典
        val sitbut: Boolean,    // 台灣植物名彙
        val taihoa: Boolean,    // 台華線頂對照典
        val taijit: Boolean,    // 台日大辭典
        val kungge: Boolean,    // 台語工藝詞庫
        val stti: Boolean,      // 學科術語辭典
        val khpoo: Boolean,     // 腔口補充資料
        val variant: Boolean,   // 異用字
        val khiin: Boolean,     // 在來字
        val lkk: Boolean        // LKK漢羅合用建議用字
    ) {
        /** 是否全部關閉 */
        fun allDisabled(): Boolean =
            !kautian && !taigitv && !itaigi && !sitbut && !taihoa && !taijit && !kungge && !stti && !khpoo && !lkk

        /** 是否全部開啟 */
        fun allEnabled(): Boolean =
            kautian && taigitv && itaigi && sitbut && taihoa && taijit && kungge && stti && khpoo && lkk
    }

    /**
     * Search using Trie exact match + prefix match + SQLite batch query
     *
     * Trie keys are prefixed by mode (tl:/poj:). Input stays in its
     * native spelling; the prefix is prepended before querying.
     * POJ display text is converted from TL at query result time.
     *
     * Note: MARISA-trie's predictive_search traverses in lexicographic order,
     * so shorter keys may appear later. We use lookup first to ensure exact
     * matches are not truncated by the limit.
     */
    private fun searchWithTrie(
        db: SQLiteDatabase,
        input: String,
        inputMode: InputMode,
        limit: Int,
        enabledDicts: EnabledDictionaries
    ): List<TaigiWord> {
        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[SEARCH] input='$input', mode=$inputMode, limit=$limit")
        }

        // Guard: trie must be loaded (match iOS DictionaryRepository.query guard)
        if (!TrieService.isReady) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[SEARCH] Trie not loaded")
            }
            throw DictionaryError.TrieNotLoaded
        }

        // Build search key using segmenter for continuous input (match iOS flow)
        val searchKey = InputNormalizer.buildSearchKey(input, inputMode)
        val normalizedInput = InputNormalizer.normalize(searchKey, inputMode)

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[NORMALIZE] '$input' -> '$searchKey' -> '$normalizedInput'")
        }

        if (normalizedInput.isEmpty()) {
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[NORMALIZE] Empty after normalization, returning empty")
            }
            return emptyList()
        }

        // Trie 查詢（使用 mode-prefixed key）
        val trieKey = DictionaryConstants.triePrefix(inputMode) + normalizedInput

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[TRIE] trieKey='$trieKey', TrieService.isReady=${TrieService.isReady}, keyCount=${TrieService.getKeyCount()}")
        }

        // 1. 完全匹配（確保短詞不被遺漏）
        val exactRowIds = TrieService.lookup(trieKey)

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[TRIE] lookup('$trieKey') -> ${exactRowIds.size} exact matches: ${exactRowIds.take(5).toList()}")
        }

        // 2. 前綴搜尋（取較多結果以供後續過濾和排序）
        val trieLimit = limit * 6
        val prefixRowIds = TrieService.prefixSearch(trieKey, trieLimit)

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[TRIE] prefixSearch('$trieKey', $trieLimit) -> ${prefixRowIds.size} prefix matches: ${prefixRowIds.take(5).toList()}")
        }

        // 3. 合併去重（後續會按 frequency 排序，順序不重要）
        val allRowIds = (exactRowIds.toList() + prefixRowIds.toList()).distinct()

        if (allRowIds.isEmpty()) {
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[TRIE] No results for: $trieKey")
            }
            return emptyList()
        }

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[TRIE] Total ${allRowIds.size} unique rowIds")
        }

        // SQLite 批次查詢
        val sqlResults = queryByIds(db, allRowIds, inputMode, limit, enabledDicts)

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[SQL] queryByIds returned ${sqlResults.size} words")
            sqlResults.take(3).forEach { word ->
                Log.d(TAG, "[SQL]   - ${word.roman} / ${word.hanzi ?: "(no hanzi)"}")
            }
        }

        return sqlResults
    }

    /**
     * 依 rowid 批次查詢 SQLite
     *
     * @param enabledDicts 各辭典開關設定
     */
    private fun queryByIds(
        db: SQLiteDatabase,
        ids: List<Int>,
        inputMode: InputMode,
        limit: Int,
        enabledDicts: EnabledDictionaries
    ): List<TaigiWord> {
        if (ids.isEmpty()) {
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[SQL] queryByIds: ids is empty")
            }
            return emptyList()
        }

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[SQL] queryByIds: ${ids.size} ids, enabledDicts=$enabledDicts")
        }

        // Always query TL column (all Trie keys are in TL format)
        // Convert to POJ at display time when needed
        val romanColumn = Column.TL

        // 建立 IN 查詢（分批處理避免 SQL 太長）
        val batchSize = 500
        val results = mutableListOf<TaigiWord>()

        for (batch in ids.chunked(batchSize)) {
            val placeholders = batch.joinToString(",") { "?" }

            // 異用字過濾：關閉時只顯示非異用字
            val variantCondition = if (!enabledDicts.variant) "AND is_variant = 0 " else ""

            // 在來字過濾：關閉時排除在來字
            val khiinCondition = if (!enabledDicts.khiin) "AND khiin = 0 " else ""

            // 建立過濾條件：使用 OR 邏輯，只要符合任一開啟的辭典即可
            val dictConditions = mutableListOf<String>()
            if (enabledDicts.kautian) dictConditions.add("${Column.KAUTIAN} = 1")
            if (enabledDicts.taigitv) dictConditions.add("${Column.TAIGITV} = 1")
            if (enabledDicts.itaigi) dictConditions.add("${Column.ITAIGI} = 1")
            if (enabledDicts.sitbut) dictConditions.add("${Column.SITBUT} = 1")
            if (enabledDicts.taihoa) dictConditions.add("${Column.TAIHOA} = 1")
            if (enabledDicts.taijit) dictConditions.add("${Column.TAIJIT} = 1")
            if (enabledDicts.kungge) dictConditions.add("${Column.KUNGGE} = 1")
            if (enabledDicts.stti) dictConditions.add("${Column.STTI} = 1")
            if (enabledDicts.khpoo) dictConditions.add("${Column.KHPOO} = 1")
            if (enabledDicts.lkk) dictConditions.add("${Column.LKK} = 1")

            // Always include dev supplement entries
            dictConditions.add("dev = 1")

            // 全部開啟時不加詞庫過濾條件
            val dictWhereCondition = if (enabledDicts.allEnabled()) {
                ""
            } else {
                "AND (" + dictConditions.joinToString(" OR ") + ")"
            }

            // 合併所有過濾條件
            val whereCondition = variantCondition + khiinCondition + dictWhereCondition

            val sql = """
                SELECT ${Column.ID}, $romanColumn, ${Column.HANZI}, ${Column.FREQUENCY}
                FROM dictionary
                WHERE ${Column.ID} IN ($placeholders)
                $whereCondition
                ORDER BY ${Column.FREQUENCY} DESC
            """.trimIndent()

            val args = batch.map { it.toString() }.toTypedArray()
            val cursor = db.rawQuery(sql, args)

            cursor.use {
                while (it.moveToNext()) {
                    val id = it.getInt(0)
                    val tlRoman = it.getString(1) ?: ""
                    val hanziText = it.getString(2)
                    val hanzi = if (hanziText.isNullOrEmpty()) null else hanziText
                    val frequency = it.getInt(3)

                    // Convert TL -> POJ for display in POJ mode
                    val roman = if (inputMode == InputMode.POJ) {
                        TaigiPhonetics.tlDisplayToPOJDisplay(tlRoman)
                    } else {
                        tlRoman
                    }

                    results.add(TaigiWord(id, roman, hanzi, frequency))
                }
            }
        }

        // 按 frequency 排序並限制結果數
        return results
            .sortedByDescending { it.lengthScore ?: 0 }
            .take(limit)
    }

    /**
     * Ensure database and trie are initialized (lazy initialization with concurrency safety)
     */
    private suspend fun ensureInitialized(context: Context) {
        if (isInitialized) return

        initMutex.withLock {
            if (isInitialized) return

            try {
                // 初始化 SQLite
                connect(context)

                // 初始化 Trie
                val trieLoaded = TrieService.init(context)
                if (!trieLoaded) {
                    Log.w(TAG, "[INIT] Trie initialization failed, will use fallback")
                }

                isInitialized = true
                if (BuildConfig.DEBUG) {
                    Log.i(TAG, "[INIT] Database initialized, Trie keys=${TrieService.getKeyCount()}")
                }
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) {
                    Log.e(TAG, "[INIT] Initialization failed", e)
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
            if (key in seen) continue
            seen.add(key)
            result.add(word)
        }

        return result
    }

    /**
     * 計算候選詞排序分數
     *
     * 公式（v4）：
     * score = userFreqScore + recencyBonus + exactBonus + closenessBonus + baseFreqScore
     *
     * 設計理念：
     * - userFreqScore 主導排序（穩定性優先）
     * - closenessBonus 讓長度較接近輸入的候選詞排序較前（cold-start 主要因素）
     * - recencyBonus 和 exactBonus 只做微調（不會讓低頻詞超過高頻詞）
     *
     * @param word 候選詞
     * @param normalizedInput 正規化後的輸入
     * @param frequencyData 使用者頻率資料（包含 count 和 lastUsedMillis）
     * @return 排序分數（越高越優先）
     */
    private fun calculateScore(
        word: TaigiWord,
        normalizedInput: String,
        frequencyData: UserFrequencyService.FrequencyData
    ): Int {
        // Normalize both sides to base form (no tones, no hyphens) for comparison
        val candidateBase = romanToBase(word.roman)
        val inputBase = inputToBase(normalizedInput)

        // 使用者頻率（主導因素，上限 100，max 10000）
        val cappedUserFreq = minOf(frequencyData.count, 100)
        val userFreqScore = cappedUserFreq * 100

        // Recency 加分（微調，最近 1 小時內用過 +200）
        val currentTime = System.currentTimeMillis()
        val oneHourMillis = 60 * 60 * 1000L
        val recencyBonus = if (frequencyData.lastUsedMillis > 0 &&
            (currentTime - frequencyData.lastUsedMillis) < oneHourMillis) {
            200
        } else {
            0
        }

        // 完全匹配加分（微調，+100）
        val exactBonus = if (candidateBase == inputBase) 100 else 0

        // Completion penalty: penalize candidates extending beyond input (aligned with RIME/Mozc)
        // Ensures exact matches rank above completions in cold-start;
        // user frequency (~15+ uses) can still overcome this penalty
        val completionPenalty = if (candidateBase != inputBase) -1000 else 0

        // Match closeness bonus (0-500): reward candidates whose length matches input
        val inputLen = maxOf(inputBase.length, 1)
        val candidateLen = maxOf(candidateBase.length, 1)
        val matchRatio = minOf(inputLen, candidateLen).toDouble() / maxOf(inputLen, candidateLen).toDouble()
        val closenessBonus = (matchRatio * 500).toInt()

        // 詞庫頻率（新詞 fallback，約 0-100）
        val baseFreqScore = (word.lengthScore ?: 0) / 10

        return userFreqScore + recencyBonus + exactBonus + closenessBonus + baseFreqScore + completionPenalty
    }

    /**
     * Strip roman to base form for matching (no tones, no hyphens/spaces, lowercase)
     * "tāi-tsì" → "taitsi", "m̄ bat" → "mbat"
     */
    private fun romanToBase(roman: String): String {
        val noHyphens = roman.replace("-", "").replace(" ", "")
        val withNasal = noHyphens.replace("\u207F", "nn").replace("\u1D3A", "nn")
        val nfd = Normalizer.normalize(withNasal, Normalizer.Form.NFD)
        val withOo = nfd.replace("\u0358", "o")
        // Strip combining marks (Unicode category Mn = NON_SPACING_MARK) and tone digits
        return withOo.filter {
            Character.getType(it) != Character.NON_SPACING_MARK.toInt()
        }.filter { !it.isDigit() }.lowercase()
    }

    /**
     * Strip tone digits from normalized input
     * "tai5tsi3" → "taitsi", "taitsi" → "taitsi"
     */
    private fun inputToBase(normalizedInput: String): String {
        return normalizedInput.filter { !it.isDigit() }.lowercase()
    }

    /**
     * 根據分數排序候選詞
     *
     * @param words 候選詞列表
     * @param normalizedInput 正規化後的輸入
     * @return 排序後的候選詞列表
     */
    private suspend fun sortByScore(words: List<TaigiWord>, normalizedInput: String): List<TaigiWord> {
        return withContext(Dispatchers.IO) {
            try {
                // 批次查詢使用者頻率資料
                val wordTexts = words.map { it.displayText }.distinct()
                val frequencyDataMap = UserFrequencyService.frequencyDataBatch(wordTexts)

                // 按分數排序
                val sorted = words
                    .map { word ->
                        val freqData = frequencyDataMap[word.displayText]
                            ?: UserFrequencyService.FrequencyData(0, 0)
                        word to calculateScore(word, normalizedInput, freqData)
                    }
                    .sortedByDescending { it.second }

                if (BuildConfig.DEBUG) {
                    logScoreDetails(sorted, normalizedInput, frequencyDataMap)
                }

                sorted.map { it.first }
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) {
                    Log.w(TAG, "[SORT] Scored sort failed, using original order", e)
                }
                words
            }
        }
    }

    /**
     * Log score breakdown for each candidate (debug only).
     * Recalculates components to match iOS logScoreDetails format.
     */
    private fun logScoreDetails(
        sorted: List<Pair<TaigiWord, Int>>,
        normalizedInput: String,
        frequencyDataMap: Map<String, UserFrequencyService.FrequencyData>
    ) {
        val inputBase = inputToBase(normalizedInput)
        val currentTime = System.currentTimeMillis()
        val oneHourMillis = 60 * 60 * 1000L

        for ((word, total) in sorted) {
            val freq = frequencyDataMap[word.displayText]
                ?: UserFrequencyService.FrequencyData(0, 0)
            val candidateBase = romanToBase(word.roman)
            val userFreqScore = minOf(freq.count, 100) * 100
            val recency = if (freq.lastUsedMillis > 0 && (currentTime - freq.lastUsedMillis) < oneHourMillis) 200 else 0
            val exact = if (candidateBase == inputBase) 100 else 0
            val completion = if (candidateBase != inputBase) -1000 else 0
            val inputLen = maxOf(inputBase.length, 1)
            val candidateLen = maxOf(candidateBase.length, 1)
            val closeness = (minOf(inputLen, candidateLen).toDouble() / maxOf(inputLen, candidateLen).toDouble() * 500).toInt()
            val base = (word.lengthScore ?: 0) / 10
            val hanzi = word.hanzi ?: ""

            Log.d(TAG, "[SCORE] input='$normalizedInput' | ${word.roman} $hanzi: user=$userFreqScore recency=$recency exact=$exact close=$closeness base=$base completion=$completion total=$total")
        }
    }

    /**
     * Search dictionary with source information (for dictionary exploration in Tab 3).
     * Searches all sources (no dictionary filter).
     */
    suspend fun searchWithSources(
        input: String,
        inputMode: InputMode,
        limit: Int = 50,
        context: Context
    ): List<DictionarySearchResult> = withContext(Dispatchers.IO) {
        if (input.isEmpty()) return@withContext emptyList()

        ensureInitialized(context)
        val db = database ?: throw DictionaryError.DatabaseNotAvailable

        if (!TrieService.isReady) {
            throw DictionaryError.TrieNotLoaded
        }

        val normalizedInput = InputNormalizer.normalize(input, inputMode)
        if (normalizedInput.isEmpty()) return@withContext emptyList()

        val trieKey = DictionaryConstants.triePrefix(inputMode) + normalizedInput
        val exactRowIds = TrieService.lookup(trieKey)
        val prefixRowIds = TrieService.prefixSearch(trieKey, limit * 6)
        val allRowIds = (exactRowIds.toList() + prefixRowIds.toList()).distinct()

        if (allRowIds.isEmpty()) return@withContext emptyList()

        val p = PrefHelper(context)
        val dictSnapshot = p.snapshotEnabledDictionaries()
        val enabledDicts = EnabledDictionaries(
            kautian = dictSnapshot.moe,
            taigitv = dictSnapshot.newword,
            itaigi = dictSnapshot.itaigi,
            sitbut = dictSnapshot.taiwanPlant,
            taihoa = dictSnapshot.taiHua,
            taijit = dictSnapshot.taiwanJapan,
            kungge = dictSnapshot.kungge,
            stti = dictSnapshot.stti,
            khpoo = dictSnapshot.khpoo,
            variant = dictSnapshot.variant,
            khiin = dictSnapshot.khiin,
            lkk = dictSnapshot.lkk
        )

        queryByIdsWithSources(db, allRowIds, inputMode, limit, enabledDicts)
    }

    /**
     * Query SQLite with source columns included
     */
    private fun queryByIdsWithSources(
        db: SQLiteDatabase,
        ids: List<Int>,
        inputMode: InputMode,
        limit: Int,
        enabledDicts: EnabledDictionaries
    ): List<DictionarySearchResult> {
        if (ids.isEmpty()) return emptyList()

        val romanColumn = Column.TL
        val batchSize = 500
        val results = mutableListOf<DictionarySearchResult>()

        // Build dictionary filter condition (same logic as queryByIds)
        val variantCondition = if (!enabledDicts.variant) "AND is_variant = 0 " else ""
        val khiinCondition = if (!enabledDicts.khiin) "AND khiin = 0 " else ""
        val dictConditions = mutableListOf<String>()
        if (enabledDicts.kautian) dictConditions.add("${Column.KAUTIAN} = 1")
        if (enabledDicts.taigitv) dictConditions.add("${Column.TAIGITV} = 1")
        if (enabledDicts.itaigi) dictConditions.add("${Column.ITAIGI} = 1")
        if (enabledDicts.sitbut) dictConditions.add("${Column.SITBUT} = 1")
        if (enabledDicts.taihoa) dictConditions.add("${Column.TAIHOA} = 1")
        if (enabledDicts.taijit) dictConditions.add("${Column.TAIJIT} = 1")
        if (enabledDicts.kungge) dictConditions.add("${Column.KUNGGE} = 1")
        if (enabledDicts.stti) dictConditions.add("${Column.STTI} = 1")
        if (enabledDicts.khpoo) dictConditions.add("${Column.KHPOO} = 1")
        if (enabledDicts.lkk) dictConditions.add("${Column.LKK} = 1")
        dictConditions.add("dev = 1")
        val dictWhereCondition = if (enabledDicts.allEnabled()) "" else "AND (" + dictConditions.joinToString(" OR ") + ")"
        val whereCondition = variantCondition + khiinCondition + dictWhereCondition

        for (batch in ids.chunked(batchSize)) {
            val placeholders = batch.joinToString(",") { "?" }

            val sql = """
                SELECT ${Column.ID}, $romanColumn, ${Column.HANZI}, ${Column.FREQUENCY},
                       ${Column.KAUTIAN}, ${Column.TAIGITV}, ${Column.ITAIGI}, sitbut,
                       ${Column.TAIHOA}, ${Column.TAIJIT}, ${Column.KUNGGE}, ${Column.STTI},
                       ${Column.KHPOO}, khiin, ${Column.LKK}, dev
                FROM dictionary
                WHERE ${Column.ID} IN ($placeholders)
                $whereCondition
                ORDER BY ${Column.FREQUENCY} DESC
            """.trimIndent()

            val args = batch.map { it.toString() }.toTypedArray()
            val cursor = db.rawQuery(sql, args)

            cursor.use {
                while (it.moveToNext()) {
                    val id = it.getInt(0)
                    val tlRoman = it.getString(1) ?: ""
                    val hanziText = it.getString(2)
                    val hanzi = if (hanziText.isNullOrEmpty()) null else hanziText
                    val frequency = it.getInt(3)

                    // Map source boolean columns (indices 4-15)
                    val sourceEnums = listOf(
                        DictionarySource.KAUTIAN, DictionarySource.TAIGITV,
                        DictionarySource.ITAIGI, DictionarySource.SITBUT,
                        DictionarySource.TAIHOA, DictionarySource.TAIJIT,
                        DictionarySource.KUNGGE, DictionarySource.STTI,
                        DictionarySource.KHPOO, DictionarySource.KHIIN,
                        DictionarySource.LKK, DictionarySource.DEV
                    )
                    val sources = sourceEnums.filterIndexed { offset, _ ->
                        it.getInt(4 + offset) == 1
                    }

                    val roman = if (inputMode == InputMode.POJ) {
                        TaigiPhonetics.tlDisplayToPOJDisplay(tlRoman)
                    } else {
                        tlRoman
                    }

                    results.add(DictionarySearchResult(
                        id = id,
                        roman = roman,
                        tl = tlRoman,
                        hanzi = hanzi,
                        frequency = frequency,
                        sources = sources
                    ))
                }
            }
        }

        return results
            .sortedByDescending { it.frequency }
            .take(limit)
    }

    /**
     * Search dictionary by hanzi (漢字) and return results with source information.
     * Uses direct SQL LIKE query since hanzi is not indexed in the trie.
     */
    suspend fun searchByHanzi(
        input: String,
        inputMode: InputMode,
        limit: Int = 50,
        context: Context
    ): List<DictionarySearchResult> = withContext(Dispatchers.IO) {
        if (input.isEmpty()) return@withContext emptyList()

        if (BuildConfig.DEBUG) Log.d(TAG, "[HANZI-SEARCH] query='$input' limit=$limit")

        ensureInitialized(context)
        val db = database ?: throw DictionaryError.DatabaseNotAvailable

        val romanColumn = Column.TL
        val results = mutableListOf<DictionarySearchResult>()

        // Build dictionary filter condition
        val p = PrefHelper(context)
        val dictSnapshot = p.snapshotEnabledDictionaries()
        val enabledDicts = EnabledDictionaries(
            kautian = dictSnapshot.moe,
            taigitv = dictSnapshot.newword,
            itaigi = dictSnapshot.itaigi,
            sitbut = dictSnapshot.taiwanPlant,
            taihoa = dictSnapshot.taiHua,
            taijit = dictSnapshot.taiwanJapan,
            kungge = dictSnapshot.kungge,
            stti = dictSnapshot.stti,
            khpoo = dictSnapshot.khpoo,
            variant = dictSnapshot.variant,
            khiin = dictSnapshot.khiin,
            lkk = dictSnapshot.lkk
        )
        val variantCondition = if (!enabledDicts.variant) "AND is_variant = 0 " else ""
        val khiinCondition = if (!enabledDicts.khiin) "AND khiin = 0 " else ""
        val dictConditions = mutableListOf<String>()
        if (enabledDicts.kautian) dictConditions.add("${Column.KAUTIAN} = 1")
        if (enabledDicts.taigitv) dictConditions.add("${Column.TAIGITV} = 1")
        if (enabledDicts.itaigi) dictConditions.add("${Column.ITAIGI} = 1")
        if (enabledDicts.sitbut) dictConditions.add("${Column.SITBUT} = 1")
        if (enabledDicts.taihoa) dictConditions.add("${Column.TAIHOA} = 1")
        if (enabledDicts.taijit) dictConditions.add("${Column.TAIJIT} = 1")
        if (enabledDicts.kungge) dictConditions.add("${Column.KUNGGE} = 1")
        if (enabledDicts.stti) dictConditions.add("${Column.STTI} = 1")
        if (enabledDicts.khpoo) dictConditions.add("${Column.KHPOO} = 1")
        if (enabledDicts.lkk) dictConditions.add("${Column.LKK} = 1")
        dictConditions.add("dev = 1")
        val dictWhereCondition = if (enabledDicts.allEnabled()) "" else "AND (" + dictConditions.joinToString(" OR ") + ")"
        val whereCondition = variantCondition + khiinCondition + dictWhereCondition

        val sql = """
            SELECT ${Column.ID}, $romanColumn, ${Column.HANZI}, ${Column.FREQUENCY},
                   ${Column.KAUTIAN}, ${Column.TAIGITV}, ${Column.ITAIGI}, sitbut,
                   ${Column.TAIHOA}, ${Column.TAIJIT}, ${Column.KUNGGE}, ${Column.STTI},
                   ${Column.KHPOO}, khiin, ${Column.LKK}, dev
            FROM dictionary
            WHERE ${Column.HANZI} LIKE ?
            $whereCondition
            ORDER BY ${Column.FREQUENCY} DESC
            LIMIT ?
        """.trimIndent()

        val args = arrayOf("%$input%", limit.toString())
        val cursor = db.rawQuery(sql, args)

        cursor.use {
            while (it.moveToNext()) {
                val id = it.getInt(0)
                val tlRoman = it.getString(1) ?: ""
                val hanziText = it.getString(2)
                val hanzi = if (hanziText.isNullOrEmpty()) null else hanziText
                val frequency = it.getInt(3)

                // Map source boolean columns (indices 4-15)
                val sourceEnums = listOf(
                    DictionarySource.KAUTIAN, DictionarySource.TAIGITV,
                    DictionarySource.ITAIGI, DictionarySource.SITBUT,
                    DictionarySource.TAIHOA, DictionarySource.TAIJIT,
                    DictionarySource.KUNGGE, DictionarySource.STTI,
                    DictionarySource.KHPOO, DictionarySource.KHIIN,
                    DictionarySource.LKK, DictionarySource.DEV
                )
                val sources = sourceEnums.filterIndexed { offset, _ ->
                    it.getInt(4 + offset) == 1
                }

                val roman = if (inputMode == InputMode.POJ) {
                    TaigiPhonetics.tlDisplayToPOJDisplay(tlRoman)
                } else {
                    tlRoman
                }

                results.add(DictionarySearchResult(
                    id = id,
                    roman = roman,
                    tl = tlRoman,
                    hanzi = hanzi,
                    frequency = frequency,
                    sources = sources
                ))
            }
        }

        val sorted = results
            .sortedByDescending { it.frequency }
            .take(limit)

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[HANZI-SEARCH] returned ${sorted.size} results (raw=${results.size})")
            sorted.firstOrNull()?.let {
                Log.d(TAG, "[HANZI-SEARCH] first: ${it.roman} / ${it.hanzi ?: ""}")
            }
        }

        sorted
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
