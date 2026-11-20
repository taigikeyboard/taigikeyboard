// Dictionary search (Manage Dictionaries page): one settings snapshot per query, handed to the engine's DictionarySearch op.

package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.engine.DictionaryToggles
import com.siansiansu.taigikeyboard.ime.settings.EngineSettingsProvider

/**
 * Search service backing dictionary search on the Manage Dictionaries page. The engine's
 * `DictionarySearch` op owns the search — Hanji vs romanization lookup, source filter and
 * badges, POJ / TL rendering, custom entries first.
 * CROSS-PLATFORM INVARIANT — mirrors iOS `Lexicon/Services/DictionarySearchService.swift`.
 */
class DictionarySearchService(
    private val lexicon: LexiconClient,
    private val settingsProvider: EngineSettingsProvider,
) {
    /** Up to [limit] custom entries, then up to [limit] dictionary rows, for [query]. */
    suspend fun search(
        query: String,
        limit: Int = 20,
    ): List<DictionarySearchResult> {
        if (query.isEmpty()) return emptyList()
        // One snapshot per query: the toggles and the custom gate must agree.
        val settings = settingsProvider.current
        return lexicon.dictionarySearch(
            query = query,
            inputMode = settings.inputMode,
            toggles = DictionaryToggles.from(settings),
            customDictionaryEnabled = settings.isCustomDictEnabled,
            limit = limit,
        )
    }
}
