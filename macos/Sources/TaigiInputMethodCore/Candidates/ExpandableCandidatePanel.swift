// The expandable layout: one row that unfolds into a column-aligned grid.

import AppKit

/// The expandable candidate window — MacishType's signature layout
/// (`references/MacishType/macos/MacishType/MacishCandidateWindow/
/// MacishHorizontalExpandablePanel.swift`; MIT, © 2026 Luke Chang): a single
/// packed row with a chevron, which unfolds in place into a column-quantized
/// grid when the user asks for more.
///
/// Ported with upstream's defaults baked in — wider expanded columns
/// (`pageSize - pageSize/3` = six), five visible grid rows, no move-on-expand
/// — and with the suspended-selection paths dropped like the other layouts.
/// The grid geometry lives in `ExpandedGridLayout`; the collapsed row is the
/// first page of the same packing the horizontal layout uses. The animated
/// unfold is kept: it is what tells the user the grid IS the row they were
/// looking at, grown — not a different window.
final class ExpandableCandidatePanel: CandidateBasePanel {
    /// How many grid rows show before the expanded window scrolls.
    private static let maxVisibleRows = 5
    /// Upstream's measured ease and duration for the unfold.
    private static let animationDuration: TimeInterval = 0.183
    private static let separatorHeight: CGFloat = 1

    enum DisplayMode {
        case collapsed
        case expanded
    }

    private var cells: [CandidateCellContent] = []
    private var measuredWidths: [CGFloat] = []
    /// Readable from outside because it is the observable half of what a
    /// navigation key did: one keystroke can both move the selection and open
    /// the grid.
    private(set) var displayMode: DisplayMode = .collapsed
    /// The collapsed row: first page of the horizontal packing.
    private var collapsedRow: [HorizontalPageLayout.Slot] = []
    private var grid = ExpandedGridLayout(rows: [], columnCount: 1)
    private var isGridBuilt = false
    private var expandedColumnWidth: CGFloat = 0
    private var expandedColumnCount: Int {
        HorizontalPageLayout.pageSize - HorizontalPageLayout.pageSize / 3
    }

    private let scrollView = NSScrollView()
    private let rowsContainer = FlippedContainerView()
    /// Cells of the collapsed row. In the expanded mode the ones that also
    /// live on the grid's first row stay (animated into their column frames);
    /// the rest — the "overflow" — fade out, and duplicates of them exist in
    /// `expandedItemViews` on later rows.
    private var row0ItemViews: [CandidateItemView] = []
    /// Cells of the grid's rows 1+, built on the first expand of each list.
    private var expandedItemViews: [CandidateItemView] = []

    /// Both lists: expanded, the grid's later rows are numbered too.
    override var allItemViews: [CandidateItemView] { row0ItemViews + expandedItemViews }
    private var separatorViews: [CandidateSeparatorView] = []
    private var chevronView: CandidateChevronView!
    /// Sequoia's expanded selection: a translucent bar under the selected
    /// row. Tahoe highlights per cell instead, like the other layouts.
    private var rowHighlightView: CandidateRowHighlightView?

    private var isAnimating = false
    private var scrollerStyleObserver: (any NSObjectProtocol)?

    override var isEmpty: Bool { cells.isEmpty }

    override init(style: CandidateWindowStyle, metrics: CandidateMetrics) {
        super.init(style: style, metrics: metrics)

        scrollView.translatesAutoresizingMaskIntoConstraints = false
        scrollView.drawsBackground = false
        scrollView.hasVerticalScroller = false
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

        if style == .sequoia {
            let highlight = CandidateRowHighlightView()
            rowsContainer.addSubview(highlight)
            rowHighlightView = highlight
        }

        chevronView = CandidateChevronView(style: style, metrics: metrics)
        chevronView.onClick = { [weak self] in
            guard let self, !isAnimating, displayMode == .collapsed, hasOverflow else { return }
            expand(animated: true)
        }
        rowsContainer.addSubview(chevronView)

        // The expanded window's width reserves room for the current scroller
        // style; an overlay↔legacy flip (plugging in a mouse) changes that
        // cost, so the frame is recomputed like the vertical layout's.
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
        if let scrollerStyleObserver {
            NotificationCenter.default.removeObserver(scrollerStyleObserver)
        }
    }

    private func handleScrollerStyleChange() {
        scrollView.scrollerStyle = NSScroller.preferredScrollerStyle
        guard isVisible, displayMode == .expanded, !cells.isEmpty, !isAnimating else { return }
        // The new style's scroller costs the grid a different share of the
        // screen budget, so the grid is computed again BEFORE the relayout:
        // laying the old columns out against a narrower window would push the
        // right-hand one past the frame the positioning clamps to.
        rebuildGrid()
        replace(panelSize: layoutForMode())
        ensureSelectedRowVisible()
    }

    // MARK: - Content

    private var itemHeight: CGFloat { metrics.itemHeight }
    private var rowHeight: CGFloat { itemHeight + Self.separatorHeight }
    /// Whether the list holds more than the collapsed row shows — what the
    /// chevron, the paging-edge corner and the expand paths all key on.
    private var hasOverflow: Bool { cells.count > collapsedRow.count }
    private var gridWidth: CGFloat { expandedColumnWidth * CGFloat(expandedColumnCount) }
    /// The grid's total width, for the tests that pin what a long candidate
    /// does to it — the panel itself renders the grid rather than reporting it.
    var expandedGridWidthForTesting: CGFloat { gridWidth }
    /// The chevron's reserved width — what the collapsed row packs around and
    /// lays the chevron out at.
    private var chevronWidth: CGFloat { chevronView.intrinsicContentSize.width }

    /// The grid's column width: the nine-slot row the collapsed mode packs to,
    /// widened until the longest candidate fits a full row of columns, and
    /// capped by what the screen leaves once the scroller has its share.
    ///
    /// Widening the columns rather than letting a cell span more of them is
    /// what keeps the grid a grid: every row still divides into
    /// `expandedColumnCount` columns, so the rows line up under each other.
    /// The scroller's width is reserved whether or not the grid ends up
    /// scrolling, since the row count follows from the column width and a
    /// budget that changed with it would not settle.
    private func resolvedExpandedColumnWidth() -> CGFloat {
        let baseline = HorizontalPageLayout.rowBudget(slotWidth: metrics.baseWidth)
        let budget = maximumWindowWidth - currentScrollerWidth
        let widest = measuredWidths.max() ?? 0
        let narrowest = metrics.baseWidth * CGFloat(expandedColumnCount)
        let width = min(max(baseline, widest), max(narrowest, budget))
        return width / CGFloat(expandedColumnCount)
    }

    override func updateCandidates(_ newCells: [CandidateCellContent]) -> CGSize {
        cells = Array(newCells.prefix(Self.maxDisplayCandidates))
        measuredWidths = cells.map(metrics.measureWidth)
        selectedIndex = 0
        return rebuildCollapsed()
    }

    /// Same list, new rendering (the 漢羅對調 swap). The window lays out for
    /// the new widths and comes back in the mode it was already in.
    ///
    /// The mode is remembered rather than re-derived from the selection: a
    /// grid that folded to its row and unfolded again would spend a visible
    /// frame one row tall and then play the 0.183s unfold, which reads as the
    /// window blinking rather than its text changing (USER 2026-08-23). The
    /// relayout runs through `rebuildCollapsed`, which resets the mode as a
    /// side effect of tearing the cells down, so the answer is taken before it
    /// and restored after — with `animated: false`, so the swap commits
    /// exactly one frame. A selection the re-packed row can no longer show
    /// opens the grid the same way, and the genuine `↓` / chevron expansion
    /// keeps its animation.
    override func rerenderCandidates(_ newCells: [CandidateCellContent]) {
        guard !cells.isEmpty, !newCells.isEmpty else { return }
        let kept = selectedIndex
        let wasExpanded = displayMode == .expanded
        cells = Array(newCells.prefix(Self.maxDisplayCandidates))
        measuredWidths = cells.map(metrics.measureWidth)
        let collapsedSize = rebuildCollapsed()
        selectedIndex = min(kept, cells.count - 1)

        guard wasExpanded || selectedIndex >= collapsedRow.count else {
            replace(panelSize: collapsedSize)
            updateHighlights()
            return
        }
        expand(animated: false)
    }

    override func clear() {
        stopFrameAnimation()
        transition = nil
        isAnimating = false
        cells = []
        measuredWidths = []
        selectedIndex = 0
        displayMode = .collapsed
        collapsedRow = []
        isGridBuilt = false
        removeAllItemViews()
        hide()
    }

    /// Tears everything down and lays the collapsed row out fresh — the shape
    /// every new list starts in, whatever mode the previous list ended in.
    private func rebuildCollapsed() -> CGSize {
        stopFrameAnimation()
        // Dropped with the animation: a retained transition would keep strong
        // references to the removed cells until the next mode change.
        transition = nil
        isAnimating = false
        displayMode = .collapsed
        isGridBuilt = false
        removeAllItemViews()
        rowHighlightView?.alphaValue = 0
        guard !cells.isEmpty else { return .zero }

        // The chevron only shows when the row cannot hold the whole list, so
        // the packing reserves its width only when it will really be there.
        collapsedRow = HorizontalPageLayout.pack(
            widths: measuredWidths,
            slotWidth: metrics.baseWidth,
            windowBudget: maximumWindowWidth,
            chromeWidth: chevronWidth,
        ).pages.first ?? []

        var x: CGFloat = 0
        for slot in collapsedRow {
            let item = makeItem(candidateIndex: slot.candidateIndex)
            item.frame = NSRect(x: x, y: 0, width: slot.width, height: itemHeight)
            row0ItemViews.append(item)
            x += slot.width
        }
        updateHighlights()

        chevronView.isHidden = !hasOverflow
        var windowWidth = x
        if hasOverflow {
            chevronView.setContentAlpha(1)
            chevronView.alphaValue = 1
            chevronView.frame = NSRect(x: x, y: 0, width: chevronWidth, height: itemHeight)
            windowWidth += chevronWidth
        }

        scrollView.hasVerticalScroller = false
        scrollView.contentView.scroll(to: .zero)
        scrollView.reflectScrolledClipView(scrollView.contentView)
        rowsContainer.frame.size = NSSize(width: windowWidth, height: itemHeight)
        return CGSize(width: windowWidth, height: itemHeight)
    }

    private func makeItem(candidateIndex: Int) -> CandidateItemView {
        let item = CandidateItemView(style: style, metrics: metrics)
        item.absoluteIndex = candidateIndex
        item.highlightColor = highlightColor
        item.configure(cells[candidateIndex])
        item.onClick = { [weak self, weak item] in
            guard let self, let item, !isAnimating else { return }
            select(item.absoluteIndex)
        }
        rowsContainer.addSubview(item, positioned: .above, relativeTo: rowHighlightView)
        return item
    }

    // MARK: - Selection

    /// The `⌃(slot+1)` chords address the row the selection is on — the
    /// collapsed row, or the selected grid row. One row at a time, so a chord
    /// always names exactly one candidate.
    override func candidateIndex(forSlot slot: Int) -> Int? {
        switch displayMode {
        case .collapsed:
            guard collapsedRow.indices.contains(slot) else { return nil }
            return collapsedRow[slot].candidateIndex
        case .expanded:
            guard let (rowIndex, _) = grid.position(of: selectedIndex),
                  grid.rows[rowIndex].indices.contains(slot) else { return nil }
            return grid.rows[rowIndex][slot].candidateIndex
        }
    }

    override func navigate(_ direction: CandidateNavigation) {
        guard !cells.isEmpty, !isAnimating else { return }
        switch displayMode {
        case .collapsed:
            navigateCollapsed(direction)
        case .expanded:
            navigateExpanded(direction)
        }
    }

    private func navigateCollapsed(_ direction: CandidateNavigation) {
        switch direction {
        case .right, .nextCandidate:
            let target = selectedIndex + 1
            guard target < cells.count else { return }
            // Walking off the row's end both expands AND lands on the next
            // candidate — upstream's `shouldMoveOnExpand` is always true for
            // `→` (`MacishHorizontalExpandablePanel.swift:627-692`); an expand
            // that kept the old selection would eat one keypress.
            select(target)
        case .left, .previousCandidate:
            select(max(selectedIndex - 1, 0))
        case .down, .pageDown:
            if hasOverflow {
                expand(animated: true)
            }
        case .up, .pageUp:
            break
        }
    }

    private func navigateExpanded(_ direction: CandidateNavigation) {
        switch direction {
        case .right, .nextCandidate:
            if selectedIndex + 1 < cells.count {
                select(selectedIndex + 1)
            }
        case .left:
            if selectedIndex > 0 {
                select(selectedIndex - 1)
            } else {
                collapse(animated: true)
            }
        // Unlike `←`, this one never folds the window: it is bound to a key
        // whose label says "previous candidate", and a key that collapses the
        // grid at index 0 would be doing something its label does not say.
        case .previousCandidate:
            if selectedIndex > 0 {
                select(selectedIndex - 1)
            }
        case .down:
            if let target = grid.verticalTarget(from: selectedIndex, rowStep: 1) {
                select(target)
            }
        case .up:
            if let target = grid.verticalTarget(from: selectedIndex, rowStep: -1) {
                select(target)
            } else if grid.position(of: selectedIndex)?.rowIndex == 0 {
                collapse(animated: true)
            }
        case .pageDown:
            pageViewport(forward: true)
        case .pageUp:
            pageViewport(forward: false)
        }
    }

    /// Pages the expanded viewport a whole `maxVisibleRows`, keeping the
    /// highlight at its visual row (`MacishHorizontalExpandablePanel.swift:
    /// 748-780`). Paging up from the very top folds the window back into the
    /// row it came from.
    private func pageViewport(forward: Bool) {
        guard let (highlightRow, cell) = grid.position(of: selectedIndex) else { return }
        let topRow = Int(floor((max(scrollView.contentView.bounds.origin.y, 0) + rowHeight / 2) / rowHeight))
        if !forward, topRow == 0, highlightRow == 0 {
            collapse(animated: true)
            return
        }
        let offset = max(highlightRow - topRow, 0)
        let step = forward ? Self.maxVisibleRows : -Self.maxVisibleRows
        let newHighlightRow = min(max(highlightRow + step, 0), grid.rows.count - 1)
        let target = grid.overlappingCandidate(
            inRow: newHighlightRow,
            columnStart: cell.columnStart,
            columnEnd: cell.columnStart + cell.columnSpan,
            forward: forward,
        ) ?? grid.rows[newHighlightRow].last?.candidateIndex
        guard let target else { return }
        scrollRowToTop(max(newHighlightRow - offset, 0))
        select(target)
    }

    private func select(_ index: Int) {
        guard cells.indices.contains(index) else { return }
        selectedIndex = index
        if displayMode == .collapsed, index >= collapsedRow.count {
            // A click or walk reached a candidate the row does not show.
            expand(animated: true)
            return
        }
        if displayMode == .expanded {
            ensureSelectedRowVisible()
        }
        updateHighlights()
    }

    // MARK: - Expand / collapse

    /// Throws the grid's cells away and computes it again — for a column width
    /// that changed under a grid already on screen. Row 0's views are the
    /// collapsed row's and are laid out from the grid rather than built by it,
    /// so they stay.
    private func rebuildGrid() {
        removeExpandedItemViews()
        isGridBuilt = false
        buildGridIfNeeded()
    }

    private func buildGridIfNeeded() {
        guard !isGridBuilt else { return }
        // Resolved here rather than with the collapsed row: the column width is
        // the grid's, and the collapsed row is what every list starts as.
        expandedColumnWidth = resolvedExpandedColumnWidth()
        grid = ExpandedGridLayout.compute(
            widths: measuredWidths,
            columnWidth: expandedColumnWidth,
            columnCount: expandedColumnCount,
        )
        // Rows 1+ get their own cells (that includes duplicates of collapsed
        // cells that do not fit the grid's first row: the original slides out
        // right while its duplicate slides in from the left).
        for row in grid.rows.dropFirst() {
            for cell in row {
                let item = makeItem(candidateIndex: cell.candidateIndex)
                item.isHidden = true
                expandedItemViews.append(item)
            }
        }
        isGridBuilt = true
        // The fresh cells carry no selection state of their own.
        updateHighlights()
    }

    /// Lays every cell out for the current mode and answers the content size.
    /// The expanded window scrolls past `maxVisibleRows`, with the same
    /// half-row peek the vertical layout uses.
    private func layoutForMode() -> CGSize {
        switch displayMode {
        case .collapsed:
            return rebuildCollapsedFrames()
        case .expanded:
            return layoutExpanded()
        }
    }

    private func rebuildCollapsedFrames() -> CGSize {
        var x: CGFloat = 0
        for (position, slot) in collapsedRow.enumerated() {
            let item = row0ItemViews[position]
            item.isHidden = false
            item.alphaValue = 1
            item.frame = NSRect(x: x, y: 0, width: slot.width, height: itemHeight)
            x += slot.width
        }
        for item in expandedItemViews {
            item.isHidden = true
        }
        for separator in separatorViews {
            separator.isHidden = true
        }
        chevronView.isHidden = !hasOverflow
        if hasOverflow {
            chevronView.frame = NSRect(x: x, y: 0, width: chevronWidth, height: itemHeight)
        }
        let windowWidth = x + (hasOverflow ? chevronWidth : 0)
        rowsContainer.frame.size = NSSize(width: windowWidth, height: itemHeight)
        return CGSize(width: windowWidth, height: itemHeight)
    }

    private func layoutExpanded() -> CGSize {
        let viewByIndex = Dictionary(
            uniqueKeysWithValues: expandedItemViews.map { ($0.absoluteIndex, $0) },
        )
        let gridRow0Cells = grid.rows.first ?? []
        let gridRow0Indices = Set(gridRow0Cells.map(\.candidateIndex))

        // Row 0: collapsed cells that survive take their column frames; the
        // overflow beyond the grid's first row hides (its duplicates on later
        // rows stand in for it).
        for item in row0ItemViews {
            guard gridRow0Indices.contains(item.absoluteIndex),
                  let cell = gridRow0Cells.first(where: { $0.candidateIndex == item.absoluteIndex })
            else {
                item.isHidden = true
                continue
            }
            item.isHidden = false
            item.alphaValue = 1
            item.frame = NSRect(
                x: CGFloat(cell.columnStart) * expandedColumnWidth,
                y: 0,
                width: CGFloat(cell.columnSpan) * expandedColumnWidth,
                height: itemHeight,
            )
        }
        for (rowIndex, row) in grid.rows.enumerated().dropFirst() {
            for cell in row {
                guard let item = viewByIndex[cell.candidateIndex] else { continue }
                item.isHidden = false
                item.alphaValue = 1
                item.frame = NSRect(
                    x: CGFloat(cell.columnStart) * expandedColumnWidth,
                    y: yForRow(rowIndex),
                    width: CGFloat(cell.columnSpan) * expandedColumnWidth,
                    height: itemHeight,
                )
            }
        }
        chevronView.isHidden = true
        layoutSeparators(width: gridWidth)

        let rowCount = grid.rows.count
        let contentHeight = CGFloat(rowCount) * itemHeight + CGFloat(max(rowCount - 1, 0)) * Self.separatorHeight
        let maxVisibleHeight = (CGFloat(Self.maxVisibleRows) + 0.5) * itemHeight
            + CGFloat(Self.maxVisibleRows - 1) * Self.separatorHeight
        let needsScrolling = contentHeight > maxVisibleHeight
        let windowHeight = needsScrolling ? maxVisibleHeight : contentHeight
        var windowWidth = gridWidth
        if needsScrolling {
            windowWidth += NSScroller.scrollerWidth(
                for: .regular, scrollerStyle: NSScroller.preferredScrollerStyle,
            )
        }
        rowsContainer.frame.size = NSSize(
            width: gridWidth,
            height: needsScrolling ? contentHeight + itemHeight / 2 : contentHeight,
        )
        return CGSize(width: windowWidth, height: windowHeight)
    }

    /// `width` is passed in rather than read off `rowsContainer`, whose frame
    /// is only widened to the grid AFTER layout — reading it here would size
    /// the first expand's hairlines to the collapsed row.
    private func layoutSeparators(width: CGFloat) {
        let needed = max(grid.rows.count - 1, 0)
        while separatorViews.count < needed {
            let separator = CandidateSeparatorView()
            separator.horizontalInset = style == .tahoe ? metrics.tahoeSeparatorInset : 0
            rowsContainer.addSubview(separator, positioned: .above, relativeTo: rowHighlightView)
            separatorViews.append(separator)
        }
        for (index, separator) in separatorViews.enumerated() {
            separator.isHidden = index >= needed
            guard index < needed else { continue }
            separator.frame = NSRect(
                x: 0, y: yForRow(index) + itemHeight,
                width: width, height: Self.separatorHeight,
            )
        }
    }

    private func expand(animated: Bool) {
        let collapsedFrames = row0ItemViews.map { ($0, $0.frame) }
        let chevronFrame = chevronView.frame
        let collapsedWindowWidth = frame.size.width

        buildGridIfNeeded()
        displayMode = .expanded
        // The grid renumbers row 0: its columns rarely match the collapsed
        // row's slots, so the surviving cells take their grid positions.

        let contentSize = layoutForMode()
        scrollView.contentView.scroll(to: .zero)
        scrollView.reflectScrolledClipView(scrollView.contentView)
        ensureSelectedRowVisible()
        updateHighlights()

        guard let targetFrame = anchoredFrame(for: contentSize) else {
            setContentSize(contentSize)
            finishModeChange()
            return
        }

        guard animated, isVisible else {
            rowHighlightView?.alphaValue = 1
            setFrame(targetFrame, display: true)
            finishModeChange()
            return
        }

        // --- The unfold: survivors glide to their columns, the overflow
        // slides out right, its duplicates slide in from the left, and the
        // chevron fades as it is pushed to the grid's edge. ---
        let gridRow0Indices = Set(grid.rows.first?.map(\.candidateIndex) ?? [])
        let overflow = Set(collapsedRow.map(\.candidateIndex)).subtracting(gridRow0Indices)

        var animations: [ItemAnimation] = []
        for (item, oldFrame) in collapsedFrames {
            if overflow.contains(item.absoluteIndex) {
                item.isHidden = false
                item.alphaValue = 1
                var exitFrame = oldFrame
                exitFrame.origin.x = collapsedWindowWidth
                animations.append(ItemAnimation(
                    item: item, from: oldFrame, to: exitFrame, fromAlpha: 1, toAlpha: 0,
                ))
                item.frame = oldFrame
            } else {
                let target = item.frame
                item.frame = oldFrame
                animations.append(ItemAnimation(
                    item: item, from: oldFrame, to: target, fromAlpha: 1, toAlpha: 1,
                ))
            }
        }
        for item in expandedItemViews where !item.isHidden {
            if overflow.contains(item.absoluteIndex) {
                let target = item.frame
                var enterFrame = target
                enterFrame.origin.x = -(target.origin.x + target.width)
                item.alphaValue = 0
                item.frame = enterFrame
                animations.append(ItemAnimation(
                    item: item, from: enterFrame, to: target, fromAlpha: 0, toAlpha: 1,
                ))
            }
        }

        chevronView.isHidden = false
        chevronView.frame = chevronFrame
        let chevronTarget = NSRect(
            x: max(gridWidth - chevronFrame.width, chevronFrame.origin.x),
            y: chevronFrame.origin.y,
            width: chevronFrame.width,
            height: chevronFrame.height,
        )
        transition = Transition(
            items: animations,
            chevronFrom: chevronFrame,
            chevronTo: chevronTarget,
            chevronAlphaFrom: 1,
            chevronAlphaTo: 0,
            highlightAlphaFrom: 0,
            highlightAlphaTo: 1,
            cornerFrom: frame.size.height / 2,
            cornerTo: Self.sequoiaCornerRadius,
            isExpanding: true,
        )
        rowHighlightView?.alphaValue = 0
        isAnimating = true
        animateFrame(to: targetFrame)
    }

    private func collapse(animated: Bool) {
        let expandedFrames = (row0ItemViews + expandedItemViews)
            .filter { !$0.isHidden }
            .map { ($0, $0.frame) }
        let gridRow0Indices = Set(grid.rows.first?.map(\.candidateIndex) ?? [])
        let overflow = Set(collapsedRow.map(\.candidateIndex)).subtracting(gridRow0Indices)

        displayMode = .collapsed
        if selectedIndex >= collapsedRow.count {
            selectedIndex = max(collapsedRow.count - 1, 0)
        }

        scrollView.hasVerticalScroller = false
        scrollView.contentView.scroll(to: .zero)
        scrollView.reflectScrolledClipView(scrollView.contentView)

        let contentSize = layoutForMode()
        updateHighlights()
        guard let targetFrame = anchoredFrame(for: contentSize) else {
            setContentSize(contentSize)
            finishModeChange()
            return
        }

        guard animated, isVisible else {
            rowHighlightView?.alphaValue = 0
            setFrame(targetFrame, display: true)
            finishModeChange()
            return
        }

        // --- The fold: the inverse choreography of the unfold. ---
        var animations: [ItemAnimation] = []
        for item in row0ItemViews {
            let target = item.frame
            if overflow.contains(item.absoluteIndex) {
                var enterFrame = target
                enterFrame.origin.x = contentSize.width
                item.isHidden = false
                item.alphaValue = 0
                item.frame = enterFrame
                animations.append(ItemAnimation(
                    item: item, from: enterFrame, to: target, fromAlpha: 0, toAlpha: 1,
                ))
            } else if let (viewedItem, oldFrame) = expandedFrames.first(where: { $0.0 === item }) {
                viewedItem.frame = oldFrame
                animations.append(ItemAnimation(
                    item: item, from: oldFrame, to: target, fromAlpha: 1, toAlpha: 1,
                ))
            }
        }
        for (item, oldFrame) in expandedFrames where expandedItemViews.contains(item) {
            item.isHidden = false
            item.alphaValue = 1
            item.frame = oldFrame
            if overflow.contains(item.absoluteIndex) {
                var exitFrame = oldFrame
                exitFrame.origin.x = -(oldFrame.origin.x + oldFrame.width)
                animations.append(ItemAnimation(
                    item: item, from: oldFrame, to: exitFrame, fromAlpha: 1, toAlpha: 0,
                ))
            }
        }

        let chevronWidth = chevronView.intrinsicContentSize.width
        let rowEnd = collapsedRow.reduce(CGFloat(0)) { $0 + $1.width }
        let chevronSize = NSSize(width: chevronWidth, height: itemHeight)
        let chevronFrom = NSRect(
            origin: NSPoint(x: max(rowEnd, gridWidth - chevronWidth), y: 0), size: chevronSize,
        )
        let chevronTo = NSRect(origin: NSPoint(x: rowEnd, y: 0), size: chevronSize)
        if hasOverflow {
            chevronView.isHidden = false
            chevronView.alphaValue = 1
            chevronView.setContentAlpha(0)
            chevronView.frame = chevronFrom
        }

        transition = Transition(
            items: animations,
            chevronFrom: chevronFrom,
            chevronTo: chevronTo,
            chevronAlphaFrom: 0,
            chevronAlphaTo: hasOverflow ? 1 : 0,
            highlightAlphaFrom: 1,
            highlightAlphaTo: 0,
            cornerFrom: Self.sequoiaCornerRadius,
            cornerTo: hasOverflow ? contentSize.height / 2 : Self.sequoiaCornerRadius,
            isExpanding: false,
        )
        isAnimating = true
        animateFrame(to: targetFrame)
    }

    /// What both mode changes do once their frames are final.
    private func finishModeChange() {
        updateCorners()
        if displayMode == .expanded {
            let needsScrolling = rowsContainer.frame.height > scrollView.contentView.bounds.height
            scrollView.hasVerticalScroller = needsScrolling
            if needsScrolling, NSScroller.preferredScrollerStyle != .legacy {
                scrollView.flashScrollers()
            }
        }
    }

    // MARK: - Rows, numbering, highlights

    private func yForRow(_ row: Int) -> CGFloat {
        CGFloat(row) * rowHeight
    }

    private func scrollRowToTop(_ row: Int) {
        let targetY = yForRow(max(row, 0))
        let maxScrollY = max(rowsContainer.frame.height - scrollView.contentView.bounds.height, 0)
        scrollView.contentView.scroll(to: NSPoint(x: 0, y: min(targetY, maxScrollY)))
        scrollView.reflectScrolledClipView(scrollView.contentView)
    }

    private func ensureSelectedRowVisible() {
        guard displayMode == .expanded,
              let (rowIndex, _) = grid.position(of: selectedIndex) else { return }
        let rowTop = yForRow(rowIndex)
        let rowBottom = rowTop + itemHeight
        let viewport = scrollView.contentView.bounds
        if rowTop < viewport.minY {
            scrollView.contentView.scroll(to: NSPoint(x: 0, y: rowTop))
            scrollView.reflectScrolledClipView(scrollView.contentView)
        } else if rowBottom > viewport.maxY {
            let maxScrollY = rowsContainer.frame.height - viewport.height
            let targetY = min(rowBottom + itemHeight / 2 - viewport.height, maxScrollY)
            scrollView.contentView.scroll(to: NSPoint(x: 0, y: max(targetY, 0)))
            scrollView.reflectScrolledClipView(scrollView.contentView)
        }
    }

    private func updateHighlights() {
        for item in allItemViews {
            item.isHighlighted = item.absoluteIndex == selectedIndex && !item.isHidden
        }
        // Renumbered with the highlight: expanded, the digits address the row
        // the selection is in, so they move when it changes rows.
        refreshCellDecorations()
        if displayMode == .expanded, style == .sequoia,
           let (rowIndex, _) = grid.position(of: selectedIndex)
        {
            rowHighlightView?.frame = NSRect(
                x: 0, y: yForRow(rowIndex),
                width: rowsContainer.frame.width, height: itemHeight,
            )
            if !isAnimating {
                rowHighlightView?.alphaValue = 1
            }
        }
    }

    // MARK: - Chrome

    override var wantsPillCorners: Bool { displayMode == .collapsed && hasOverflow }

    private func removeExpandedItemViews() {
        expandedItemViews.forEach { $0.removeFromSuperview() }
        expandedItemViews = []
    }

    private func removeAllItemViews() {
        row0ItemViews.forEach { $0.removeFromSuperview() }
        row0ItemViews = []
        removeExpandedItemViews()
        separatorViews.forEach { $0.removeFromSuperview() }
        separatorViews = []
    }

    // MARK: - Frame animation

    /// One cell's journey through a mode change.
    private struct ItemAnimation {
        let item: CandidateItemView
        let from: NSRect
        let to: NSRect
        let fromAlpha: CGFloat
        let toAlpha: CGFloat
    }

    private struct Transition {
        let items: [ItemAnimation]
        let chevronFrom: NSRect
        let chevronTo: NSRect
        let chevronAlphaFrom: CGFloat
        let chevronAlphaTo: CGFloat
        let highlightAlphaFrom: CGFloat
        let highlightAlphaTo: CGFloat
        let cornerFrom: CGFloat
        let cornerTo: CGFloat
        let isExpanding: Bool
    }

    private var transition: Transition?
    private var frameDisplayLink: CADisplayLink?
    private var frameAnimationStart: CFTimeInterval = 0
    private var frameAnimationFrom: NSRect = .zero
    private var frameAnimationTo: NSRect = .zero

    /// Window frames cannot ride Core Animation, so the mode change drives its
    /// own display link and interpolates everything — frame, cells, chevron,
    /// corner radius — with the same eased clock, exactly as upstream does
    /// (`MacishBasePanel.swift:311-390`).
    private func animateFrame(to targetFrame: NSRect) {
        stopFrameAnimation()
        guard targetFrame != frame else {
            frameAnimationDidFinish()
            return
        }
        frameAnimationFrom = frame
        frameAnimationTo = targetFrame
        frameAnimationStart = CACurrentMediaTime()
        let link = displayLink(target: self, selector: #selector(frameAnimationTick))
        link.add(to: .main, forMode: .common)
        frameDisplayLink = link
    }

    private func stopFrameAnimation() {
        frameDisplayLink?.invalidate()
        frameDisplayLink = nil
    }

    /// The ease-in-out cubic bezier (0.42, 0, 0.58, 1), solved by Newton
    /// iteration to match `CAMediaTimingFunction(.easeInEaseOut)`.
    private static func easeInOut(_ x: CGFloat) -> CGFloat {
        let x1: CGFloat = 0.42
        let x2: CGFloat = 0.58
        var t = x
        for _ in 0 ..< 8 {
            let mt = 1 - t
            let xError = 3 * mt * mt * t * x1 + 3 * mt * t * t * x2 + t * t * t - x
            if abs(xError) < 1e-7 { break }
            let derivative = 3 * mt * mt * x1 + 6 * mt * t * (x2 - x1) + 3 * t * t * (1 - x2)
            if abs(derivative) < 1e-7 { break }
            t -= xError / derivative
        }
        return 3 * t * t - 2 * t * t * t
    }

    private static func interpolate(_ from: NSRect, _ to: NSRect, _ t: CGFloat) -> NSRect {
        NSRect(
            x: from.origin.x + (to.origin.x - from.origin.x) * t,
            y: from.origin.y + (to.origin.y - from.origin.y) * t,
            width: from.width + (to.width - from.width) * t,
            height: from.height + (to.height - from.height) * t,
        )
    }

    @objc private func frameAnimationTick() {
        let progress = min((CACurrentMediaTime() - frameAnimationStart) / Self.animationDuration, 1)
        let t = Self.easeInOut(progress)
        setFrame(Self.interpolate(frameAnimationFrom, frameAnimationTo, t), display: true)

        if let transition {
            let size = frame.size
            if size.width > 0, size.height > 0, style == .sequoia {
                let radius = transition.cornerFrom + (transition.cornerTo - transition.cornerFrom) * t
                backdrop.applyAsymmetricCorners(
                    size: size, leftRadius: Self.sequoiaCornerRadius, rightRadius: radius,
                )
            }
            for animation in transition.items {
                animation.item.frame = Self.interpolate(animation.from, animation.to, t)
                animation.item.alphaValue = animation.fromAlpha
                    + (animation.toAlpha - animation.fromAlpha) * t
            }
            chevronView.frame = Self.interpolate(transition.chevronFrom, transition.chevronTo, t)
            chevronView.setContentAlpha(
                transition.chevronAlphaFrom
                    + (transition.chevronAlphaTo - transition.chevronAlphaFrom) * t,
            )
            rowHighlightView?.alphaValue = transition.highlightAlphaFrom
                + (transition.highlightAlphaTo - transition.highlightAlphaFrom) * t
        }

        if progress >= 1 {
            stopFrameAnimation()
            frameAnimationDidFinish()
        }
    }

    private func frameAnimationDidFinish() {
        if let transition {
            if transition.isExpanding {
                for animation in transition.items where animation.toAlpha == 0 {
                    animation.item.isHidden = true
                }
                chevronView.isHidden = true
            } else {
                for item in expandedItemViews {
                    item.isHidden = true
                }
                for separator in separatorViews {
                    separator.isHidden = true
                }
            }
        }
        transition = nil
        isAnimating = false
        finishModeChange()
        updateHighlights()
    }
}

/// The translucent bar Sequoia lays under the expanded grid's selected row,
/// from MacishType's `MacishHighlightView` (`MacishHighlightView.swift`; MIT,
/// © 2026 Luke Chang). Tahoe has no equivalent — its cells carry their own
/// highlight.
final class CandidateRowHighlightView: NSView {
    override init(frame: NSRect) {
        super.init(frame: frame)
        translatesAutoresizingMaskIntoConstraints = true
        alphaValue = 0
    }

    @available(*, unavailable)
    required init?(coder _: NSCoder) { fatalError() }

    override func draw(_: NSRect) {
        let isDark = effectiveAppearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
        NSColor.white.withAlphaComponent(isDark ? 0.1 : 0.6).setFill()
        bounds.fill()
    }
}
