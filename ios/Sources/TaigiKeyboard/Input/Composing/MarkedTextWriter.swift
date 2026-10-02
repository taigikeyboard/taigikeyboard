import UIKit

/// Writes the composition to the host as marked text and commits it.
///
/// Remembers whether the keyboard currently holds a marked region, because a
/// commit over one must not go through `insertText`: Flutter before
/// flutter/flutter#191062 inserts at the caret and keeps the marked text as
/// committed text, and clearing the marked text first does not help when the
/// insert follows in the same turn (issue #352). Replacing the marked text and
/// then unmarking it is Apple's documented commit flow for custom keyboards.
final class MarkedTextWriter {
    private(set) var hasMarkedText = false

    /// Show `text` as the whole marked region, caret at its end. **Model B**:
    /// `text` is the whole composition (`Σ nailed.display_text` + derived
    /// pending tail); the host renders it as one marked region until a hard
    /// finalize.
    func update(_ text: String, on proxy: UITextDocumentProxy) {
        proxy.setMarkedText(text, selectedRange: NSRange(location: text.utf16.count, length: 0))
        hasMarkedText = !text.isEmpty
    }

    /// Remove the whole marked region (nailed + pending) without committing
    /// it — nailed segments were never literal document text. Two steps are
    /// required by UITextInput.
    func clear(on proxy: UITextDocumentProxy) {
        proxy.setMarkedText("", selectedRange: NSRange(location: 0, length: 0))
        proxy.unmarkText()
        hasMarkedText = false
    }

    /// Commit `text` in place of the marked region, or insert it at the caret
    /// when there is none.
    func commit(_ text: String, on proxy: UITextDocumentProxy) {
        guard hasMarkedText else {
            proxy.insertText(text)
            return
        }
        update(text, on: proxy)
        proxy.unmarkText()
        hasMarkedText = false
    }

    /// The host field changed: any marked region belonged to the old field.
    func forget() {
        hasMarkedText = false
    }
}
