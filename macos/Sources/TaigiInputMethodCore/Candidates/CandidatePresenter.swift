// What the controller can ask of the candidate window, and what it hands over.

import AppKit

/// The full candidate list as the window renders it, in display order — a
/// cell's position IS the candidate's absolute index, which is the identity
/// every other call on the seam speaks in.
///
/// Rendered cells rather than candidates: which script a cell leads with
/// depends on the output settings the engine call uses
/// (`CandidateCellContent.cell(for:settings:)`), so resolving it on this side
/// of the seam keeps the settings in one place and makes the window impossible
/// to render out of step with the snapshot the fetch used.
///
/// The whole list rather than one page, unlike the SwiftUI bar this window
/// replaced: page boundaries are now a function of measured glyph widths
/// (`HorizontalPageLayout`), so only the window — which owns the measuring —
/// can know where a page ends.
struct CandidateWindowContent: Equatable, Sendable {
    let cells: [CandidateCellContent]

    /// Which key picks a candidate at this moment — what the window draws
    /// beside each cell. Part of the content rather than a setting, because it
    /// is a fact about the composition the cells came from: the same list
    /// typed one keystroke later can be picked by a different key
    /// (`CandidateSlotKeyStyle`).
    let slotKeyStyle: CandidateSlotKeyStyle
}

/// Which way a navigation key asks the candidate window to move.
///
/// The six physical keys are handed through raw rather than as a digested
/// "highlight vs page" pair, because what each key means depends on the layout:
/// `↓` pages a horizontal window and walks a vertical list. The window is where
/// the layout lives, so the window is where the key is interpreted.
///
/// `nextCandidate` and `previousCandidate` are the exception, and they are the
/// reason the two kinds share one type: they name an OUTCOME, not a key. Every
/// layout must read them as one step along the list and nothing else, so that a
/// binding whose label says "next candidate" cannot turn into a page jump on a
/// vertical window — which is exactly what `.right` does there
/// (`VerticalCandidatePanel.navigate`).
enum CandidateNavigation: Sendable, Equatable {
    case left
    case right
    case up
    case down
    case pageUp
    case pageDown
    /// One candidate forward in the list, clamped at the end.
    case nextCandidate
    /// One candidate back in the list, clamped at the start.
    case previousCandidate
}

/// The candidate window, as the controller sees it.
///
/// A protocol because the window is an `NSPanel`, and a window is not something
/// a unit test can bring up: the routing this seam separates — which key
/// reaches the window, which commit resolves to which absolute index, and
/// which session is allowed to hide it — is the part that has to be pinned by
/// tests.
///
/// The window is authoritative for the selection. The controller retains the
/// `ContinuousCandidate` array and maps the absolute indices this seam answers
/// with back onto it; nothing mirrors the selection into the controller,
/// because a mirror is a second copy of the truth that can only ever disagree.
@MainActor
protocol CandidatePresenter {
    /// Shows `content` anchored to the caret, selects the first candidate, and
    /// records `owner` as the session the window now belongs to.
    ///
    /// `caretRect` and `hostWindowLevel` come from the client, and only from
    /// inside a key event: asking during activation deadlocks Chromium hosts
    /// (`TaigiInputController.activateServer`). `hostBundleIdentifier` feeds
    /// the Multicolour accent resolution — when the system has no fixed accent
    /// colour, the highlight takes the HOST app's own accent, which is what
    /// the native candidate window does.
    func show(
        _ content: CandidateWindowContent,
        anchoredTo caretRect: CGRect,
        hostWindowLevel: CGWindowLevel,
        hostBundleIdentifier: String?,
        ownedBy owner: ComposingSessionToken,
    )

    /// Replaces every cell's content in place — the same candidate list under
    /// a new rendering (the 漢羅對調 flip) — keeping the window up, its anchor,
    /// and the selection on the same absolute index. A no-op unless `owner`
    /// owns a visible, non-empty window.
    ///
    /// A separate contract from `show` on purpose: `show` is the fresh-list
    /// path and resets the selection to the first candidate, which a pure
    /// display change must not do. Callable without a client — the window is
    /// already anchored, so no caret query is needed (the queries `show`
    /// depends on are only allowed inside key events).
    func updateCells(_ cells: [CandidateCellContent], ownedBy owner: ComposingSessionToken)

    /// Redraws the key beside every numbered cell under `style`, keeping the
    /// cells, the page, the anchor and the selection exactly as they are.
    ///
    /// Its own contract because the selection latch flips the live key WITHOUT
    /// a new candidate list: navigating does not re-fetch, so nothing calls
    /// `show` on the keystroke that latches, and a window left drawing `q`
    /// while a bare `1` picks would be naming a key that does something else —
    /// the one thing `CandidateSlotKeyStyle` exists to prevent.
    func updateSlotKeyStyle(_ style: CandidateSlotKeyStyle, ownedBy owner: ComposingSessionToken)

    /// Moves the selection the way the current layout reads `direction`, if
    /// `owner` still owns the window. Clamps at both ends — never wraps
    /// (McBopomofo `HorizontalCandidateController.swift:509`; the D4 rule).
    func navigate(_ direction: CandidateNavigation, ownedBy owner: ComposingSessionToken)

    /// The absolute index of the selected candidate, or nil when `owner` does
    /// not own a visible window. What Space commits.
    func selectedCandidateIndex(ownedBy owner: ComposingSessionToken) -> Int?

    /// The absolute index the `⌃(slot+1)` chord addresses — a position within
    /// the page or row the user can currently see, never an absolute rank
    /// somewhere off screen. Nil for an empty slot, which is what a short last
    /// page ends with.
    func candidateIndex(forSlot slot: Int, ownedBy owner: ComposingSessionToken) -> Int?

    /// Hides the window if `owner` still owns it, and does nothing if another
    /// session has taken it over since.
    ///
    /// The guard is what an owner token exists for. A session losing focus is
    /// told so AFTER the session gaining it has been activated, so an
    /// unconditional hide here would take down the incoming session's window.
    ///
    /// Hiding drops the candidate list and the selection with the window, not
    /// merely the pixels: state kept alive behind a hidden window would let
    /// `selectedCandidateIndex` answer for candidates nobody can see.
    func hide(ownedBy owner: ComposingSessionToken)

    /// Hides the window whoever owns it, and leaves it unowned — the incoming
    /// session's step, which is what makes the outgoing session's late
    /// `hide(ownedBy:)` a no-op.
    func hideForHandover()
}
