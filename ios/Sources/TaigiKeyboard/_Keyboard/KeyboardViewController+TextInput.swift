import Foundation
import KeyboardKit

// MARK: - ComposingDelegate

extension KeyboardViewController {
    // insertText(_:) and deleteBackward() are in KeyboardViewController.swift
    // (protocol methods that override superclass must be in the class body)

    /// Selection point at end of markedText for cursor positioning
    func setMarkedText(_ text: String) {
        textDocumentProxy.setMarkedText(text, selectedRange: NSRange(location: text.utf16.count, length: 0))
    }

    /// Clear + unmark (two steps required by UITextInput)
    func clearMarkedText() {
        textDocumentProxy.setMarkedText("", selectedRange: NSRange(location: 0, length: 0))
        textDocumentProxy.unmarkText()
    }

    func resetAutocompleteContext() {
        state.autocompleteContext.reset()
    }
}
