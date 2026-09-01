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
    /// Which chrome generation this window draws. Injected rather than read
    /// from `CandidateWindowStyle.systemStyle` so the geometry tests can pin
    /// `.sequoia` and assert fixed numbers whatever OS they run on; production
    /// has one caller and it passes the system's (`CandidatePanel`).
    let style: CandidateWindowStyle
    /// The size metrics this window was built for. Fixed at construction, and
    /// settings-driven, so a change rebuilds the panel
    /// (`CandidatePanel.panel(for:)` compares against this one).
    let configuredMetrics: CandidateMetrics
    /// The size metrics every cell and layout in this window renders at:
    /// `configuredMetrics`, resolved for the list on screen — a stacked layout
    /// showing a list with no annotated cell renders it one line tall
    /// (`CandidateMetrics.forContent(hasAnnotations:)`). Re-resolved by the two
    /// entry points that hand the panel cells, `layout(_:forCaret:)` and
    /// `rerender(_:)`, BEFORE the layout reads it, so every row of one list
    /// shares one height and a mode change under an open window reflows it.
    private(set) var metrics: CandidateMetrics
    private(set) var backdrop: CandidateBackdrop
    /// The layouts' canvas, origin at the top-left like the layouts think.
    let contentContainer = FlippedContainerView()

    static let sequoiaCornerRadius: CGFloat = 6

    /// Upstream's display cap, kept on purpose: a continuous fetch can answer
    /// with a very long tail of low-rank candidates, and a window that tries to
    /// build a cell for every one of them pays for candidates nobody will page
    /// to. Stated in the port plan so the truncation is a decision.
    static let maxDisplayCandidates = 200

    /// How much of its screen a stretched window leaves unspent, so a long
    /// candidate never grows the window edge-to-edge across the display.
    private static let screenEdgeMargin: CGFloat = 12

    /// The width budget a panel lays out against before any caret has named a
    /// screen — wide enough for a long phrase, narrow enough to fit the
    /// smallest display this input method runs on.
    private static let fallbackMaximumWindowWidth: CGFloat = 640

    /// The widest this window may render, resolved from the screen the caret
    /// sits on (`layout(_:forCaret:)`).
    ///
    /// The layouts size their cells against it rather than against a fixed
    /// multiple of the slot width: a candidate too long for its cell truncates
    /// with an ellipsis, so the cell — and the window with it — grows instead,
    /// up to what the screen can hold. Past that the truncation is the right
    /// answer, since `CandidatePanelPositioning` clamps an oversized window to
    /// the screen and text outside it would be CLIPPED rather than elided.
    private(set) var maximumWindowWidth: CGFloat = fallbackMaximumWindowWidth

    /// The vertical scroller's width for the style the system currently
    /// prefers. Read through one accessor because the scrolling layouts both
    /// subtract it from the width budget and add it back onto the window: two
    /// spellings could disagree and push the content past the frame.
    var currentScrollerWidth: CGFloat {
        NSScroller.scrollerWidth(for: .regular, scrollerStyle: NSScroller.preferredScrollerStyle)
    }

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
        configuredMetrics = metrics
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

    /// Lays `cells` out for the screen `caretRect` is on and answers with the
    /// window size that fits them.
    ///
    /// The budget is resolved here rather than in `present`, which is handed
    /// the same caret only AFTER the layout has answered with a size: cells are
    /// measured against the budget, so it has to be known before they are laid
    /// out, not once they are placed. `final` so no layout can spend a budget
    /// that was never resolved.
    final func layout(_ cells: [CandidateCellContent], forCaret caretRect: CGRect) -> CGSize {
        resolveMetrics(for: cells)
        if let screen = ScreenLookup.screen(containing: caretRect.origin) {
            maximumWindowWidth = max(
                metrics.baseWidth,
                screen.visibleFrame.width - 2 * Self.screenEdgeMargin,
            )
        }
        return updateCandidates(cells)
    }

    /// The in-place counterpart of `layout(_:forCaret:)`: the same list under
    /// a new rendering, through `rerenderCandidates`. `final` for the same
    /// reason — the metrics the new cells render at are resolved here, before
    /// any layout reads them.
    final func rerender(_ cells: [CandidateCellContent]) {
        resolveMetrics(for: cells)
        rerenderCandidates(cells)
    }

    private func resolveMetrics(for cells: [CandidateCellContent]) {
        metrics = configuredMetrics.forContent(hasAnnotations: cells.contains { $0.annotation != nil })
    }

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
    /// fresh-list contract and resets the selection and the page. Reached
    /// through `rerender(_:)`, which resolves `metrics` first.
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

    /// Which keys pick the candidates — the window draws each beside its
    /// numbered cell.
    ///
    /// Set from the content each `show` carries, before the cells it belongs
    /// to are built, so the keys are resolved as the cells are. A
    /// display-only re-render keeps it.
    var slotKeySet: CandidateSlotKeySet = .bareKeys

    /// Every cell this layout currently draws a key beside, in any order.
    ///
    /// Overridden rather than held here because each layout keeps its own item
    /// views — the expandable one keeps two lists, since its grid rows are
    /// built separately from the row it unfolds from. Traps like its siblings
    /// above: a layout that forgot it would leave stale keys on the cells now
    /// on screen after a page turn or a scroll, which is the silent failure
    /// they all guard against.
    var numberedItemViews: [CandidateItemView] {
        preconditionFailure("layout subclasses must override numberedItemViews")
    }

    /// Draws the key that picks each numbered cell, and blanks the rest.
    ///
    /// Derived by ASKING `candidateIndex(forSlot:)` — the same override the key
    /// handler resolves a `1`…`9` press against (`TaigiInputController`) — so
    /// the digit a cell shows and the candidate that key commits cannot drift
    /// apart. Each layout's own numbering falls out of its slot mapping: the
    /// horizontal page restarts at `1`, the vertical column follows its scroll
    /// anchor, and the expanded grid numbers only the row the selection is in.
    ///
    /// The layouts keep only their triggers — a page rebuild, an anchor change,
    /// a selection or mode change — since that is the part their geometries do
    /// not share.
    func refreshIndexLabels() {
        var keyByCandidate: [Int: String] = [:]
        for slot in 0 ..< HorizontalPageLayout.pageSize {
            guard let candidateIndex = candidateIndex(forSlot: slot) else { continue }
            keyByCandidate[candidateIndex] = CandidateIndexLabel.text(forSlot: slot, keySet: slotKeySet)
        }
        for item in numberedItemViews {
            item.setIndexLabel(keyByCandidate[item.absoluteIndex] ?? "")
        }
    }

    /// Repaints every cell with the freshly resolved highlight colour.
    func applyHighlightColor(_: NSColor) {}

    /// Re-applies the corner mask for the current frame size and page state.
    func updateCorners() {
        let size = frame.size
        guard size.width > 0, size.height > 0 else { return }
        let radius: CGFloat = switch style {
        case .sequoia: Self.sequoiaCornerRadius
        case .tahoe: tahoeCornerRadius(for: size)
        }
        backdrop.applyUniformCorners(size: size, radius: radius)
    }

    /// The shape a paged window's turning edge takes: Sequoia rounds only the
    /// arrow side into a half-height pill, Tahoe keeps the one radius it
    /// rounds every edge to, paged or not.
    func applyPillCorners(size: NSSize) {
        switch style {
        case .sequoia:
            backdrop.applyAsymmetricCorners(
                size: size,
                leftRadius: Self.sequoiaCornerRadius,
                rightRadius: size.height / 2,
            )
        case .tahoe:
            backdrop.applyUniformCorners(size: size, radius: tahoeCornerRadius(for: size))
        }
    }

    /// The window's Tahoe radius, held to what this window can round.
    private func tahoeCornerRadius(for size: NSSize) -> CGFloat {
        CandidateMetrics.cornerRadius(metrics.tahoeContainerCornerRadius, fitting: size)
    }
}

/// A plain view whose origin is the top-left corner, so layout code can place
/// row 0 at y=0 and grow downward the way it reads.
final class FlippedContainerView: NSView {
    override var isFlipped: Bool { true }
}
