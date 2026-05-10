// 中文: iOS 端組字管理器 — Rust 組字引擎與 KeyboardKit / SwiftUI 之間的薄包裝。
// 中文: 引擎狀態(phase / rawInput / selectedCandidateIndex)由 Rust 端持有,這裡只做 mirror + effect dispatch。

import Combine
import Foundation

/// Minimal write-only view of the composing-context state that the keyboard
/// extension needs updated when composing starts/stops. KeyboardKit's
/// `KeyboardContext` conforms via `KeyboardContext+Composing` so this
/// wrapper stays Foundation-only.
// 中文: 組字 context 的最小可寫 protocol — 只暴露 isComposingText 一個欄位。
// 中文: KeyboardContext 透過 KeyboardContext+Composing 來符合此 protocol。
protocol ComposingContextSink: AnyObject {
    var isComposingText: Bool { get set }
}

/// iOS platform wrapper around the Rust shared-core composing engine
/// (`engine/composing` crate, accessed through
/// `RustEngineBridge.composing*` methods).
///
/// Engine state (phase + raw input + selectedCandidateIndex) lives inside
/// the Rust singleton `EngineHandle`; this wrapper:
/// - mirrors the latest response into `@Published` properties for SwiftUI,
/// - dispatches the bridge-emitted `Effect[]` through `ComposingDelegate`
///   in proto-list order,
/// - notifies the `ComposingContextSink` once state is settled.
///
/// Lifecycle: `currentGeneration` ticks once per real input-context
/// change (per plan §4.2 + Codex P1.4). The bridge passes it on every call;
/// the engine compares against last-seen and silently drops state on
/// mismatch.
// 中文: iOS 端的組字管理器(ObservableObject)。負責三件事:
// 中文:   1) 把引擎回傳鏡射到 @Published 給 SwiftUI;
// 中文:   2) 依 proto 順序派送 Effect 給 ComposingDelegate;
// 中文:   3) 結束後通知 ComposingContextSink。
// 中文: currentGeneration 每次 input-context 切換 +1,引擎會丟掉舊 generation 的 stale 請求。
public class ComposingManager: ObservableObject, ComposingStateProvider, ContinuousCandidateFetcher {
    // MARK: - Published Mirror

    @Published public private(set) var isComposing: Bool = false
    @Published public private(set) var composingText: String = ""
    @Published public private(set) var rawInput: String = ""
    @Published public private(set) var selectedCandidateIndex: Int = -1

    // MARK: - Lifecycle Generation

    /// Bumped by `KeyboardViewController` lifecycle hooks (commit 11) when
    /// a real input-context change is detected. Engine-side generation
    /// mismatch then drops state silently before applying the next request.
    // 中文: input-context 切換時由 KeyboardViewController 生命週期 +1。
    // 中文: 引擎收到舊 generation 的請求會直接丟棄,避免 stale state 滲入。
    private var currentGeneration: UInt64 = 1

    /// `true` while the platform is dispatching effects from a self-driven
    /// commit. Suppresses redundant generation bumps from `textWillChange`
    /// firing on candidate taps / self-commits.
    // 中文: 自我送出 commit 期間設為 true,壓掉 textWillChange 觸發的多餘 generation bump。
    public internal(set) var selfCommitInProgress: Bool = false

    // MARK: - Collaborators

    private weak var contextSink: ComposingContextSink?
    weak var delegate: (any ComposingDelegate)?

    private let settingsProvider: EngineSettingsProvider
    private let logger = DebugLogger(category: "ComposingManager")

    // MARK: - Init

    init(settingsProvider: EngineSettingsProvider = SharedSettings.shared) {
        self.settingsProvider = settingsProvider
    }

    func setContextSink(_ sink: ComposingContextSink) {
        contextSink = sink
    }

    /// Called by `KeyboardViewController` lifecycle hooks (per plan §4.2)
    /// when a NEW input context is detected. Subsequent bridge calls carry
    /// the bumped generation; engine drops stale state silently.
    // 中文: 當偵測到新的 input context 時,由 KeyboardViewController 生命週期呼叫,把 generation +1。
    public func bumpGeneration() {
        currentGeneration &+= 1
    }

    // MARK: - Composing Operations

    // 中文: 從外部給定的 text 啟動一段組字。
    public func startComposing(with text: String) {
        logger.debug("[COMPOSE] fn=startComposing text='\(text)'")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingStart(
            text,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        ))
        promoteToContinuousIfEligible(settings: settings)
    }

    // 中文: 把單一字元追加到 raw input。
    public func appendCharacter(_ char: String) {
        logger.debug("[COMPOSE] fn=appendCharacter char='\(char)'")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingAppend(
            char,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        ))
        promoteToContinuousIfEligible(settings: settings)
    }

    // 中文: 追加連字號 — POJ / TL 的音節分隔符。
    public func appendHyphen() {
        logger.debug("[COMPOSE] fn=appendHyphen")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingAppendHyphen(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        ))
        promoteToContinuousIfEligible(settings: settings)
    }

    /// TPS auto-correct — preserves `selectedCandidateIndex`.
    // 中文: TPS 自動更正用 — 用 replacement 取代 raw 最後一個字元;保留 selectedCandidateIndex。
    public func replaceLastCharacter(with replacement: String) {
        logger.debug("[COMPOSE] fn=replaceLastCharacter replacement='\(replacement)'")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingReplaceLast(
            replacement,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        ))
        promoteToContinuousIfEligible(settings: settings)
    }

    // MARK: - v3.5.8 Phase 7B — Continuous-input adapters

    /// Synchronous Continuous-mode promotion fired immediately after each
    /// raw-input mutation (`startComposing` / `appendCharacter` / `appendHyphen`
    /// / `replaceLastCharacter`). Per Codex 2026-05-10 ANALYSIS-ONLY consult
    /// (Fork A modify): MUST run on the same thread frame as the triggering
    /// intent so the `EnterContinuous` request shares the caller's
    /// `currentGeneration` snapshot — the engine resets to Idle on any
    /// generation mismatch (`engine/composing/src/handle.rs:61-65`), so a
    /// delayed/async call could silently wipe newer composing state.
    /// Engine no-ops the request when `Phase::Composing { raw }` is empty or
    /// when already in `Phase::Continuous` (`engine/composing/src/transition.rs:496-502`),
    /// so unconditional issuance is safe and avoids platform-side eligibility
    /// heuristics.
    // 中文: 同步 Continuous 推進。同一執行緒 frame 內 fire,共享 caller generation
    // 中文: 快照,避免 stale 引擎重置抹掉新狀態。引擎在空 raw / 已是 Continuous 時 no-op,
    // 中文: 所以可無條件呼叫,不需要平台端啟發式判斷。
    private func promoteToContinuousIfEligible(settings: EngineSettings) {
        let transition = RustEngineBridge.composingEnterContinuous(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        )
        // EnterContinuous emits zero effects (transition.rs:514). The mirror
        // refresh keeps `composingText` in sync with the engine's preedit
        // even though no platform side-effects fire.
        apply(transition)
    }

    /// Synchronous span-local candidate query. Read-only; engine returns the
    /// current `Phase::Continuous { raw }` candidate set in score-desc order.
    /// Returns `[]` when not in Continuous phase, when no syllable inventory
    /// is installed, or when the FST returns no hits — the caller cannot
    /// distinguish these cases (Codex Risk 4: graceful degrade is OK because
    /// the lexicon path then handles the same input via its own search).
    // 中文: 同步擷取 Continuous 候選詞。只讀,non-Continuous / 無 inventory / 無命中
    // 中文: 都回傳空 []。呼叫端無需區分,fall-through 到既有 lexicon path 即可。
    public func fetchContinuousCandidates() -> [RustEngineBridge.ContinuousCandidate] {
        let settings = settingsProvider.current
        let result = RustEngineBridge.composingFetchAtPos(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        )
        // FetchAtPos.effects MUST be empty (read-only RPC, dispatch.rs:103-160).
        // apply() still runs to refresh the @Published mirror with the engine's
        // returned snapshot.
        apply(result.transition)
        return result.candidates ?? []
    }

    /// Commit one Continuous candidate. `displayText` / `consumedBytes` /
    /// `syllableCount` MUST come verbatim from a `ContinuousCandidate`
    /// returned by an immediately preceding `fetchContinuousCandidates()`
    /// call — the engine collapses to noop on UTF-8/syllable boundary
    /// violations, so caller-side validation is unnecessary
    /// (`engine/composing/src/transition.rs:613-679`).
    ///
    /// Returns an effect-backed signal so callers can gate side effects
    /// (frequency recording, auto-space) on actual commit success rather
    /// than coarse `isComposing` mirror state. Codex PR #257 r3214932308:
    /// generation mismatch can silently reset the engine to Idle in
    /// `engine/composing/src/handle.rs:61-65` BEFORE the intent runs, in
    /// which case `Intent::CommitContinuous` becomes a phase-mismatch noop
    /// — the post-call mirror flips to `isComposing=false` (engine is
    /// Idle) but no `CommitTextReplacingPreedit` effect is emitted.
    /// Without an effect-backed gate, callers would record frequency for
    /// uncommitted text and append a stray space.
    ///
    /// - Returns:
    ///   - `didCommit`: `true` iff the engine actually wrote text to the
    ///     document (`transition.effects` contains `.commitTextReplacingPreedit`).
    ///     Mid-commits and final-commits both emit this effect; noops do not.
    ///   - `didFinalCommit`: `didCommit && transition` exited Continuous.
    ///     Implies `didCommit` — invariant `didFinalCommit => didCommit`.
    ///
    /// Mid-commit emits `[CommitTextReplacingPreedit, UpdatePreedit,
    /// NextWordUpdateLastSelectedWord, PerformAutocomplete]`; final-commit
    /// (`consumedBytes >= pending.utf8.count`) emits `[CommitTextReplacingPreedit,
    /// ResetAutocomplete, ResetAutocompleteContext, NextWordWordSelected]`
    /// and exits Continuous.
    // 中文: 送出一個 Continuous 候選詞段;回傳 effect-backed (didCommit, didFinalCommit) 旗標,
    // 中文: 讓 caller 用真實 commit signal 過濾 frequency / auto-space side-effects,
    // 中文: 而不是 isComposing mirror — 後者在 generation 不對齊 silent reset 時會誤報。
    public func commitContinuous(
        displayText: String,
        consumedBytes: UInt32,
        syllableCount: UInt32,
    ) -> (didCommit: Bool, didFinalCommit: Bool) {
        logger.debug(
            "[COMPOSE] fn=commitContinuous displayLen=\(displayText.count) "
                + "consumedBytes=\(consumedBytes) syllCount=\(syllableCount)",
        )
        let settings = settingsProvider.current
        let transition = RustEngineBridge.composingCommitContinuous(
            displayText: displayText,
            consumedBytes: consumedBytes,
            syllableCount: syllableCount,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        )
        // Inspect transition BEFORE dispatching effects so we can return an
        // effect-backed signal. `applyAsSelfCommit` body inlined (3 lines)
        // for the same reason — semantics identical to the helper.
        let didCommit = transition.effects.contains { effect in
            if case .commitTextReplacingPreedit = effect { return true }
            return false
        }
        let didFinalCommit = didCommit && !transition.isComposing
        selfCommitInProgress = true
        defer { selfCommitInProgress = false }
        apply(transition)
        return (didCommit: didCommit, didFinalCommit: didFinalCommit)
    }

    /// Abort Continuous-input. Drops `Phase::Continuous`'s pending + committed
    /// list, exits to Idle, emits the standard abort effect trio
    /// (`ClearPreeditWithoutCommit` + `ResetAutocomplete` +
    /// `NextWordClearForNewComposing`). Committed segments stay in the
    /// document — earlier `CommitTextReplacingPreedit` effects already wrote
    /// them.
    /// Used by `KeyboardViewController+Setup.syncSettings` on input-mode swap
    /// (TL ↔ POJ ↔ TPS) so stale Continuous state can't leak across modes.
    // 中文: 中止 Continuous;committed segments 不回退(已在 document)。Settings inputMode
    // 中文: 切換時呼叫,確保跨模式無殘留狀態。
    public func resetContinuous() {
        logger.debug("[COMPOSE] fn=resetContinuous")
        applyAsSelfCommit(RustEngineBridge.composingResetContinuous(generation: currentGeneration))
    }

    // 中文: 退格 — 刪掉 raw input 最後一個字元。
    public func deleteBackward() {
        logger.debug("[COMPOSE] fn=deleteBackward")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingDeleteBackward(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        ))
    }

    // 中文: 把目前 derived 顯示文字送出(commit derived) — 結束組字。
    public func commitComposition() {
        logger.debug("[COMPOSE] fn=commitComposition")
        // v3.5.8 Phase 7B (Codex post-impl P1, 2026-05-10):
        // `Intent::CommitDerived` is a no-op in `Phase::Continuous`
        // (`engine/composing/tests/continuous_phase.rs:656`). Phase 7B
        // auto-promotes every active composition into Continuous, so the
        // straight CommitDerived would silently drop the user's commit. Reroute
        // through `SelectSuggestion(text: composingText)` — engine handles all
        // three phases (Idle / Composing / Continuous) by committing the text
        // and exiting to Idle. Empty preedit → fall through to the canonical
        // CommitDerived which is correctly a no-op on Idle.
        // 中文: Continuous 下 CommitDerived 引擎 noop;改走 SelectSuggestion 統一三 phase。
        let derived = composingText
        guard !derived.isEmpty else {
            let settings = settingsProvider.current
            applyAsSelfCommit(RustEngineBridge.composingCommitDerived(
                mode: settings.inputMode,
                toggles: settings.toneToggles,
                generation: currentGeneration,
            ))
            return
        }
        applyAsSelfCommit(RustEngineBridge.composingSelectSuggestion(derived, generation: currentGeneration))
    }

    // 中文: 把 raw input 直接送出(不經 derived 轉換),結束組字。
    public func commitRawInput() {
        logger.debug("[COMPOSE] fn=commitRawInput")
        // v3.5.8 Phase 7B (Codex post-impl P1, 2026-05-10):
        // `Intent::CommitRaw` is also a no-op in `Phase::Continuous`
        // (`engine/composing/tests/continuous_phase.rs:662`). Same fix shape
        // as `commitComposition`: route through SelectSuggestion with the raw
        // string so the user's "Enter at index 0 commits literal raw" gesture
        // works in all phases. Empty rawInput → fall through to canonical
        // CommitRaw (no-op on Idle).
        // 中文: 同 commitComposition fix;raw 字串走 SelectSuggestion 統一三 phase。
        let raw = rawInput
        guard !raw.isEmpty else {
            applyAsSelfCommit(RustEngineBridge.composingCommitRaw(generation: currentGeneration))
            return
        }
        applyAsSelfCommit(RustEngineBridge.composingSelectSuggestion(raw, generation: currentGeneration))
    }

    // 中文: 使用者點選候選詞時呼叫,送出 text 並結束組字。
    public func selectSuggestion(text: String) {
        logger.debug("[COMPOSE] fn=selectSuggestion len=\(text.count)")
        applyAsSelfCommit(RustEngineBridge.composingSelectSuggestion(text, generation: currentGeneration))
    }

    // 中文: 先把 preedit 送出再插入外部 text — 例如剪貼或 NextWord 觸發時用。
    public func commitPreeditThenInsertExternal(_ text: String) {
        logger.debug("[COMPOSE] fn=commitPreeditThenInsertExternal len=\(text.count)")
        let settings = settingsProvider.current
        applyAsSelfCommit(RustEngineBridge.composingCommitPreeditThenInsertExternal(
            text,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        ))
    }

    /// Commit the currently-selected candidate, given the visible candidate
    /// strings. KK-side callers pass `suggestions.map(\.text)`.
    // 中文: 把目前選中的候選詞送出。呼叫端傳入目前可見候選文字列表(KK 是 suggestions.map(\.text))。
    public func confirmSelectedCandidate(availableTexts: [String]) -> Bool {
        logger.debug("[COMPOSE] fn=confirmSelectedCandidate index=\(selectedCandidateIndex) count=\(availableTexts.count)")
        guard isComposing,
              selectedCandidateIndex >= 0,
              selectedCandidateIndex < availableTexts.count
        else { return false }
        selectSuggestion(text: availableTexts[selectedCandidateIndex])
        return true
    }

    // 中文: 重置組字狀態 — 不送出,僅清空。
    public func reset() {
        logger.debug("[COMPOSE] fn=reset")
        applyAsSelfCommit(RustEngineBridge.composingReset(generation: currentGeneration))
    }

    // 中文: 更新目前選中的候選詞 index,給鍵盤方向鍵 / 候選列點擊使用。
    public func setSelectedCandidateIndex(_ index: Int) {
        logger.debug("[COMPOSE] fn=setSelectedCandidateIndex index=\(index)")
        apply(RustEngineBridge.composingSetSelectedCandidateIndex(index, generation: currentGeneration))
    }

    // MARK: - Apply Transition (three-phase, see boundary doc §2.4)

    // 中文: apply 的自我送出版本 — 設好 selfCommitInProgress 旗標壓掉多餘 generation bump。
    private func applyAsSelfCommit(_ transition: RustEngineBridge.ComposingTransition) {
        selfCommitInProgress = true
        defer { selfCommitInProgress = false }
        apply(transition)
    }

    // 中文: 套用一次 ComposingTransition,三階段:鏡射 → 派送 effect → 通知 sink。
    private func apply(_ transition: RustEngineBridge.ComposingTransition) {
        // Phase 1 — mutate published mirror (guarded-inequality writes keep
        // idle→idle silent and avoid redundant SwiftUI invalidation).
        if isComposing != transition.isComposing { isComposing = transition.isComposing }
        if rawInput != transition.rawInput { rawInput = transition.rawInput }
        if !transition.effects.isEmpty, composingText != transition.displayText {
            composingText = transition.displayText
        }
        if selectedCandidateIndex != transition.selectedCandidateIndex {
            selectedCandidateIndex = transition.selectedCandidateIndex
        }

        // Phase 2 — execute platform effects in proto-list order.
        for effect in transition.effects {
            delegate?.execute(effect)
        }

        // Phase 3 — notify composing-context sink once state is settled.
        contextSink?.isComposingText = transition.isComposing
    }
}
