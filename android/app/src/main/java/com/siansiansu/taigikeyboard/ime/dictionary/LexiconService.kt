package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.engine.DictionaryToggles
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.dictionarySearch
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * The engine-backed [LexiconClient]: the `DictionarySearch` op behind
 * `DictionarySearchService` (dictionary search). The keyboard candidate path does
 * not route through here — the Continuous-input engine is the single candidate
 * source (`docs/engine/continuous-candidate-display.md` §15.4).
 *
 * Owned by `CompositionRoot`, which passes its lexicon readiness gate. Engine state is
 * installed at app startup via `RustEngineBridge.lexiconInstall(...)` from `AppInitializer`.
 */
class LexiconService(
    private val awaitLexiconReady: suspend () -> Boolean,
) : LexiconClient {
    override suspend fun dictionarySearch(
        query: String,
        inputMode: String,
        toggles: DictionaryToggles,
        customDictionaryEnabled: Boolean,
        limit: Int,
    ): List<DictionarySearchResult> {
        // A search during bootstrap waits for the dictionary rather than answering custom rows
        // only; a failed install still searches (the engine then answers the custom rows).
        awaitLexiconReady()
        // The op waits for the user data to finish opening — off the main thread.
        return withContext(Dispatchers.IO) {
            RustEngineBridge
                .dictionarySearch(query, inputMode, toggles, customDictionaryEnabled, limit)
                ?.map { it.toSearchResult() }
                ?: emptyList()
        }
    }
}
