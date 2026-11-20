// The engine's dictionary search as `DictionarySearchService` sees it; `LexiconService` is the engine-backed one, JVM tests fake it.

package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.engine.DictionaryToggles
import com.siansiansu.taigikeyboard.engine.platformDictionarySource
import com.siansiansu.taigikeyboard.engine.proto.DictionarySearchRow

/**
 * The engine's `DictionarySearch` op as `DictionarySearchService` sees it.
 * Mirrors iOS `Lexicon/Services/LexiconClient.swift`; the shipped implementation is
 * [LexiconService], the same shape as `UserDataClient` / `EngineUserDataClient`.
 */
interface LexiconClient {
    /**
     * The rows for [query] — custom entries first, then dictionary rows — as the engine's
     * `DictionarySearch` answers them; empty when the engine did not answer.
     */
    suspend fun dictionarySearch(
        query: String,
        inputMode: String,
        toggles: DictionaryToggles,
        customDictionaryEnabled: Boolean,
        limit: Int,
    ): List<DictionarySearchResult>
}

/** One engine row as the search list draws it. Mirrors iOS `DictionarySearchResult.init(_:)`. */
internal fun DictionarySearchRow.toSearchResult(): DictionarySearchResult =
    DictionarySearchResult(
        roman = roman,
        tl = lookupTl,
        hanji = if (hasHanji()) hanji else null,
        sources = sourcesList.mapNotNull(::platformDictionarySource),
    )
