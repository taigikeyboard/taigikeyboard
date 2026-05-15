// 中文: 字典查詢編排器 — Tab3(Dictionary tab)字典瀏覽用。
// 中文: v3.5.8 Item 13 後 keyboard 候選詞路徑已退役平台 lexicon fallback(engine
// 中文: 為唯一來源),本 service 只保留 Tab3 的 hanzi / roman 探索查詢 + source filter。

package com.siansiansu.taigikeyboard.ime.dictionary

import android.content.Context
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.engine.LexiconBridge
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.Outcome
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Dictionary search orchestrator for the Dictionary tab (Tab3).
 *
 * Provides hanzi / roman exploration queries with per-query source-tag
 * filtering for badge display. The keyboard candidate path no longer
 * routes through this service — v3.5.8 Item 13 retired the platform
 * lexicon fallback and the Continuous-input engine is now the single
 * candidate source (`docs/engine/continuous-candidate-display.md` §15.4).
 *
 * Owned by `CompositionRoot`; collaborators injected through the ctor.
 * Engine state is installed at app startup via `LexiconBridge.install(...)`
 * from `AppInitializer`.
 */
class LexiconService(
    appContext: Context,
    private val logger: LoggerBackend,
) {
    private val appContext: Context = appContext.applicationContext

    companion object {
        private const val TAG = "LexiconService"
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
            LexiconBridge.searchByHanzi(
                query = input,
                inputMode = bridgeMode,
                limit = limit.toUInt(),
                enabledSourcesBitmask = filterBitmask,
            )
        } else {
            LexiconBridge.searchWithSources(
                input = input,
                inputMode = bridgeMode,
                limit = limit.toUInt(),
                enabledSourcesBitmask = filterBitmask,
            )
        }
    }

    private fun rowsToSearchResults(
        rows: List<LexiconBridge.Row>,
        inputMode: InputMode,
        limit: Int,
    ): List<DictionarySearchResult> =
        rows
            .map { row ->
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
