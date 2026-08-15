// Runs engine effects against one IMK client: marked text in, committed text out.

import InputMethodKit

/// Writes the engine's effects into the client that is currently focused.
///
/// The composition lives entirely in the client's marked region until the
/// engine says to commit, which is what makes an abort free: nothing was ever
/// written to the document, so nothing has to be taken back.
@MainActor
final class ClientEffectExecutor: ComposingEffectExecutor {
    /// "Wherever the insertion point is." Both IMK text calls read a location of
    /// `NSNotFound` as "use the current selection" and ignore the range
    /// entirely for clients without `TSMDocumentAccess`
    /// (`IMKInputSession.h:66-72` for `insertText`, `:85-87` for
    /// `setMarkedText`), which is exactly the behaviour an inline composition
    /// wants — the client owns caret placement, we never move it.
    private static let atInsertionPoint = NSRange(location: NSNotFound, length: NSNotFound)

    private let client: IMKTextInput
    private static let logger = DebugLogger(category: "EffectExecutor")

    init(client: IMKTextInput) {
        self.client = client
    }

    func execute(_ effect: ComposingTransition.Effect) {
        switch effect {
        case let .updatePreedit(text):
            client.setMarkedText(
                Self.markedText(text),
                // Collapsed at the end of the composition: the engine has no
                // caret inside the preedit, so the client should show the
                // insertion point after everything typed so far. UTF-16 because
                // that is the unit `NSRange` counts in.
                selectionRange: NSRange(location: text.utf16.count, length: 0),
                replacementRange: Self.atInsertionPoint,
            )

        case .clearPreeditWithoutCommit:
            // An empty marked string is how IMK is told the composition ended
            // without producing text; the client drops the marked region.
            client.setMarkedText(
                "",
                selectionRange: NSRange(location: 0, length: 0),
                replacementRange: Self.atInsertionPoint,
            )

        case let .commitTextReplacingPreedit(text):
            // No `setMarkedText("")` first: `insertText` already replaces the
            // active marked region (`IMKInputSession.h:63-74`). Clearing first
            // would be a second document mutation for one keystroke, which
            // clients render as a visible flicker and undo as two steps.
            client.insertText(text, replacementRange: Self.atInsertionPoint)

        case .deleteBackwardFromDocument:
            // NAMED CROSS-PLATFORM DIVERGENCE (`cross-platform-alignment.md` §3,
            // intentional): iOS forwards this to `proxy.deleteBackward()`
            // because its preedit is real document text. Under IMK the preedit
            // only ever existed in the marked region, which the preceding
            // `ClearPreeditWithoutCommit` already removed, so forwarding would
            // eat a character the user typed before the composition started.
            Self.logger.debug("deleteBackwardFromDocument ignored — nothing was written to the document")

        case .resetAutocomplete, .performAutocomplete, .resetAutocompleteContext:
            // macOS has no autocomplete surface yet; the engine emits these
            // unconditionally. Explicitly ignored rather than filtered out at
            // decode time so the case is visible when that surface lands.
            break

        case .nextWordUpdateLastSelectedWord, .nextWordWordSelected, .nextWordClearForNewComposing:
            // Next-word learning is a later slice (roadmap D7): macOS sends no
            // nextword requests yet, so there is no store for these handshakes
            // to update.
            break
        }
    }

    /// Underlined, single clause. Mirrors the marked-text styling every
    /// mainstream macOS IME uses for an unconverted composition — McBopomofo
    /// `references/McBopomofo/Source/InputState.swift:340-345`. Passing a plain
    /// `String` would also underline it, but only with IMK's default styling,
    /// and the clause segment is what tells the client this is one unit rather
    /// than a run of unrelated characters.
    private static func markedText(_ text: String) -> NSAttributedString {
        NSAttributedString(string: text, attributes: markedTextAttributes)
    }

    /// Hoisted because the styling never varies, and this runs once per
    /// keystroke.
    private static let markedTextAttributes: [NSAttributedString.Key: Any] = [
        .underlineStyle: NSUnderlineStyle.single.rawValue,
        .markedClauseSegment: 0,
    ]
}
