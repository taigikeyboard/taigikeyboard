// An IMK client double that records what an input method did to it.

import InputMethodKit

/// Records both directions of the client protocol.
///
/// Reads matter to the activation rule (asking the client anything during
/// activation deadlocks Chromium hosts); writes matter to the effect executor
/// (what the user sees in their document). One double serves both because a
/// test that asserts on writes usually also wants to know no read sneaked in.
final class RecordingTextInputClient: NSObject, IMKTextInput {
    /// One entry per document mutation, in the order it arrived — the order is
    /// half of what the executor is responsible for.
    enum Write: Equatable {
        case insertText(String)
        case setMarkedText(String, selectionLocation: Int)
    }

    private(set) var writes: [Write] = []
    private(set) var readCalls: [String] = []
    private(set) var keyboardLayoutOverrides: [String] = []
    var afterKeyboardLayoutOverride: ((String) -> Void)?

    /// The selection this client reports. Defaults to "cannot answer", which is
    /// what a client without `TSMDocumentAccess` does — the state every case
    /// not about selection-dependent behaviour should stay in.
    var selectedRangeToReturn = NSRange(location: NSNotFound, length: NSNotFound)

    /// The document text `attributedSubstring(from:)` slices out of. Nil — the
    /// default — answers nil for every range, like a client that cannot serve
    /// substring queries at all. When set, every `insertText` and
    /// `setMarkedText` is applied to it and moves `selectedRangeToReturn` the
    /// way a real client's document and caret follow the edits — so a
    /// multi-step case (commit → auto space → swap → swap again, or mark a
    /// placeholder → clear it → swap) verifies each step against the state
    /// the previous one produced rather than against its fixture.
    var documentTextForReads: String?

    /// Where the marked text sits in `documentTextForReads`, while there is
    /// one. Marked text goes in over the selection (or the previous marked
    /// region), as the real client does, and an at-caret `insertText`
    /// replaces it (`IMKInputSession.h:63-74`).
    private(set) var markedRangeInDocument: NSRange?

    /// Plays a Chromium host: `selectedRange()` keeps answering the selection
    /// from before this key's first write until `settleSelection()` — the
    /// renderer's update arriving between two key events.
    var selectionLagsWrites = false
    private var laggingSelection: NSRange?

    /// The renderer caught up: `selectedRange()` answers the live selection.
    func settleSelection() {
        laggingSelection = nil
    }

    /// The replacement range of each `insertText`, in write order — how a case
    /// tells an at-caret insert from a rewrite of committed text.
    private(set) var insertReplacementRanges: [NSRange] = []

    /// The line rectangle this client reports for a character index of the
    /// marked region. Indices absent from the map answer with a zero rectangle,
    /// which is what a real client does for a position it cannot place — and
    /// what makes the caret walk-back observable.
    var caretRects: [Int: CGRect] = [:]

    /// Every index the caret walk asked about, in order.
    private(set) var caretRectQueries: [Int] = []

    /// Called after each `insertText` is recorded — where a case plays a host
    /// whose callback re-enters the input method mid-write.
    var afterInsertText: ((String) -> Void)?

    /// The attributes of the most recent marked-text write, which is where the
    /// composition's underline lives.
    private(set) var lastMarkedTextAttributes: [NSAttributedString.Key: Any] = [:]

    var readCallCount: Int {
        readCalls.count
    }

    /// The texts committed to the document, in order — what the user keeps.
    var insertedTexts: [String] {
        writes.compactMap { write in
            if case let .insertText(text) = write {
                return text
            }
            return nil
        }
    }

    /// Drops what has been recorded so far, so a case can assert on the writes
    /// of one step without restating the setup that led to it.
    func clearWrites() {
        writes.removeAll()
    }

    // MARK: Writes

    func insertText(_ string: Any!, replacementRange: NSRange) {
        let text = Self.plainText(string)
        writes.append(.insertText(text))
        holdSelectionIfLagging()
        insertReplacementRanges.append(replacementRange)
        simulateDocumentMutation(text, replacementRange: replacementRange)
        afterInsertText?(text)
    }

    /// Applies an insert to the simulated document, when one is configured.
    /// `NSNotFound` — "at the insertion point" — resolves through the current
    /// selection, like a real client; with neither a usable range nor a usable
    /// selection the document is left alone, which is also what a case that
    /// never configured one gets.
    private func simulateDocumentMutation(_ text: String, replacementRange: NSRange) {
        guard let document = documentTextForReads else { return }
        let documentText = document as NSString
        let target: NSRange
        if replacementRange.location != NSNotFound, NSMaxRange(replacementRange) <= documentText.length {
            target = replacementRange
        } else if let marked = markedRangeInDocument {
            target = marked
        } else if selectedRangeToReturn.location != NSNotFound,
                  NSMaxRange(selectedRangeToReturn) <= documentText.length
        {
            target = selectedRangeToReturn
        } else {
            return
        }
        markedRangeInDocument = nil
        documentTextForReads = documentText.replacingCharacters(in: target, with: text)
        selectedRangeToReturn = NSRange(
            location: target.location + (text as NSString).length,
            length: 0,
        )
    }

    /// Applies a marked-text write to the simulated document: the text
    /// replaces the marked region if there is one, else the selection; an
    /// empty write removes the region and leaves the caret where it began.
    private func simulateMarkedText(_ text: String, selectionRange: NSRange) {
        guard let document = documentTextForReads else { return }
        let documentText = document as NSString
        let target: NSRange
        if let marked = markedRangeInDocument {
            target = marked
        } else if selectedRangeToReturn.location != NSNotFound,
                  NSMaxRange(selectedRangeToReturn) <= documentText.length
        {
            target = selectedRangeToReturn
        } else {
            return
        }
        documentTextForReads = documentText.replacingCharacters(in: target, with: text)
        let length = (text as NSString).length
        markedRangeInDocument = length > 0 ? NSRange(location: target.location, length: length) : nil
        selectedRangeToReturn = NSRange(location: target.location + min(selectionRange.location, length), length: 0)
    }

    func setMarkedText(_ string: Any!, selectionRange: NSRange, replacementRange _: NSRange) {
        let text = Self.plainText(string)
        writes.append(.setMarkedText(text, selectionLocation: selectionRange.location))
        holdSelectionIfLagging()
        simulateMarkedText(text, selectionRange: selectionRange)
        lastMarkedTextAttributes = (string as? NSAttributedString)
            .flatMap { $0.length > 0 ? $0.attributes(at: 0, effectiveRange: nil) : [:] } ?? [:]
    }

    private func holdSelectionIfLagging() {
        if selectionLagsWrites, laggingSelection == nil {
            laggingSelection = selectedRangeToReturn
        }
    }

    func overrideKeyboard(withKeyboardNamed name: String!) {
        keyboardLayoutOverrides.append(name)
        afterKeyboardLayoutOverride?(name)
    }

    func selectMode(_: String!) {}

    /// IMK hands text as either an `NSString` or an `NSAttributedString`; the
    /// distinction is styling, and the assertions are about content.
    private static func plainText(_ string: Any!) -> String {
        (string as? NSAttributedString)?.string ?? (string as? String) ?? ""
    }

    // MARK: Reads

    func selectedRange() -> NSRange {
        readCalls.append(#function)
        return laggingSelection ?? selectedRangeToReturn
    }

    /// The simulated marked region; "none" without a simulated document, or
    /// with nothing marked in it.
    func markedRange() -> NSRange {
        readCalls.append(#function)
        return markedRangeInDocument ?? NSRange(location: NSNotFound, length: NSNotFound)
    }

    func attributedSubstring(from range: NSRange) -> NSAttributedString! {
        readCalls.append(#function)
        guard let documentTextForReads else { return nil }
        let text = documentTextForReads as NSString
        guard range.location != NSNotFound, NSMaxRange(range) <= text.length else { return nil }
        return NSAttributedString(string: text.substring(with: range))
    }

    func length() -> Int {
        readCalls.append(#function)
        return NSNotFound
    }

    func characterIndex(
        for _: NSPoint,
        tracking _: IMKLocationToOffsetMappingMode,
        inMarkedRange _: UnsafeMutablePointer<ObjCBool>!,
    ) -> Int {
        readCalls.append(#function)
        return NSNotFound
    }

    func attributes(
        forCharacterIndex index: Int,
        lineHeightRectangle lineRect: UnsafeMutablePointer<NSRect>!,
    ) -> [AnyHashable: Any]! {
        readCalls.append(#function)
        caretRectQueries.append(index)
        lineRect?.pointee = caretRects[index] ?? .zero
        return [:]
    }

    func validAttributesForMarkedText() -> [Any]! {
        readCalls.append(#function)
        return []
    }

    func supportsUnicode() -> Bool {
        readCalls.append(#function)
        return true
    }

    func bundleIdentifier() -> String! {
        readCalls.append(#function)
        return "com.example.RecordingTextInputClient"
    }

    func windowLevel() -> CGWindowLevel {
        readCalls.append(#function)
        return 0
    }

    func supportsProperty(_: TSMDocumentPropertyTag) -> Bool {
        readCalls.append(#function)
        return false
    }

    func uniqueClientIdentifierString() -> String! {
        readCalls.append(#function)
        return "recording-client"
    }

    func string(from _: NSRange, actualRange _: NSRangePointer!) -> String! {
        readCalls.append(#function)
        return ""
    }

    func firstRect(forCharacterRange _: NSRange, actualRange _: NSRangePointer!) -> NSRect {
        readCalls.append(#function)
        return .zero
    }
}
