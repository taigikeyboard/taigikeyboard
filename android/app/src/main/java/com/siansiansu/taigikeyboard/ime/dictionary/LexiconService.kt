package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.Outcome
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream
import java.io.IOException

/**
 * Dictionary search orchestrator.
 *
 * Search flow:
 * 1. Trie prefix match to get candidate rowids.
 * 2. `DictionaryBinaryReader` reads the mmap binary records.
 * 3. Bitmask filter keeps only user-enabled dictionaries.
 * 4. `CandidateProcessor.sortByScore` ranks by user-frequency + input affinity.
 *
 * Owned by `CompositionRoot`; collaborators are injected through the ctor.
 * [close] releases `dictionary.bin` mmap; subsequent calls re-open on demand.
 */
class LexiconService(
    appContext: Context,
    private val logger: LoggerBackend,
    private val trie: TrieService,
    private val customDict: CustomDictionaryService,
    private val userFreq: UserFrequencyService,
) {
    private val appContext: Context = appContext.applicationContext

    companion object {
        private const val TAG = "LexiconService"
    }

    @Volatile private var binaryReader: DictionaryBinaryReader? = null

    @Volatile private var isInitialized = false
    private val initMutex = Mutex()

    /**
     * Search the dictionary.
     *
     * @param input Search query (preprocessed, may be lowercased for search).
     * @param inputType Input classification — hanzi / roman with or without tone.
     * @param inputMode POJ or TL.
     * @param limit Max number of results.
     * @param settings Engine-facing settings view; falls back to a fresh
     * `PrefHelper(appContext)` when null. Only the non-null `settings`
     * drives the TPS display-dedup gate — passing null preserves the
     * legacy "skip display dedup" behavior used by callers that do not
     * own a settings reference.
     */
    suspend fun search(
        input: String,
        inputType: InputType,
        inputMode: InputMode = InputMode.POJ,
        limit: Int = DictionaryConstants.DEFAULT_SEARCH_LIMIT,
        settings: EngineSettings? = null,
    ): Outcome<List<TaigiWord>, DictionaryError> {
        if (input.isEmpty()) return Outcome.Success(emptyList())
        // Hanzi input cannot be searched via trie (matching iOS guard)
        if (inputType is InputType.Hanzi) return Outcome.Success(emptyList())

        return withContext(Dispatchers.IO) {
            val searchStart = System.currentTimeMillis()

            val initStart = System.currentTimeMillis()
            try {
                ensureInitialized()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                logger.e(TAG, "[SEARCH] Initialization failed", e)
                return@withContext Outcome.Failure(
                    DictionaryError.DatabaseConnectionFailed(e.message ?: "Unknown error"),
                )
            }
            if (BuildConfig.DEBUG) {
                logger.d("PERF", "[3a] ensureInitialized: ${System.currentTimeMillis() - initStart}ms")
            }

            val reader =
                binaryReader
                    ?: return@withContext Outcome.Failure(DictionaryError.DatabaseNotAvailable)
            if (!trie.isReady) {
                logger.e(TAG, "[SEARCH] Trie not loaded")
                return@withContext Outcome.Failure(DictionaryError.TrieNotLoaded)
            }

            val activeSettings: EngineSettings = settings ?: PrefHelper(appContext)
            val enabledDicts = EnabledDictionaries.fromSettings(activeSettings)

            try {
                val customWords = lookupCustomDictionary(input, activeSettings)
                val systemWords = querySystemDictionaries(reader, input, inputMode, limit, enabledDicts, activeSettings)
                // Merge: custom words first, then system words (matching iOS).
                val merged = customWords + systemWords

                val sortStart = System.currentTimeMillis()
                val ranked = rankByFrequency(merged, input, inputMode)
                // Display dedup fires only when the CALLER supplied settings
                // and the caller is in TPS mode — preserves prior behavior
                // where a null `prefs` skipped dedup entirely.
                val result = applyDisplayDedup(ranked, settings)
                if (BuildConfig.DEBUG) {
                    logger.d("PERF", "[3d] sort: ${System.currentTimeMillis() - sortStart}ms")
                    logger.d("PERF", "[3-TOTAL] LexiconService.search: ${System.currentTimeMillis() - searchStart}ms")
                }
                Outcome.Success(result)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                logger.e(TAG, "[SEARCH] Query failed", e)
                Outcome.Failure(DictionaryError.QueryExecutionFailed(e.message ?: "Unknown error"))
            }
        }
    }

    /**
     * Phase 1: query the custom user dictionary by prefix. Returns early
     * if the custom-dict toggle is off.
     */
    private suspend fun lookupCustomDictionary(
        input: String,
        settings: EngineSettings,
    ): List<TaigiWord> {
        if (!settings.isCustomDictEnabled) return emptyList()

        val isToneAware = input.any { it.isDigit() }
        val searchPrefix =
            if (isToneAware) {
                input.lowercase().replace("-", "").replace(" ", "")
            } else {
                CustomDictionaryDerivation.generateNotone(input)
            }
        return try {
            customDict
                .search(prefix = searchPrefix, isToneAware = isToneAware, limit = 20)
                .also { entries ->
                    logger.debug(TAG) {
                        "[SEARCH] customDict prefix='$searchPrefix' toneAware=$isToneAware results=${entries.size}"
                    }
                }.map { entry ->
                    TaigiWord(id = -2, roman = entry.roman, hanzi = entry.hanzi, lengthScore = null)
                }
        } catch (e: Exception) {
            logger.w(TAG, "[SEARCH] Custom dictionary query failed: ${e.message}", e)
            emptyList()
        }
    }

    /**
     * Phase 2: query the system trie + binary reader, including the TPS
     * er↔or variant expansion when the toggle is on. Preserves
     * `existingIds`-based dedup with the primary result set.
     */
    private fun querySystemDictionaries(
        reader: DictionaryBinaryReader,
        input: String,
        inputMode: InputMode,
        limit: Int,
        enabledDicts: EnabledDictionaries,
        settings: EngineSettings,
    ): List<TaigiWord> {
        val trieStart = System.currentTimeMillis()
        val words = searchWithTrie(reader, input, inputMode, limit, enabledDicts)
        if (BuildConfig.DEBUG) {
            logger.d(
                "PERF",
                "[3b] searchWithTrie (${words.size} results): ${System.currentTimeMillis() - trieStart}ms",
            )
        }

        // TPS ㄜ expansion: also search "or" variant when toggle is ON.
        if (!(RustEngineBridge.containsTps(input) && settings.isTpsOrMappedToER)) return words
        val tlInput = RustEngineBridge.tpsToTl(input)
        if (!tlInput.contains("er")) return words

        val orVariant = tlInput.replace("er", "or")
        val orWords = searchWithTrie(reader, orVariant, inputMode, limit, enabledDicts)
        val existingIds = words.map { it.id }.toSet()
        return words + orWords.filter { it.id !in existingIds }
    }

    /**
     * Phase 3: dedup + score-based ordering. Normalizes the input once for
     * scoring, batches user-frequency lookups, and delegates ranking to
     * `CandidateProcessor`. Mirrors iOS: user-frequency batch fetch lives
     * at the caller of the pure scoring function, not inside it.
     */
    private suspend fun rankByFrequency(
        merged: List<TaigiWord>,
        input: String,
        inputMode: InputMode,
    ): List<TaigiWord> {
        val uniqueWords = CandidateProcessor.removeDuplicates(merged)
        val normalizedInput = InputNormalizer.normalize(input, inputMode)
        val wordTexts = uniqueWords.map { it.displayText }.distinct()
        val frequencyData = userFreq.frequencyDataBatch(wordTexts)
        return CandidateProcessor.sortByScore(
            words = uniqueWords,
            normalizedInput = normalizedInput,
            frequencyData = frequencyData,
            currentTime = System.currentTimeMillis(),
            logger = logger,
        )
    }

    /**
     * Phase 4: TPS mode hides visual duplicates (same hanzi, different
     * roman). Non-TPS modes return the ranked list unchanged. A null
     * [settings] argument preserves the prior behavior of skipping
     * display dedup for callers that did not pass preferences.
     */
    private fun applyDisplayDedup(
        ranked: List<TaigiWord>,
        settings: EngineSettings?,
    ): List<TaigiWord> = if (settings?.inputMode == "tps") CandidateProcessor.removeDisplayDuplicates(ranked) else ranked

    /**
     * Trie exact match + prefix match → binary reader lookup with bitmask filter.
     */
    private fun searchWithTrie(
        reader: DictionaryBinaryReader,
        input: String,
        inputMode: InputMode,
        limit: Int,
        enabledDicts: EnabledDictionaries,
    ): List<TaigiWord> {
        logger.debug(TAG) { "[SEARCH] input='$input', mode=$inputMode, limit=$limit" }

        val searchKey = InputNormalizer.buildSearchKey(input, inputMode)
        val normalizedInput = InputNormalizer.normalize(searchKey, inputMode)

        logger.debug(TAG) { "[NORMALIZE] '$input' -> '$searchKey' -> '$normalizedInput'" }

        if (normalizedInput.isEmpty()) {
            logger.d(TAG, "[NORMALIZE] Empty after normalization, returning empty")
            return emptyList()
        }

        val trieKey = DictionaryConstants.triePrefix(inputMode) + normalizedInput

        logger.debug(TAG) {
            "[TRIE] trieKey='$trieKey', TrieService.isReady=${trie.isReady}, keyCount=${trie.getKeyCount()}"
        }

        val rowIds = lookupRowIds(trieKey)
        if (rowIds.isEmpty()) {
            logger.debug(TAG) { "[TRIE] No results for: $trieKey" }
            return emptyList()
        }

        logger.debug(TAG) { "[TRIE] Total ${rowIds.size} unique rowIds" }

        val results = mutableListOf<TaigiWord>()
        for (id in rowIds) {
            val record = reader.record(id) ?: continue
            if (!DictionaryBinaryReader.passesFilter(record.bitmask, enabledDicts)) continue

            val roman =
                if (inputMode == InputMode.POJ) {
                    RustEngineBridge.tlToPoj(record.tl)
                } else {
                    record.tl
                }

            results.add(
                TaigiWord(
                    id = id,
                    roman = roman,
                    hanzi = record.hanzi,
                    lengthScore = record.frequency,
                    sourceBitmask = record.bitmask,
                ),
            )
        }

        if (BuildConfig.DEBUG) {
            logger.d(TAG, "[BIN] ${results.size} words after filter")
            results.take(3).forEach { word ->
                logger.d(TAG, "[BIN]   - ${word.roman} / ${word.hanzi ?: "(no hanzi)"}")
            }
        }

        return results
            .sortedByDescending { it.lengthScore ?: 0 }
            .take(limit)
    }

    /**
     * Trie lookup: exact match + prefix search merged and deduplicated.
     * Shared by [searchWithTrie], [searchWithSources], and [searchByHanzi].
     */
    private fun lookupRowIds(trieKey: String): List<Int> {
        val exactRowIds = trie.lookup(trieKey)
        val prefixRowIds = trie.prefixSearch(trieKey)
        return (exactRowIds.toList() + prefixRowIds.toList()).distinct()
    }

    /** Search with source metadata (tab3 dictionary exploration). */
    suspend fun searchWithSources(
        input: String,
        inputMode: InputMode,
        limit: Int = 50,
    ): Outcome<List<DictionarySearchResult>, DictionaryError> {
        if (input.isEmpty()) return Outcome.Success(emptyList())

        return withContext(Dispatchers.IO) {
            try {
                ensureInitialized()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                logger.e(TAG, "[SOURCES] Initialization failed", e)
                return@withContext Outcome.Failure(
                    DictionaryError.DatabaseConnectionFailed(e.message ?: "Unknown error"),
                )
            }

            val reader =
                binaryReader
                    ?: return@withContext Outcome.Failure(DictionaryError.DatabaseNotAvailable)
            if (!trie.isReady) return@withContext Outcome.Failure(DictionaryError.TrieNotLoaded)

            val normalizedInput = InputNormalizer.normalize(input, inputMode)
            if (normalizedInput.isEmpty()) return@withContext Outcome.Success(emptyList())

            val trieKey = DictionaryConstants.triePrefix(inputMode) + normalizedInput
            val rowIds = lookupRowIds(trieKey)
            if (rowIds.isEmpty()) return@withContext Outcome.Success(emptyList())

            val enabledDicts = EnabledDictionaries.fromSettings(PrefHelper(appContext))

            try {
                Outcome.Success(buildSearchResults(reader, rowIds, inputMode, limit, enabledDicts))
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                logger.e(TAG, "[SOURCES] Query failed", e)
                Outcome.Failure(DictionaryError.QueryExecutionFailed(e.message ?: "Unknown error"))
            }
        }
    }

    /** Build a list of `DictionarySearchResult` rows from trie rowids. */
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
                    RustEngineBridge.tlToPoj(tlRoman)
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
     * Search by hanzi (漢字) via trie prefix with `hanzi:` namespace key.
     * Matches iOS `searchByHanzi`.
     */
    suspend fun searchByHanzi(
        input: String,
        inputMode: InputMode,
        limit: Int = 50,
    ): Outcome<List<DictionarySearchResult>, DictionaryError> {
        if (input.isEmpty()) return Outcome.Success(emptyList())

        return withContext(Dispatchers.IO) {
            logger.debug(TAG) { "[HANZI-SEARCH] query='$input' limit=$limit" }

            try {
                ensureInitialized()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                logger.e(TAG, "[HANZI-SEARCH] Initialization failed", e)
                return@withContext Outcome.Failure(
                    DictionaryError.DatabaseConnectionFailed(e.message ?: "Unknown error"),
                )
            }

            val reader =
                binaryReader
                    ?: return@withContext Outcome.Failure(DictionaryError.DatabaseNotAvailable)
            if (!trie.isReady) return@withContext Outcome.Failure(DictionaryError.TrieNotLoaded)

            val trieKey = DictionaryConstants.TRIE_PREFIX_HANZI + input
            val rowIds = lookupRowIds(trieKey)

            if (rowIds.isEmpty()) {
                logger.debug(TAG) { "[HANZI-SEARCH] No trie results for: $trieKey" }
                return@withContext Outcome.Success(emptyList())
            }

            val enabledDicts = EnabledDictionaries.fromSettings(PrefHelper(appContext))

            try {
                val sorted = buildSearchResults(reader, rowIds, inputMode, limit, enabledDicts)
                if (BuildConfig.DEBUG) {
                    logger.d(TAG, "[HANZI-SEARCH] returned ${sorted.size} results")
                    sorted.firstOrNull()?.let {
                        logger.d(TAG, "[HANZI-SEARCH] first: ${it.roman} / ${it.hanzi ?: ""}")
                    }
                }
                Outcome.Success(sorted)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                logger.e(TAG, "[HANZI-SEARCH] Query failed", e)
                Outcome.Failure(DictionaryError.QueryExecutionFailed(e.message ?: "Unknown error"))
            }
        }
    }

    // --- Initialization ---

    /** Copy binary assets and open the mmap reader + trie on first use. */
    private suspend fun ensureInitialized() {
        if (isInitialized) return

        initMutex.withLock {
            if (isInitialized) return

            try {
                copyAssetsIfNeeded(appContext)

                val trieLoaded = trie.init()
                if (!trieLoaded) {
                    logger.w(TAG, "[INIT] Trie initialization failed")
                }

                val binFile = File(appContext.filesDir, DictionaryConstants.DICT_BIN_NAME)
                binaryReader = DictionaryBinaryReader.open(binFile)
                if (binaryReader == null) {
                    logger.e(TAG, "[INIT] Failed to open dictionary.bin")
                }

                isInitialized = true
                logger.i(
                    TAG,
                    "[INIT] Initialized: Trie keys=${trie.getKeyCount()}, " +
                        "records=${binaryReader?.recordCount ?: 0}",
                )
            } catch (e: Exception) {
                close()
                logger.e(TAG, "[INIT] Initialization failed", e)
                throw e
            }
        }
    }

    /**
     * Copy binary assets to `filesDir` when the app version changes.
     * Copies: `dictionary.bin`, `association.bin`. `dictionary.trie` is
     * copied by `TrieService`.
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
                    logger.i(TAG, "[COPY] $fileName (${destFile.length()} bytes)")
                } catch (e: Exception) {
                    logger.e(TAG, "[COPY] Failed to copy $fileName", e)
                    throw IOException("Failed to copy $fileName: ${e.message}", e)
                }
            }
        }

        if (needsCopy) {
            versionFile.writeText(currentAppVersion.toString())
            logger.i(TAG, "[UPDATE] Dictionary assets updated from v$lastCopiedVersion to v$currentAppVersion")
        }
    }

    /**
     * Release the mmap reader and flip the instance back to an
     * uninitialized state. Subsequent [search] calls re-run
     * [ensureInitialized] and reopen `dictionary.bin`. Does **not** close
     * `TrieService` — the trie is process-wide and owned separately.
     */
    fun close() {
        binaryReader = null
        isInitialized = false
        logger.i(TAG, "[CLOSE] Resources released")
    }
}
