package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.dictionaryFilters
import com.siansiansu.taigikeyboard.engine.isHanji
import com.siansiansu.taigikeyboard.engine.searchByHanji
import com.siansiansu.taigikeyboard.engine.searchWithSources
import com.siansiansu.taigikeyboard.engine.tlToPoj
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.Outcome
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * The engine-backed [LexiconClient]: the system-dictionary lookups behind
 * `DictionarySearchService` (dictionary search). The keyboard candidate path does
 * not route through here — v3.5.8 Item 13 retired the platform lexicon
 * fallback and the Continuous-input engine is the single candidate source
 * (`docs/engine/continuous-candidate-display.md` §15.4).
 *
 * Owned by `CompositionRoot`; collaborators injected through the ctor.
 * Engine state is installed at app startup via `RustEngineBridge.lexiconInstall(...)`
 * from `AppInitializer`.
 */
class LexiconService(
    appContext: Context,
    private val logger: LoggerBackend,
) : LexiconClient {
    private val appContext: Context = appContext.applicationContext

    companion object {
        private const val TAG = "LexiconService"
    }

    override fun isHanji(text: String): Boolean = RustEngineBridge.isHanji(text)

    override fun dictionaryFilters(toggles: RustEngineBridge.DictionaryToggles): RustEngineBridge.DictionaryFilters = RustEngineBridge.dictionaryFilters(toggles)

    /** Romanization search with source metadata; `filterBitmask` comes from [dictionaryFilters]. */
    override suspend fun searchWithSources(
        input: String,
        inputMode: RustEngineBridge.LexiconInputMode,
        filterBitmask: UInt,
        limit: Int,
    ): Outcome<List<DictionarySearchResult>, DictionaryError> {
        if (input.isEmpty()) return Outcome.Success(emptyList())
        if (!CompositionRoot.shared(appContext).awaitLexiconReady()) {
            return Outcome.Success(emptyList())
        }

        return withContext(Dispatchers.IO) {
            try {
                val rows = bridgeSearchByHanjiOrRoman(input, inputMode, limit, filterBitmask, isCJK = false)
                Outcome.Success(rowsToSearchResults(rows, inputMode, limit))
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                logger.e(TAG, "[SOURCES] Query failed", e)
                Outcome.Failure(DictionaryError.QueryExecutionFailed(e.message ?: "Unknown error"))
            }
        }
    }

    /** Search by hanji prefix (dictionary search exploration). */
    override suspend fun searchByHanji(
        input: String,
        inputMode: RustEngineBridge.LexiconInputMode,
        filterBitmask: UInt,
        limit: Int,
    ): Outcome<List<DictionarySearchResult>, DictionaryError> {
        if (input.isEmpty()) return Outcome.Success(emptyList())
        if (!CompositionRoot.shared(appContext).awaitLexiconReady()) {
            return Outcome.Success(emptyList())
        }

        return withContext(Dispatchers.IO) {
            logger.debug(TAG) { "[HANJI-SEARCH] query='$input' limit=$limit" }
            try {
                val rows = bridgeSearchByHanjiOrRoman(input, inputMode, limit, filterBitmask, isCJK = true)
                val results = rowsToSearchResults(rows, inputMode, limit)
                if (BuildConfig.DEBUG) {
                    logger.d(TAG, "[HANJI-SEARCH] returned ${results.size} results")
                }
                Outcome.Success(results)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                logger.e(TAG, "[HANJI-SEARCH] Query failed", e)
                Outcome.Failure(DictionaryError.QueryExecutionFailed(e.message ?: "Unknown error"))
            }
        }
    }

    private fun bridgeSearchByHanjiOrRoman(
        input: String,
        bridgeMode: RustEngineBridge.LexiconInputMode,
        limit: Int,
        filterBitmask: UInt,
        isCJK: Boolean,
    ): List<RustEngineBridge.LexiconRow> =
        if (isCJK) {
            RustEngineBridge.searchByHanji(
                query = input,
                inputMode = bridgeMode,
                limit = limit.toUInt(),
                enabledSourcesBitmask = filterBitmask,
            )
        } else {
            RustEngineBridge.searchWithSources(
                input = input,
                inputMode = bridgeMode,
                limit = limit.toUInt(),
                enabledSourcesBitmask = filterBitmask,
            )
        }

    private fun rowsToSearchResults(
        rows: List<RustEngineBridge.LexiconRow>,
        inputMode: RustEngineBridge.LexiconInputMode,
        limit: Int,
    ): List<DictionarySearchResult> =
        rows
            .map { row ->
                // The engine returns raw TL; render POJ in POJ mode. TPS keeps the TL reading (as iOS).
                val roman =
                    if (inputMode == RustEngineBridge.LexiconInputMode.POJ) RustEngineBridge.tlToPoj(row.roman) else row.roman
                DictionarySearchResult(
                    id = row.id.toInt(),
                    roman = roman,
                    tl = row.roman,
                    hanji = row.hanji,
                    sources = row.sources,
                )
            }.take(limit)
}
