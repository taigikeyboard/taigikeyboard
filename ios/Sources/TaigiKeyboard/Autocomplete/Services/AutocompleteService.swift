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
class AutocompleteService: KeyboardKit.AutocompleteService {
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

    let logger = DebugLogger(category: "AutocompleteService")

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
    func autocomplete(_ text: String) async throws -> Autocomplete.Result {
        guard !text.isEmpty, activeComposingContext() != nil else {
            return Autocomplete.Result(inputText: text, suggestions: [])
        }
        let candidates = continuousFetcher?.fetchContinuousCandidates() ?? []
        let suggestions = buildContinuousSuggestions(from: candidates)
        return Autocomplete.Result(inputText: text, suggestions: suggestions)
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
    /// Tap-0 commits `candidate[0].display_text` via `commitContinuous(...)`
    /// (clarification γ: canonical `display_text`, NOT a roman-with-spaces
    /// visual form). The inline pre-edit (`markedText`) is the only
    /// composing-text surface; Enter commits the pending tail via Item 3's
    /// `Phase::Continuous` `Intent::CommitRaw` arm.
    ///
    /// Item 6 dual-line render: `text` / `title` carry `c.roman` (TL
    /// romanization) and `subtitle` carries `c.hanji`, so the cell renders
    /// dual-line on HANT/MIXED and single-line on TAILO.
    ///
    /// `additionalInfo` carries `consumedBytes` + `syllableCount` (decimal
    /// strings) plus `displayText` (engine-supplied raw value = `hanji ??
    /// roman` per `record_to_candidate`) so
    /// `ActionHandler.handleSuggestionSelection` can route the tap to
    /// `composingManager.commitContinuous(...)` with the engine byte offsets
    /// AND the canonical commit string. `suggestion.text` (= roman) may
    /// diverge from `additionalInfo["displayText"]` (= hanji ?? roman) on
    /// HANT/MIXED — Tap-0 must commit the sidechannel value, not the visual
    /// roman form (clarification γ). All three keys are strict-required at
    /// the consumer; a missing sidechannel drops the tap (Item 4 fork F2=A —
    /// no `?? suggestion.text` fallback that would commit the visual roman
    /// form). The sidechannel also defends against TPS layout's
    /// `CandidateCellHelper.suggestionToHandle` rewriting `suggestion.text`
    /// via `tlNumericToTPS` when the subtitle is nil/empty (Codex PR #257
    /// r3214912627). NextWord uses the same `displayText` key convention.
    ///
    /// `subtitle` collapses present-empty `c.hanji == ""` to `nil` so a wire
    /// defect (producer emitted `Some("")` instead of `None` for a TAILO
    /// record) renders single-line rather than as an empty hanji line.
    /// Whitespace-only hanji is passed through unchanged — engine invariant
    /// is `hanji = DictionaryRecord.hanzi` (real CJK text). The bridge decode
    /// layer defends the inverse case (empty `c.roman` → falls back to
    /// `displayText`), so the builder trusts both fields as
    /// non-empty-when-meaningful.
    // 中文: text/title 用 c.roman、subtitle 用 c.hanji,候選列 dual-line render;
    // 中文: displayText sidechannel 嚴格必須(commit 走它,不走 visual 化的 roman)。
    internal func buildContinuousSuggestions(
        from candidates: [RustEngineBridge.ContinuousCandidate],
    ) -> [Autocomplete.Suggestion] {
        candidates.map { c in
            let hanji = c.hanji
            let subtitle = (hanji?.isEmpty == false) ? hanji : nil
            return Autocomplete.Suggestion(
                text: c.roman,
                title: c.roman,
                subtitle: subtitle,
                additionalInfo: [
                    "isContinuous": "true",
                    "consumedBytes": String(c.consumedSpanEnd),
                    "syllableCount": String(c.syllableCount),
                    "displayText": c.displayText,
                ],
            )
        }
    }
}
