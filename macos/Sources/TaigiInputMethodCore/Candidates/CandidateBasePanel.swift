// The chrome every candidate layout shares: panel, backdrop, placement, drag.

import AppKit

/// The window the candidate layouts render in, ported from MacishType's
/// `MacishBasePanel` (`references/MacishType/macos/MacishType/
/// MacishCandidateWindow/MacishBasePanel.swift`; MIT, © 2026 Luke Chang) with
/// its placement replaced by this repo's own: `CandidatePanelPositioning` is
/// pure, tested, clamps oversized panels, and works from the real caret rect —
/// upstream's `topLeftPoint` needs composition-bound x coordinates this input
/// method does not track.
///
/// One subclass per layout. The panel is a renderer plus the selection it is
/// authoritative for; who OWNS the window (which session may hide it) is
/// `CandidatePanel`'s business, one level up.
class CandidateBasePanel: NSPanel, CandidateWindowDragging {
    let style: CandidateWindowStyle
    /// The size metrics every cell and layout in this window renders at.
    /// Fixed at construction like `style`: a change rebuilds the panel
    /// (`CandidatePanel.panel(for:)`).
    let metrics: CandidateMetrics
    private(set) var backdrop: CandidateBackdrop
    /// The layouts' canvas, origin at the top-left like the layouts think.
    let contentContainer = FlippedContainerView()

    static let sequoiaCornerRadius: CGFloat = 6

    /// Upstream's display cap, kept on purpose: a continuous fetch can answer
    /// with a very long tail of low-rank candidates, and a window that tries to
    /// build a cell for every one of them pays for candidates nobody will page
    /// to. Stated in the port plan so the truncation is a decision.
    static let maxDisplayCandidates = 200

    private(set) var highlightColor: NSColor = .selectedContentBackgroundColor
    /// The Multicolour accent follows the HOST app; recorded at show time.
    private var hostBundleIdentifier: String?

    /// Where the window was last anchored — pages that change the window's
    /// size mid-navigation re-place it against the same caret.
    private(set) var lastCaretRect: CGRect = .zero

    private var accentObserver: (any NSObjectProtocol)?

    private(set) var didDrag = false
    private var dragOffsetInWindow: NSPoint = .zero
    private var dragStartOnScreen: NSPoint = .zero

    init(style: CandidateWindowStyle, metrics: CandidateMetrics) {
        self.style = style
        self.metrics = metrics
        backdrop = CandidateBackdrop.make(style: style)
        super.init(
            contentRect: .zero,
            // Non-activating is what keeps the host's caret blinking and its
            // menu bar its own while the window is up: an input method that
            // steals key focus to show a list has taken the user out of their
            // document.
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false,
        )
        isFloatingPanel = true
        hidesOnDeactivate = false
        isReleasedWhenClosed = false
        isOpaque = false
        backgroundColor = .clear
        hasShadow = true
        // The window follows the user to whichever space and full-screen app
        // they are typing in. `.stationary` is deliberately not set: it would
        // hold the window pinned through Mission Control, where the document it
        // is anchored to is no longer where it was.
        collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]

        contentView = backdrop.view
        contentContainer.translatesAutoresizingMaskIntoConstraints = false
        backdrop.contentArea.addSubview(contentContainer)
        NSLayoutConstraint.activate([
            contentContainer.topAnchor.constraint(equalTo: backdrop.contentArea.topAnchor),
            contentContainer.leadingAnchor.constraint(equalTo: backdrop.contentArea.leadingAnchor),
            contentContainer.trailingAnchor.constraint(equalTo: backdrop.contentArea.trailingAnchor),
            contentContainer.bottomAnchor.constraint(equalTo: backdrop.contentArea.bottomAnchor),
        ])

        accentObserver = NotificationCenter.default.addObserver(
            forName: CandidateAccentColor.didChange,
            object: nil,
            queue: .main,
        ) { [weak self] _ in
            // The notification arrives on the main queue; hop through the actor
            // rather than assume it, so the compiler keeps the isolation honest.
            Task { @MainActor [weak self] in
                self?.syncTheme()
            }
        }
    }

    @MainActor deinit {
        if let accentObserver {
            NotificationCenter.default.removeObserver(accentObserver)
        }
    }

    // MARK: - Theme

    /// Re-resolves the highlight colour for the current accent and host, and
    /// hands it to the subclass's cells.
    func syncTheme() {
        highlightColor = CandidateAccentColor.shared.highlightColor(
            style: style,
            hostBundleIdentifier: hostBundleIdentifier,
            appearance: effectiveAppearance,
        )
        applyHighlightColor(highlightColor)
    }

    // MARK: - Placement

    /// Places the window sized `panelSize` near `caretRect` and brings it on
    /// screen, one level above the host so the host's own floating panels do
    /// not cover it and it does not float over unrelated apps.
    func present(
        panelSize: CGSize,
        anchoredTo caretRect: CGRect,
        hostWindowLevel: CGWindowLevel,
        hostBundleIdentifier: String?,
        forcedAppearance: NSAppearance?,
    ) -> Bool {
        guard let screen = ScreenLookup.screen(containing: caretRect.origin) else {
            return false
        }
        lastCaretRect = caretRect
        self.hostBundleIdentifier = hostBundleIdentifier
        // Nil resolves against the system — the 自動 behaviour. Set before
        // `syncTheme` below, whose Tahoe correction reads the effective
        // appearance this assignment decides.
        appearance = forcedAppearance
        setFrame(
            CandidatePanelPositioning.frame(
                anchoredTo: caretRect,
                panelSize: panelSize,
                within: screen.visibleFrame,
            ),
            display: true,
        )
        level = NSWindow.Level(rawValue: Int(hostWindowLevel) + 1)
        updateCorners()
        syncTheme()
        // `orderFront(nil)` is documented to do nothing for an application
        // that is not active, which an input method never is.
        orderFrontRegardless()
        return true
    }

    /// Re-places the window after navigation changed its size (a shorter last
    /// page, an expanded grid), against the caret it was shown for.
    ///
    /// A same-size call is a no-op on purpose: re-anchoring would also snap the
    /// window back from wherever the user dragged it, and a page turn that
    /// changes nothing about the frame is not a reason to take that position
    /// away.
    func replace(panelSize: CGSize) {
        guard panelSize != frame.size else { return }
        guard isVisible, let newFrame = anchoredFrame(for: panelSize) else { return }
        setFrame(newFrame, display: true)
        updateCorners()
    }

    /// The frame a window sized `panelSize` would take against the caret it
    /// was last shown for, or nil when there is no anchor to place it by. The
    /// expandable layout computes its animation target through this.
    func anchoredFrame(for panelSize: CGSize) -> NSRect? {
        guard lastCaretRect != .zero,
              let screen = ScreenLookup.screen(containing: lastCaretRect.origin)
        else { return nil }
        return CandidatePanelPositioning.frame(
            anchoredTo: lastCaretRect,
            panelSize: panelSize,
            within: screen.visibleFrame,
        )
    }

    func hide() {
        orderOut(nil)
        lastCaretRect = .zero
    }

    // MARK: - Dragging

    // The window can be dragged out of the way of the text it covers. The next
    // keystroke re-fetches and re-anchors to the caret — the drag holds for
    // the current list only, which is the rule rather than a bug: a window
    // parked far from a moving caret would soon describe text somewhere else.

    override func mouseDown(with event: NSEvent) {
        dragOffsetInWindow = event.locationInWindow
        dragStartOnScreen = NSEvent.mouseLocation
        didDrag = false
    }

    override func mouseDragged(with _: NSEvent) {
        let screenPoint = NSEvent.mouseLocation
        if !didDrag {
            let dx = screenPoint.x - dragStartOnScreen.x
            let dy = screenPoint.y - dragStartOnScreen.y
            // A few points of slop keeps a slightly wobbly click a click.
            guard dx * dx + dy * dy > 9 else { return }
            didDrag = true
        }
        setFrameOrigin(NSPoint(
            x: screenPoint.x - dragOffsetInWindow.x,
            y: screenPoint.y - dragOffsetInWindow.y,
        ))
    }

    // MARK: - Subclass override points

    /// The layout interface `CandidatePanel` routes through, so the router can
    /// hold whichever layout the setting names without knowing which subclass
    /// it is. Every layout is authoritative for its own selection.

    var isEmpty: Bool { true }

    /// The absolute index of the selected candidate. Stored here because every
    /// layout speaks the same absolute-index identity; meaningful only while
    /// `isEmpty` is false — every fresh list selects index 0. Written by the
    /// subclass that owns the navigation.
    var selectedIndex = 0

    /// Replaces the list, selecting its first candidate, and answers with the
    /// size the window wants. The caller places and shows it.
    ///
    /// The layout methods trap rather than default to a no-op: a layout that
    /// forgot one would otherwise fail silently — a window that shows but
    /// never navigates — and the trap turns that into the first keystroke of
    /// development.
    func updateCandidates(_: [CandidateCellContent]) -> CGSize {
        preconditionFailure("layout subclasses must override updateCandidates")
    }

    /// Replaces every cell's content in place — the same list under a new
    /// rendering — keeping the selection on the same absolute index, and
    /// re-places its own frame for the new measured widths (`replace` keeps a
    /// same-size drag where it is). NOT `updateCandidates`: that is the
    /// fresh-list contract and resets the selection and the page.
    func rerenderCandidates(_: [CandidateCellContent]) {
        preconditionFailure("layout subclasses must override rerenderCandidates")
    }

    /// Empties the window so nothing can be selected or committed from it —
    /// hiding must drop the state, not just the pixels.
    func clear() { hide() }

    /// Moves the selection the way this layout reads `direction`.
    func navigate(_: CandidateNavigation) {
        preconditionFailure("layout subclasses must override navigate")
    }

    /// The absolute index the `⌃(slot+1)` chord addresses in the rows or page
    /// the user can currently see.
    func candidateIndex(forSlot _: Int) -> Int? {
        preconditionFailure("layout subclasses must override candidateIndex(forSlot:)")
    }

    /// Repaints every cell with the freshly resolved highlight colour.
    func applyHighlightColor(_: NSColor) {}

    /// Re-applies the corner mask for the current frame size and page state.
    func updateCorners() {
        let size = frame.size
        guard size.width > 0, size.height > 0 else { return }
        let radius: CGFloat = switch style {
        case .sequoia: Self.sequoiaCornerRadius
        case .tahoe: metrics.itemHeight / 2
        }
        backdrop.applyUniformCorners(size: size, radius: radius)
    }

    /// The pill shape a paged window's turning edge takes: Sequoia rounds only
    /// the arrow side into a half-height pill, Tahoe is a capsule either way.
    func applyPillCorners(size: NSSize) {
        switch style {
        case .sequoia:
            backdrop.applyAsymmetricCorners(
                size: size,
                leftRadius: Self.sequoiaCornerRadius,
                rightRadius: size.height / 2,
            )
        case .tahoe:
            backdrop.applyUniformCorners(size: size, radius: metrics.itemHeight / 2)
        }
    }
}

/// A plain view whose origin is the top-left corner, so layout code can place
/// row 0 at y=0 and grow downward the way it reads.
final class FlippedContainerView: NSView {
    override var isFlipped: Bool { true }
}
