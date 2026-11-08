// The vertical expandable layout: a column that unfolds into paged columns.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// Pins the column axis against `taigi-desktop-core` `ExpandableListModel`
/// (`ExpandableAxis::Column`): the collapsed list is the first page stacked,
/// the arrows along it walk, `→` unfolds into nine-row columns, up to three a
/// page — Microsoft Bopomofo's shape (USER 2026-11-08).
@MainActor
final class VerticalExpandableCandidatePanelTests: XCTestCase {
    private static let metrics = TestFixtures.defaultCandidateMetrics.arranged(.inline)

    private static func cells(_ count: Int) -> [CandidateCellContent] {
        (0 ..< count).map { CandidateCellContent(text: "候\($0)", annotation: "hau\($0)") }
    }

    private func makeEmptyPanel() -> ExpandableCandidatePanel {
        ExpandableCandidatePanel(style: .sequoia, metrics: Self.metrics, axis: .column)
    }

    private func makePanel(_ count: Int = 20) -> ExpandableCandidatePanel {
        let panel = makeEmptyPanel()
        _ = panel.updateCandidates(Self.cells(count))
        return panel
    }

    private func navigate(_ panel: ExpandableCandidatePanel, _ direction: CandidateNavigation, times: Int) {
        for _ in 0 ..< times {
            panel.navigate(direction)
        }
    }

    private func slots(_ panel: ExpandableCandidatePanel) -> [Int?] {
        (0 ..< 10).map { panel.candidateIndex(forSlot: $0) }
    }

    private func shownIndices(_ panel: ExpandableCandidatePanel) -> [Int] {
        TestFixtures.candidateCells(in: panel).map(\.absoluteIndex)
    }

    /// The one column width every cell takes, collapsed or unfolded.
    private func columnWidth(_ panel: ExpandableCandidatePanel) -> CGFloat {
        TestFixtures.candidateCells(in: panel).first?.frame.width ?? 0
    }

    private func chevron(_ panel: ExpandableCandidatePanel) -> CandidateChevronView? {
        TestFixtures.descendants(of: panel, as: CandidateChevronView.self).first
    }

    func testCollapsed_isTheFirstPageStackedAtTheGridPitch() {
        let metrics = Self.metrics
        let panel = makeEmptyPanel()

        let size = panel.updateCandidates(Self.cells(40))

        // trace: pageSize 9 → 9 rows of itemHeight + 8 one-point hairlines.
        XCTAssertEqual(size.height, 9 * metrics.itemHeight + 8)
        XCTAssertEqual(slots(panel), Array(0 ..< 9) + [nil])
        let frames = TestFixtures.candidateCells(in: panel).map(\.frame)
        XCTAssertEqual(frames.count, 9)
        XCTAssertEqual(Set(frames.map(\.origin.x)), [0], "one column")
        XCTAssertEqual(Set(frames.map(\.width)).count, 1, "every row the column's one width")
    }

    func testShortList_hasNoChevronColumn() {
        let metrics = Self.metrics
        let panel = makeEmptyPanel()

        let size = panel.updateCandidates(Self.cells(3))

        XCTAssertEqual(size.height, 3 * metrics.itemHeight + 2)
        XCTAssertEqual(size.width, columnWidth(panel), "the window is the column alone")
        panel.navigate(.right)
        XCTAssertEqual(panel.displayMode, .collapsed, "nothing to unfold")
        XCTAssertEqual(panel.selectedIndex, 0)
    }

    func testDownWalksTheColumnAndExpandsOffItsEnd() {
        let panel = makePanel()

        navigate(panel, .down, times: 8)
        XCTAssertEqual(panel.selectedIndex, 8)
        XCTAssertEqual(panel.displayMode, .collapsed)

        panel.navigate(.down)

        XCTAssertEqual(panel.selectedIndex, 9, "the step that unfolds also lands on the next candidate")
        XCTAssertEqual(panel.displayMode, .expanded)
    }

    func testLeftAndUpDoNotUnfoldTheColumn() {
        let panel = makePanel()
        panel.navigate(.down)

        panel.navigate(.left)
        XCTAssertEqual(panel.displayMode, .collapsed)
        XCTAssertEqual(panel.selectedIndex, 1, "`←` runs across the column, which only `→` unfolds")

        panel.navigate(.up)
        XCTAssertEqual(panel.selectedIndex, 0)
        XCTAssertEqual(panel.displayMode, .collapsed)
    }

    /// `→` unfolds AND steps into the next column on the same row; `←` walks
    /// back a column, and only the first column folds — onto the row it is on.
    func testRightUnfoldsIntoTheNextColumnAndLeftFoldsFromTheFirst() {
        let panel = makePanel()
        navigate(panel, .down, times: 4)

        // trace: 4 → 13 (column 1, row 4).
        panel.navigate(.right)
        XCTAssertEqual(panel.displayMode, .expanded)
        XCTAssertEqual(panel.selectedIndex, 13)

        // trace: 13 → min(22, 19) = 19 on the short last column 18-19.
        panel.navigate(.right)
        XCTAssertEqual(panel.selectedIndex, 19, "clamped to the short column")
        panel.navigate(.right)
        XCTAssertEqual(panel.selectedIndex, 19, "no column after 18-19")

        panel.navigate(.left)
        XCTAssertEqual(panel.selectedIndex, 10)
        panel.navigate(.left)
        XCTAssertEqual(panel.selectedIndex, 1)
        XCTAssertEqual(panel.displayMode, .expanded, "`←` walks every column but the first")

        panel.navigate(.left)
        XCTAssertEqual(panel.displayMode, .collapsed)
        XCTAssertEqual(panel.selectedIndex, 1)
    }

    /// Folding from a later column keeps the row: 9 % 9 = 0.
    func testFold_keepsTheRow() {
        let panel = makePanel()
        navigate(panel, .down, times: 9)
        XCTAssertEqual(panel.selectedIndex, 9)

        panel.navigate(.pageUp)

        XCTAssertEqual(panel.displayMode, .collapsed, "PageUp on the first page folds")
        XCTAssertEqual(panel.selectedIndex, 0)
    }

    func testExpanded_upAndPageKeys() {
        let panel = makePanel()

        panel.navigate(.pageDown)
        XCTAssertEqual(panel.displayMode, .expanded, "PageDown unfolds")
        XCTAssertEqual(panel.selectedIndex, 0, "in place")

        // `↑` walks up and on into the previous column.
        navigate(panel, .down, times: 9)
        panel.navigate(.up)
        XCTAssertEqual(panel.selectedIndex, 8)

        navigate(panel, .up, times: 8)
        panel.navigate(.previousCandidate)
        XCTAssertEqual(panel.displayMode, .expanded, "\"previous candidate\" never folds")
        panel.navigate(.up)
        XCTAssertEqual(panel.displayMode, .collapsed, "`↑` on the first candidate folds")

        panel.navigate(.pageDown)
        navigate(panel, .down, times: 19)
        XCTAssertEqual(panel.selectedIndex, 19)
        panel.navigate(.down)
        panel.navigate(.nextCandidate)
        XCTAssertEqual(panel.selectedIndex, 19, "the list's last candidate")
    }

    /// The column unfolds to the right at its own height (USER 2026-10-31),
    /// paging rather than scrolling.
    func testExpanded_keepsTheCollapsedHeight() {
        for count in [20, 200] {
            let panel = makeEmptyPanel()
            let collapsed = panel.updateCandidates(Self.cells(count))

            panel.navigate(.right)

            XCTAssertEqual(panel.displayMode, .expanded)
            XCTAssertEqual(panel.frame.height, collapsed.height, "\(count) candidates")
            XCTAssertGreaterThan(panel.frame.width, collapsed.width, "it grows to the right")
        }
    }

    /// A list that fits one page shows it whole, column-major, with no
    /// chevron column left over.
    func testOnePage_columnMajorWithoutAChevronColumn() {
        let panel = makePanel()
        let collapsedWidth = columnWidth(panel)

        panel.navigate(.pageDown)

        let w = columnWidth(panel)
        // Measured over the whole list — "候19 hau19" outgrows the first
        // page's "候8 hau8" — so it holds still while paging.
        XCTAssertGreaterThanOrEqual(w, collapsedWidth, "column 0 never shrinks as it unfolds")
        let cells = TestFixtures.candidateCells(in: panel)
        // trace: columns 0-8 / 9-17 / 18-19 → visual row 0 = [0, 9, 18].
        XCTAssertEqual(cells.prefix(3).map(\.absoluteIndex), [0, 9, 18])
        XCTAssertEqual(Set(cells.map(\.absoluteIndex)), Set(0 ..< 20), "each candidate once")
        XCTAssertEqual(Set(cells.map(\.frame.origin.x)), [0, w, 2 * w])
        XCTAssertEqual(panel.frame.width, 3 * w, "one page: no chevron column")
        XCTAssertEqual(chevron(panel)?.isHidden, true)

        // trace: 12 → 2 columns; the page is no wider than the list.
        let two = makePanel(12)
        two.navigate(.pageDown)
        XCTAssertEqual(two.frame.width, 2 * columnWidth(two))
    }

    func testPaged_pagesThreeColumnsAtAFixedWidth() throws {
        // trace: 60 → 7 columns, 3 a page → pages of 27 at 0 / 27 / 54.
        let panel = makePanel(60)
        panel.navigate(.pageDown)
        let w = columnWidth(panel)
        let chevron = try XCTUnwrap(chevron(panel))
        let pagedWidth = 3 * w + chevron.frame.width
        XCTAssertEqual(panel.frame.width, pagedWidth)
        XCTAssertFalse(chevron.isHidden)
        XCTAssertEqual(chevron.frame.origin.x, 3 * w, "right of the grid")
        XCTAssertEqual(chevron.frame.height, panel.frame.height, "the window's full height")
        XCTAssertEqual(Set(shownIndices(panel)), Set(0 ..< 27))

        panel.navigate(.pageDown)
        XCTAssertEqual(panel.selectedIndex, 27)
        XCTAssertEqual(Set(shownIndices(panel)), Set(27 ..< 54))

        panel.navigate(.pageDown)
        XCTAssertEqual(panel.selectedIndex, 54)
        XCTAssertEqual(Set(shownIndices(panel)), Set(54 ..< 60), "one short column")
        XCTAssertFalse(chevron.isHidden, "its column kept on the last page")
        XCTAssertEqual(panel.frame.width, pagedWidth, "paging never resizes the window")
        panel.navigate(.pageDown)
        XCTAssertEqual(panel.selectedIndex, 54)

        // trace: 26 = page 0 column 2 → `→` 35 opens page 1.
        navigate(panel, .pageUp, times: 2)
        XCTAssertEqual(panel.selectedIndex, 0)
        navigate(panel, .down, times: 26)
        XCTAssertEqual(Set(shownIndices(panel)), Set(0 ..< 27))
        panel.navigate(.right)
        XCTAssertEqual(panel.selectedIndex, 35)
        XCTAssertEqual(Set(shownIndices(panel)), Set(27 ..< 54))

        panel.navigate(.pageUp)
        XCTAssertEqual(panel.selectedIndex, 8)
        panel.navigate(.pageUp)
        XCTAssertEqual(panel.displayMode, .collapsed)
        XCTAssertEqual(panel.selectedIndex, 8)
    }

    func testPaged_chevronClickTurnsThePage() throws {
        let panel = makePanel(60)
        panel.navigate(.pageDown)

        try XCTUnwrap(chevron(panel)?.onClick)()

        XCTAssertEqual(panel.selectedIndex, 27)
        XCTAssertEqual(panel.displayMode, .expanded)
    }

    func testPaged_pageDownClampsToTheLastCandidate() {
        let panel = makePanel(200)
        panel.navigate(.pageDown)
        navigate(panel, .down, times: 188)

        // trace: 188 + 27 = 215 clamps to 199.
        panel.navigate(.pageDown)

        XCTAssertEqual(panel.selectedIndex, 199)
    }

    /// Keys 1–9 address the selected column, never past its ninth cell into
    /// the next one.
    func testExpanded_slotsAddressTheKeyedColumn() {
        let panel = makePanel()
        navigate(panel, .down, times: 4)
        panel.navigate(.right)
        XCTAssertEqual(panel.selectedIndex, 13)

        XCTAssertEqual(slots(panel), (9 ..< 18).map(Optional.some) + [nil])

        // trace: 19 sits in the short column 18-19.
        panel.navigate(.right)
        XCTAssertEqual(slots(panel), [18, 19] + Array(repeating: nil, count: 8))

        // A leading literal shifts the keys by one: key 9 asks slot 9, which
        // must not reach candidate 9 in the next column.
        navigate(panel, .left, times: 2)
        XCTAssertEqual(panel.selectedIndex, 1)
        panel.leadCellIsUnkeyed = true
        XCTAssertEqual(panel.candidateIndex(forKeySlot: 0), 1)
        XCTAssertEqual(panel.candidateIndex(forKeySlot: 7), 8)
        XCTAssertNil(panel.candidateIndex(forKeySlot: 8))
    }
}
