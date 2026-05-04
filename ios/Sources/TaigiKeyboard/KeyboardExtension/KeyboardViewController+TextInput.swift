import Foundation
import KeyboardKit
import UIKit

// MARK: - ComposingDelegate

extension KeyboardViewController {
    /// Translate a platform-neutral `RustEngineBridge.ComposingTransition.Effect`
    /// to the iOS `UITextDocumentProxy` surface. Binding contract (iOS +
    /// Android) is documented in `composing-state-boundary.md` §2.2.
    func execute(_ effect: RustEngineBridge.ComposingTransition.Effect) {
        logger.debug({
            let kind: String
            switch effect {
            case let .updatePreedit(text):
                kind = "updatePreedit len=\(text.count)"
            case .clearPreeditWithoutCommit:
                kind = "clearPreeditWithoutCommit"
            case let .commitTextReplacingPreedit(text):
                kind = "commitTextReplacingPreedit len=\(text.count)"
            case .deleteBackwardFromDocument:
                kind = "deleteBackwardFromDocument"
            case .resetAutocomplete:
                kind = "resetAutocomplete"
            case .performAutocomplete:
                kind = "performAutocomplete"
            case .resetAutocompleteContext:
                kind = "resetAutocompleteContext"
            }
            return "[COMMIT] fn=execute effect=\(kind)"
        }())
        switch effect {
        case let .updatePreedit(text):
            setMarkedText(text)
        case .clearPreeditWithoutCommit:
            clearMarkedText()
        case let .commitTextReplacingPreedit(text):
            // Clear marked text first so the commit replaces the preedit
            // region atomically from the user's perspective (Android
            // achieves this via `commitText(text, 1)`).
            clearMarkedText()
            textDocumentProxy.insertText(text)
        case .deleteBackwardFromDocument:
            textDocumentProxy.deleteBackward()
        case .resetAutocomplete:
            resetAutocomplete()
        case .performAutocomplete:
            performAutocomplete()
        case .resetAutocompleteContext:
            state.autocompleteContext.reset()
        }
    }

    /// Set marked (composing) text with the caret placed at the end.
    /// Used by `.updatePreedit(_)` effect and by the explicit preedit
    /// clears in the textDidChange handler.
    func setMarkedText(_ text: String) {
        textDocumentProxy.setMarkedText(text, selectedRange: NSRange(location: text.utf16.count, length: 0))
    }

    /// Clear marked text + unmark (two steps required by UITextInput).
    /// Used by `.clearPreeditWithoutCommit` and `.commitTextReplacingPreedit`.
    func clearMarkedText() {
        textDocumentProxy.setMarkedText("", selectedRange: NSRange(location: 0, length: 0))
        textDocumentProxy.unmarkText()
    }
}
