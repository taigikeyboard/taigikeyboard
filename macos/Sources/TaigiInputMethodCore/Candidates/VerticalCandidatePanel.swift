// The vertical layout: a scrolling column of rows, chords follow the viewport.

import AppKit

/// The scrolling vertical candidate window — MacishType's `MacishVerticalPanel`
/// (`references/MacishType/macos/MacishType/MacishCandidateWindow/
/// MacishVerticalPanel.swift`; MIT, © 2026 Luke Chang) with two simplifications
/// the Codex pre-impl confirmed:
///
/// - No annotation column, so its column-alignment, annotation-visibility and
///   top-3 width heuristic all collapse: the width is the widest of the
///   displayed labels, measured eagerly — with the 200-candidate display cap
///   that is bounded work, and it removes the mid-scroll window-widening
///   animation upstream needs to correct a heuristic miss.
/// - Rows are built up front rather than lazily for the same reason: the cap
///   bounds them, and the lazy build exists to hide measurement work this port
///   no longer defers.
///
/// What is kept, because it is the layout's behaviour: the scroll-anchored
/// numbering (the `⌃n` chords address the nine rows around the viewport, and
/// move with it), the half-row bottom peek that shows there is more to scroll
/// to, the scroller-style-aware geometry, and Tahoe's separator treatment.
final class VerticalCandidatePanel: CandidateBasePanel {
    /// How many rows the window shows and the chords address — the same nine
    /// the horizontal page holds.
    private static let visibleRows = HorizontalPageLayout.pageSize

    /// Content width cap, in slot-widths. Independent of the row count.
    private static let maxContentColumns: CGFloat = 6

    /// Air between an overlay scroller and the text it would otherwise touch.
    private static let overlayScrollerGap: CGFloat = 2

    private static let separatorHeight: CGFloat = 1

    private var labels: [String] = []
    /// The first row of the nine the `⌃n` chords currently address, derived
    /// from the scroll position — labels renumber as the user scrolls.
    private var anchorRow = 0
    /// The content height all rows want; scrolling shrinks back toward it
    /// after a page jump temporarily grows the container.
    private var naturalContentHeight: CGFloat = 0
    /// Guards the scroll observer during rebuilds: a bounds-change fired by
    /// resizing the container mid-build would renumber a half-built row set.
    private var isBuildingLayout = false

    private let scrollView = NSScrollView()
    private let rowsContainer = FlippedContainerView()
    private var itemViews: [CandidateItemView] = []
    private var separatorViews: [CandidateSeparatorView] = []
    private var boundsObserver: (any NSObjectProtocol)?
    private var scrollerStyleObserver: (any NSObjectProtocol)?

    override var isEmpty: Bool { labels.isEmpty }

    override init(style: CandidateWindowStyle) {
        super.init(style: style)

        scrollView.translatesAutoresizingMaskIntoConstraints = false
        scrollView.drawsBackground = false
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = false
        scrollView.autohidesScrollers = true
        scrollView.scrollerStyle = NSScroller.preferredScrollerStyle
        scrollView.documentView = rowsContainer
        contentContainer.addSubview(scrollView)
        NSLayoutConstraint.activate([
            scrollView.topAnchor.constraint(equalTo: contentContainer.topAnchor),
            scrollView.leadingAnchor.constraint(equalTo: contentContainer.leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: contentContainer.trailingAnchor),
            scrollView.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor),
        ])

        scrollView.contentView.postsBoundsChangedNotifications = true
        boundsObserver = NotificationCenter.default.addObserver(
            forName: NSView.boundsDidChangeNotification,
            object: scrollView.contentView,
            queue: .main,
        ) { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.scrollViewDidScroll()
            }
        }
        // Plugging in a mouse can flip the whole system from overlay to legacy
        // scrollers, which changes how much width the scroller costs.
        scrollerStyleObserver = NotificationCenter.default.addObserver(
            forName: NSScroller.preferredScrollerStyleDidChangeNotification,
            object: nil,
            queue: .main,
        ) { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.handleScrollerStyleChange()
            }
        }
    }

    @MainActor deinit {
        if let boundsObserver {
            NotificationCenter.default.removeObserver(boundsObserver)
        }
        if let scrollerStyleObserver {
            NotificationCenter.default.removeObserver(scrollerStyleObserver)
        }
    }

    // MARK: - Content

    private var rowHeight: CGFloat { CandidateItemView.Metrics.itemHeight + Self.separatorHeight }

    override func updateCandidates(_ newLabels: [String]) -> CGSize {
        labels = Array(newLabels.prefix(Self.maxDisplayCandidates))
        selectedIndex = 0
        return rebuildRows()
    }

    override func clear() {
        labels = []
        selectedIndex = 0
        anchorRow = 0
        removeRowViews()
        hide()
    }

    // MARK: - Selection

    /// The `⌃(slot+1)` chord addresses the rows as currently numbered — the
    /// nine starting at the viewport's anchor, which is what the visible
    /// labels say.
    override func candidateIndex(forSlot slot: Int) -> Int? {
        guard (0 ..< Self.visibleRows).contains(slot) else { return nil }
        let index = anchorRow + slot
        return labels.indices.contains(index) ? index : nil
    }

    override func navigate(_ direction: CandidateNavigation) {
        guard !labels.isEmpty else { return }
        switch direction {
        case .up:
            select(max(selectedIndex - 1, 0))
        case .down:
            select(min(selectedIndex + 1, labels.count - 1))
        // A column has no candidate to the left or right, so the horizontal
        // keys page — backward and forward respectively, as upstream binds
        // them (`MacishVerticalPanel.swift:333-372`).
        case .right, .pageDown:
            jumpPage(by: 1)
        case .left, .pageUp:
            jumpPage(by: -1)
        }
    }

    /// Moves the selection a whole viewport, keeping it at the same visual
    /// row within it — the highlight stays put on screen while the list moves
    /// underneath, which is what a page key reads as. Clamps into the ends.
    private func jumpPage(by pages: Int) {
        let visualOffset = max(selectedIndex - anchorRow, 0)
        let targetAnchor = anchorRow + pages * Self.visibleRows
        if targetAnchor >= 0, targetAnchor < labels.count {
            let target = min(targetAnchor + visualOffset, labels.count - 1)
            scrollRowToTop(targetAnchor)
            select(target)
        } else if pages < 0, anchorRow > 0 {
            // Less than a whole page above: anchor to the top but keep the
            // visual row, as upstream does (`MacishVerticalPanel.swift:355-359`)
            // — jumping the highlight to 0 would move it on screen.
            scrollRowToTop(0)
            select(min(visualOffset, labels.count - 1))
        } else {
            // No page left in that direction at all: land on that end.
            select(pages > 0 ? labels.count - 1 : 0)
        }
    }

    private func select(_ index: Int) {
        guard labels.indices.contains(index) else { return }
        selectedIndex = index
        ensureSelectionVisible()
        updateHighlights()
    }

    // MARK: - Layout

    private func rebuildRows() -> CGSize {
        isBuildingLayout = true
        defer { isBuildingLayout = false }

        anchorRow = 0
        removeRowViews()
        guard !labels.isEmpty else { return .zero }

        let itemHeight = CandidateItemView.Metrics.itemHeight
        let hasOverflow = labels.count > Self.visibleRows

        // Width: the widest displayed label, floored at one slot and capped so
        // one long phrase cannot stretch the window across the screen.
        let widest = labels.map(CandidateItemView.measureWidth).max() ?? 0
        let contentWidth = min(
            max(widest, CandidateItemView.baseWidth),
            CandidateItemView.baseWidth * Self.maxContentColumns,
        )
        let geometry = scrollerGeometry(contentWidth: contentWidth, hasOverflow: hasOverflow)

        // Height: nine rows, plus half a row peeking when there is more — the
        // cut-off row is what says "scroll me".
        let visibleCount = min(labels.count, Self.visibleRows)
        let bottomPeek: CGFloat = hasOverflow ? itemHeight / 2 : 0
        let windowHeight = CGFloat(visibleCount) * itemHeight
            + CGFloat(max(visibleCount - 1, 0)) * Self.separatorHeight
            + bottomPeek
        naturalContentHeight = CGFloat(labels.count) * itemHeight
            + CGFloat(max(labels.count - 1, 0)) * Self.separatorHeight
            + bottomPeek
        rowsContainer.frame.size = NSSize(width: geometry.itemWidth, height: naturalContentHeight)

        for (index, label) in labels.enumerated() {
            let item = CandidateItemView(style: style)
            item.absoluteIndex = index
            item.highlightColor = highlightColor
            item.trailingInset = geometry.itemTrailing
            item.configure(slotLabel: "", candidate: label)
            item.frame = NSRect(x: 0, y: yForRow(index), width: geometry.itemWidth, height: itemHeight)
            item.onClick = { [weak self, weak item] in
                guard let self, let item else { return }
                select(item.absoluteIndex)
            }
            rowsContainer.addSubview(item)
            itemViews.append(item)
        }
        for index in 0 ..< max(labels.count - 1, 0) {
            let separator = CandidateSeparatorView()
            separator.horizontalInset = style == .tahoe ? 8 : 0
            separator.frame = NSRect(
                x: 0, y: yForRow(index) + itemHeight,
                width: geometry.itemWidth, height: Self.separatorHeight,
            )
            rowsContainer.addSubview(separator)
            separatorViews.append(separator)
        }

        scrollView.contentView.scroll(to: .zero)
        scrollView.reflectScrolledClipView(scrollView.contentView)
        anchorRow = -1 // Force the renumber below — 0 would read as "already current".
        updateRowNumbering()
        updateHighlights()

        if hasOverflow, NSScroller.preferredScrollerStyle != .legacy {
            scrollView.flashScrollers()
        }
        return CGSize(width: geometry.windowWidth, height: windowHeight)
    }

    /// How the scroller style splits the window's width, from upstream
    /// (`MacishVerticalPanel.swift:283-300`): legacy scrollers get their own
    /// column outside the rows so rounded corners are not clipped; overlay
    /// scrollers float over a widened trailing inset so text stays clear.
    private func scrollerGeometry(
        contentWidth: CGFloat,
        hasOverflow: Bool,
    ) -> (windowWidth: CGFloat, itemWidth: CGFloat, itemTrailing: CGFloat) {
        let naturalPadding = CandidateItemView.Metrics.trailingPadding
        guard hasOverflow else { return (contentWidth, contentWidth, naturalPadding) }
        if NSScroller.preferredScrollerStyle == .legacy {
            let scrollerWidth = NSScroller.scrollerWidth(for: .regular, scrollerStyle: .legacy)
            return (contentWidth + scrollerWidth, contentWidth, naturalPadding)
        }
        let scrollerWidth = NSScroller.scrollerWidth(for: .regular, scrollerStyle: .overlay)
        let trailing = max(naturalPadding, scrollerWidth + Self.overlayScrollerGap)
        let width = contentWidth - naturalPadding + trailing
        return (width, width, trailing)
    }

    private func yForRow(_ row: Int) -> CGFloat {
        CGFloat(row) * rowHeight
    }

    // MARK: - Scrolling

    private func scrollViewDidScroll() {
        guard !isBuildingLayout else { return }
        // Shrink the container back toward its natural height after a page
        // jump grew it — growing is `scrollRowToTop`'s alone.
        let viewport = scrollView.contentView.bounds
        let neededHeight = viewport.origin.y + viewport.height
        let targetHeight = max(naturalContentHeight, neededHeight)
        if targetHeight < rowsContainer.frame.height {
            rowsContainer.frame.size.height = targetHeight
        }
        updateRowNumbering()
    }

    private func scrollRowToTop(_ row: Int) {
        let targetY = yForRow(max(row, 0))
        // A jump near the end may need more container than the rows fill, so
        // the anchor row can still reach the top.
        let neededHeight = targetY + scrollView.contentView.bounds.height
        if neededHeight > rowsContainer.frame.height {
            rowsContainer.frame.size.height = neededHeight
        }
        scrollView.contentView.scroll(to: NSPoint(x: 0, y: targetY))
        scrollView.reflectScrolledClipView(scrollView.contentView)
    }

    private func ensureSelectionVisible() {
        let itemHeight = CandidateItemView.Metrics.itemHeight
        let rowTop = yForRow(selectedIndex)
        let rowBottom = rowTop + itemHeight
        let viewport = scrollView.contentView.bounds

        if rowTop < viewport.minY {
            scrollView.contentView.scroll(to: NSPoint(x: 0, y: rowTop))
            scrollView.reflectScrolledClipView(scrollView.contentView)
        } else if rowBottom > viewport.maxY {
            let maxScrollY = rowsContainer.frame.height - viewport.height
            // Half a row deeper than strictly needed, so the next row peeks
            // and the list does not look like it ends at the selection.
            let targetY = min(rowBottom + itemHeight / 2 - viewport.height, maxScrollY)
            scrollView.contentView.scroll(to: NSPoint(x: 0, y: max(targetY, 0)))
            scrollView.reflectScrolledClipView(scrollView.contentView)
        }
        updateRowNumbering()
    }

    // MARK: - Numbering & highlights

    private func updateRowNumbering() {
        let scrollOffset = max(scrollView.contentView.bounds.origin.y, 0)
        // Half-row bias: the row MOSTLY on screen is the one the chord names.
        let newAnchor = max(Int(floor((scrollOffset + rowHeight / 2) / rowHeight)), 0)
        guard newAnchor != anchorRow else { return }
        anchorRow = newAnchor

        let numberedRows = anchorRow ..< min(anchorRow + Self.visibleRows, labels.count)
        for item in itemViews {
            if numberedRows.contains(item.absoluteIndex) {
                item.showsSlotLabel = true
                item.configure(
                    slotLabel: CandidateItemView.slotLabels[item.absoluteIndex - anchorRow],
                    candidate: labels[item.absoluteIndex],
                )
            } else {
                item.showsSlotLabel = false
            }
        }
    }

    private func updateHighlights() {
        for item in itemViews {
            item.isHighlighted = item.absoluteIndex == selectedIndex
        }
        guard style == .tahoe else { return }
        // Tahoe suppresses the hairlines touching the selection pill — the
        // pill supplies the row's edges (`MacishVerticalPanel.swift:430-437`).
        for (index, separator) in separatorViews.enumerated() {
            let touchesSelection = index == selectedIndex - 1 || index == selectedIndex
            separator.alphaValue = touchesSelection ? 0 : 1
        }
    }

    // MARK: - Chrome

    override func applyHighlightColor(_ color: NSColor) {
        for item in itemViews {
            item.highlightColor = color
        }
    }

    private func handleScrollerStyleChange() {
        scrollView.scrollerStyle = NSScroller.preferredScrollerStyle
        guard isVisible, !labels.isEmpty else { return }
        replace(panelSize: rebuildRows())
        // The rebuild scrolled back to the top; the selection survives it, so
        // bring its row back on screen — a Space against an off-screen
        // highlight would commit a candidate the user cannot see.
        ensureSelectionVisible()
    }

    private func removeRowViews() {
        itemViews.forEach { $0.removeFromSuperview() }
        itemViews = []
        separatorViews.forEach { $0.removeFromSuperview() }
        separatorViews = []
    }
}
