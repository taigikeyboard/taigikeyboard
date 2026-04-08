import Foundation
import KeyboardKit

// MARK: - ComposingDelegate

extension KeyboardViewController: ComposingDelegate {
    /// Insert text into the text document proxy
    func insertText(_ text: String) {
        textDocumentProxy.insertText(text)
    }

    /// Delete backward in the text document proxy
    func deleteBackward() {
        textDocumentProxy.deleteBackward()
    }

    /// Set markedText (composing underline)
    func setMarkedText(_ text: String) {
        textDocumentProxy.setMarkedText(text, selectedRange: NSRange(location: text.utf16.count, length: 0))
    }

    /// Clear markedText completely
    func clearMarkedText() {
        textDocumentProxy.setMarkedText("", selectedRange: NSRange(location: 0, length: 0))
        textDocumentProxy.unmarkText()
    }

    /// Reset autocomplete context (clear suggestions)
    func resetAutocompleteContext() {
        state.autocompleteContext.reset()
    }

    // resetAutocomplete() and performAutocomplete() are inherited from KeyboardInputViewController
}
