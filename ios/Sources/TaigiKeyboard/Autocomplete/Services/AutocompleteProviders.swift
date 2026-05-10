// 中文: Autocomplete 的依賴抽象層 — 把 ComposingManager / NextWordController / ActionHandler
// 中文: 解耦,讓 AutocompleteService 不直接綁 KeyboardKit 控制器階層。

import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Provides composing state for autocomplete without coupling to ComposingManager.
// 中文: 提供組字狀態給 autocomplete 使用,實作端不必依賴 ComposingManager。
protocol ComposingStateProvider: AnyObject {
    var isComposing: Bool { get }
    var rawInput: String { get }
    var composingText: String { get }
}

/// Synchronous Continuous-input candidate fetch surface (v3.5.8 Phase 7B).
/// Kept distinct from `ComposingStateProvider` because it carries
/// `RustEngineBridge.ContinuousCandidate` and is therefore not Foundation-only.
/// ComposingManager conforms; the implementation calls
/// `RustEngineBridge.composingFetchAtPos` synchronously on the calling thread,
/// guaranteeing the fetch shares the caller's generation snapshot.
// 中文: 連續輸入候選詞同步擷取介面。實作端同步呼叫 FFI 確保 generation 一致。
protocol ContinuousCandidateFetcher: AnyObject {
    func fetchContinuousCandidates() -> [RustEngineBridge.ContinuousCandidate]
}

/// Provides selection context for autocomplete without coupling to NextWordController.
// 中文: 提供 last selected word 與 envelope generation,讓 autocomplete 做 NextWord boost。
protocol SelectionContextProvider: AnyObject {
    var lastSelectedWord: String? { get }
    /// Envelope generation owned by the NextWord platform executor (`NextWordController`)
    /// so all bridge calls within the same IME session share Rust-side state.
    /// `AutocompleteService` uses this for `nextwordBoostCandidates` to avoid
    /// spurious state resets in the singleton `EngineHandle`.
    var nextwordEnvelopeGeneration: UInt64 { get }
}

/// Abstracts autocomplete context updates so NextWordController doesn't depend
/// on KeyboardKit's controller hierarchy. The conforming adapter (`ActionHandler`)
/// translates engine-layer `NextWordEnginePrediction` values into KeyboardKit
/// `Autocomplete.Suggestion`s at the boundary.
// 中文: 把 NextWord 預測列表寫回 autocomplete UI 的抽象 protocol,實作通常是 ActionHandler 適配器。
protocol AutocompleteContextUpdater: AnyObject {
    func setNextWordPredictions(_ predictions: [RustEngineBridge.NextWordEnginePrediction])
    func resetNextWordSuggestions()
}
