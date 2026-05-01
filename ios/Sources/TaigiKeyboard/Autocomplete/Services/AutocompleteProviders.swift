import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Provides composing state for autocomplete without coupling to ComposingManager.
protocol ComposingStateProvider: AnyObject {
    var isComposing: Bool { get }
    var rawInput: String { get }
    var composingText: String { get }
}

/// Provides selection context for autocomplete without coupling to NextWordController.
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
protocol AutocompleteContextUpdater: AnyObject {
    func setNextWordPredictions(_ predictions: [RustEngineBridge.NextWordEnginePrediction])
    func resetNextWordSuggestions()
}
