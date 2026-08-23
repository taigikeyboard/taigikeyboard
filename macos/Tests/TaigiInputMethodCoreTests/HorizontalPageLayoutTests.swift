// The horizontal window's page geometry: packing, chord slots, navigation.

@testable import TaigiInputMethodCore
import XCTest

/// Pins the pure geometry the horizontal candidate window navigates by. These
/// are the rules the retired `CandidateListModel` tests pinned — clamp at both
/// ends, page turns land where the chords say — restated over measured widths,
/// which is what made them move here.
final class HorizontalPageLayoutTests: XCTestCase {
    /// A slot width of 10 keeps every trace mental-arithmetic: the row budget
    /// is `10 × max(9, 4) = 90`.
    private static let slotWidth: CGFloat = 10

    /// A cell cap wide enough that only the tests that name their own see it —
    /// the screen budget the panels pass is far above the row budget too.
    private static let roomyCellWidth: CGFloat = 1_000

    private func pack(
        _ widths: [CGFloat],
        maxCellWidth: CGFloat = HorizontalPageLayoutTests.roomyCellWidth,
    ) -> HorizontalPageLayout {
        HorizontalPageLayout.pack(
            widths: widths,
            slotWidth: Self.slotWidth,
            maxCellWidth: maxCellWidth,
        )
    }

    /// The packing a panel really asks for: a window budget, and the chrome
    /// that only shows once the list pages.
    private func packWindow(
        _ widths: [CGFloat],
        windowBudget: CGFloat,
        chromeWidth: CGFloat,
    ) -> HorizontalPageLayout {
        HorizontalPageLayout.pack(
            widths: widths,
            slotWidth: Self.slotWidth,
            windowBudget: windowBudget,
            chromeWidth: chromeWidth,
        )
    }

    // MARK: - Packing

    func testPack_narrowItemsFillNinePerPage() {
        // trace: every width clamps up to 10; 9 × 10 = 90 = the budget, and the
        // tenth item hits the pageSize cap → pages of 9, 9, 2.
        let layout = pack(Array(repeating: 5, count: 20))

        XCTAssertEqual(layout.pages.map(\.count), [9, 9, 2])
        XCTAssertEqual(layout.candidateCount, 20)
        XCTAssertEqual(
            layout.pages[1].first?.candidateIndex,
            9,
            "pages partition the list in display order — page 2 starts where page 1 ended",
        )
    }

    func testPack_wideItemBreaksThePageAndKeepsItsMeasuredWidth() {
        // trace: item0 = 10. item1 = 200 > the 90 budget; 10 + 200 > 90 with a
        // non-empty page → break, and an empty page takes it whole. item2:
        // 200 + 10 > 90 → break again.
        let layout = pack([10, 200, 10])

        XCTAssertEqual(layout.pages.map { $0.map(\.candidateIndex) }, [[0], [1], [2]])
        XCTAssertEqual(
            layout.pages[1][0].width,
            200,
            "a candidate wider than the page budget gets a page at its measured width, "
                + "so the window widens instead of truncating the text",
        )
    }

    func testPack_cellNeverExceedsTheScreenBudget() {
        // trace: the screen leaves 120; item1 wants 200 → clamped to 120, which
        // is what the window can render without running off the display.
        let layout = pack([10, 200, 10], maxCellWidth: 120)

        XCTAssertEqual(layout.pages[1][0].width, 120)
    }

    func testPack_pageBudgetNeverExceedsTheScreenBudget() {
        // trace: nine 10pt slots want a 90pt budget, but the screen leaves 45 —
        // the page breaks at 45 instead, so a full page still fits the window.
        let layout = pack(Array(repeating: 5, count: 20), maxCellWidth: 45)

        XCTAssertEqual(layout.pageBudget, 45)
        XCTAssertEqual(layout.pages.map(\.count), [4, 4, 4, 4, 4])
    }

    func testPack_singlePageSpendsTheChromeWidthOnText() {
        // trace: one 90pt candidate against a 90pt window. It fits a single
        // page, so no arrow shows and the cell keeps its measured width —
        // reserving the 20pt arrow here would truncate text for chrome that
        // is not on screen.
        let layout = packWindow([90], windowBudget: 90, chromeWidth: 20)

        XCTAssertEqual(layout.pages.count, 1)
        XCTAssertEqual(layout.pages[0][0].width, 90)
    }

    func testPack_pagedListReservesTheChromeWidth() {
        // trace: two 90pt candidates against a 90pt window page separately, so
        // the arrow shows and the second pass packs against 90 - 20 = 70.
        let layout = packWindow([90, 90], windowBudget: 90, chromeWidth: 20)

        XCTAssertEqual(layout.pages.map { $0.map(\.candidateIndex) }, [[0], [1]])
        XCTAssertEqual(layout.pages[0][0].width, 70)
        XCTAssertEqual(layout.pageBudget, 70)
    }

    // MARK: - Chord slots

    func testSlotAddressesTheVisiblePage() {
        let layout = pack(Array(repeating: 5, count: 20))

        // trace: page 2 holds absolute indices 18 and 19 — ⌃2 on it is 19, and
        // ⌃3 addresses a slot that page does not fill.
        XCTAssertEqual(layout.candidateIndex(forSlot: 1, onPage: 2), 19)
        XCTAssertNil(
            layout.candidateIndex(forSlot: 2, onPage: 2),
            "the last page's unused slots select nothing",
        )
        XCTAssertNil(layout.candidateIndex(forSlot: 0, onPage: 3), "no such page")
    }

    // MARK: - Walking

    func testArrows_walkOneCandidateAndClampAtBothEnds() {
        let layout = pack(Array(repeating: 5, count: 20))

        XCTAssertEqual(layout.target(for: .right, from: 0), 1)
        XCTAssertEqual(
            layout.target(for: .right, from: 8),
            9,
            "→ crosses a page boundary by adjacency",
        )
        XCTAssertNil(layout.target(for: .left, from: 0), "clamps — never wraps to the last")
        XCTAssertNil(layout.target(for: .right, from: 19), "clamps — never wraps to the first")
    }

    // MARK: - Paging

    func testPaging_landsOnTheCandidateUnderTheHighlight() {
        // trace: page 0 = [idx0 w10, idx1 w80] (10+80 = 90 exactly); idx2 w90
        // would overflow → page 1 = [idx2 w30, idx3 w30, idx4 w30].
        let layout = pack([10, 80, 30, 30, 30])
        XCTAssertEqual(layout.pages.map { $0.map(\.candidateIndex) }, [[0, 1], [2, 3, 4]])

        // trace: idx1 spans x 10–90; on page 1, idx2 (0–30) overlaps 10..90
        // first → the highlight drops onto what sits under its left edge.
        XCTAssertEqual(layout.target(for: .pageDown, from: 1), 2)
        // trace: idx3 spans x 30–60; back on page 0, idx0 (0–10) misses,
        // idx1 (10–90) overlaps → 1.
        XCTAssertEqual(layout.target(for: .pageUp, from: 3), 1)
    }

    func testPagingBackward_prefersTheNeighbourUnderTheHighlightsBody() {
        // trace: page 0 = [idx0 w30, idx1 w30, idx2 w30] (= 90); idx3 breaks →
        // page 1 = [idx3 w10, idx4 w80].
        let layout = pack([30, 30, 30, 10, 80])
        XCTAssertEqual(layout.pages.map { $0.map(\.candidateIndex) }, [[0, 1, 2], [3, 4]])

        // trace: idx4 spans x 10–90. On page 0, idx0 (0–30) overlaps first, but
        // it starts left of the highlight's edge and idx1 (30–60) still ends
        // inside the span → the backward tiebreak steps right to idx1, the cell
        // the highlight visually sits over.
        XCTAssertEqual(layout.target(for: .pageUp, from: 4), 1)
    }

    func testPaging_clampsAtTheOuterPages() {
        let layout = pack(Array(repeating: 5, count: 20))

        XCTAssertNil(layout.target(for: .pageUp, from: 0), "no page before the first")
        XCTAssertNil(layout.target(for: .pageDown, from: 19), "no page after the last")
    }

    func testVerticalArrows_pageInTheHorizontalLayout() {
        // A single row has no line above or below — ↑/↓ turn pages, exactly as
        // the page keys do.
        let layout = pack(Array(repeating: 5, count: 20))

        XCTAssertEqual(layout.target(for: .down, from: 0), layout.target(for: .pageDown, from: 0))
        XCTAssertEqual(layout.target(for: .up, from: 9), layout.target(for: .pageUp, from: 9))
    }

    /// A bound key whose label says "next candidate" must move exactly one
    /// candidate here too — in a row that is what `→` already does, and the
    /// two staying identical is what makes the semantic direction safe to
    /// route through the same layout.
    func testSemanticSteps_walkOneCandidateLikeTheArrowsDo() {
        let layout = pack(Array(repeating: 5, count: 20))

        XCTAssertEqual(layout.target(for: .nextCandidate, from: 0), layout.target(for: .right, from: 0))
        XCTAssertEqual(layout.target(for: .previousCandidate, from: 5), layout.target(for: .left, from: 5))
        XCTAssertEqual(
            layout.target(for: .nextCandidate, from: 8),
            9,
            "one step crosses a page boundary rather than stopping at the page's end",
        )
        XCTAssertNil(layout.target(for: .previousCandidate, from: 0), "clamps — never wraps")
        XCTAssertNil(layout.target(for: .nextCandidate, from: 19), "clamps — never wraps")
    }
}
