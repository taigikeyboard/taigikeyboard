// Dictionary search (Manage Dictionaries page): CJK vs roman path, the custom-dictionary gate, kautian-first order, badge retag.

package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.Outcome
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.settings.EngineSettingsProvider

/**
 * Search service backing dictionary search on the Manage Dictionaries page. Mirrors iOS
 * `Lexicon/Services/DictionarySearchService.swift` step for step: a Hanji query
 * takes the CJK path, a romanization query also consults the user's custom
 * dictionary, results come kautian (MOE) first then by frequency with custom
 * hits leading, and each row's source badges are trimmed to the enabled sources.
 */
class DictionarySearchService(
    private val lexicon: LexiconClient,
    private val userData: UserDataClient,
    private val settingsProvider: EngineSettingsProvider,
    private val logger: LoggerBackend,
) {
    companion object {
        private const val TAG = "DictionarySearchService"

        /**
         * The FST family a stored `inputMode` selects. `"tps"` hits the `tps:` family
         * so a Zhuyin query finds its rows; `"english"` never reaches the lexicon
         * from the keyboard, so it reads the TL family; an unknown value falls back
         * to POJ like `InputMode.fromPrefString`.
         * CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Lexicon/Services/DictionarySearchService.swift
         * `lexiconMode(_:)`. Drift changes which index a dictionary-search query searches.
         */
        fun lexiconMode(inputMode: String): RustEngineBridge.LexiconInputMode =
            when (inputMode) {
                "tl", "english" -> RustEngineBridge.LexiconInputMode.TL
                "tps" -> RustEngineBridge.LexiconInputMode.TPS
                else -> RustEngineBridge.LexiconInputMode.POJ
            }
    }

    suspend fun search(
        query: String,
        limit: Int = 20,
    ): List<DictionarySearchResult> {
        if (query.isEmpty()) return emptyList()
        val settings = settingsProvider.current
        // Kotlin `Char.code` is 16-bit, so an inline CJK range check misses supplementary-plane
        // codepoints; the engine owns the range classification (INVARIANT_LEX_INPUT_CLASSIFICATION_HANJI_RANGE).
        val isCJK = lexicon.isHanji(query)
        logger.debug(TAG) { "[SEARCH] query='$query' isCJK=$isCJK inputMode=${settings.inputMode}" }

        // Resolve the filter bitmask + enabled-source set ONCE per query and hand both down
        // the pipeline: a toggle change mid-search must not produce a mask / badge mismatch.
        val filters = lexicon.dictionaryFilters(RustEngineBridge.DictionaryToggles.from(settings))
        val mode = lexiconMode(settings.inputMode)
        val outcome =
            if (isCJK) {
                lexicon.searchByHanji(query, mode, filters.dictionaryFilterBitmask, limit)
            } else {
                lexicon.searchWithSources(query, mode, filters.dictionaryFilterBitmask, limit)
            }
        val systemResults =
            when (outcome) {
                is Outcome.Success -> outcome.value
                is Outcome.Failure -> {
                    logger.w(TAG, "[SEARCH] ${if (isCJK) "hanji" else "roman"} path failed: ${outcome.error}")
                    return emptyList()
                }
            }
        logger.debug(TAG) { "[SEARCH] ${if (isCJK) "hanji" else "roman"} path returned ${systemResults.size} results" }

        val customResults = if (isCJK) emptyList() else lookupCustomDictionary(query, limit)
        logger.debug(TAG) { "[SEARCH] custom dictionary returned ${customResults.size} results" }

        // System rows keep the engine's order (corpus order, MOE rows first — `lexicon::search`).
        val prepared = systemResults.map { retagSources(it, filters.enabledSources) }
        return customResults + prepared
    }

    /**
     * The user's own words for [query], found the way the keyboard finds them —
     * by the key the query derives, prefix-matched (`SearchCustomEntries`).
     * CROSS-PLATFORM INVARIANT — mirrors iOS `lookupCustomDictionary`: Enable Custom
     * Dictionary gates dictionary search as it gates the keyboard.
     */
    private suspend fun lookupCustomDictionary(
        query: String,
        limit: Int,
    ): List<DictionarySearchResult> {
        val settings = settingsProvider.current
        if (!settings.isCustomDictEnabled) return emptyList()
        return try {
            userData
                .search(query = query, inputMode = settings.inputMode, limit = limit)
                .map { entry ->
                    DictionarySearchResult(
                        id = DictionarySearchResult.CUSTOM_DICT_MARKER_ID,
                        roman = entry.roman,
                        tl = entry.roman,
                        hanji = entry.hanji,
                        sources = listOf(DictionarySource.CUSTOM),
                    )
                }
        } catch (e: Exception) {
            logger.w(TAG, "[SEARCH] Custom dictionary query failed: ${e.message}", e)
            emptyList()
        }
    }

    /**
     * Drop source tags the user has disabled so badges reflect current toggles.
     * The engine's filter has already excluded rows whose sources are all disabled.
     */
    private fun retagSources(
        result: DictionarySearchResult,
        enabled: Set<DictionarySource>,
    ): DictionarySearchResult = result.copy(sources = result.sources.filter { it in enabled })
}
