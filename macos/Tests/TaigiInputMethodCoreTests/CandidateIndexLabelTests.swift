// The digit beside a candidate names the key that picks it — in every layout.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// What these pin: the digits drawn in the window and the slots the key handler
/// resolves are one contract. A cell showing `3` must be the candidate
/// `candidateIndex(forSlot: 2)` answers with, in whichever layout is up and
/// wherever the viewport has scrolled to — a digit that named a different
/// candidate would commit the wrong word on a keypress.
@MainActor
final class CandidateIndexLabelTests: XCTestCase {
    private static let cells: [CandidateCellContent] = (0 ..< 40).map {
        CandidateCellContent(text: "候\($0)", annotation: "hau\($0)")
    }

    // MARK: - The keys themselves

    /// The digit set draws the bare digit; a tenth position stays reserved
    /// but no key names it — a bare `0` is document text.
    func testSlotKeys_underTheDigits_areTheDigits_andStopAtTheNinth() {
        XCTAssertEqual(CandidateIndexLabel.text(forSlot: 0, keySet: .digits), "1")
        XCTAssertEqual(CandidateIndexLabel.text(forSlot: 8, keySet: .digits), "9")
        XCTAssertEqual(CandidateIndexLabel.text(forSlot: 9, keySet: .digits), "")
        XCTAssertEqual(CandidateIndexLabel.text(forSlot: -1, keySet: .bareKeys), "")
    }

    /// The bare keys name all nine slots, drawn lowercase — the way each key
    /// is pressed.
    func testSlotKeys_underTheBareKeys_areTheNineKeysInSlotOrder() {
        XCTAssertEqual(
            (0 ..< 9).map { CandidateIndexLabel.text(forSlot: $0, keySet: .bareKeys) },
            ["q", "w", "d", "f", "z", "x", "v", "y", ";"],
        )
        XCTAssertEqual(CandidateIndexLabel.text(forSlot: 9, keySet: .bareKeys), "")
    }

    // MARK: - What each layout draws

    /// The horizontal page renumbers from the first slot key on every page,
    /// which is what its slot mapping resolves against. The panels draw the
    /// shipped bare keys unless told otherwise.
    private static let keys = CandidateSlotKeySet.bareKeyRow

    func testHorizontal_numbersEveryPageFromOne() {
        let panel = HorizontalCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        _ = panel.updateCandidates(Self.cells)

        assertDigitsMatchSlots(in: panel)
        let firstPage = numberedCells(in: panel)
        XCTAssertEqual(firstPage.first?.digit, Self.keys[0])
        XCTAssertEqual(firstPage.map(\.digit), Array(Self.keys.prefix(firstPage.count)))

        panel.navigate(.pageDown)

        let secondPage = numberedCells(in: panel)
        XCTAssertNotEqual(
            secondPage.first?.item.absoluteIndex, firstPage.first?.item.absoluteIndex,
            "the page turned",
        )
        XCTAssertEqual(secondPage.first?.digit, Self.keys[0], "a page's keys start over")
        assertDigitsMatchSlots(in: panel)
    }

    /// The vertical column numbers the nine rows from the viewport's anchor and
    /// nothing else: rows below the fold keep their slot but draw no digit.
    func testVertical_numbersOnlyTheRowsTheChordsCanReach() {
        let panel = VerticalCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        _ = panel.updateCandidates(Self.cells)

        let numbered = numberedCells(in: panel)
        XCTAssertEqual(numbered.map(\.item.absoluteIndex), Array(0 ..< 9))
        XCTAssertEqual(numbered.map(\.digit), Self.keys)
        assertDigitsMatchSlots(in: panel)

        // Every row still reserves the slot, drawn or not — the column the
        // candidates align on is one width all the way down.
        let widths = TestFixtures.candidateCells(in: panel).map { cell -> CGFloat in
            let labels = cell.subviews.compactMap { $0 as? NSTextField }
            return labels.first?.frame.width ?? 0
        }
        XCTAssertEqual(Set(widths).count, 1, "the digit slot is the same width on every row")
    }

    /// The collapsed row is the one row the chords address, so it carries the
    /// digits; expanded, only the selected row does — `candidateIndex(forSlot:)`
    /// resolves a slot within THAT row, and numbering the others would name
    /// keys that pick something else.
    func testExpandable_numbersTheRowTheChordsAddress() {
        let panel = ExpandableCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        _ = panel.updateCandidates(Self.cells)

        XCTAssertEqual(panel.displayMode, .collapsed)
        assertDigitsMatchSlots(in: panel)
        XCTAssertEqual(numberedCells(in: panel).first?.digit, Self.keys[0])

        // Walking off the collapsed row's end unfolds the grid.
        for _ in 0 ..< 20 {
            panel.navigate(.nextCandidate)
        }
        XCTAssertEqual(panel.displayMode, .expanded)

        let numbered = numberedCells(in: panel)
        XCTAssertTrue(
            numbered.contains { $0.item.absoluteIndex == panel.selectedIndex },
            "the selected row is the numbered one",
        )
        XCTAssertEqual(numbered.map(\.digit), Array(Self.keys.prefix(numbered.count)))
        assertDigitsMatchSlots(in: panel)
    }

    /// Walking past the ninth row scrolls the viewport, and the digits follow
    /// it: the rows the chords reach are the ones on screen, never the list's
    /// first nine.
    func testVertical_renumbersAsTheViewportScrolls() {
        let panel = VerticalCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        _ = panel.updateCandidates(Self.cells)
        // Shown, because the numbering is read off the live scroll viewport —
        // an unshown window never scrolls, so the anchor could not move.
        panel.setFrame(NSRect(x: 0, y: 0, width: 320, height: 320), display: false)
        panel.orderFront(nil)
        defer { panel.orderOut(nil) }

        for _ in 0 ..< 20 {
            panel.navigate(.nextCandidate)
        }
        XCTAssertEqual(panel.selectedIndex, 20)

        let numbered = numberedCells(in: panel)
        XCTAssertFalse(numbered.isEmpty, "the visible rows carry the digits")
        XCTAssertNotEqual(
            numbered.map(\.item.absoluteIndex), Array(0 ..< 9),
            "the digits must have left the list's first nine rows",
        )
        // Which nine rows those are is the panel's scroll rule, and a test
        // window's viewport is not the one a user gets — what has to hold is
        // that whatever is drawn is what the chords pick.
        assertDigitsMatchSlots(in: panel)
    }

    /// The expanded grid renumbers as the selection walks between rows — the
    /// digits name keys that pick from the row the selection is in.
    func testExpandable_renumbersWhenTheSelectionChangesRows() {
        let panel = ExpandableCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        _ = panel.updateCandidates(Self.cells)
        for _ in 0 ..< 20 {
            panel.navigate(.nextCandidate)
        }
        XCTAssertEqual(panel.displayMode, .expanded)
        let before = numberedCells(in: panel).map(\.item.absoluteIndex)

        panel.navigate(.down)

        let after = numberedCells(in: panel).map(\.item.absoluteIndex)
        XCTAssertNotEqual(before, after, "a row step moves the digits to the new row")
        assertDigitsMatchSlots(in: panel)

        panel.navigate(.up)
        assertDigitsMatchSlots(in: panel)

        // And back to one row: collapsing renumbers the row that is left.
        while panel.displayMode == .expanded {
            panel.navigate(.previousCandidate)
            if panel.selectedIndex == 0 {
                panel.navigate(.left)
            }
        }
        assertDigitsMatchSlots(in: panel)
    }

    /// The 漢羅 swap re-renders every cell against the same list. The digits
    /// belong to the POSITIONS, so they must survive it unmoved.
    func testEveryLayout_keepsItsDigitsThroughARerender() {
        let swapped = Self.cells.map {
            CandidateCellContent(text: $0.annotation ?? $0.text, annotation: $0.text)
        }
        for panel in TestFixtures.candidatePanels() {
            _ = panel.updateCandidates(Self.cells)
            panel.navigate(.nextCandidate)

            panel.rerenderCandidates(swapped)

            XCTAssertFalse(
                numberedCells(in: panel).isEmpty,
                "\(type(of: panel)): the digits survive a display flip",
            )
            assertDigitsMatchSlots(in: panel)
        }
    }

    /// Under the digit set every layout draws the bare digit, and the
    /// mapping is untouched — `3` still picks what slot 2 answers.
    func testEveryLayout_drawsTheDigitUnderTheDigitSet() {
        for panel in TestFixtures.candidatePanels() {
            panel.slotKeySet = .digits
            _ = panel.updateCandidates(Self.cells)

            let numbered = numberedCells(in: panel)
            XCTAssertFalse(numbered.isEmpty, "\(type(of: panel)): the keys are drawn")
            for (key, _) in numbered {
                XCTAssertNotNil(
                    Int(key),
                    "\(type(of: panel)): drew \"\(key)\" where the digit picks",
                )
            }
            assertDigitsMatchSlots(in: panel)
        }
    }

    /// The expandable layout builds its grid on the first expand, long after
    /// the set was chosen — those cells carry the digit too.
    func testExpandable_lazyGridKeepsTheDigit() {
        let panel = ExpandableCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        panel.slotKeySet = .digits
        _ = panel.updateCandidates(Self.cells)

        for _ in 0 ..< 20 {
            panel.navigate(.nextCandidate)
        }
        XCTAssertEqual(panel.displayMode, .expanded, "the walk built the grid")

        let numbered = numberedCells(in: panel)
        XCTAssertFalse(numbered.isEmpty)
        for (key, _) in numbered {
            XCTAssertNotNil(Int(key), "a grid cell drew \"\(key)\" instead of its digit")
        }
        assertDigitsMatchSlots(in: panel)
    }

    /// The slot is as wide as the widest key it can draw, so the column does
    /// not shift when the user switches tone scheme.
    func testTheKeyColumn_keepsOneWidthAcrossTheSets() {
        let panel = HorizontalCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        _ = panel.updateCandidates(Self.cells)
        let bareWidths = TestFixtures.candidateCells(in: panel).map(\.frame.width)

        panel.slotKeySet = .digits
        _ = panel.updateCandidates(Self.cells)

        XCTAssertEqual(
            TestFixtures.candidateCells(in: panel).map(\.frame.width), bareWidths,
            "a cell is the same width whichever key picks it",
        )
    }

    // MARK: - The unkeyed §34 literal

    /// With the literal leading, the first cell draws no key and the keys
    /// start on the cell after it: `q` picks candidate 1, not candidate 0
    /// (USER 2026-09-09).
    func testLeadCellIsUnkeyed_startsTheKeysOnTheSecondCell() {
        let panel = HorizontalCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        panel.leadCellIsUnkeyed = true
        _ = panel.updateCandidates(Self.cells)

        let numbered = numberedCells(in: panel)
        XCTAssertEqual(numbered.first?.item.absoluteIndex, 1, "cell 0 draws no key")
        XCTAssertEqual(numbered.first?.digit, Self.keys[0], "the first key moved onto cell 1")
        XCTAssertEqual(panel.candidateIndex(forKeySlot: 0), 1)
        XCTAssertEqual(panel.candidateIndex(forKeySlot: 1), 2)
        assertDigitsMatchSlots(in: panel)
    }

    /// The literal page keys eight candidates: the ninth key falls off the
    /// end of the shifted row rather than the page growing a tenth cell.
    func testLeadCellIsUnkeyed_leavesTheNinthKeyIdleOnThatPage() {
        let panel = HorizontalCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        panel.leadCellIsUnkeyed = true
        _ = panel.updateCandidates(Self.cells)

        let keyed = (0 ..< 9).compactMap { panel.candidateIndex(forKeySlot: $0) }
        XCTAssertEqual(keyed.count, numberedCells(in: panel).count)
        XCTAssertNil(panel.candidateIndex(forKeySlot: 8), "the ninth key picks nothing here")
        XCTAssertNil(panel.candidateIndex(forKeySlot: 9), "past the key row")
    }

    /// A page the literal is not on keys every cell from the first key —
    /// the shift follows the row, not the list.
    func testLeadCellIsUnkeyed_leavesLaterPagesFullyKeyed() {
        let panel = HorizontalCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        panel.leadCellIsUnkeyed = true
        _ = panel.updateCandidates(Self.cells)
        panel.navigate(.pageDown)

        let numbered = numberedCells(in: panel)
        XCTAssertNotEqual(numbered.first?.item.absoluteIndex, 1, "the page turned")
        XCTAssertEqual(numbered.first?.digit, Self.keys[0])
        XCTAssertEqual(panel.candidateIndex(forKeySlot: 0), numbered.first?.item.absoluteIndex)
        assertDigitsMatchSlots(in: panel)
    }

    /// The vertical column shifts only while the literal is the row the keys
    /// start on; once the viewport has scrolled past it every key is back on
    /// its own row.
    func testLeadCellIsUnkeyed_stopsShiftingOnceTheColumnScrollsPastTheLiteral() {
        let panel = VerticalCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        panel.leadCellIsUnkeyed = true
        _ = panel.updateCandidates(Self.cells)
        XCTAssertEqual(panel.candidateIndex(forKeySlot: 0), 1, "shifted at the top")

        panel.setFrame(NSRect(x: 0, y: 0, width: 320, height: 320), display: false)
        panel.orderFront(nil)
        defer { panel.orderOut(nil) }
        for _ in 0 ..< 20 {
            panel.navigate(.nextCandidate)
        }

        let anchoredIndex = panel.candidateIndex(forSlot: 0)
        XCTAssertNotEqual(anchoredIndex, 0, "the viewport left the literal behind")
        XCTAssertEqual(panel.candidateIndex(forKeySlot: 0), anchoredIndex, "no shift off the top")
        assertDigitsMatchSlots(in: panel)
    }

    /// The flag off leaves the first cell keyed, which is what a fetch with
    /// no literal (the setting off) presents.
    func testLeadCellKeyed_whenNoLiteralLeadsTheList() {
        let panel = HorizontalCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        _ = panel.updateCandidates(Self.cells)

        XCTAssertEqual(panel.candidateIndex(forKeySlot: 0), 0)
        XCTAssertEqual(numberedCells(in: panel).first?.item.absoluteIndex, 0)
    }

    /// The expanded grid keys the row the selection is on, so the shift is
    /// the first row's alone: walking down leaves every key on its own cell.
    func testLeadCellIsUnkeyed_shiftsOnlyTheGridRowTheLiteralStarts() {
        let panel = ExpandableCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        panel.leadCellIsUnkeyed = true
        _ = panel.updateCandidates(Self.cells)

        XCTAssertEqual(panel.displayMode, .collapsed)
        XCTAssertEqual(panel.candidateIndex(forKeySlot: 0), 1, "the collapsed row leads with it")
        assertDigitsMatchSlots(in: panel)

        for _ in 0 ..< 20 {
            panel.navigate(.nextCandidate)
        }
        XCTAssertEqual(panel.displayMode, .expanded)

        let rowStart = panel.candidateIndex(forSlot: 0)
        XCTAssertNotEqual(rowStart, 0, "the selection walked off the row the literal starts")
        XCTAssertEqual(panel.candidateIndex(forKeySlot: 0), rowStart, "no shift on that row")
        assertDigitsMatchSlots(in: panel)
    }

    /// The controller double shifts like the real panels, ninth key included —
    /// a slot that falls off the page must not reach the tenth cell.
    func testTheControllerDouble_shiftsAndDropsTheNinthKey() {
        let presenter = RecordingCandidatePresenter()
        let owner = ComposingSessionToken()
        presenter.show(
            CandidateWindowContent(
                cells: Array(Self.cells.prefix(12)), slotKeySet: .bareKeys, leadCellIsUnkeyed: true,
            ),
            anchoredTo: .zero,
            hostWindowLevel: 0,
            hostBundleIdentifier: nil,
            ownedBy: owner,
        )

        XCTAssertEqual(presenter.candidateIndex(forKeySlot: 0, ownedBy: owner), 1)
        XCTAssertEqual(presenter.candidateIndex(forKeySlot: 7, ownedBy: owner), 8)
        XCTAssertNil(
            presenter.candidateIndex(forKeySlot: 8, ownedBy: owner),
            "the ninth key falls off the page rather than reaching a tenth cell",
        )
    }

    // MARK: - Helpers

    /// Every digit drawn in `panel` resolves to the candidate its slot chord
    /// commits — the whole point of drawing them.
    private func assertDigitsMatchSlots(
        in panel: CandidateBasePanel, file: StaticString = #filePath, line: UInt = #line,
    ) {
        for (key, item) in numberedCells(in: panel) {
            // A digit label names its slot directly; the bare keys map by
            // position instead.
            guard let digit = Int(key)
                ?? CandidateSlotKeySet.bareKeyRow.firstIndex(of: key).map({ $0 + 1 })
            else {
                return XCTFail(
                    "\(type(of: panel)): drew a keyless label \"\(key)\"", file: file, line: line,
                )
            }
            XCTAssertEqual(
                panel.candidateIndex(forKeySlot: digit - 1), item.absoluteIndex,
                "\(type(of: panel)): the cell drawn \"\(key)\" is not what that key picks",
                file: file, line: line,
            )
        }
    }

    /// The cells drawing a digit, in the order they are laid out.
    private func numberedCells(
        in panel: CandidateBasePanel,
    ) -> [(digit: String, item: CandidateItemView)] {
        TestFixtures.candidateCells(in: panel).compactMap { item in
            item.indexLabelText.isEmpty ? nil : (item.indexLabelText, item)
        }
    }
}
