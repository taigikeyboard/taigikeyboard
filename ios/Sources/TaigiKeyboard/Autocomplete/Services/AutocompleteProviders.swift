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

/// NextWord selection-context abstraction (last-selected word + envelope
/// generation). Its autocomplete context-boost consumer was retired in
/// v3.5.8 Item 13 (engine is now the single candidate source); the
/// protocol + `NextWordController` conformance are retained as the
/// NextWord state-context model. A consumer-less protocol cleanup is a
/// deferred follow-up, intentionally out of the Item 13 retire charter.
// 中文: NextWord selection-context 抽象;autocomplete consumer 已 Item 13 退役,
// 中文: protocol 與 NextWordController conformance 保留作 NextWord 狀態模型。
protocol SelectionContextProvider: AnyObject {
    var lastSelectedWord: String? { get }
    /// Envelope generation owned by the NextWord platform executor
    /// (`NextWordController`) so all NextWord bridge calls within the same
    /// IME session share Rust-side state.
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
