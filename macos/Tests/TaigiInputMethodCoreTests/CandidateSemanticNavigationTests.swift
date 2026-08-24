// What "next candidate" means in each of the three layouts.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The semantic directions exist because the physical ones do not mean the same
/// thing everywhere: `→` walks a row, pages a column, and can expand a grid. A
/// key labelled "next candidate" has to move exactly one candidate in all three,
/// so all three are asserted here against the real panels rather than a double.
@MainActor
final class CandidateSemanticNavigationTests: XCTestCase {
    private static let candidateCount = 40

    private static let cells: [CandidateCellContent] = (0 ..< candidateCount).map {
        CandidateCellContent(text: "候\($0)", annotation: "hau\($0)")
    }

    /// The 漢羅 swap path: same list, new rendering. Every layout keeps the
    /// selection on its absolute index and swaps every cell's scripts —
    /// `rerenderCandidates` is a distinct contract from the fresh-list
    /// `updateCandidates`, which resets the selection to the first candidate.
    func testEveryLayout_rerenderKeepsTheSelection() {
        let swapped = Self.cells.map {
            CandidateCellContent(text: $0.annotation ?? $0.text, annotation: $0.text)
        }
        for panel in TestFixtures.candidatePanels() {
            _ = panel.updateCandidates(Self.cells)
            panel.navigate(.nextCandidate)
            panel.navigate(.nextCandidate)
            panel.navigate(.nextCandidate)
            XCTAssertEqual(panel.selectedIndex, 3, "\(type(of: panel)): walked to the fourth candidate")

            panel.rerenderCandidates(swapped)

            XCTAssertEqual(
                panel.selectedIndex, 3,
                "\(type(of: panel)): a display flip must not move the highlight",
            )
            XCTAssertFalse(panel.isEmpty, "\(type(of: panel)): the list survives a re-render")
            // Slot-chord reachability is only assertable where the mapping is
            // pure page geometry: the vertical layout numbers its slots off
            // the live scroll viewport, which a never-presented window does
            // not have.
            if !(panel is VerticalCandidatePanel) {
                XCTAssertTrue(
                    (0 ..< 9).contains { panel.candidateIndex(forSlot: $0) == panel.selectedIndex },
                    "\(type(of: panel)): the selection must sit on a page a slot chord can address",
                )
            }
        }
    }

    /// A re-render keeps the grid open, and keeps the highlight on the
    /// candidate it was on — a selection the collapsed row cannot show must
    /// never be folded away out of sight.
    func testExpandable_rerenderKeepsAnExpandedSelectionExpanded() {
        let swapped = Self.cells.map {
            CandidateCellContent(text: $0.annotation ?? $0.text, annotation: $0.text)
        }
        let panel = ExpandableCandidatePanel(
            style: .sequoia,
            metrics: TestFixtures.defaultCandidateMetrics,
        )
        _ = panel.updateCandidates(Self.cells)
        // Far past any collapsed row's packing budget, so the walk expanded it.
        for _ in 0 ..< 20 {
            panel.navigate(.nextCandidate)
        }
        XCTAssertEqual(panel.selectedIndex, 20)
        XCTAssertEqual(panel.displayMode, .expanded, "the walk must have expanded the grid")

        panel.rerenderCandidates(swapped)

        XCTAssertEqual(panel.selectedIndex, 20)
        XCTAssertEqual(panel.displayMode, .expanded)
    }

    /// The mode a re-render comes back in is the mode it went in with, NOT one
    /// re-derived from where the selection sits: a grid opened with `↓` while
    /// the first candidate is still selected must stay a grid across a 漢羅
    /// swap. Folding it back to a row and unfolding again is what made the
    /// window blink on device (USER 2026-08-23).
    func testExpandable_rerenderKeepsTheGridOpenEvenWithTheSelectionOnRowZero() {
        let swapped = Self.cells.map {
            CandidateCellContent(text: $0.annotation ?? $0.text, annotation: $0.text)
        }
        let panel = ExpandableCandidatePanel(
            style: .sequoia,
            metrics: TestFixtures.defaultCandidateMetrics,
        )
        _ = panel.updateCandidates(Self.cells)
        panel.navigate(.down)
        XCTAssertEqual(panel.displayMode, .expanded, "`↓` opens the grid")
        XCTAssertEqual(panel.selectedIndex, 0, "and leaves the selection where it was")

        panel.rerenderCandidates(swapped)

        XCTAssertEqual(panel.displayMode, .expanded)
        XCTAssertEqual(panel.selectedIndex, 0)
    }

    /// A cleared panel has nothing to re-render: the call is a no-op rather
    /// than a resurrection.
    func testEveryLayout_rerenderAfterClear_staysEmpty() {
        for panel in TestFixtures.candidatePanels() {
            _ = panel.updateCandidates(Self.cells)
            panel.clear()

            panel.rerenderCandidates(Self.cells)

            XCTAssertTrue(panel.isEmpty, "\(type(of: panel)): a cleared panel must stay empty")
        }
    }

    func testEveryLayout_movesExactlyOneCandidatePerStep() {
        for panel in TestFixtures.candidatePanels() {
            _ = panel.updateCandidates(Self.cells)
            XCTAssertEqual(panel.selectedIndex, 0, "\(type(of: panel)) selects the first candidate")

            panel.navigate(.nextCandidate)
            XCTAssertEqual(panel.selectedIndex, 1, "\(type(of: panel)): one step forward")

            panel.navigate(.nextCandidate)
            panel.navigate(.previousCandidate)
            XCTAssertEqual(panel.selectedIndex, 1, "\(type(of: panel)): a step back undoes a step forward")
        }
    }

    /// The D4 rule: navigation clamps and never wraps. A wrap would make a key
    /// held down cycle the list forever instead of stopping at the answer.
    func testEveryLayout_clampsAtBothEnds() {
        for panel in TestFixtures.candidatePanels() {
            _ = panel.updateCandidates(Self.cells)

            panel.navigate(.previousCandidate)
            XCTAssertEqual(panel.selectedIndex, 0, "\(type(of: panel)): no wrap to the last candidate")

            for _ in 0 ..< Self.candidateCount + 5 {
                panel.navigate(.nextCandidate)
            }
            XCTAssertEqual(
                panel.selectedIndex,
                Self.candidateCount - 1,
                "\(type(of: panel)): walking past the end stops on the last candidate",
            )
        }
    }

    /// A step forward that leaves the collapsed row opens the grid AND lands on
    /// the next candidate, the way `→` does — an expand that kept the old
    /// selection would eat the keypress.
    func testExpandablePanel_expandsWhenAStepLeavesTheCollapsedRow() {
        let panel = ExpandableCandidatePanel(style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics)
        _ = panel.updateCandidates(Self.cells)

        var steps = 0
        while panel.displayMode == .collapsed, steps < Self.candidateCount {
            panel.navigate(.nextCandidate)
            steps += 1
        }

        XCTAssertEqual(panel.displayMode, .expanded, "walking off the row's end opens the grid")
        XCTAssertEqual(panel.selectedIndex, steps, "every step moved the selection, including the one that expanded")
    }

    /// The one place the semantic direction is deliberately NOT `.left`: at the
    /// start of the expanded grid, `←` folds the window back into its row, and a
    /// key whose label says "previous candidate" must not do that.
    func testExpandablePanel_previousCandidateAtTheStart_doesNotCollapseTheGrid() {
        let panel = ExpandableCandidatePanel(style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics)
        _ = panel.updateCandidates(Self.cells)
        panel.navigate(.down) // expands without animating past the assertions
        XCTAssertEqual(panel.displayMode, .expanded)
        panel.navigate(.nextCandidate)
        panel.navigate(.previousCandidate)
        XCTAssertEqual(panel.selectedIndex, 0)

        panel.navigate(.previousCandidate)

        XCTAssertEqual(panel.selectedIndex, 0, "already at the first candidate")
        XCTAssertEqual(panel.displayMode, .expanded, "the grid stays open — only `←` folds it")
    }

    func testExpandablePanel_leftAtTheStart_stillCollapsesTheGrid() {
        let panel = ExpandableCandidatePanel(style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics)
        _ = panel.updateCandidates(Self.cells)
        panel.navigate(.down)
        XCTAssertEqual(panel.displayMode, .expanded)

        panel.navigate(.left)

        XCTAssertEqual(panel.displayMode, .collapsed, "`←` out of the first cell is how the grid folds")
    }

    /// `→` pages a vertical column (`VerticalCandidatePanel.navigate`), which is
    /// exactly the confusion the semantic direction exists to avoid.
    func testVerticalPanel_stepsWhereTheRightArrowWouldHavePaged() {
        let stepping = VerticalCandidatePanel(style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics)
        let paging = VerticalCandidatePanel(style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics)
        _ = stepping.updateCandidates(Self.cells)
        _ = paging.updateCandidates(Self.cells)

        stepping.navigate(.nextCandidate)
        paging.navigate(.right)

        XCTAssertEqual(stepping.selectedIndex, 1)
        XCTAssertGreaterThan(
            paging.selectedIndex,
            stepping.selectedIndex,
            "`→` jumps a whole viewport here, which is why 'next candidate' is its own direction",
        )
    }

}
