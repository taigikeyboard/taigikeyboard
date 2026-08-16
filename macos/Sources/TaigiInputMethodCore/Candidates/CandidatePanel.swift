// The window the candidate bar lives in: one non-activating panel, process-wide.

import AppKit
import SwiftUI

/// The candidate bar's window.
///
/// One per process, like the composing engine behind it: only one input session
/// is focused at a time, so a panel per session would be a pile of hidden
/// windows and a race over which of them is on top. Ownership is tracked here
/// instead — see `CandidatePresenter.hide(ownedBy:)` for what that guard buys.
///
/// Not `IMKCandidates`: it renders a fixed table of Apple's own design, with no
/// way to label cells with the `⌃n` chords this input method binds or to show
/// the romanization alongside the Hanji.
@MainActor
final class CandidatePanel: CandidatePresenter {
    static let shared = CandidatePanel()

    private static let logger = DebugLogger(category: "CandidatePanel")

    /// The session the bar is currently showing for, or nil when nothing is
    /// showing.
    private var owner: ComposingSessionToken?

    /// The window and the view inside it, built together the first time a
    /// candidate is actually shown.
    ///
    /// Optional rather than `lazy` because every path that only HIDES has to
    /// leave it alone: a session that never offers a candidate — an English
    /// document, a password field — must not cause an input method to open a
    /// window, and `activateServer` hides on every focus change.
    private var bar: Bar?

    private struct Bar {
        let panel: NSPanel
        let hostingView: NSHostingView<CandidateBarView>
    }

    private init() {}

    func show(
        _ content: CandidateBarContent,
        anchoredTo caretRect: CGRect,
        hostWindowLevel: CGWindowLevel,
        ownedBy owner: ComposingSessionToken,
    ) {
        let bar = bar ?? makeBar()
        self.bar = bar

        bar.hostingView.rootView = CandidateBarView(content: content)
        // Measured after the content is set and before the frame is applied:
        // `fittingSize` is what the row wants, and asking for it a keystroke
        // late is how a bar ends up sized for the candidates it just replaced.
        bar.hostingView.layoutSubtreeIfNeeded()
        let panelSize = bar.hostingView.fittingSize

        guard let screen = ScreenLookup.screen(containing: caretRect.origin) else {
            // No display to place it on. The window still holds the previous
            // content, so leaving it up would show a stale list — it comes down,
            // and ownership goes with it.
            Self.logger.debug("no screen for caret \(String(describing: caretRect))")
            hideNow()
            return
        }

        bar.panel.setFrame(
            CandidatePanelPositioning.frame(
                anchoredTo: caretRect,
                panelSize: panelSize,
                within: screen.visibleFrame,
            ),
            display: true,
        )
        // Just above the window being typed into, so the bar is not hidden by
        // the host's own floating panels and does not float over unrelated apps.
        bar.panel.level = NSWindow.Level(rawValue: Int(hostWindowLevel) + 1)
        // `orderFront(nil)` is documented to do nothing for an application that
        // is not active, which an input method never is.
        bar.panel.orderFrontRegardless()
        // Taken only once the bar is really on screen: an owner recorded for a
        // window that was never shown would turn the NEXT session's
        // `hide(ownedBy:)` into a no-op against a bar it does not own.
        self.owner = owner
    }

    func hide(ownedBy owner: ComposingSessionToken) {
        guard self.owner == owner else { return }
        hideNow()
    }

    func hideForHandover() {
        hideNow()
    }

    private func hideNow() {
        owner = nil
        bar?.panel.orderOut(nil)
    }

    private func makeBar() -> Bar {
        let hostingView = NSHostingView(
            rootView: CandidateBarView(content: CandidateBarContent(labels: [], highlightedSlot: nil)),
        )
        let panel = NSPanel(
            contentRect: .zero,
            // Non-activating is what keeps the host's caret blinking and its
            // menu bar its own while the bar is up: an input method that steals
            // key focus to show a list has taken the user out of their document.
            styleMask: [.nonactivatingPanel, .borderless],
            backing: .buffered,
            defer: true,
        )
        panel.isFloatingPanel = true
        panel.hidesOnDeactivate = false
        panel.isReleasedWhenClosed = false
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        // The bar follows the user to whichever space and full-screen app they
        // are typing in. `.stationary` is deliberately not set: it would hold
        // the bar visible and pinned through Mission Control, where the document
        // it is anchored to is no longer where it was.
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        panel.contentView = hostingView
        return Bar(panel: panel, hostingView: hostingView)
    }
}
