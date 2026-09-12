// Taigi candidate autocomplete service — the continuous-input engine is the only candidate source.

import Foundation
import KeyboardKit

/// §42 漢羅濫 split-cell wire vocabulary — shared by the builder
/// (`buildContinuousSuggestions`), the render/commit guard
/// (`CandidateCellHelper.suggestionToHandle`), and the commit resolver
/// (`ActionHandler.markedCellCommit`). Values mirror Android
/// `TaigiWord.MetadataKeys.CELL_SCRIPT*`; wire strings must not drift.
enum CandidateCellScript {
    /// `additionalInfo` key carrying the cell's script marker.
    static let infoKey = "cellScript"
    /// Marker value: the cell shows and commits the 漢字.
    static let hanji = "hanji"
    /// Marker value: the cell shows and commits the bare roman.
    static let roman = "roman"
    /// `additionalInfo` key on a hanji cell carrying the roman it appends
    /// under 括號標註 (`漢字 (羅馬字)`).
    static let bracketRomanKey = "roman"

    /// The §42 marker this suggestion commits by, or `nil` when it is not a
    /// split cell. A marker is honoured only when it is one this build knows
    /// AND the cell carries a payload to commit — a wire defect (unknown
    /// value, empty `text`) resolves to `nil` so BOTH the render guard
    /// (`CandidateCellHelper.suggestionToHandle`) and the commit resolver
    /// (`ActionHandler.markedCellCommit`) fall back to the unmarked
    /// mode-derived path together. Splitting that predicate is what let a
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

/// Whether the candidate strip renders 漢羅濫 split cells: the picker is set
/// to 漢羅濫 and the layout is not TPS (TPS is hanji-first by construction and
/// ignores the picker). Read per fetch, never snapshotted, so a settings change
/// takes effect on the next keystroke.
// CROSS-PLATFORM INVARIANT — mirrors android/.../composing/TaigiAutocompleteService.kt
// `shouldSplitCombinedCells`. Drift causes silent divergence (one platform still
// splitting under TPS, or not splitting under 漢羅濫).
func shouldSplitCombinedCells(
    keyboardLayoutType: KeyboardLayoutType,
    candidateDisplayMode: CandidateDisplayMode,
) -> Bool {
    keyboardLayoutType != .tps && candidateDisplayMode == .combined
}

/// Turns `ComposingManager`'s span-local engine candidates into KeyboardKit suggestions.
/// An empty engine result means an empty candidate row — the inline pre-edit still holds
/// the composing buffer, and Enter commits the pending tail.
class TaigiAutocompleteService: KeyboardKit.AutocompleteService {
    // MARK: - KeyboardKit Protocol Properties

    var locale: Locale = .current

    // MARK: - KeyboardKit Learning Stubs (protocol requirement, unused by Taigi)

    // KeyboardKit's AutocompleteService protocol mandates these members without defaults.
    // Taigi keyboard does not surface ignore/learn UX, so all implementations are no-ops.

    var canIgnoreWords: Bool {
        false
    }

    var canLearnWords: Bool {
        false
    }

    var ignoredWords: [String] = []
    var learnedWords: [String] = []

    func hasIgnoredWord(_: String) -> Bool {
        false
    }

    func hasLearnedWord(_: String) -> Bool {
        false
    }

    func ignoreWord(_: String) {}
    func learnWord(_: String) {}
    func removeIgnoredWord(_: String) {}
    func unlearnWord(_: String) {}

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
    func autocomplete(_ text: String) async throws -> AutocompleteResult {
        guard !text.isEmpty, activeComposingContext() != nil else {
            return AutocompleteResult(inputText: text, suggestions: [])
        }
        let candidates = continuousFetcher?.fetchContinuousCandidates() ?? []
        // §42 漢羅濫 splits cells at the builder below; TPS ignores the picker
        // (hanji-first by construction), so a TPS layout never splits
        // regardless of the stored mode. Mirrors Android's
        // `splitCombinedCellsProvider`.
        let settings = SharedSettings.shared
        let splitCombinedCells = shouldSplitCombinedCells(
            keyboardLayoutType: settings.keyboardLayoutType,
            candidateDisplayMode: settings.candidateDisplayMode,
        )
        let suggestions = buildContinuousSuggestions(
            from: candidates,
            splitCombinedCells: splitCombinedCells,
        )
        return AutocompleteResult(inputText: text, suggestions: suggestions)
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
    /// Per `docs/engine/continuous-input-ranking.md` §10.1.2 (supersedes legacy
    /// slot-0 model) + §10.3 commit contract: in Continuous mode the strip has
    /// NO composing-text cell. `candidate[0]` is the engine ranker top and
    /// Tap routes through `commitContinuous(displayText:canonicalText:)`.
    /// Per §10.3 clarification γ (REVISED, Bug 1): the **document** string is
    /// the swap/TPS/both-scripts form `ActionHandler` derives from `c.roman`
    /// / `c.hanji` via the legacy `parseRomanAndHanzi`+`formatOutputText`
    /// helpers — NOT the canonical `display_text` and NOT the segmented
    /// visual form. The inline pre-edit (`markedText`) is the only
    /// composing-text surface; Enter commits the pending tail via Item 3's
    /// `Phase::Continuous` `Intent::CommitRaw` arm.
    ///
    /// Item 6 dual-line render: `text` / `title` carry `c.roman` (the
    /// engine-rendered display romanization — TL, or POJ-display when
    /// the input mode is POJ; the builder stays mode-agnostic per
    /// Item 13) and `subtitle` carries `c.hanji`, so the cell renders
    /// dual-line on HANT/MIXED and single-line on TAILO.
    ///
    /// `additionalInfo` carries `consumedBytes` + `syllableCount` (decimal
    /// strings) plus `displayText` (engine-supplied canonical value = `hanji
    /// ?? roman` per `record_to_candidate`) so
    /// `ActionHandler.handleSuggestionSelection` can route the tap to
    /// `composingManager.commitContinuous(...)` with the engine byte offsets
    /// AND the canonical key. Post-Bug-1 the `displayText` sidechannel is the
    /// **canonical key** forwarded as `commitContinuous(canonicalText:)` for
    /// `user_frequency.db` + NextWord (mode-independent learning, decision
    /// b) — it is no longer the document commit string. All three keys are
    /// strict-required at the consumer; a missing sidechannel drops the tap
    /// (Item 4 fork F2=A). The sidechannel also still defends against TPS
    /// layout's `CandidateCellHelper.suggestionToHandle` rewriting
    /// `suggestion.text` via `tlNumericToTPS` when the subtitle is nil/empty
    /// (Codex PR #257 r3214912627): `parseRomanAndHanzi` consumes the
    /// (possibly pre-swapped/rewritten) `Suggestion` exactly as the legacy
    /// branch does, so document parity holds while the canonical key stays
    /// clean on the sidechannel.
    ///
    /// `subtitle` collapses present-empty `c.hanji == ""` to `nil` so a wire
    /// defect (producer emitted `Some("")` instead of `None` for a TAILO
    /// record) renders single-line rather than as an empty hanji line.
    /// Whitespace-only hanji is passed through unchanged — engine invariant
    /// is `hanji = DictionaryRecord.hanzi` (real CJK text). The bridge decode
    /// layer defends the inverse case (empty `c.roman` → falls back to
    /// `displayText`), so the builder trusts both fields as
    /// non-empty-when-meaningful.
    ///
    /// §42 漢羅濫 (`splitCombinedCells == true`): each hanji-bearing
    /// candidate is emitted as TWO adjacent single-script suggestions — a
    /// 漢字 cell then its 羅馬字 cell, neither with a subtitle. The
    /// `additionalInfo["cellScript"]` marker ("hanji" | "roman") says what the
    /// cell shows and commits; the SEMANTIC sidechannels (`displayText`,
    /// `canonicalTl`, spans) are copied verbatim onto BOTH cells so 詞頻 /
    /// NextWord identity never moves.
    ///
    /// BOTH scripts dedupe on the TEXT THE CELL SHOWS, in fetched order,
    /// first-seen wins (the §34 literal absorbs 台's roman; 食/𤆬 share one
    /// `tsia̍h`; 重/tîng and 重/tāng draw ONE 重 cell and keep both roman
    /// cells). A one-script cell carries nothing that could tell it from an
    /// earlier cell reading the same, so a second one is a defect, not a
    /// second offer (USER 2026-09-03 「相同的漢字 or 羅馬字不能重複出現」); 漢字
    /// cells were exempt until then on Core Principle #7 grounds. The two
    /// scripts keep separate keys. Split OFF (the default — 並排 / 羅馬字 / TPS
    /// all resolve to `false` at the caller) emits the un-split shape
    /// byte-identically — 並排's subtitle tells 重/tîng from 重/tāng.
    func buildContinuousSuggestions(
        from candidates: [RustEngineBridge.ContinuousCandidate],
        splitCombinedCells: Bool = false,
    ) -> [AutocompleteSuggestion] {
        guard splitCombinedCells else {
            return candidates.map { dualScriptSuggestion(for: $0) }
        }

        // CROSS-PLATFORM INVARIANT — mirrors the desktop split (§42 second
        // exception, #666) and Android buildContinuousSuggestionsForCandidates:
        // hanji cell before its roman cell; each script deduped on the rendered
        // cell text, first-seen wins. Drift causes silent divergence (cell order
        // or dedupe survivor differs on one platform).
        var suggestions: [AutocompleteSuggestion] = []
        var seenHanjiCells = Set<String>()
        var seenRomanCells = Set<String>()
        for c in candidates {
            let sidechannels = continuousSidechannels(for: c)
            let hanji = (c.hanji?.isEmpty == false) ? c.hanji : nil
            if let hanji, seenHanjiCells.insert(hanji).inserted {
                var hanjiInfo = sidechannels
                hanjiInfo[CandidateCellScript.infoKey] = CandidateCellScript.hanji
                // Bracket form carrier: 括號標註 ON commits `漢字 (羅馬字)`.
                hanjiInfo[CandidateCellScript.bracketRomanKey] = c.roman
                suggestions.append(AutocompleteSuggestion(
                    text: hanji,
                    title: hanji,
                    subtitle: nil,
                    additionalInfo: hanjiInfo,
                ))
            }
            guard seenRomanCells.insert(c.roman).inserted else { continue }
            var romanInfo = sidechannels
            romanInfo[CandidateCellScript.infoKey] = CandidateCellScript.roman
            suggestions.append(AutocompleteSuggestion(
                text: c.roman,
                title: c.roman,
                subtitle: nil,
                additionalInfo: romanInfo,
            ))
        }
        return suggestions
    }

    /// The un-split dual-script suggestion every non-濫 mode emits (today's shape).
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
        [
            "isContinuous": "true",
            "consumedBytes": String(c.consumedSpanEnd),
            "syllableCount": String(c.syllableCount),
            "displayText": c.displayText,
            // R2: canonical TL identity → round-trips to
            // commitContinuous(associationTl:) for the NextWord write.
            "canonicalTl": c.canonicalTl,
        ]
    }
}
