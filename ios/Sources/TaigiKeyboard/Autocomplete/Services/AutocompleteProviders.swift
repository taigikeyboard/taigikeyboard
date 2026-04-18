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
}

/// Abstracts autocomplete context updates so NextWordController doesn't depend
/// on KeyboardKit's controller hierarchy. The conforming adapter (`ActionHandler`)
/// translates engine-layer `EnginePrediction` values into KeyboardKit
/// `Autocomplete.Suggestion`s at the boundary.
protocol AutocompleteContextUpdater: AnyObject {
    func setNextWordPredictions(_ predictions: [EnginePrediction])
    func resetNextWordSuggestions()
}
