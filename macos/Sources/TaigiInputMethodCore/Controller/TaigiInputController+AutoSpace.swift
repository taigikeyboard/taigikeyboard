// Auto-space swap client checks of TaigiInputController: where a reply writes, whether the space is still there.

import InputMethodKit

extension TaigiInputController {
    /// Where the writes of a reply that arms the swap land: the start of the
    /// marked region they commit over, or with nothing marked the selection
    /// they replace. Asked BEFORE the reply writes anything, so the arm is
    /// this plus what the reply inserted — never a `selectedRange()` read
    /// after the writes. Chromium hosts answer that read from a copy the
    /// renderer has not updated yet (the caret before the commit), which
    /// left every swap there declined.
    ///
    /// A client that cannot answer, or answers mid-selection, never arms:
    /// the swap degrades to pass-through (`guá ?`). A wrong start cannot
    /// rewrite the wrong text either — the swap still needs the client's
    /// caret to equal the arm and a space before it (`canSwapAutoSpace`).
    /// Asking here is safe: this runs inside a key event, like every client
    /// query (see `caretRect`'s activation-only deadlock rule).
    @MainActor
    static func writeStart(in client: IMKTextInput) -> Int? {
        let marked = client.markedRange()
        if marked.location != NSNotFound {
            return marked.location
        }
        let selection = client.selectedRange()
        guard selection.location != NSNotFound, selection.length == 0 else { return nil }
        return selection.location
    }

    /// The client half of the smart-punctuation swap (`guá ` + `?` → `guá? `):
    /// two verifications before the back end may answer a rewrite, because
    /// `replacementRange` is a real edit of committed text — the caret must
    /// still be a collapsed selection exactly where the space left it, and
    /// the character under the range must still be a space. Any client that
    /// fails one — including one that cannot answer a substring query at
    /// all — gets the key passed through untouched.
    @MainActor
    static func canSwapAutoSpace(armedAt armedCaret: Int, in client: IMKTextInput) -> Bool {
        let caret = client.selectedRange()
        guard caret.length == 0, caret.location == armedCaret else { return false }
        guard let preceding = client.attributedSubstring(from: autoSpaceRange(armedAt: armedCaret)) else { return false }
        return preceding.string == " "
    }

    /// The auto space in front of an arm: what the swap checks and what it
    /// replaces.
    static func autoSpaceRange(armedAt armedCaret: Int) -> NSRange {
        NSRange(location: armedCaret - 1, length: 1)
    }
}
