// How a horizontal candidate window pages: pure width arithmetic, no AppKit.

import CoreGraphics

/// The page structure of a horizontal candidate window, computed from measured
/// item widths.
///
/// Ported from MacishType's packing and page-preserving navigation
/// (`references/MacishType/macos/MacishType/MacishCandidateWindow/
/// MacishHorizontalBasePanel.swift:12-34`, `MacishHorizontalSimplePanel.swift:
/// 178-227`; MIT, © 2026 Luke Chang), extracted into a value type so the
/// geometry that decides which candidate a key lands on stays testable without
/// a window. The panel measures and renders; this type decides.
struct HorizontalPageLayout: Equatable {
    /// One candidate's place on a page: its absolute index and the width its
    /// cell renders at. Indices run in display order across pages, so the page
    /// structure is a partition of `0 ..< candidateCount`.
    struct Slot: Equatable {
        let candidateIndex: Int
        let width: CGFloat
    }

    let pages: [[Slot]]

    /// How many candidates one page of the window can hold, which is also how
    /// many the `⌃1`…`⌃9` chords can address.
    static let pageSize = 9

    /// A page never packs fewer potential columns than this: a run of very wide
    /// candidates still gets a window wide enough to read them in
    /// (`MacishHorizontalBasePanel.swift:6`).
    static let minimumPageColumns = 4

    /// Packs `widths` into pages. Each cell is clamped to
    /// `slotWidth ... rowWidthLimit`, a page breaks before its total width
    /// passes the limit — unless the page is still empty, so a single oversized
    /// candidate always gets a page — and no page holds more than `pageSize`
    /// candidates, because the chords cannot address a tenth.
    static func pack(widths: [CGFloat], slotWidth: CGFloat) -> HorizontalPageLayout {
        let rowWidthLimit = slotWidth * CGFloat(max(pageSize, minimumPageColumns))
        var pages: [[Slot]] = []
        var page: [Slot] = []
        var usedWidth: CGFloat = 0

        for (index, rawWidth) in widths.enumerated() {
            let width = max(slotWidth, min(rawWidth, rowWidthLimit))
            if page.count == pageSize || (usedWidth + width > rowWidthLimit && !page.isEmpty) {
                pages.append(page)
                page = []
                usedWidth = 0
            }
            page.append(Slot(candidateIndex: index, width: width))
            usedWidth += width
        }
        if !page.isEmpty {
            pages.append(page)
        }
        return HorizontalPageLayout(pages: pages)
    }

    var candidateCount: Int {
        pages.reduce(0) { $0 + $1.count }
    }

    /// The page holding `candidateIndex`, found by arithmetic over the
    /// partition rather than a search: pages hold consecutive indices.
    func pageIndex(containing candidateIndex: Int) -> Int? {
        var start = 0
        for (pageIndex, page) in pages.enumerated() {
            if candidateIndex < start + page.count {
                return candidateIndex >= start ? pageIndex : nil
            }
            start += page.count
        }
        return nil
    }

    /// The absolute index the `⌃(slot+1)` chord picks on `pageIndex`, or nil
    /// for a slot the (usually last) page does not fill.
    func candidateIndex(forSlot slot: Int, onPage pageIndex: Int) -> Int? {
        guard pages.indices.contains(pageIndex) else { return nil }
        let page = pages[pageIndex]
        guard page.indices.contains(slot) else { return nil }
        return page[slot].candidateIndex
    }

    /// Where `direction` moves the selection, or nil at an end — the caller
    /// leaves the selection where it is, which is the no-wrap clamp rule.
    ///
    /// `←`/`→` walk one candidate and cross page boundaries by simple
    /// adjacency. The paging directions land on the candidate of the target
    /// page whose horizontal extent overlaps the current selection's — the
    /// highlight moves straight down or up visually, not to a fixed slot
    /// (`MacishHorizontalSimplePanel.swift:178-227`).
    func target(for direction: CandidateNavigation, from selection: Int) -> Int? {
        switch direction {
        case .right, .nextCandidate:
            return selection + 1 < candidateCount ? selection + 1 : nil
        case .left, .previousCandidate:
            return selection > 0 ? selection - 1 : nil
        case .down, .pageDown:
            return pagedTarget(from: selection, step: 1)
        case .up, .pageUp:
            return pagedTarget(from: selection, step: -1)
        }
    }

    private func pagedTarget(from selection: Int, step: Int) -> Int? {
        guard let currentPage = pageIndex(containing: selection) else { return nil }
        let targetPage = currentPage + step
        guard pages.indices.contains(targetPage) else { return nil }

        // The selection's horizontal extent on its own page.
        var xStart: CGFloat = 0
        var xEnd: CGFloat = 0
        for slot in pages[currentPage] {
            xEnd += slot.width
            if slot.candidateIndex == selection {
                break
            }
            xStart = xEnd
        }

        return overlappingCandidate(
            onPage: targetPage,
            xStart: xStart,
            xEnd: xEnd,
            forward: step > 0,
        ) ?? pages[targetPage].last?.candidateIndex
    }

    /// The first candidate on `pageIndex` whose extent overlaps
    /// `xStart ..< xEnd`. Paging backward prefers the neighbour on the right
    /// when the overlap starts left of the origin extent, matching upstream's
    /// direction-aware tiebreak (`MacishHorizontalSimplePanel.swift:207-227`):
    /// coming back up should land on the cell the highlight visually sits
    /// over, not the one that merely begins under its left edge.
    private func overlappingCandidate(
        onPage pageIndex: Int,
        xStart: CGFloat,
        xEnd: CGFloat,
        forward: Bool,
    ) -> Int? {
        var x: CGFloat = 0
        let page = pages[pageIndex]
        for (position, slot) in page.enumerated() {
            let slotEnd = x + slot.width
            if x < xEnd, slotEnd > xStart {
                if !forward, x < xStart, position + 1 < page.count, slotEnd < xEnd {
                    return page[position + 1].candidateIndex
                }
                return slot.candidateIndex
            }
            x = slotEnd
        }
        return nil
    }
}
