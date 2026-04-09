import Foundation

/// Provides composing state for autocomplete without coupling to ComposingManager.
protocol ComposingStateProvider: AnyObject {
    var isComposing: Bool { get }
    var rawInput: String { get }
    var composingText: String { get }
}

/// Provides selection context for autocomplete without coupling to ActionHandler.
protocol SelectionContextProvider: AnyObject {
    var lastSelectedWord: String? { get }
}
