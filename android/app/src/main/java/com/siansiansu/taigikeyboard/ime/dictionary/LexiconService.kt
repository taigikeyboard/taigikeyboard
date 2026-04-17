package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream

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
            if (input.isEmpty()) return@withContext emptyList()
            // Hanzi input cannot be searched via trie (matching iOS guard)
            if (inputType is InputType.Hanzi) return@withContext emptyList()

            val initStart = System.currentTimeMillis()
            ensureInitialized(context)
            if (BuildConfig.DEBUG) Log.d("PERF", "[3a] ensureInitialized: ${System.currentTimeMillis() - initStart}ms")

            val reader = binaryReader ?: throw DictionaryError.DatabaseNotAvailable
            // 讀取搜尋設定（atomic snapshot to avoid torn reads across multiple getters）
            val prefHelper = prefs ?: PrefHelper(context)
            val enabledDicts = EnabledDictionaries.fromSnapshot(prefHelper.snapshotEnabledDictionaries())

            try {
                val customWords = lookupCustomDictionary(input, prefHelper)
                val systemWords = querySystemDictionaries(reader, input, inputMode, limit, enabledDicts, prefHelper)
                // Merge: custom words first, then system words (matching iOS)
                val merged = customWords + systemWords

                val sortStart = System.currentTimeMillis()
                val ranked = rankByFrequency(merged, input, inputMode)
                val result = applyDisplayDedup(ranked, prefs)
                if (BuildConfig.DEBUG) {
                    Log.d("PERF", "[3d] sort: ${System.currentTimeMillis() - sortStart}ms")
                    Log.d("PERF", "[3-TOTAL] LexiconService.search: ${System.currentTimeMillis() - searchStart}ms")
                }
                result
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) Log.e(TAG, "[SEARCH] Query failed", e)
                throw DictionaryError.QueryExecutionFailed(e.message ?: "Unknown error")
            }
        }

    /**
     * Phase 1: query the custom user dictionary by prefix.
     * Returns early if the custom-dict toggle is off.
     */
    private suspend fun lookupCustomDictionary(
        input: String,
        prefHelper: PrefHelper,
    ): List<TaigiWord> {
        if (!prefHelper.customDictEnabled) return emptyList()

        val isToneAware = input.any { it.isDigit() }
        val searchPrefix =
            if (isToneAware) {
                input.lowercase().replace("-", "").replace(" ", "")
            } else {
                CustomDictionaryService.generateNotone(input)
            }
        return try {
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
                    TaigiWord(id = -2, roman = entry.roman, hanzi = entry.hanzi, lengthScore = null)
                }
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) Log.w(TAG, "[SEARCH] Custom dictionary query failed: ${e.message}", e)
            emptyList()
        }
    }

    /**
     * Phase 2: query the system trie + binary reader, including the TPS
     * er↔or variant expansion when the toggle is on. Preserves
     * existingIds-based dedup with the primary result set.
     */
    private fun querySystemDictionaries(
        reader: DictionaryBinaryReader,
        input: String,
        inputMode: InputMode,
        limit: Int,
        enabledDicts: EnabledDictionaries,
        prefHelper: PrefHelper,
    ): List<TaigiWord> {
        val trieStart = System.currentTimeMillis()
        val words = searchWithTrie(reader, input, inputMode, limit, enabledDicts)
        if (BuildConfig.DEBUG) {
            Log.d(
                "PERF",
                "[3b] searchWithTrie (${words.size} results): ${System.currentTimeMillis() - trieStart}ms",
            )
        }

        // TPS ㄜ expansion: also search "or" variant when toggle is ON
        if (!(TPSConverter.containsTPS(input) && prefHelper.tpsOrMapsToER)) return words
        val tlInput = TPSConverter.toTL(input)
        if (!tlInput.contains("er")) return words

        val orVariant = tlInput.replace("er", "or")
        val orWords = searchWithTrie(reader, orVariant, inputMode, limit, enabledDicts)
        val existingIds = words.map { it.id }.toSet()
        return words + orWords.filter { it.id !in existingIds }
    }

    /**
     * Phase 3: dedup + score-based ordering. Normalizes the input once
     * for scoring and delegates ranking to CandidateProcessor.
     */
    private suspend fun rankByFrequency(
        merged: List<TaigiWord>,
        input: String,
        inputMode: InputMode,
    ): List<TaigiWord> {
        val uniqueWords = CandidateProcessor.removeDuplicates(merged)
        val normalizedInput = InputNormalizer.normalize(input, inputMode)
        return CandidateProcessor.sortByScore(uniqueWords, normalizedInput)
    }

    /**
     * Phase 4: TPS mode hides visual duplicates (same hanzi, different
     * roman). Non-TPS modes return the ranked list unchanged.
     */
    private fun applyDisplayDedup(
        ranked: List<TaigiWord>,
        prefs: PrefHelper?,
    ): List<TaigiWord> = if (prefs?.inputMode == "tps") CandidateProcessor.removeDisplayDuplicates(ranked) else ranked

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

        val rowIds = lookupRowIds(trieKey)
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
     * Trie lookup: exact match + prefix search, merged and deduplicated (no artificial limit).
     * Shared between searchWithTrie(), searchWithSources(), and searchByHanzi().
     */
    private fun lookupRowIds(trieKey: String): List<Int> {
        val exactRowIds = TrieService.lookup(trieKey)
        val prefixRowIds = TrieService.prefixSearch(trieKey)
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
            val rowIds = lookupRowIds(trieKey)
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
            val rowIds = lookupRowIds(trieKey)

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
