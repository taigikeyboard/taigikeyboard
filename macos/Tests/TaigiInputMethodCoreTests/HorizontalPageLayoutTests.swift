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

    private func pack(_ widths: [CGFloat]) -> HorizontalPageLayout {
        HorizontalPageLayout.pack(widths: widths, slotWidth: Self.slotWidth)
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

    func testPack_wideItemBreaksThePageButAlwaysGetsOne() {
        // trace: item0 = 10. item1 = 200 clamps to the 90 budget; 10 + 90 > 90
        // with a non-empty page → break. item2: 90 + 10 > 90 → break again.
        let layout = pack([10, 200, 10])

        XCTAssertEqual(layout.pages.map { $0.map(\.candidateIndex) }, [[0], [1], [2]])
        XCTAssertEqual(
            layout.pages[1][0].width,
            90,
            "an oversized candidate is clamped to the page budget, not dropped",
        )
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
}
