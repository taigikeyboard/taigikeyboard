// Taigi candidate autocomplete service — the continuous-input engine is the only candidate source.

import Foundation
import KeyboardKit

/// §42 Hanji with Romanization split-cell wire vocabulary — shared by the builder
/// (`buildContinuousSuggestions`), the render/commit guard
/// (`CandidateCellHelper.suggestionToHandle`), and the commit script
/// (`ActionHandler.commitScript(for:)`, Continuous and prediction taps alike).
/// Values mirror Android `TaigiWord.MetadataKeys.CELL_SCRIPT*`; wire strings
/// must not drift.
enum CandidateCellScript {
    /// `additionalInfo` key carrying the cell's script marker.
    static let infoKey = "cellScript"
    /// Marker value: the cell shows and commits the Hanji.
    static let hanji = "hanji"
    /// Marker value: the cell shows and commits the bare roman.
    static let roman = "roman"
    /// `additionalInfo` key on a hanji cell carrying the roman it appends
    /// under Annotate in Brackets (`Hanji (romanization)`).
    static let bracketRomanKey = "roman"

    /// The §42 marker this suggestion commits by, or `nil` when it is not a
    /// split cell. A marker is honoured only when it is one this build knows
    /// AND the cell carries a payload to commit — a wire defect (unknown
    /// value, empty `text`) resolves to `nil` so BOTH the render guard
    /// (`CandidateCellHelper.suggestionToHandle`) and the commit
    /// (`ActionHandler.commitScript(for:)`) fall back to the unmarked lead
    /// together. Splitting that predicate is what let a
    /// defective marker skip the swap rewrite and then be re-parsed as an
    /// un-split dual-script suggestion.
    static func marker(for suggestion: AutocompleteSuggestion) -> String? {
        guard let marker = suggestion.additionalInfo[infoKey],
              marker == hanji || marker == roman,
              !suggestion.text.isEmpty
        else {
            return nil
        }
        return marker
    }
}

/// One §42 split cell as the engine listed it (`Taigi_Engine_CellScript`, Hanji with
/// Romanization — the engine splits each candidate into a Hanji row then a
/// romanization row, deduped per script): the cell's own script as `text`, no
/// subtitle, its `CandidateCellScript` marker on top of the candidate's
/// `sidechannels` (the hanji cell also carries the roman under `bracketRomanKey`
/// for Annotate in Brackets). `nil` for a `.paired` row — the caller builds its
/// dual-script suggestion. The Continuous and NextWord builders share it.
// CROSS-PLATFORM INVARIANT — mirrors android/.../candidates/TaigiAutocompleteService.kt singleScriptWord.
// Drift causes silent divergence (one platform's split cell shows or commits the other script).
func singleScriptCell(
    _ script: Taigi_Engine_CellScript,
    hanji: String?,
    roman: String?,
    sidechannels: [String: String],
) -> AutocompleteSuggestion? {
    var info = sidechannels
    let text: String
    switch script {
    case .hanji:
        text = hanji ?? ""
        info[CandidateCellScript.infoKey] = CandidateCellScript.hanji
        info[CandidateCellScript.bracketRomanKey] = roman
    case .roman:
        text = roman ?? ""
        info[CandidateCellScript.infoKey] = CandidateCellScript.roman
    case .paired, .UNRECOGNIZED:
        return nil
    }
    return AutocompleteSuggestion(text: text, title: text, subtitle: nil, additionalInfo: info)
}

/// Turns `ComposingManager`'s span-local engine candidates into KeyboardKit suggestions.
/// An empty engine result means an empty candidate row — the inline pre-edit still holds
/// the composing buffer, and Enter commits the pending tail.
class TaigiAutocompleteService: EngineFreeAutocompleteService {
    // MARK: - Core Properties

    /// Composing state provider (decoupled from ComposingManager)
    private weak var composingState: (any ComposingStateProvider)?

    /// Continuous-input candidate fetcher (decoupled from ComposingManager).
    /// Distinct protocol from `composingState` because the fetch surface is
    /// not Foundation-only (`RustEngineBridge.ContinuousCandidate`); same
    /// backing instance in practice (ComposingManager conforms to both).
    private weak var continuousFetcher: (any ContinuousCandidateFetcher)?

    let logger = DebugLogger(category: "TaigiAutocompleteService")

    // MARK: - Public API

    func setComposingManager(_ provider: any ComposingStateProvider) {
        composingState = provider
        continuousFetcher = provider as? any ContinuousCandidateFetcher
    }

    /// The engine is the only candidate source: `fetchContinuousCandidates()` calls
    /// `RustEngineBridge.composingFetchAtPos` synchronously, so the generation snapshot always
    /// matches the current `rawInput` and no stale-result guard is needed. An empty engine
    /// result means an empty candidate row (see
    /// `docs/engine/continuous-candidate-display.md` §15.4/§15.6).
    func autocomplete(text: String, in _: Locale) async throws -> AutocompleteResult {
        guard !text.isEmpty, activeComposingContext() != nil else {
            return AutocompleteResult(inputText: text, suggestions: [])
        }
        let candidates = continuousFetcher?.fetchContinuousCandidates() ?? []
        return AutocompleteResult(inputText: text, suggestions: buildContinuousSuggestions(from: candidates))
    }

    // MARK: - Internal

    /// The active composing buffer, or nil — callers show no candidates when nil.
    private func activeComposingContext() -> (rawInput: String, displayText: String)? {
        guard let composingState,
              composingState.isComposing,
              !composingState.rawInput.isEmpty
        else {
            return nil
        }
        return (composingState.rawInput, composingState.composingText)
    }

    /// v3.5.8 Phase 9 — Continuous candidate suggestions.
    ///
    /// Per `docs/engine/continuous-commit-and-display.md` §10.1.2 (supersedes legacy
    /// slot-0 model) + §10.3 commit contract: in Continuous mode the strip has
    /// NO composing-text cell. `candidate[0]` is the engine ranker top and
    /// Tap routes through `ComposingManager.commitContinuous(_:)`, which has
    /// the engine resolve the **document** string from `c.roman` / `c.hanji`
    /// under the live settings (R5) — NOT the canonical `display_text` and
    /// NOT the segmented visual form. The inline pre-edit (`markedText`) is
    /// the only composing-text surface; Enter commits the pending tail via
    /// Item 3's `Phase::Continuous` `Intent::CommitRaw` arm.
    ///
    /// Item 6 dual-line render: `text` / `title` carry `c.roman` (the
    /// engine-rendered display romanization — TL, or POJ-display when
    /// the input mode is POJ; the builder stays mode-agnostic per
    /// Item 13) and `subtitle` carries `c.hanji`, so the cell renders
    /// dual-line on HANT/MIXED and single-line on TAILO.
    ///
    /// `additionalInfo` carries `consumedBytes` + `syllableCount` (decimal
    /// strings), `roman` (the display romanization) and `displayText`
    /// (engine-supplied canonical value = `hanji ?? roman` per
    /// `record_to_candidate`) so `ActionHandler.continuousPick(for:)` builds
    /// the engine request from metadata alone: the view rewrites
    /// `suggestion.text` (`CandidateCellHelper.suggestionToHandle` — the swap,
    /// the TPS rendering; Codex PR #257 r3214912627). `displayText` is the
    /// **canonical key** the engine counts the pick under for
    /// `user_frequency.db` + NextWord (mode-independent learning, decision
    /// b) — never the document string. These keys are strict-required at the
    /// consumer; a missing sidechannel drops the tap (Item 4 fork F2=A).
    ///
    /// `subtitle` collapses present-empty `c.hanji == ""` to `nil` so a wire
    /// defect (producer emitted `Some("")` instead of `None` for a TAILO
    /// record) renders single-line rather than as an empty hanji line.
    /// Whitespace-only hanji is passed through unchanged — engine invariant
    /// is `hanji = DictionaryRecord.hanji` (real CJK text). The bridge decode
    /// layer defends the inverse case (empty `c.roman` → falls back to
    /// `displayText`), so the builder trusts both fields as
    /// non-empty-when-meaningful.
    ///
    /// §42 Hanji with Romanization: the engine lists each hanji-bearing candidate as
    /// two rows, a Hanji cell then its romanization cell, already deduped per
    /// script (`CandidateMessage.cell_script`); each becomes a single-script
    /// suggestion (`singleScriptCell`) whose SEMANTIC sidechannels
    /// (`displayText`, `canonicalTl`, spans) are the candidate's, so frequency /
    /// NextWord identity never moves. Every other display lists `.paired` rows,
    /// emitted in today's dual-script shape — Hanji–Romanization Pairing's
    /// subtitle tells 重/tîng from 重/tāng.
    func buildContinuousSuggestions(
        from candidates: [RustEngineBridge.ContinuousCandidate],
    ) -> [AutocompleteSuggestion] {
        candidates.map { c in
            singleScriptCell(
                c.cellScript,
                hanji: c.hanji,
                roman: c.roman,
                sidechannels: continuousSidechannels(for: c),
            ) ?? dualScriptSuggestion(for: c)
        }
    }

    /// The un-split dual-script suggestion every non-Hanji-with-Romanization mode emits (today's shape).
    private func dualScriptSuggestion(
        for c: RustEngineBridge.ContinuousCandidate,
    ) -> AutocompleteSuggestion {
        let hanji = c.hanji
        let subtitle = (hanji?.isEmpty == false) ? hanji : nil
        return AutocompleteSuggestion(
            text: c.roman,
            title: c.roman,
            subtitle: subtitle,
            additionalInfo: continuousSidechannels(for: c),
        )
    }

    /// Semantic sidechannels shared by every emitted cell of a candidate —
    /// identity (`displayText`, `canonicalTl`) and engine byte offsets never
    /// vary with the display split.
    private func continuousSidechannels(
        for c: RustEngineBridge.ContinuousCandidate,
    ) -> [String: String] {
        var info = [
            "isContinuous": "true",
            "consumedBytes": String(c.consumedSpanEnd),
            "syllableCount": String(c.syllableCount),
            "displayText": c.displayText,
            // R2: canonical TL identity → round-trips to
            // `ContinuousPick.associationTl` for the NextWord write.
            "canonicalTl": c.canonicalTl,
            // R5: the display romanization the engine renders the commit from
            // (`ContinuousPick.roman`) — the view may rewrite `text`. The same
            // key and value the §42 hanji cell's bracket roman rides
            // (`CandidateCellScript.bracketRomanKey`).
            "roman": c.roman,
            CandidateCellHelper.tpsTextKey: c.tpsText,
        ]
        // §50: the pick's hanji → `ContinuousPick.hanji`; absent for a
        // hanji-less candidate so the engine never learns a literal / OOV.
        if let hanji = c.hanji, !hanji.isEmpty {
            info["hanji"] = hanji
        }
        return info
    }
}
