// Taigi autocomplete: turns the engine's span-local continuous candidates into a ranked list.
// Engine is the sole candidate source since v3.5.8 Item 13 — syllable splitting / prefix / custom
// dict / hanji guard all live there, no platform lexicon fallback. The continuous path has no
// composing-text cell (slot 0 = candidate[0]); the inline pre-edit is that surface.

package com.siansiansu.taigikeyboard.ime.text.candidates

import com.siansiansu.taigikeyboard.engine.ContinuousCandidate
import com.siansiansu.taigikeyboard.engine.proto.CellScript
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
     * Continuous-input candidate fetcher. Thread-agnostic: production wires
     * [com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager
     * .fetchContinuousCandidates], a read-only engine query that never
     * touches InputConnection, and invokes it on `Dispatchers.Default`.
     */
    private val continuousFetcher: suspend () -> List<ContinuousCandidate>,
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

// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Candidates/Services/TaigiAutocompleteService.swift buildContinuousSuggestions
// and desktop-core `composing::presentation`. Drift causes silent divergence (one platform still renders the superseded one-label mixed cell).

/**
 * Wrap a list of engine [ContinuousCandidate] into the
 * platform `[TaigiWord]` shape with metadata sidechannel pre-populated for
 * [com.siansiansu.taigikeyboard.ime.text.candidates.CandidateClickHandler]
 * tap routing.
 *
 * Per `docs/engine/continuous-commit-and-display.md` §10.1.2 (supersedes legacy
 * slot-0 model) + §10.3 commit contract, Continuous mode has NO
 * composing-text cell at slot 0. `candidate[0]` is the engine ranker top
 * and the tap commits the document string the engine resolves from
 * `roman` / `hanji` under the live settings (R5). The inline pre-edit (`setComposingText`) is the only
 * composing-text surface; Enter commits the pending tail via Item 3.
 *
 * v3.5.8 Phase 9 Item 6: `roman` carries `candidate.roman` (the
 * engine-rendered display romanization — TL, or POJ-display when the
 * input mode is POJ; this builder stays mode-agnostic per Item 13)
 * and `TaigiWord.hanji` carries `candidate.hanji` so [TaigiCandidateStrip] renders
 * dual-line cells (roman + hanji) on HANT/MIXED and single-line (roman
 * only) on TAILO.
 *
 * `TaigiWord.roman` (the display romanization) may diverge from
 * `additionalInfo[DISPLAY_TEXT]` (= `hanji ?? roman` per
 * `record_to_candidate`) on HANT/MIXED candidates. The tap routes through
 * [com.siansiansu.taigikeyboard.ime.text.candidates.CandidateClickHandler],
 * which sends `roman`/`hanji` for the engine to resolve the document string
 * from (R5) and the `DISPLAY_TEXT` sidechannel as
 * `ContinuousPick.canonicalText` — the canonical key for
 * `user_frequency.db` + NextWord, NOT the document commit string.
 *
 * `TaigiWord.hanji` collapses present-empty `candidate.hanji == ""` to `null` via
 * `takeIf { it.isNotEmpty() }` so a wire defect (producer emitted
 * `Some("")` instead of `None` for a TAILO record) renders as single-line
 * rather than as an empty hanji line. Whitespace-only hanji is passed
 * through unchanged — engine invariant is `hanji =
 * DictionaryRecord.hanji` (real CJK text). The bridge decode layer
 * defends the inverse case (empty `candidate.roman` → falls back to
 * `displayText`), so the builder trusts both fields as
 * non-empty-when-meaningful.
 *
 * Top-level so the contract is unit-testable without instantiating
 * collaborators.
 *
 * Hanji with Romanization (`behavioral-invariants.md` §42 second exception): the engine
 * lists a hanji-bearing candidate as two rows, a Hanji cell then a romanization
 * cell, already deduped per script (`CandidateMessage.cell_script`). Each row
 * becomes one word carrying the SAME identity sidechannels plus a
 * [TaigiWord.MetadataKeys.CELL_SCRIPT] marker ([cellScriptMarker]) saying what the
 * cell shows and commits. The roman cell KEEPS `hanji` so `TaigiWord.displayText`
 * and the word-frequency `(displayText, canonicalTl)` pair-key stay
 * marker-independent. Every other display lists paired rows, emitted without a
 * marker — Pairing's subtitle tells 重/tîng from 重/tāng.
 */
internal fun buildContinuousSuggestionsForCandidates(candidates: List<ContinuousCandidate>): List<TaigiWord> =
    candidates.mapIndexed { index, candidate ->
        continuousWord(
            id = index + 1,
            candidate = candidate,
            hanji = candidate.hanji?.takeIf { it.isNotEmpty() },
            cellScript = cellScriptMarker(candidate.cellScript),
        )
    }

// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Candidates/Services/TaigiAutocompleteService.swift singleScriptCell.
// Drift causes silent divergence (one platform's split cell shows or commits the other script).

/**
 * The [TaigiWord.MetadataKeys.CELL_SCRIPT] marker for an engine §42 split row, or
 * `null` for a paired row (and an unknown value from a newer engine). The
 * Continuous and NextWord builders share it.
 */
internal fun cellScriptMarker(cellScript: CellScript): String? =
    when (cellScript) {
        CellScript.CELL_SCRIPT_HANJI -> TaigiWord.MetadataKeys.CELL_SCRIPT_HANJI
        CellScript.CELL_SCRIPT_ROMAN -> TaigiWord.MetadataKeys.CELL_SCRIPT_ROMAN
        CellScript.CELL_SCRIPT_PAIRED, CellScript.UNRECOGNIZED -> null
    }

/**
 * One continuous-candidate [TaigiWord]. The marker-less form ([cellScript]
 * `null`) is the unsplit carrier; a mixed split cell adds its
 * [TaigiWord.MetadataKeys.CELL_SCRIPT] marker on top of the same
 * sidechannels. Synthetic [id] ≥ 1 keeps Continuous candidates outside
 * English (id ≤ -100) and NextWord (-99..-1) sentinel ranges, and clear of
 * the lexicon-path slot-0 composing-text cell (id == 0) — routing keys off
 * additionalInfo; id is defense-in-depth.
 */
private fun continuousWord(
    id: Int,
    candidate: ContinuousCandidate,
    hanji: String?,
    cellScript: String?,
): TaigiWord =
    TaigiWord(
        id = id,
        roman = candidate.roman,
        hanji = hanji,
        additionalInfo =
            if (cellScript == null) {
                continuousSidechannels(candidate)
            } else {
                continuousSidechannels(candidate) + (TaigiWord.MetadataKeys.CELL_SCRIPT to cellScript)
            },
    )

private fun continuousSidechannels(candidate: ContinuousCandidate): Map<String, String> =
    mapOf(
        TaigiWord.MetadataKeys.IS_CONTINUOUS to "true",
        TaigiWord.MetadataKeys.CONSUMED_BYTES to candidate.consumedSpanEnd.toString(),
        TaigiWord.MetadataKeys.SYLLABLE_COUNT to candidate.syllableCount.toString(),
        TaigiWord.MetadataKeys.DISPLAY_TEXT to candidate.displayText,
        // R2: canonical TL identity → round-trips to
        // commitContinuous(associationTl) for the NextWord write.
        TaigiWord.MetadataKeys.CANONICAL_TL to candidate.canonicalTl,
        TaigiWord.MetadataKeys.TPS_TEXT to candidate.tpsText,
    )
