package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream
import java.text.Normalizer

/**
 * Service for querying Taigi dictionary
 *
 * 查詢流程：
 * 1. 使用 Trie 前綴匹配取得候選 rowid
 * 2. 使用 DictionaryBinaryReader 讀取 binary mmap 格式
 * 3. 使用 bitmask 過濾詞庫來源
 * 4. 按 frequency 排序
 *
 * Thread-safe singleton with lazy initialization
 */
object LexiconService {
    private const val TAG = "LexiconService"

    @Volatile private var binaryReader: DictionaryBinaryReader? = null

    @Volatile private var isInitialized = false
    private val initMutex = Mutex()

    /**
     * Search for words in the dictionary
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
        prefs: PrefHelper? = null,
    ): List<TaigiWord> =
        withContext(Dispatchers.IO) {
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

            val reader = binaryReader ?: throw DictionaryError.DatabaseNotAvailable

            // 讀取搜尋設定（atomic snapshot to avoid torn reads across multiple getters）
            val prefHelper = prefs ?: PrefHelper(context)
            val enabledDicts = EnabledDictionaries.fromSnapshot(prefHelper.snapshotEnabledDictionaries())

            try {
                // Query custom dictionary by prefix (highest priority, matching iOS)
                val customWords =
                    if (prefHelper.customDictEnabled) {
                        val isToneAware = input.any { it.isDigit() }
                        val searchPrefix =
                            if (isToneAware) {
                                input.lowercase().replace("-", "").replace(" ", "")
                            } else {
                                CustomDictionaryService.generateNotone(input)
                            }
                        try {
                            CustomDictionaryService
                                .search(prefix = searchPrefix, isToneAware = isToneAware, limit = 20)
                                .also { entries ->
                                    if (BuildConfig.DEBUG) {
                                        Log.d(
                                            TAG,
                                            "[SEARCH] customDict prefix='$searchPrefix' toneAware=$isToneAware results=${entries.size}",
                                        )
                                    }
                                }.map { entry ->
                                    TaigiWord(
                                        id = -2,
                                        roman = entry.roman,
                                        hanzi = entry.hanzi,
                                        lengthScore = null,
                                    )
                                }
                        } catch (e: Exception) {
                            if (BuildConfig.DEBUG) Log.w(TAG, "[SEARCH] Custom dictionary query failed: ${e.message}", e)
                            emptyList()
                        }
                    } else {
                        emptyList()
                    }

                // 使用 Trie + Binary Reader 查詢
                val trieStart = System.currentTimeMillis()
                val words = searchWithTrie(reader, input, inputMode, limit, enabledDicts)
                if (BuildConfig.DEBUG) {
                    Log.d(
                        "PERF",
                        "[3b] searchWithTrie (${words.size} results): ${System.currentTimeMillis() - trieStart}ms",
                    )
                }

                // TPS ㄜ expansion: also search "or" variant when toggle ON
                val allSystemWords =
                    if (TPSConverter.containsTPS(input) && prefHelper.tpsOrMapsToER) {
                        val tlInput = TPSConverter.toTL(input)
                        if (tlInput.contains("er")) {
                            val orVariant = tlInput.replace("er", "or")
                            val orWords = searchWithTrie(reader, orVariant, inputMode, limit, enabledDicts)
                            val existingIds = words.map { it.id }.toSet()
                            words + orWords.filter { it.id !in existingIds }
                        } else {
                            words
                        }
                    } else {
                        words
                    }

                // Merge: custom words first, then system words (matching iOS)
                val mergedWords = customWords + allSystemWords

                val sortStart = System.currentTimeMillis()
                val uniqueWords = removeDuplicates(mergedWords)
                val normalizedInput = InputNormalizer.normalize(input, inputMode)
                val sorted = sortByScore(uniqueWords, normalizedInput)
                // TPS mode: remove visual duplicates (same hanzi, different roman)
                val result =
                    if (prefs?.inputMode == "tps") {
                        removeDisplayDuplicates(sorted)
                    } else {
                        sorted
                    }
                if (BuildConfig.DEBUG) {
                    Log.d("PERF", "[3d] sort: ${System.currentTimeMillis() - sortStart}ms")
                    Log.d("PERF", "[3-TOTAL] LexiconService.search: ${System.currentTimeMillis() - searchStart}ms")
                }
                result
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) {
                    Log.e(TAG, "[SEARCH] Query failed", e)
                }
                throw DictionaryError.QueryExecutionFailed(e.message ?: "Unknown error")
            }
        }

    /**
     * Search using Trie exact match + prefix match + binary reader
     */
    private fun searchWithTrie(
        reader: DictionaryBinaryReader,
        input: String,
        inputMode: InputMode,
        limit: Int,
        enabledDicts: EnabledDictionaries,
    ): List<TaigiWord> {
        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[SEARCH] input='$input', mode=$inputMode, limit=$limit")
        }

        if (!TrieService.isReady) {
            if (BuildConfig.DEBUG) Log.e(TAG, "[SEARCH] Trie not loaded")
            throw DictionaryError.TrieNotLoaded
        }

        val searchKey = InputNormalizer.buildSearchKey(input, inputMode)
        val normalizedInput = InputNormalizer.normalize(searchKey, inputMode)

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[NORMALIZE] '$input' -> '$searchKey' -> '$normalizedInput'")
        }

        if (normalizedInput.isEmpty()) {
            if (BuildConfig.DEBUG) Log.d(TAG, "[NORMALIZE] Empty after normalization, returning empty")
            return emptyList()
        }

        val trieKey = DictionaryConstants.triePrefix(inputMode) + normalizedInput

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[TRIE] trieKey='$trieKey', TrieService.isReady=${TrieService.isReady}, keyCount=${TrieService.getKeyCount()}")
        }

        val rowIds = lookupRowIds(trieKey, limit)
        if (rowIds.isEmpty()) {
            if (BuildConfig.DEBUG) Log.d(TAG, "[TRIE] No results for: $trieKey")
            return emptyList()
        }

        if (BuildConfig.DEBUG) Log.d(TAG, "[TRIE] Total ${rowIds.size} unique rowIds")

        // Binary reader lookup + bitmask filter
        val results = mutableListOf<TaigiWord>()
        for (id in rowIds) {
            val record = reader.record(id) ?: continue
            if (!DictionaryBinaryReader.passesFilter(record.bitmask, enabledDicts)) continue

            val roman =
                if (inputMode == InputMode.POJ) {
                    TaigiPhonetics.tlDisplayToPOJDisplay(record.tl)
                } else {
                    record.tl
                }

            results.add(TaigiWord(id, roman, record.hanzi, record.frequency))
        }

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[BIN] ${results.size} words after filter")
            results.take(3).forEach { word ->
                Log.d(TAG, "[BIN]   - ${word.roman} / ${word.hanzi ?: "(no hanzi)"}")
            }
        }

        return results
            .sortedByDescending { it.lengthScore ?: 0 }
            .take(limit)
    }

    /**
     * Trie lookup: exact match + prefix search, merged and deduplicated.
     * Shared between searchWithTrie() and searchWithSources().
     */
    private fun lookupRowIds(
        trieKey: String,
        limit: Int,
    ): List<Int> {
        val exactRowIds = TrieService.lookup(trieKey)
        val prefixRowIds = TrieService.prefixSearch(trieKey, limit * 6)
        return (exactRowIds.toList() + prefixRowIds.toList()).distinct()
    }

    /**
     * Search dictionary with source information (for dictionary exploration in Tab 3).
     */
    suspend fun searchWithSources(
        input: String,
        inputMode: InputMode,
        limit: Int = 50,
        context: Context,
    ): List<DictionarySearchResult> =
        withContext(Dispatchers.IO) {
            if (input.isEmpty()) return@withContext emptyList()

            ensureInitialized(context)
            val reader = binaryReader ?: throw DictionaryError.DatabaseNotAvailable

            if (!TrieService.isReady) throw DictionaryError.TrieNotLoaded

            val normalizedInput = InputNormalizer.normalize(input, inputMode)
            if (normalizedInput.isEmpty()) return@withContext emptyList()

            val trieKey = DictionaryConstants.triePrefix(inputMode) + normalizedInput
            val rowIds = lookupRowIds(trieKey, limit)
            if (rowIds.isEmpty()) return@withContext emptyList()

            val enabledDicts = EnabledDictionaries.fromSnapshot(PrefHelper(context).snapshotEnabledDictionaries())

            buildSearchResults(reader, rowIds, inputMode, limit, enabledDicts)
        }

    /**
     * Build DictionarySearchResult list from rowIds using binary reader
     */
    private fun buildSearchResults(
        reader: DictionaryBinaryReader,
        ids: List<Int>,
        inputMode: InputMode,
        limit: Int,
        enabledDicts: EnabledDictionaries,
    ): List<DictionarySearchResult> {
        val results = mutableListOf<DictionarySearchResult>()

        for (id in ids) {
            val record = reader.record(id) ?: continue
            if (!DictionaryBinaryReader.passesFilter(record.bitmask, enabledDicts)) continue

            val tlRoman = record.tl
            val roman =
                if (inputMode == InputMode.POJ) {
                    TaigiPhonetics.tlDisplayToPOJDisplay(tlRoman)
                } else {
                    tlRoman
                }

            results.add(
                DictionarySearchResult(
                    id = id,
                    roman = roman,
                    tl = tlRoman,
                    hanzi = record.hanzi,
                    frequency = record.frequency,
                    sources = DictionaryBinaryReader.sourcesFromBitmask(record.bitmask),
                ),
            )
        }

        return results
            .sortedByDescending { it.frequency }
            .take(limit)
    }

    /**
     * Search dictionary by hanzi (漢字) using trie prefix search.
     * Uses "hanzi:" prefix in trie (matching iOS searchByHanzi).
     */
    suspend fun searchByHanzi(
        input: String,
        inputMode: InputMode,
        limit: Int = 50,
        context: Context,
    ): List<DictionarySearchResult> =
        withContext(Dispatchers.IO) {
            if (input.isEmpty()) return@withContext emptyList()

            if (BuildConfig.DEBUG) Log.d(TAG, "[HANZI-SEARCH] query='$input' limit=$limit")

            ensureInitialized(context)
            val reader = binaryReader ?: throw DictionaryError.DatabaseNotAvailable

            if (!TrieService.isReady) throw DictionaryError.TrieNotLoaded

            val trieKey = DictionaryConstants.TRIE_PREFIX_HANZI + input
            val rowIds = lookupRowIds(trieKey, limit)

            if (rowIds.isEmpty()) {
                if (BuildConfig.DEBUG) Log.d(TAG, "[HANZI-SEARCH] No trie results for: $trieKey")
                return@withContext emptyList()
            }

            val enabledDicts = EnabledDictionaries.fromSnapshot(PrefHelper(context).snapshotEnabledDictionaries())
            val sorted = buildSearchResults(reader, rowIds, inputMode, limit, enabledDicts)

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[HANZI-SEARCH] returned ${sorted.size} results")
                sorted.firstOrNull()?.let {
                    Log.d(TAG, "[HANZI-SEARCH] first: ${it.roman} / ${it.hanzi ?: ""}")
                }
            }

            sorted
        }

    // --- Initialization ---

    /**
     * Ensure trie and binary reader are initialized
     */
    private suspend fun ensureInitialized(context: Context) {
        if (isInitialized) return

        initMutex.withLock {
            if (isInitialized) return

            try {
                // Copy binary assets to filesDir
                copyAssetsIfNeeded(context)

                // Initialize Trie
                val trieLoaded = TrieService.init(context)
                if (!trieLoaded) {
                    if (BuildConfig.DEBUG) Log.w(TAG, "[INIT] Trie initialization failed")
                }

                // Initialize binary reader
                val binFile = File(context.filesDir, DictionaryConstants.DICT_BIN_NAME)
                binaryReader = DictionaryBinaryReader.open(binFile)
                if (binaryReader == null) {
                    if (BuildConfig.DEBUG) Log.e(TAG, "[INIT] Failed to open dictionary.bin")
                }

                isInitialized = true
                if (BuildConfig.DEBUG) {
                    Log.i(
                        TAG,
                        "[INIT] Initialized: Trie keys=${TrieService.getKeyCount()}, " +
                            "records=${binaryReader?.recordCount ?: 0}",
                    )
                }
            } catch (e: Exception) {
                close()
                if (BuildConfig.DEBUG) Log.e(TAG, "[INIT] Initialization failed", e)
                throw e
            }
        }
    }

    /**
     * Copy dictionary binary files from assets to filesDir if app version changed.
     * Copies: dictionary.bin, association.bin (for NextWordService)
     * dictionary.trie is copied by TrieService.
     */
    private fun copyAssetsIfNeeded(context: Context) {
        val versionFile = File(context.filesDir, "dictionary_app_version.txt")
        val currentAppVersion = BuildConfig.VERSION_CODE
        val lastCopiedVersion =
            if (versionFile.exists()) {
                versionFile.readText().trim().toIntOrNull() ?: 0
            } else {
                0
            }

        val needsCopy = currentAppVersion > lastCopiedVersion

        val filesToCopy =
            listOf(
                DictionaryConstants.DICT_BIN_NAME,
                DictionaryConstants.ASSOC_BIN_NAME,
            )

        for (fileName in filesToCopy) {
            val destFile = File(context.filesDir, fileName)
            if (needsCopy || !destFile.exists()) {
                try {
                    context.assets.open(fileName).use { input ->
                        FileOutputStream(destFile).use { output ->
                            input.copyTo(output)
                        }
                    }
                    if (BuildConfig.DEBUG) {
                        Log.i(TAG, "[COPY] $fileName (${destFile.length()} bytes)")
                    }
                } catch (e: Exception) {
                    if (BuildConfig.DEBUG) Log.e(TAG, "[COPY] Failed to copy $fileName", e)
                    throw DictionaryError.DatabaseConnectionFailed("Failed to copy $fileName: ${e.message}")
                }
            }
        }

        if (needsCopy) {
            versionFile.writeText(currentAppVersion.toString())
            if (BuildConfig.DEBUG) {
                Log.i(TAG, "[UPDATE] Dictionary assets updated from v$lastCopiedVersion to v$currentAppVersion")
            }
        }
    }

    // --- Scoring & Sorting (unchanged) ---

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

    private fun removeDisplayDuplicates(words: List<TaigiWord>): List<TaigiWord> {
        val seenHanzi = mutableSetOf<String>()
        return words.filter { word ->
            val hanzi = word.hanzi
            if (hanzi.isNullOrEmpty()) {
                true
            } else {
                seenHanzi.add(hanzi)
            }
        }
    }

    private fun calculateScore(
        word: TaigiWord,
        normalizedInput: String,
        frequencyData: UserFrequencyService.FrequencyData,
    ): Int {
        val candidateBase = romanToBase(word.roman)
        val inputBase = inputToBase(normalizedInput)

        val cappedUserFreq = minOf(frequencyData.count, 100)
        val userFreqScore = cappedUserFreq * 100

        val currentTime = System.currentTimeMillis()
        val oneHourMillis = 60 * 60 * 1000L
        val recencyBonus =
            if (frequencyData.lastUsedMillis > 0 &&
                (currentTime - frequencyData.lastUsedMillis) < oneHourMillis
            ) {
                200
            } else {
                0
            }

        val exactBonus = if (candidateBase == inputBase) 100 else 0
        val completionPenalty = if (candidateBase != inputBase) -1000 else 0

        val inputLen = maxOf(inputBase.length, 1)
        val candidateLen = maxOf(candidateBase.length, 1)
        val matchRatio = minOf(inputLen, candidateLen).toDouble() / maxOf(inputLen, candidateLen).toDouble()
        val closenessBonus = (matchRatio * 500).toInt()

        val baseFreqScore = (word.lengthScore ?: 0) / 10

        return userFreqScore + recencyBonus + exactBonus + closenessBonus + baseFreqScore + completionPenalty
    }

    private fun romanToBase(roman: String): String {
        val noHyphens = roman.replace("-", "").replace(" ", "")
        val withNasal = noHyphens.replace("\u207F", "nn").replace("\u1D3A", "nn")
        val nfd = Normalizer.normalize(withNasal, Normalizer.Form.NFD)
        val withOo = nfd.replace("\u0358", "o")
        return withOo
            .filter {
                Character.getType(it) != Character.NON_SPACING_MARK.toInt()
            }.filter { !it.isDigit() }
            .lowercase()
    }

    private fun inputToBase(normalizedInput: String): String = normalizedInput.filter { !it.isDigit() }.lowercase()

    private suspend fun sortByScore(
        words: List<TaigiWord>,
        normalizedInput: String,
    ): List<TaigiWord> =
        withContext(Dispatchers.IO) {
            try {
                val wordTexts = words.map { it.displayText }.distinct()
                val frequencyDataMap = UserFrequencyService.frequencyDataBatch(wordTexts)

                val sorted =
                    words
                        .map { word ->
                            val freqData =
                                frequencyDataMap[word.displayText]
                                    ?: UserFrequencyService.FrequencyData(0, 0)
                            word to calculateScore(word, normalizedInput, freqData)
                        }.sortedByDescending { it.second }

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

    private fun logScoreDetails(
        sorted: List<Pair<TaigiWord, Int>>,
        normalizedInput: String,
        frequencyDataMap: Map<String, UserFrequencyService.FrequencyData>,
    ) {
        val inputBase = inputToBase(normalizedInput)
        val currentTime = System.currentTimeMillis()
        val oneHourMillis = 60 * 60 * 1000L

        for ((word, total) in sorted) {
            val freq =
                frequencyDataMap[word.displayText]
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

            if (BuildConfig.DEBUG) {
                Log.d(
                    TAG,
                    "[SCORE] input='$normalizedInput' | ${word.roman} $hanzi: user=$userFreqScore recency=$recency exact=$exact close=$closeness base=$base completion=$completion total=$total",
                )
            }
        }
    }

    /**
     * Close resources (call in service cleanup)
     */
    fun close() {
        binaryReader = null
        isInitialized = false
        if (BuildConfig.DEBUG) {
            Log.i(TAG, "[CLOSE] Resources released")
        }
    }
}
