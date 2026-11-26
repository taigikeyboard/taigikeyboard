// Symbol picker of TaigiInputController: the chord, the list over a placeholder, the pick.

import InputMethodKit
import KeyboardShortcuts

extension TaigiInputController {
    /// What the picker marks in the document while it is up: one space,
    /// underlined like a composition (vChewing `getAttributedStringPlaceholder`).
    /// A space rather than a zero-width character: what the client shows is
    /// what the user is picking over, and a zero-width leftover would be
    /// invisible.
    private static let symbolPickerPlaceholder = " "

    /// Whether `key` is the chord recorded on the picker row: the same key
    /// CODE and chording modifiers the registry stores, compared as Carbon
    /// would for the other rows — not the character, which ⌃ and ⌥ rewrite.
    @MainActor
    func isSymbolPickerChord(_ key: KeyEventSnapshot) -> Bool {
        guard let shortcut = symbolPickerShortcut,
              let keyCode = key.keyCode, Int(keyCode) == shortcut.carbonKeyCode
        else { return false }
        let chording = KeyEventSnapshot.chordingModifiers
        return key.modifiers.intersection(chording) == shortcut.modifiers.intersection(chording)
    }

    /// Ends whatever is composing, then puts the symbol list up over the
    /// caret (`ComposingBackend.commitForSymbolPicker`). A commit that only
    /// NAILED a segment leaves the composition running, and the picker waits
    /// for a key that ends it.
    @MainActor
    func openSymbolPicker(client: IMKTextInput) {
        if backend.isComposing(sessionToken) {
            // The commit moves the caret; whatever it earns re-arms.
            armedAutoSpaceCaret = nil
            send(client: client, armedSwap: nil) { backend.commitForSymbolPicker(in: $0) }
            guard !backend.isComposing(sessionToken) else { return }
        }
        presentSymbolPicker(in: client)
    }

    /// Shows the whole table anchored to the caret — one list, in file
    /// order, so the first pick is the symbol itself (USER 2026-09-09: a
    /// category to choose first "it would interrupt the user's flow") — and records the
    /// picker as open.
    ///
    /// The list goes up over a PLACEHOLDER marked region — one underlined
    /// space, the caret after it — and not over a bare insertion point, even
    /// though the composition is over by the time the picker opens. A host
    /// that is not a Cocoa text view reads "no marked text" as "no input
    /// method at work": Chromium (Chrome, and every Electron app) forwards
    /// the real key event to the page unless marked text was present around
    /// the input method's turn, so with nothing marked the arrows walked the
    /// list AND moved the page's caret, and Return picked the symbol AND
    /// submitted the message (USER report 2026-09-18: "after pressing ↑↓←→ the
    /// typing position wanders off by itself, and pressing Enter types nothing"). vChewing keeps the same
    /// placeholder for the same list — `.ofSymbolTable` with nothing typed
    /// (`IMEStateParsed4Darwin.swift:256-266`, `InputSession_HandleStates.swift:58-64`:
    /// "keep the inline display from being empty, so IMK does not misjudge the
    /// composition as ended and leak the keyboard event to the client app").
    ///
    /// Not over a selection: marked text replaces the selection the way
    /// typing does (`NSTextInputClient.setMarkedText`), and clearing it on
    /// Escape would not bring the selected text back. A selection is what
    /// the pick replaces, so the list opens over it with nothing marked —
    /// the one case that keeps the bare anchor. A client that cannot say
    /// what is selected is read as a caret, as the auto-space swap reads it.
    ///
    /// The placeholder goes in first, so the caret walk reads its rectangle
    /// — index 0 of the marked region, which is what a client answers a line
    /// height for. A list that did not reach the screen — no caret, no
    /// display for it — leaves nothing behind, the placeholder included: a
    /// picker recorded as open over a window nobody can see would go on
    /// swallowing the slot keys.
    @MainActor
    private func presentSymbolPicker(in client: IMKTextInput) {
        // The core's list, recents first, read once: this is the list the
        // pick will index. None — the table did not validate — no picker,
        // and nothing marked for one.
        guard let cells = KeyRules.symbolPickerSymbols(recentSymbols: settings.recentSymbols) else { return }
        symbolPickerCells = cells
        let selection = client.selectedRange()
        var markedTextLength = 0
        if selection.location == NSNotFound || selection.length == 0 {
            isSymbolPickerPlaceholderMarked = true
            markedTextLength = Self.symbolPickerPlaceholder.utf16.count
            ClientWriter(client: client)
                .setMarkedText(Self.symbolPickerPlaceholder, caretUTF16: markedTextLength)
        }
        guard let caretRect = caretRect(in: client, anchorEnd: markedTextLength) else {
            dismissSymbolPicker()
            return
        }
        symbolPickerPresenter.show(
            CandidateWindowContent(
                cells: symbolPickerCells.map { CandidateCellContent(text: $0, annotation: nil) },
                slotKeySet: settings.candidateSlotKeySet,
                leadCellIsUnkeyed: false,
            ),
            anchoredTo: caretRect,
            hostWindowLevel: client.windowLevel(),
            hostBundleIdentifier: client.bundleIdentifier(),
            ownedBy: sessionToken,
        )
        // The window answers a selection only while it owns a visible list.
        if symbolPickerPresenter.selectedCandidateIndex(ownedBy: sessionToken) == nil {
            dismissSymbolPicker()
        }
    }

    /// One key while the picker is up. Answers whether the key was consumed;
    /// false means the picker has closed and the key goes on through the
    /// composing contract as if the picker had never been there.
    @MainActor
    func handleSymbolPickerKey(_ key: KeyEventSnapshot, client: IMKTextInput) -> Bool {
        // Read by desktop-core under this session's settings; a seam failure
        // (logged) takes the picker down and lets the key go on.
        switch KeyRules.symbolPickerIntent(for: key, in: settings.userDefaults) ?? .closeAndPassThrough {
        case .close:
            dismissSymbolPicker()
        case let .navigate(direction):
            symbolPickerPresenter.navigate(direction, ownedBy: sessionToken)
        case let .pickSlot(slot):
            // An empty slot on a short last page is consumed all the same,
            // as it is on the bar: the key is the picker's while it is up.
            pickSymbolCell(
                at: symbolPickerPresenter.candidateIndex(forKeySlot: slot, ownedBy: sessionToken),
                client: client,
            )
        case .confirm:
            pickSymbolCell(
                at: symbolPickerPresenter.selectedCandidateIndex(ownedBy: sessionToken),
                client: client,
            )
        case .closeAndPassThrough:
            dismissSymbolPicker()
            return false
        }
        return true
    }

    /// Writes the symbol at `index`, closes the picker and moves the symbol
    /// to the front of the recents when the write succeeded. Nil — a slot with
    /// no cell — does nothing, and keeps the picker up.
    @MainActor
    private func pickSymbolCell(at index: Int?, client: IMKTextInput) {
        guard let index, symbolPickerCells.indices.contains(index) else { return }
        let symbol = symbolPickerCells[index]
        dismissSymbolPicker()
        guard insertSymbol(symbol, client: client) else { return }
        settings.noteRecentSymbol(symbol)
    }

    /// Writes `symbol` through the back end (`ComposingBackend.insertSymbol`)
    /// — the one picker key that touches the document, so the one that
    /// spends the auto-space arm. Returns whether the symbol was actually
    /// written to `client`.
    @MainActor
    private func insertSymbol(_ symbol: String, client: IMKTextInput) -> Bool {
        let armedSwap = armedAutoSpaceCaret
        armedAutoSpaceCaret = nil
        return sendOutcome(client: client, armedSwap: armedSwap) { backend.insertSymbol(symbol, in: $0) }.wroteText
    }

    /// Takes the list down and its placeholder with it — from `lastClient`,
    /// the client this session marks (`inputControllerWillClose` clears a
    /// leftover region through the same reference). Cleared BEFORE anything
    /// the picker's key then writes: a pick inserts over a document with no
    /// marked region, exactly as it did before the placeholder existed, so
    /// the auto-space swap's `selectedRange()` and absolute `replacementRange`
    /// still measure committed text — with a marked region up, IMK reads a
    /// replacement range from the start of the marked text instead. Only
    /// what went in comes out, so a host never gets an empty region it did
    /// not have (McBopomofo #346); the flag drops before the write, so a
    /// client callback that re-enters here finds nothing left to clear.
    @MainActor
    func dismissSymbolPicker() {
        symbolPickerCells = []
        symbolPickerPresenter.hide(ownedBy: sessionToken)
        guard isSymbolPickerPlaceholderMarked else { return }
        isSymbolPickerPlaceholderMarked = false
        if let client = lastClient {
            ClientWriter(client: client).clearMarkedText()
        }
    }
}
