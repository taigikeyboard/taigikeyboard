// 中文: Taigi 候選詞 autocomplete service — 連續輸入引擎為唯一候選來源。
// 中文: v3.5.8 Item 13 後不再有 platform lexicon fallback;engine 內部處理所有
// 中文: 切音節 / 前綴 / 自訂詞 / hanzi guard 邏輯,平台只負責把 engine 候選
// 中文: 轉成 KeyboardKit Suggestion 列表(單向資料流,對齊 MOE tutgInputLine model)。

import Foundation
import KeyboardKit

/// 自動完成服務
///
/// 處理台語連續輸入候選詞。`autocomplete(_:)` 把 `ComposingManager`
/// 的 span-local engine 候選轉成 KeyboardKit Suggestion;engine 回空時
/// 候選列即為空(inline pre-edit 仍保留組字緩衝,Enter 由 Item 3 的
/// `Phase::Continuous` `Intent::CommitRaw` arm 提交 pending tail)。
class TaigiAutocompleteService: KeyboardKit.AutocompleteService {
    // MARK: - KeyboardKit 協議屬性

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

    // MARK: - 核心屬性

    /// Composing state provider (decoupled from ComposingManager)
    private weak var composingState: (any ComposingStateProvider)?

    /// Continuous-input candidate fetcher (decoupled from ComposingManager).
    /// Distinct protocol from `composingState` because the fetch surface is
    /// not Foundation-only (`RustEngineBridge.ContinuousCandidate`); same
    /// backing instance in practice (ComposingManager conforms to both).
    // 中文: 連續輸入 fetcher protocol。實作端與 composingState 是同一個 ComposingManager。
    private weak var continuousFetcher: (any ContinuousCandidateFetcher)?

    let logger = DebugLogger(category: "TaigiAutocompleteService")

    // MARK: - 公開介面

    // 中文: 注入組字狀態 provider(通常是 ComposingManager)。同一實例也供應
    // 中文: 連續輸入 fetch surface(ComposingManager 同時 conform 兩個 protocol)。
    func setComposingManager(_ provider: any ComposingStateProvider) {
        composingState = provider
        continuousFetcher = provider as? any ContinuousCandidateFetcher
    }

    /// 自動完成核心方法
    ///
    /// 連續輸入引擎為唯一候選來源:`ComposingManager.fetchContinuousCandidates()`
    /// 同步呼叫 `RustEngineBridge.composingFetchAtPos`,generation snapshot 與
    /// 當前 `rawInput` 一致。engine 回空 ⇒ 候選列為空(per
    /// `docs/engine/continuous-candidate-display.md` §15.4/§15.6:engine 單一
    /// 來源,inline pre-edit 才是 composing-text surface,候選列 §10.1.2 起
    /// 沒有 slot-0 cell)。同步 fetch 無 suspension window,故不需 stale-result
    /// 防護(KeyboardKit `autocomplete(_:updating:)` 的 untracked Task 只在有
    /// `await` 時才有過期風險,本路徑已無 await)。
    func autocomplete(_ text: String) async throws -> AutocompleteResult {
        guard !text.isEmpty, activeComposingContext() != nil else {
            return AutocompleteResult(inputText: text, suggestions: [])
        }
        let candidates = continuousFetcher?.fetchContinuousCandidates() ?? []
        // §42 漢羅濫 splits cells at the builder below; TPS ignores the picker
        // (hanji-first by construction), so a TPS layout always builds the
        // un-split dual-script shape regardless of the stored mode.
        // 中文: TPS 佈局不理會候選詞顯示模式,恆走未拆分的並排 shape。
        let settings = SharedSettings.shared
        let candidateDisplayMode: CandidateDisplayMode =
            settings.keyboardLayoutType == .tps ? .sideBySide : settings.candidateDisplayMode
        let suggestions = buildContinuousSuggestions(
            from: candidates,
            candidateDisplayMode: candidateDisplayMode,
        )
        return AutocompleteResult(inputText: text, suggestions: suggestions)
    }

    // MARK: - Internal

    /// 是否有作用中的組字緩衝;沒有時回 nil,呼叫端即不顯示候選詞。
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
    /// §42 漢羅濫 (`candidateDisplayMode == .combined`): each hanji-bearing
    /// candidate is emitted as TWO adjacent single-script suggestions — a
    /// 漢字 cell then its 羅馬字 cell, neither with a subtitle. The
    /// `additionalInfo["cellScript"]` marker ("hanji" | "roman") says what the
    /// cell shows and commits; the SEMANTIC sidechannels (`displayText`,
    /// `canonicalTl`, spans) are copied verbatim onto BOTH cells so 詞頻 /
    /// NextWord identity never moves. Roman cells are deduped on
    /// `(c.roman, consumedBytes)` in fetched order — first seen wins (the §34
    /// literal absorbs 台's roman; 食/𤆬 share one `tsia̍h`); 漢字 cells are
    /// never deduped. Every other mode emits the un-split shape byte-identically.
    // 中文: text/title 用 c.roman、subtitle 用 c.hanji,候選列 dual-line render;
    // 中文: Bug 1 後 displayText sidechannel = canonical key,走 canonicalText
    // 中文: (freq/NextWord);文件 commit 字串由 roman/hanji 經 legacy formatter 產生。
    // 中文: 漢羅濫 = 拆成相鄰的 漢字 cell + 羅馬字 cell(無副標題),cellScript 標記
    // 中文: 該 cell 顯示/送出的 script;semantic sidechannel 兩個 cell 皆原樣複製。
    internal func buildContinuousSuggestions(
        from candidates: [RustEngineBridge.ContinuousCandidate],
        candidateDisplayMode: CandidateDisplayMode = .sideBySide,
    ) -> [AutocompleteSuggestion] {
        guard candidateDisplayMode == .combined else {
            return candidates.map { dualScriptSuggestion(for: $0) }
        }

        // CROSS-PLATFORM INVARIANT — mirrors the desktop split (§42 second
        // exception, #666) and Android buildContinuousSuggestionsForCandidates:
        // hanji cell before its roman cell; roman-cell dedupe key = (rendered
        // roman, consumed span), first-seen wins. Drift causes silent divergence
        // (cell order or dedupe survivor differs on one platform).
        var suggestions: [AutocompleteSuggestion] = []
        var seenRomanCells = Set<RomanCellKey>()
        for c in candidates {
            let sidechannels = continuousSidechannels(for: c)
            let hanji = (c.hanji?.isEmpty == false) ? c.hanji : nil
            if let hanji {
                var hanjiInfo = sidechannels
                hanjiInfo["cellScript"] = "hanji"
                // Bracket form carrier: 括號標註 ON commits `漢字 (羅馬字)`.
                hanjiInfo["roman"] = c.roman
                suggestions.append(AutocompleteSuggestion(
                    text: hanji,
                    title: hanji,
                    subtitle: nil,
                    additionalInfo: hanjiInfo,
                ))
            }
            let romanKey = RomanCellKey(roman: c.roman, consumedBytes: c.consumedSpanEnd)
            guard seenRomanCells.insert(romanKey).inserted else { continue }
            var romanInfo = sidechannels
            romanInfo["cellScript"] = "roman"
            if let hanji {
                romanInfo["hanji"] = hanji
            }
            suggestions.append(AutocompleteSuggestion(
                text: c.roman,
                title: c.roman,
                subtitle: nil,
                additionalInfo: romanInfo,
            ))
        }
        return suggestions
    }

    /// 濫 roman-cell dedupe key: same rendered roman over the same consumed
    /// span reads identically, so only the first (fetched order) is listed.
    private struct RomanCellKey: Hashable {
        let roman: String
        let consumedBytes: UInt32
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
