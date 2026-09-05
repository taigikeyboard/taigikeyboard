import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Provides composing state for autocomplete without coupling to ComposingManager.
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
protocol ContinuousCandidateFetcher: AnyObject {
    func fetchContinuousCandidates() -> [RustEngineBridge.ContinuousCandidate]
}

/// Abstracts autocomplete context updates so NextWordController doesn't depend
/// on KeyboardKit's controller hierarchy. The conforming adapter (`ActionHandler`)
/// translates engine-layer `NextWordEnginePrediction` values into KeyboardKit
/// `AutocompleteSuggestion`s at the boundary.
protocol AutocompleteContextUpdater: AnyObject {
    func setNextWordPredictions(_ predictions: [RustEngineBridge.NextWordEnginePrediction])
    func resetNextWordSuggestions()
}
