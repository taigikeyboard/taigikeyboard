// 中文: 字典查詢編排器 — bootstrap RustEngineBridge.install + custom-dict 寫路徑 + Tab3 字典查詢協調。
// 中文: 流程:Hanzi guard short-circuit → custom dict → system dict(Rust lexicon::search)→ ranking。

package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.engine.LexiconBridge
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.Outcome
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Dictionary search orchestrator.
 *
 * Search flow:
 * 1. **D-8 hanzi guard** — `inputType is InputType.Hanzi` short-circuits to
 *    `[]` BEFORE custom-dict / system-dict. Pinned by
 *    `INVARIANT_LEX_HANZI_GUARD` (`docs/architecture/behavioral-invariants.md` §14).
 * 2. Custom-dict lookup (user-added entries, highest priority).
 * 3. System-dict query through `LexiconBridge.search` (Rust engine handles
 *    trie, binary-reader filter, TPS er↔or expansion atomically).
 * 4. `RustEngineBridge.processCandidates` dedups + ranks + (TPS-gated)
 *    display-dedup in one FFI round-trip into `engine/ranking/`.
 *
 * Owned by `CompositionRoot`; collaborators injected through the ctor.
 * Engine state is installed at app startup via `LexiconBridge.install(...)`
 * from `AppInitializer`.
 */
class LexiconService(
    appContext: Context,
    private val logger: LoggerBackend,
    private val customDict: CustomDictionaryService,
    private val userFreq: UserFrequencyService,
) {
    private val appContext: Context = appContext.applicationContext

    companion object {
        private const val TAG = "LexiconService"
    }

    /**
     * Search the dictionary.
     *
     * @param input Search query (preprocessed, may be lowercased for search).
     * @param inputType Input classification — hanzi / roman with or without tone.
     * @param inputMode POJ or TL.
     * @param limit Max number of results.
     * @param settings Engine-facing settings view; falls back to a fresh
     * `PrefHelper(appContext)` when null. Only the non-null `settings`
     * drives the TPS display-dedup gate.
     */
    suspend fun search(
        input: String,
        inputType: InputType,
        inputMode: InputMode = InputMode.POJ,
        limit: Int = DictionaryConstants.DEFAULT_SEARCH_LIMIT,
        settings: EngineSettings? = null,
    ): Outcome<List<TaigiWord>, DictionaryError> {
        if (input.isEmpty()) return Outcome.Success(emptyList())
        // D-8 hard guard: hanzi inputs short-circuit before any reader is touched.
        // See `behavioral-invariants.md` §14.
        if (inputType is InputType.Hanzi) return Outcome.Success(emptyList())
        // Cold-start gate: install runs on Application's IO scope; without
        // this await, queries can race ahead and the bridge swallows
        // engine-not-initialized into empty results (Codex r3173440132).
        if (!CompositionRoot.shared(appContext).awaitLexiconReady()) {
            return Outcome.Success(emptyList())
        }

        return withContext(Dispatchers.IO) {
            val searchStart = System.currentTimeMillis()
            val activeSettings: EngineSettings = settings ?: PrefHelper(appContext)
            val toggles = LexiconBridge.DictionaryToggles.from(activeSettings)
            val filterBitmask = LexiconBridge.dictionaryFilters(toggles).dictionaryFilterBitmask

            try {
                val customWords = lookupCustomDictionary(input, activeSettings)
                val systemWords = querySystemDictionaries(input, inputMode, limit, filterBitmask, activeSettings)
                val merged = customWords + systemWords

                val sortStart = System.currentTimeMillis()
                val result = processCandidates(merged, input, inputMode, settings)
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
     * Phase 1: custom user dictionary by prefix. Returns early if the
     * custom-dict toggle is off.
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
     * Phase 2: system dictionary through the Rust shared-core lexicon
     * engine. `LexiconBridge.search` runs trie lookup + binary-reader
     * filter + TPS er↔or expansion internally per audit D-1 + D-2.
     */
    private fun querySystemDictionaries(
        input: String,
        inputMode: InputMode,
        limit: Int,
        filterBitmask: UInt,
        settings: EngineSettings,
    ): List<TaigiWord> {
        // Android `InputMode` has no TPS case (only POJ / TL / ENGLISH);
        // ENGLISH falls back to TL because the lexicon engine never receives
        // English-mode queries on the autocomplete path. Aligning the
        // InputMode enum across iOS / Android is a separate scope.
        val bridgeMode = when (inputMode) {
            InputMode.POJ -> LexiconBridge.LexiconInputMode.POJ
            InputMode.TL -> LexiconBridge.LexiconInputMode.TL
            InputMode.ENGLISH -> LexiconBridge.LexiconInputMode.TL
        }
        val rows = LexiconBridge.search(
            input = input,
            inputType = LexiconBridge.LexiconInputType.ROMAN_WITH_TONE,
            inputMode = bridgeMode,
            limit = limit.toUInt(),
            tpsOrMappedToER = settings.isTpsOrMappedToER,
            enabledSourcesBitmask = filterBitmask,
        )
        return rows.map { row ->
            val roman = if (inputMode == InputMode.POJ) RustEngineBridge.tlToPoj(row.roman) else row.roman
            TaigiWord(
                id = row.id.toInt(),
                roman = roman,
                hanzi = row.hanzi,
                lengthScore = row.lengthScore,
                sourceBitmask = row.sourceBitmask?.toInt(),
            )
        }
    }

    /**
     * Phase 3+4: hand the merged candidate list to the Rust ranking
     * pipeline. Atomic FFI roundtrip into `engine/ranking/`.
     */
    private suspend fun processCandidates(
        merged: List<TaigiWord>,
        input: String,
        inputMode: InputMode,
        settings: EngineSettings?,
    ): List<TaigiWord> {
        val normalizedInput = RustEngineBridge.normalizeInput(input)
        val wordTexts = merged.map { it.displayText }.distinct()
        val frequencyData = userFreq.frequencyDataBatch(wordTexts)
        return RustEngineBridge.processCandidates(
            raw = merged,
            normalizedInput = normalizedInput,
            tpsDedupEnabled = settings?.inputMode == "tps",
            frequencyData = frequencyData,
            nowMs = System.currentTimeMillis(),
        )
    }

    /**
     * Search with source metadata (Dictionary tab exploration).
     *
     * `filterBitmask` is resolved by the caller (Dictionary tab VM) once per query
     * via `LexiconBridge.dictionaryFilters(...)` and reused for retag —
     * keeping mask and badge filter on the same snapshot per Codex
     * pre-impl BLOCK 6.
     */
    suspend fun searchWithSources(
        input: String,
        inputMode: InputMode,
        filterBitmask: UInt,
        limit: Int = 50,
    ): Outcome<List<DictionarySearchResult>, DictionaryError> {
        if (input.isEmpty()) return Outcome.Success(emptyList())
        if (!CompositionRoot.shared(appContext).awaitLexiconReady()) {
            return Outcome.Success(emptyList())
        }

        return withContext(Dispatchers.IO) {
            try {
                val rows = bridgeSearchByHanziOrRoman(input, inputMode, limit, filterBitmask, isCJK = false)
                Outcome.Success(rowsToSearchResults(rows, inputMode, limit))
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                logger.e(TAG, "[SOURCES] Query failed", e)
                Outcome.Failure(DictionaryError.QueryExecutionFailed(e.message ?: "Unknown error"))
            }
        }
    }

    /** Search by hanzi prefix (Dictionary tab exploration). */
    suspend fun searchByHanzi(
        input: String,
        inputMode: InputMode,
        filterBitmask: UInt,
        limit: Int = 50,
    ): Outcome<List<DictionarySearchResult>, DictionaryError> {
        if (input.isEmpty()) return Outcome.Success(emptyList())
        if (!CompositionRoot.shared(appContext).awaitLexiconReady()) {
            return Outcome.Success(emptyList())
        }

        return withContext(Dispatchers.IO) {
            logger.debug(TAG) { "[HANZI-SEARCH] query='$input' limit=$limit" }
            try {
                val rows = bridgeSearchByHanziOrRoman(input, inputMode, limit, filterBitmask, isCJK = true)
                val results = rowsToSearchResults(rows, inputMode, limit)
                if (BuildConfig.DEBUG) {
                    logger.d(TAG, "[HANZI-SEARCH] returned ${results.size} results")
                }
                Outcome.Success(results)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                logger.e(TAG, "[HANZI-SEARCH] Query failed", e)
                Outcome.Failure(DictionaryError.QueryExecutionFailed(e.message ?: "Unknown error"))
            }
        }
    }

    private fun bridgeSearchByHanziOrRoman(
        input: String,
        inputMode: InputMode,
        limit: Int,
        filterBitmask: UInt,
        isCJK: Boolean,
    ): List<LexiconBridge.Row> {
        // Android `InputMode` has no TPS case (only POJ / TL / ENGLISH);
        // ENGLISH falls back to TL because the lexicon engine never receives
        // English-mode queries on the autocomplete path. Aligning the
        // InputMode enum across iOS / Android is a separate scope.
        val bridgeMode = when (inputMode) {
            InputMode.POJ -> LexiconBridge.LexiconInputMode.POJ
            InputMode.TL -> LexiconBridge.LexiconInputMode.TL
            InputMode.ENGLISH -> LexiconBridge.LexiconInputMode.TL
        }
        return if (isCJK) {
            LexiconBridge.searchByHanzi(query = input, inputMode = bridgeMode, limit = limit.toUInt(), enabledSourcesBitmask = filterBitmask)
        } else {
            LexiconBridge.searchWithSources(input = input, inputMode = bridgeMode, limit = limit.toUInt(), enabledSourcesBitmask = filterBitmask)
        }
    }

    private fun rowsToSearchResults(
        rows: List<LexiconBridge.Row>,
        inputMode: InputMode,
        limit: Int,
    ): List<DictionarySearchResult> {
        return rows.map { row ->
            val roman = if (inputMode == InputMode.POJ) RustEngineBridge.tlToPoj(row.roman) else row.roman
            val bitmask = row.sourceBitmask?.toInt() ?: 0
            DictionarySearchResult(
                id = row.id.toInt(),
                roman = roman,
                tl = row.roman,
                hanzi = row.hanzi,
                frequency = row.lengthScore ?: 0,
                sources = LexiconBitmask.sourcesFromBitmask(bitmask),
            )
        }.take(limit)
    }
}
