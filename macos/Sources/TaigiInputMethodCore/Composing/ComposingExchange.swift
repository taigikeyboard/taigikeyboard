// What the controller and the key path exchange: one request in, the effects
// to replay out.

import Foundation

/// The session a request is for and what only the controller can answer.
@MainActor
struct ComposingRequest {
    let session: ComposingSessionToken

    /// The controller's own store — the one a test points at its own suite —
    /// read for every setting the key path consults.
    let settings: SettingsStore

    let panel: ComposingPanelState
}

/// The controller's side of a request: its window and its client.
///
/// The list on screen is the window's truth: a request that says none is up
/// drops whatever list the core still holds before anything reads it (a
/// list the client gave no caret rectangle for, a dismissal outside the key
/// path). The three questions are closures so the back end asks only those
/// a request needs, all before it sends.
@MainActor
struct ComposingPanelState {
    let isListOnScreen: Bool

    /// The window's highlighted cell, if it shows one.
    let selectedIndex: @MainActor () -> Int?

    /// The cell the `slot`-th selection key addresses on the page shown.
    let indexForKeySlot: @MainActor (Int) -> Int?

    /// Whether the auto space the last commit left can be swapped now: the
    /// caret still collapsed where the space left it, and the character
    /// before it still that space. `nil` while nothing is armed.
    let canSwapPrecedingSpace: (@MainActor () -> Bool)?
}

/// A key's answer: whether it was consumed, and what to replay.
struct ComposingKeyReply {
    let handled: Bool
    let effects: [ComposingEffect]
}

/// One thing the controller does to its client or its window, in reply order.
enum ComposingEffect: Equatable {
    /// The composition, underlined, with the caret at `caretUTF16`.
    case setMarkedText(String, caretUTF16: Int)

    /// The composition ended without writing anything.
    case clearMarkedText

    /// Written at the insertion point, over the marked region if there is one.
    case insertText(String)

    /// The armed auto space replaced by `replacement` (`guá ` + `?` → `guá? `).
    case swapPrecedingSpace(replacement: String)

    /// Arm the swap at the caret the writes before it left.
    case armSwap

    /// A list to show, anchored at the caret.
    case candidatesChanged(CandidateListUpdate)

    case candidatesClosed

    case navigate(CandidateNavigation)

    /// The engine took a TPS key: the key panel flashes the cap at `row`,
    /// `cap` (both counted from 0, as `KeyRules.tpsKeyboardRows` lists them).
    case tpsKeyboardKeyTyped(row: Int, cap: Int)
}

/// What a represent did to the list on screen — repainted in place, never
/// anchored again.
enum CandidateListRepresentation: Equatable {
    case unchanged
    case changed(CandidateListUpdate)
    case closed
}

/// A list as the window takes it, with what anchors it.
struct CandidateListUpdate: Equatable {
    let cells: [CandidateCellContent]

    /// The first cell is the §34 literal, which takes no slot key.
    let leadsWithLiteralRoman: Bool

    /// The window sits under the character of the marked text that ends
    /// here — where the caret walk starts. The end of the marked text, or the
    /// caret under TPS, whose list is usually the word before it.
    let anchorEndUTF16: Int

    /// A represent's list fetched again: the cell the selection follows to
    /// (`CandidatesChanged.selected_index`). Nil everywhere else.
    var selectedIndex: Int?
}
