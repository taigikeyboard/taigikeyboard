// Taigi autocomplete: turns the engine's span-local continuous candidates into a ranked list.
// Engine is the sole candidate source since v3.5.8 Item 13 — syllable splitting / prefix / custom
// dict / hanzi guard all live there, no platform lexicon fallback. The continuous path has no
// composing-text cell (slot 0 = candidate[0]); the inline pre-edit is that surface.

package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.settings.CandidateDisplayMode
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
     * Continuous-input candidate fetcher. Thread-agnostic: production wires
     * [com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager
     * .fetchContinuousCandidates], a read-only engine query that never
     * touches InputConnection, and invokes it on `Dispatchers.Default`.
     */
    private val continuousFetcher: suspend () -> List<RustEngineBridge.ContinuousCandidate>,
    /**
     * Live-read: `true` iff the strip renders 漢羅濫 split cells (mode ==
     * COMBINED and the layout is not TPS — TPS ignores the picker). Read
     * per fetch, never snapshotted, so a settings change takes effect on
     * the next keystroke (android-guidelines §6 live-read rule).
     */
    private val splitCombinedCellsProvider: () -> Boolean = { false },
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
            buildContinuousSuggestionsForCandidates(continuousFetcher(), splitCombinedCellsProvider())
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            logger.e(TAG, "[ERROR] autocomplete failed for: $rawInput", e)
            emptyList()
        }
    }
}

/**
 * Whether the candidate strip renders 漢羅濫 split cells: the picker is set to
 * [CandidateDisplayMode.COMBINED] and the layout is not TPS (TPS is hanji-first
 * by construction and ignores the picker). Read per fetch, never snapshotted,
 * so a settings change takes effect on the next keystroke
 * (android-guidelines §6 live-read rule).
 */
// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Autocomplete/Services/TaigiAutocompleteService.swift
// `shouldSplitCombinedCells`. Drift causes silent divergence (one platform still splitting
// under TPS, or not splitting under 漢羅濫).
internal fun shouldSplitCombinedCells(
    candidateDisplayMode: CandidateDisplayMode,
    isTpsLayout: Boolean,
): Boolean = candidateDisplayMode == CandidateDisplayMode.COMBINED && !isTpsLayout

/**
 * Wrap a list of engine [RustEngineBridge.ContinuousCandidate] into the
 * platform `[TaigiWord]` shape with metadata sidechannel pre-populated for
 * [com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateClickHandler]
 * tap routing.
 *
 * Per `docs/engine/continuous-input-ranking.md` §10.1.2 (supersedes legacy
 * slot-0 model) + §10.3 commit contract, Continuous mode has NO
 * composing-text cell at slot 0. `candidate[0]` is the engine ranker top
 * and the tap commits the swap/TPS/both-scripts-formatted document string
 * built from `roman` / `hanzi` by the legacy formatter (clarification γ,
 * REVISED — Bug 1). The inline pre-edit (`setComposingText`) is the only
 * composing-text surface; Enter commits the pending tail via Item 3.
 *
 * v3.5.8 Phase 9 Item 6: `roman` carries `candidate.roman` (the
 * engine-rendered display romanization — TL, or POJ-display when the
 * input mode is POJ; this builder stays mode-agnostic per Item 13)
 * and `hanzi` carries `candidate.hanji` so [com.siansiansu
 * .taigikeyboard.ime.text.smartbar.SmartbarCandidateStrip] renders
 * dual-line cells (roman + hanji) on HANT/MIXED and single-line (roman
 * only) on TAILO.
 *
 * `TaigiWord.roman` (the display romanization) may diverge from
 * `additionalInfo[DISPLAY_TEXT]` (= `hanji ?? roman` per
 * `record_to_candidate`) on HANT/MIXED candidates. The tap routes through
 * [com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateClickHandler],
 * which formats the document string from `roman`/`hanzi` (legacy parity)
 * and forwards the `DISPLAY_TEXT` sidechannel as
 * `commitContinuous(canonicalText = …)` — the canonical key for
 * `user_frequency.db` + NextWord, NOT the document commit string
 * (clarification γ, REVISED — Bug 1).
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
/**
 * 漢羅濫 split (`behavioral-invariants.md` §42 second exception, desktop
 * shipped first in #666): when [splitCombinedCells] is `true`, a
 * hanji-bearing candidate emits TWO adjacent one-script cells — a 漢字 cell
 * then a 羅馬字 cell — each carrying the SAME identity sidechannels and a
 * [TaigiWord.MetadataKeys.CELL_SCRIPT] marker saying what the cell shows
 * and commits. The roman cell KEEPS `hanzi` so `TaigiWord.displayText` and
 * the 詞頻 `(displayText, canonicalTl)` pair-key stay marker-independent.
 * Hanji-less candidates emit their roman cell alone.
 *
 * BOTH scripts dedupe on the TEXT THE CELL SHOWS, first-seen (fetched order)
 * wins: a one-script cell carries nothing that could tell it from an earlier
 * cell reading the same, so a second one is a defect, not a second offer
 * (USER 2026-09-03 「相同的漢字 or 羅馬字不能重複出現」) — 重/tîng and 重/tāng draw
 * ONE 重 cell and keep both roman cells. 漢字 cells were exempt until then on
 * Core Principle #7 grounds. The two scripts keep separate keys. Every other
 * mode ([splitCombinedCells] `false`, the default) emits exactly the
 * pre-split shape — 並排's subtitle tells 重/tîng from 重/tāng.
 */
// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Autocomplete/Services/TaigiAutocompleteService.swift buildContinuousSuggestions
// and the desktop PresentedCandidate split. Drift causes silent divergence (one platform still renders the superseded one-label 濫 cell).
internal fun buildContinuousSuggestionsForCandidates(
    candidates: List<RustEngineBridge.ContinuousCandidate>,
    splitCombinedCells: Boolean = false,
): List<TaigiWord> {
    if (!splitCombinedCells) {
        return candidates.mapIndexed { index, candidate ->
            continuousWord(
                id = index + 1,
                candidate = candidate,
                hanzi = candidate.hanji?.takeIf { it.isNotEmpty() },
                cellScript = null,
            )
        }
    }

    // The §34 literal, being hanji-less and fetched first, absorbs a later
    // same roman (e.g. 台's `tâi`); 食/𤆬 share one `tsia̍h` cell; 重/tîng and
    // 重/tāng share one 重. The roman cell keeps its candidate's hanji:
    // displayText and the 詞頻 pair-key must not move (§42 — identity is
    // shared, only the marker decides the shown/committed script).
    return splitIntoSingleScriptCells(
        items = candidates,
        hanziOf = { it.hanji },
        romanOf = { it.roman },
    ) { candidate, cellScript, ordinal ->
        continuousWord(
            id = ordinal + 1,
            candidate = candidate,
            hanzi = candidate.hanji?.takeIf { it.isNotEmpty() },
            cellScript = cellScript,
        )
    }
}

/**
 * The 漢羅濫 split (§42): each item becomes a 漢字 cell (when [hanziOf] is
 * non-empty) then a 羅馬字 cell (when [romanOf] is non-null), each script
 * deduped on the text its cell shows, first-seen wins — a one-script cell
 * carries nothing that could tell it from an earlier cell reading the same
 * (USER 2026-09-03 「相同的漢字 or 羅馬字不能重複出現」). [emit] builds the cell
 * for `(item, cellScript, ordinal)`; the Continuous and NextWord builders
 * differ only in that constructor.
 */
// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Autocomplete/Services/TaigiAutocompleteService.swift splitIntoSingleScriptCells
// and the desktop PresentedCandidate split. Drift causes silent divergence (cell order or dedupe survivor differs on one platform).
internal fun <T> splitIntoSingleScriptCells(
    items: List<T>,
    hanziOf: (T) -> String?,
    romanOf: (T) -> String?,
    emit: (item: T, cellScript: String, ordinal: Int) -> TaigiWord,
): List<TaigiWord> {
    val result = ArrayList<TaigiWord>(items.size * 2)
    val seenHanjiCells = HashSet<String>()
    val seenRomanCells = HashSet<String>()
    for (item in items) {
        val hanzi = hanziOf(item)?.takeIf { it.isNotEmpty() }
        if (hanzi != null && seenHanjiCells.add(hanzi)) {
            result += emit(item, TaigiWord.MetadataKeys.CELL_SCRIPT_HANJI, result.size)
        }
        val roman = romanOf(item)
        if (roman != null && seenRomanCells.add(roman)) {
            result += emit(item, TaigiWord.MetadataKeys.CELL_SCRIPT_ROMAN, result.size)
        }
    }
    return result
}

/**
 * One continuous-candidate [TaigiWord]. The marker-less form ([cellScript]
 * `null`) is the unsplit carrier; a 濫 split cell adds its
 * [TaigiWord.MetadataKeys.CELL_SCRIPT] marker on top of the same
 * sidechannels. Synthetic [id] ≥ 1 keeps Continuous candidates outside
 * English (id ≤ -100) and NextWord (-99..-1) sentinel ranges, and clear of
 * the lexicon-path slot-0 composing-text cell (id == 0) — routing keys off
 * additionalInfo; id is defense-in-depth.
 */
private fun continuousWord(
    id: Int,
    candidate: RustEngineBridge.ContinuousCandidate,
    hanzi: String?,
    cellScript: String?,
): TaigiWord =
    TaigiWord(
        id = id,
        roman = candidate.roman,
        hanzi = hanzi,
        lengthScore = null,
        additionalInfo =
            if (cellScript == null) {
                continuousSidechannels(candidate)
            } else {
                continuousSidechannels(candidate) + (TaigiWord.MetadataKeys.CELL_SCRIPT to cellScript)
            },
    )

private fun continuousSidechannels(candidate: RustEngineBridge.ContinuousCandidate): Map<String, String> =
    mapOf(
        TaigiWord.MetadataKeys.IS_CONTINUOUS to "true",
        TaigiWord.MetadataKeys.CONSUMED_BYTES to candidate.consumedSpanEnd.toString(),
        TaigiWord.MetadataKeys.SYLLABLE_COUNT to candidate.syllableCount.toString(),
        TaigiWord.MetadataKeys.DISPLAY_TEXT to candidate.displayText,
        // R2: canonical TL identity → round-trips to
        // commitContinuous(associationTl) for the NextWord write.
        TaigiWord.MetadataKeys.CANONICAL_TL to candidate.canonicalTl,
    )
