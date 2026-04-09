import Foundation
import KeyboardKit

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

/// Abstracts autocomplete context updates so NextWordController doesn't depend on KeyboardKit's controller hierarchy.
protocol AutocompleteContextUpdater: AnyObject {
    func setNextWordSuggestions(_ suggestions: [Autocomplete.Suggestion])
    func resetNextWordSuggestions()
}
