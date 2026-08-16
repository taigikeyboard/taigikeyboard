// What the controller can ask of the candidate bar, and what it hands over.

import AppKit

/// One page of candidates as the bar renders it.
///
/// Plain strings rather than candidates: the bar shows exactly what committing
/// writes into the document, and that rendering depends on the output settings
/// the engine call uses (`CandidateDocumentText`). Handing the view the strings
/// keeps the settings on one side of the seam and makes the bar impossible to
/// render out of step with what the user will get.
struct CandidateBarContent: Equatable, Sendable {
    let labels: [String]
    /// Which label the highlight is on, counting from zero within this page —
    /// the `⌃n` chord that selects it. Nil for an empty page.
    let highlightedSlot: Int?
}

/// The candidate bar, as the controller sees it.
///
/// A protocol because the bar is an `NSPanel`, and a window is not something a
/// unit test can bring up: the routing this seam separates — which key changes
/// which part of the list, and which session is allowed to hide the bar — is the
/// part that has to be pinned by tests.
@MainActor
protocol CandidatePresenter {
    /// Shows `content` anchored to the caret, and records `owner` as the session
    /// the bar now belongs to.
    ///
    /// `caretRect` and `hostWindowLevel` come from the client, and only from
    /// inside a key event: asking during activation deadlocks Chromium hosts
    /// (`TaigiInputController.activateServer`).
    func show(
        _ content: CandidateBarContent,
        anchoredTo caretRect: CGRect,
        hostWindowLevel: CGWindowLevel,
        ownedBy owner: ComposingSessionToken,
    )

    /// Hides the bar if `owner` still owns it, and does nothing if another
    /// session has taken it over since.
    ///
    /// The guard is what an owner token exists for. A session losing focus is
    /// told so AFTER the session gaining it has been activated, so an
    /// unconditional hide here would take down the incoming session's bar.
    func hide(ownedBy owner: ComposingSessionToken)

    /// Hides the bar whoever owns it, and leaves it unowned — the incoming
    /// session's step, which is what makes the outgoing session's late
    /// `hide(ownedBy:)` a no-op.
    func hideForHandover()
}
