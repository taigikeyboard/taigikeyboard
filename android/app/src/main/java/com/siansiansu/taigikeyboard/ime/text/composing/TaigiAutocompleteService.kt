// 中文: 台語 autocomplete 服務:把連續輸入引擎的 span-local 候選轉成排序後的候選清單。
// 中文: v3.5.8 Item 13 後 engine 為唯一候選來源 — 不再有 platform lexicon fallback;
// 中文: 切音節 / 前綴 / 自訂詞 / hanzi guard 全在 engine 內處理(對齊 MOE tutgInputLine
// 中文: 單向資料流)。Continuous path(§10.1.2 supersedes)沒有 composing-text cell,
// 中文: slot 0 = candidate[0];inline pre-edit 才是 composing-text surface。

package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import kotlinx.coroutines.CancellationException

/**
 * Taigi autocomplete service.
 *
 * Turns the Continuous-input engine's span-local candidates into the
 * platform `[TaigiWord]` candidate list. The engine is the single
 * candidate source (v3.5.8 Item 13 retired the platform lexicon
 * fallback): an empty engine result yields an empty strip — the inline
 * pre-edit (`InputConnection.setComposingText`) is the only
 * composing-text surface and Enter commits the pending tail via Item 3.
 *
 * Mode-agnostic after Item 13 (the engine owns input-mode handling), so
 * `CandidateUpdateCoordinator` creates one instance and reuses it for the
 * IME session lifetime.
 */
class TaigiAutocompleteService(
    private val logger: LoggerBackend,
    /**
     * Continuous-input candidate fetcher. Caller MUST hop to the IME main
     * thread before invoking [com.siansiansu.taigikeyboard.ime.text.composing
     * .ComposingManager.fetchContinuousCandidates] — the fetch's
     * `applyTransition` writes to InputConnection.
     */
    private val continuousFetcher: suspend () -> List<RustEngineBridge.ContinuousCandidate>,
) {
    companion object {
        private const val TAG = "TaigiAutocompleteService"
    }

    suspend fun autocomplete(
        rawInput: String,
        displayText: String,
    ): List<TaigiWord> {
        if (rawInput.isEmpty() || displayText.isEmpty()) {
            return emptyList()
        }

        logger.debug(TAG) { "[INPUT] rawInput='$rawInput', displayText='$displayText'" }

        return try {
            buildContinuousSuggestionsForCandidates(continuousFetcher())
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            logger.e(TAG, "[ERROR] autocomplete failed for: $rawInput", e)
            emptyList()
        }
    }
}

/**
 * Wrap a list of engine [RustEngineBridge.ContinuousCandidate] into the
 * platform `[TaigiWord]` shape with metadata sidechannel pre-populated for
 * [com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateClickHandler]
 * tap routing.
 *
 * Per `docs/engine/continuous-input-ranking.md` §10.1.2 (supersedes legacy
 * slot-0 model) + §10.3 commit contract, Continuous mode has NO
 * composing-text cell at slot 0. `candidate[0]` is the engine ranker top
 * and tap-0 commits `candidate[0].display_text` via `commitContinuous`
 * (clarification γ). The inline pre-edit (`setComposingText`) is the only
 * composing-text surface; Enter commits the pending tail via Item 3.
 *
 * v3.5.8 Phase 9 Item 6: `roman` carries `candidate.roman` (TL
 * romanization) and `hanzi` carries `candidate.hanji` so [com.siansiansu
 * .taigikeyboard.ime.text.smartbar.SmartbarCandidateStrip] renders
 * dual-line cells (roman + hanji) on HANT/MIXED and single-line (roman
 * only) on TAILO.
 *
 * `TaigiWord.roman` (= TL romanization) may diverge from
 * `additionalInfo[DISPLAY_TEXT]` (= `hanji ?? roman` per
 * `record_to_candidate`) on HANT/MIXED candidates. Tap-0 routes through
 * [com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateClickHandler]
 * which commits the sidechannel value (clarification γ — canonical commit
 * string, NOT visual roman form).
 *
 * `hanzi` collapses present-empty `candidate.hanji == ""` to `null` via
 * `takeIf { it.isNotEmpty() }` so a wire defect (producer emitted
 * `Some("")` instead of `None` for a TAILO record) renders as single-line
 * rather than as an empty hanji line. Whitespace-only hanji is passed
 * through unchanged — engine invariant is `hanji =
 * DictionaryRecord.hanzi` (real CJK text). The bridge decode layer
 * defends the inverse case (empty `candidate.roman` → falls back to
 * `displayText`), so the builder trusts both fields as
 * non-empty-when-meaningful.
 *
 * Top-level so the contract is unit-testable without instantiating
 * collaborators.
 */
// 中文: Item 6 — roman 用 c.roman、hanzi 用 c.hanji,候選列 dual-line render;
// 中文: DISPLAY_TEXT sidechannel 嚴格必須(commit 走它,不走 visual 化的 roman)。
internal fun buildContinuousSuggestionsForCandidates(
    candidates: List<RustEngineBridge.ContinuousCandidate>,
): List<TaigiWord> =
    candidates.mapIndexed { index, candidate ->
        TaigiWord(
            // Synthetic id ≥ 1 keeps Continuous candidates outside English
            // (id ≤ -100) and NextWord (-99..-1) sentinel ranges, and clear
            // of the lexicon-path slot-0 composing-text cell (id == 0).
            // Routing keys off additionalInfo — id is defense-in-depth.
            id = index + 1,
            roman = candidate.roman,
            hanzi = candidate.hanji?.takeIf { it.isNotEmpty() },
            lengthScore = null,
            additionalInfo = mapOf(
                TaigiWord.MetadataKeys.IS_CONTINUOUS to "true",
                TaigiWord.MetadataKeys.CONSUMED_BYTES to candidate.consumedSpanEnd.toString(),
                TaigiWord.MetadataKeys.SYLLABLE_COUNT to candidate.syllableCount.toString(),
                TaigiWord.MetadataKeys.DISPLAY_TEXT to candidate.displayText,
            ),
        )
    }
