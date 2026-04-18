import Foundation

/// Delegate protocol for text input and autocomplete operations.
///
/// Decouples `ComposingManager` from `KeyboardViewController` so that
/// `Input/` has no compile-time dependency on `_Keyboard/`.
///
/// Implemented by `KeyboardViewController` (via class body + extension).
protocol ComposingDelegate: AnyObject {
    /// Insert text at the caret (commits marked text first).
    func insertText(_ text: String)

    /// Delete one character backward at the caret.
    func deleteBackward()

    /// Set marked (composing) text, with caret placed at the end.
    func setMarkedText(_ text: String)

    /// Clear marked text and unmark the text range.
    func clearMarkedText()

    /// Reset the autocomplete suggestion list.
    func resetAutocomplete()

    /// Trigger a fresh autocomplete pass based on current composing buffer.
    func performAutocomplete()

    /// Reset the autocomplete context (including selection / history).
    func resetAutocompleteContext()
}
