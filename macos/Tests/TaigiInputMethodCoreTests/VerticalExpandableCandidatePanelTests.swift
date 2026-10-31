// The vertical expandable layout: a column that unfolds into the expandable grid.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// Pins the column axis against `taigi-desktop-core` `ExpandableListModel`
/// (`ExpandableAxis::Column`): the collapsed list is the first page stacked,
/// the arrows along it walk, `→` unfolds, and the grid folds back exactly as
/// the row's does.
@MainActor
final class VerticalExpandableCandidatePanelTests: XCTestCase {
    private static let cells: [CandidateCellContent] = (0 ..< 40).map {
        CandidateCellContent(text: "候\($0)", annotation: "hau\($0)")
    }

    private static let metrics = TestFixtures.defaultCandidateMetrics.arranged(.inline)

    private func makeEmptyPanel() -> ExpandableCandidatePanel {
        ExpandableCandidatePanel(style: .sequoia, metrics: Self.metrics, axis: .column)
    }

    private func makePanel() -> ExpandableCandidatePanel {
        let panel = makeEmptyPanel()
        _ = panel.updateCandidates(Self.cells)
        return panel
    }

    func testCollapsed_isTheFirstPageStackedAtTheGridPitch() {
        let metrics = Self.metrics
        let panel = makeEmptyPanel()

        let size = panel.updateCandidates(Self.cells)

        // trace: pageSize 9 → 9 rows of itemHeight + 8 one-point hairlines.
        XCTAssertEqual(size.height, 9 * metrics.itemHeight + 8)
        XCTAssertEqual((0 ..< 9).map { panel.candidateIndex(forSlot: $0) }, Array(0 ..< 9))
        XCTAssertNil(panel.candidateIndex(forSlot: 9))
        let frames = TestFixtures.candidateCells(in: panel).map(\.frame)
        XCTAssertEqual(frames.count, 9)
        XCTAssertEqual(Set(frames.map(\.origin.x)), [0], "one column")
        XCTAssertEqual(Set(frames.map(\.width)).count, 1, "every row the column's one width")
    }

    func testShortList_hasNoChevronColumn() {
        let short = Array(Self.cells.prefix(3))
        let metrics = Self.metrics
        let panel = makeEmptyPanel()

        let size = panel.updateCandidates(short)

        XCTAssertEqual(size.height, 3 * metrics.itemHeight + 2)
        XCTAssertEqual(
            size.width, TestFixtures.candidateCells(in: panel).first?.frame.width,
            "the window is the column alone",
        )
    }

    func testDownWalksTheColumnAndExpandsOffItsEnd() {
        let panel = makePanel()

        for _ in 0 ..< 8 {
            panel.navigate(.down)
        }
        XCTAssertEqual(panel.selectedIndex, 8)
        XCTAssertEqual(panel.displayMode, .collapsed)

        panel.navigate(.down)

        XCTAssertEqual(panel.selectedIndex, 9, "the step that unfolds also lands on the next candidate")
        XCTAssertEqual(panel.displayMode, .expanded)
    }

    func testRightUnfoldsKeepingTheSelection() {
        let panel = makePanel()
        panel.navigate(.down)

        panel.navigate(.right)

        XCTAssertEqual(panel.displayMode, .expanded)
        XCTAssertEqual(panel.selectedIndex, 1)
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

    func testExpanded_foldKeysMatchTheRow() {
        let panel = makePanel()
        panel.navigate(.right)
        panel.navigate(.left)
        XCTAssertEqual(panel.displayMode, .collapsed, "`←` at index 0 folds")

        panel.navigate(.right)
        panel.navigate(.up)
        XCTAssertEqual(panel.displayMode, .collapsed, "`↑` on grid row 0 folds")

        panel.navigate(.pageDown)
        XCTAssertEqual(panel.displayMode, .expanded, "PageDown unfolds")
        panel.navigate(.pageUp)
        XCTAssertEqual(panel.displayMode, .collapsed, "PageUp at the top folds")
    }

    /// `←` folds the column from anywhere in the grid — the arrow opposite
    /// the `→` that unfolded it. The row's `←` still walks (negative control).
    func testExpanded_leftFoldsFromAnywhere() {
        let panel = makePanel()
        panel.navigate(.right)
        for _ in 0 ..< 12 {
            panel.navigate(.right)
        }
        // trace: `→` unfolds keeping index 0, then 12 steps → 12.
        XCTAssertEqual(panel.selectedIndex, 12)

        panel.navigate(.left)

        XCTAssertEqual(panel.displayMode, .collapsed)
        XCTAssertEqual(panel.selectedIndex, 8, "clamped to the column's foot")

        let row = ExpandableCandidatePanel(style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics)
        _ = row.updateCandidates(Self.cells)
        row.navigate(.down)
        row.navigate(.right)
        row.navigate(.right)
        row.navigate(.left)
        XCTAssertEqual(row.displayMode, .expanded, "a row's `←` walks")
        XCTAssertEqual(row.selectedIndex, 1)
    }

    /// The column unfolds to the right at its own height: a short grid pads
    /// down to it, a long one scrolls inside it (USER 2026-10-31).
    func testExpanded_keepsTheCollapsedHeight() {
        for count in [20, 200] {
            let panel = makeEmptyPanel()
            let cells = (0 ..< count).map {
                CandidateCellContent(text: "候\($0)", annotation: "hau\($0)")
            }
            let collapsed = panel.updateCandidates(cells)

            panel.navigate(.right)

            XCTAssertEqual(panel.displayMode, .expanded)
            XCTAssertEqual(panel.frame.height, collapsed.height, "\(count) candidates")
            XCTAssertGreaterThan(panel.frame.width, collapsed.width, "it grows to the right")
        }
    }

    func testExpanded_pageDownStepsAWholeColumnHeight() {
        let panel = makeEmptyPanel()
        let cells = (0 ..< 200).map { CandidateCellContent(text: "候\($0)", annotation: "hau\($0)") }
        _ = panel.updateCandidates(cells)
        panel.navigate(.right)

        panel.navigate(.pageDown)

        // trace: an inline "候N hauN" cell spans two of the six grid columns →
        // three per row; index 0 on row 0 pages 9 rows to row 9 → 27.
        XCTAssertEqual(panel.selectedIndex, 27)
    }

    func testExpanded_showsTheGridNotTheColumn() {
        let panel = makePanel()

        panel.navigate(.right)

        let cells = TestFixtures.candidateCells(in: panel)
        XCTAssertEqual(cells.map(\.absoluteIndex), Array(0 ..< Self.cells.count), "each candidate once, in order")
        XCTAssertGreaterThan(Set(cells.map(\.frame.origin.x)).count, 1, "the grid has columns")
    }
}
