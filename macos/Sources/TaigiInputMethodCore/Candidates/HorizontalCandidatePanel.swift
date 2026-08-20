// The horizontal layout: one row of candidates, paged by measured width.

import AppKit

/// The width-packed, paged horizontal candidate window — MacishType's
/// `MacishHorizontalSimplePanel` (`references/MacishType/macos/MacishType/
/// MacishCandidateWindow/MacishHorizontalSimplePanel.swift`; MIT, © 2026 Luke
/// Chang) with its page geometry and navigation extracted into
/// `HorizontalPageLayout`, and its per-page view caching dropped: a page holds
/// at most nine cells, so rebuilding the visible page on every change is
/// cheaper to reason about than keeping every page's views alive and hidden.
final class HorizontalCandidatePanel: CandidateBasePanel {
    private var cells: [CandidateCellContent] = []
    private var pageLayout = HorizontalPageLayout(pages: [])
    // `selectedIndex` (base): always a valid index into `cells` while the
    // list is non-empty — this input method selects the first candidate of
    // every fresh list, so the upstream `-1` suspended state is unreachable.
    private var currentPage = 0

    private var itemViews: [CandidateItemView] = []
    private lazy var pageArrowView: CandidatePageArrowView = {
        let view = CandidatePageArrowView(style: style)
        view.onPageUp = { [weak self] in self?.navigate(.pageUp) }
        view.onPageDown = { [weak self] in self?.navigate(.pageDown) }
        contentContainer.addSubview(view)
        return view
    }()

    override var isEmpty: Bool { cells.isEmpty }

    // MARK: - Content

    /// Replaces the list, selecting its first candidate, and answers with the
    /// size the window wants. The caller places and shows it.
    ///
    /// The selection resets rather than being preserved by index: a fresh
    /// keystroke re-ranks the whole list, so holding position would leave the
    /// highlight on an unrelated word that happens to have landed there.
    override func updateCandidates(_ newCells: [CandidateCellContent]) -> CGSize {
        cells = Array(newCells.prefix(Self.maxDisplayCandidates))
        pageLayout = HorizontalPageLayout.pack(
            widths: cells.map(CandidateItemView.measureWidth),
            slotWidth: CandidateItemView.baseWidth,
        )
        selectedIndex = 0
        currentPage = 0
        return rebuildVisiblePage()
    }

    /// Empties the window so nothing can be selected or committed from it —
    /// hiding must drop the state, not just the pixels.
    override func clear() {
        cells = []
        pageLayout = HorizontalPageLayout(pages: [])
        selectedIndex = 0
        currentPage = 0
        itemViews.forEach { $0.removeFromSuperview() }
        itemViews = []
        hide()
    }

    // MARK: - Selection

    /// The absolute index the `⌃(slot+1)` chord addresses on the visible page.
    override func candidateIndex(forSlot slot: Int) -> Int? {
        pageLayout.candidateIndex(forSlot: slot, onPage: currentPage)
    }

    override func navigate(_ direction: CandidateNavigation) {
        guard !cells.isEmpty else { return }
        guard let target = pageLayout.target(for: direction, from: selectedIndex) else { return }
        select(target)
    }

    /// Selects `index`, turning to its page when it lives on another one — a
    /// page turn changes the window's width, so the frame is re-anchored.
    func select(_ index: Int) {
        guard cells.indices.contains(index),
              let targetPage = pageLayout.pageIndex(containing: index) else { return }
        selectedIndex = index
        if targetPage != currentPage {
            currentPage = targetPage
            replace(panelSize: rebuildVisiblePage())
        } else {
            updateHighlights()
        }
    }

    // MARK: - Layout

    /// Rebuilds the visible page's cells and answers with the window size that
    /// fits them.
    private func rebuildVisiblePage() -> CGSize {
        itemViews.forEach { $0.removeFromSuperview() }
        itemViews = []

        guard pageLayout.pages.indices.contains(currentPage) else { return .zero }
        let page = pageLayout.pages[currentPage]
        let itemHeight = CandidateItemView.Metrics.itemHeight

        var x: CGFloat = 0
        for slot in page {
            let item = CandidateItemView(style: style)
            item.absoluteIndex = slot.candidateIndex
            item.highlightColor = highlightColor
            item.configure(cells[slot.candidateIndex])
            item.frame = NSRect(x: x, y: 0, width: slot.width, height: itemHeight)
            item.onClick = { [weak self, weak item] in
                guard let self, let item else { return }
                select(item.absoluteIndex)
            }
            contentContainer.addSubview(item)
            itemViews.append(item)
            x += slot.width
        }
        updateHighlights()

        let hasMultiplePages = pageLayout.pages.count > 1
        pageArrowView.isHidden = !hasMultiplePages
        guard hasMultiplePages else {
            return CGSize(width: x, height: itemHeight)
        }

        pageArrowView.canPageUp = currentPage > 0
        pageArrowView.canPageDown = currentPage < pageLayout.pages.count - 1
        // Short pages still get the full packing budget's width, so the arrow
        // edge does not wander left and right as the pages turn.
        let contentWidth = max(
            x,
            CandidateItemView.baseWidth
                * CGFloat(max(HorizontalPageLayout.pageSize, HorizontalPageLayout.minimumPageColumns)),
        )
        let arrowWidth = pageArrowView.intrinsicContentSize.width
        pageArrowView.frame = NSRect(x: contentWidth, y: 0, width: arrowWidth, height: itemHeight)
        return CGSize(width: contentWidth + arrowWidth, height: itemHeight)
    }

    private func updateHighlights() {
        for item in itemViews {
            item.isHighlighted = item.absoluteIndex == selectedIndex
        }
    }

    // MARK: - Chrome

    override func applyHighlightColor(_ color: NSColor) {
        for item in itemViews {
            item.highlightColor = color
        }
    }

    override func updateCorners() {
        let size = frame.size
        guard size.width > 0, size.height > 0 else { return }
        if pageLayout.pages.count > 1 {
            applyPillCorners(size: size)
        } else {
            super.updateCorners()
        }
    }
}
