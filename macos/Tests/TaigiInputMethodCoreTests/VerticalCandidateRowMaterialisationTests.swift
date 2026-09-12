// The vertical window builds rows as the viewport reaches them, not the whole list per keystroke.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// What these pin: a vertical list of two hundred candidates must not cost two
/// hundred row views before the window is shown — that was a visible pause on
/// every keystroke on a slower Mac — and the rows that ARE built must be the
/// ones the selection, the scroll and the key labels can reach, decorated the
/// same as if every row had been built up front.
@MainActor
final class VerticalCandidateRowMaterialisationTests: XCTestCase {
    private static let cellCount = 50
    private static let cells: [CandidateCellContent] = (0 ..< cellCount).map {
        CandidateCellContent(text: "候\($0)", annotation: "hau\($0)")
    }

    /// One row's pitch in the rows container: the annotated item height plus
    /// the one-point separator.
    private static let rowHeight =
        TestFixtures.defaultCandidateMetrics.forContent(hasAnnotations: true).itemHeight + 1

    /// A fresh list builds the rows the opening viewport shows plus a small
    /// buffer — nine rows, the half-row peek, two beyond — and nothing else.
    func testFreshList_buildsOnlyTheRowsTheViewportReaches() {
        let panel = makePanel()
        _ = panel.updateCandidates(Self.cells)

        let built = builtRows(in: panel)
        XCTAssertEqual(built, Array(0 ..< 12), "nine visible + the peeking tenth + a two-row buffer")
        XCTAssertLessThan(built.count, Self.cellCount)
        XCTAssertEqual(
            TestFixtures.descendants(of: panel, as: CandidateSeparatorView.self).count, built.count,
            "each built row carries its separator",
        )
    }

    /// Walking the selection down materialises the rows it scrolls into,
    /// highlighted and numbered as it goes, without building the rest.
    func testWalkingDown_materialisesTheRowsItScrollsInto() {
        let panel = makePanel()
        _ = panel.updateCandidates(Self.cells)
        showForScrolling(panel)
        defer { panel.orderOut(nil) }

        for _ in 0 ..< 20 {
            panel.navigate(.nextCandidate)
        }
        XCTAssertEqual(panel.selectedIndex, 20)

        let items = itemsByRow(in: panel)
        XCTAssertNotNil(items[20], "the selected row exists")
        XCTAssertEqual(items[20]?.isHighlighted, true, "and is highlighted")
        XCTAssertLessThan(items.count, Self.cellCount, "the tail is still unbuilt")
        assertDigitsMatchSlots(in: panel)
    }

    /// A page jump lands on rows nothing had built yet: the target viewport
    /// is materialised in the same call, not a scroll notification later.
    func testPageJump_acrossUnbuiltRows_landsOnBuiltRows() {
        let panel = makePanel()
        _ = panel.updateCandidates(Self.cells)
        showForScrolling(panel)
        defer { panel.orderOut(nil) }

        panel.navigate(.pageDown)
        panel.navigate(.pageDown)
        panel.navigate(.pageDown)
        XCTAssertEqual(panel.selectedIndex, 27)

        let items = itemsByRow(in: panel)
        for row in 27 ..< 36 {
            XCTAssertNotNil(items[row], "row \(row) of the jumped-to page is built")
        }
        XCTAssertEqual(items[27]?.isHighlighted, true)
        XCTAssertNil(items[Self.cellCount - 1], "the far end was not built for a jump that stopped short of it")
        assertDigitsMatchSlots(in: panel)
    }

    /// A display re-render keeps a selection that sits far down the list, and
    /// its row exists after the rebuild.
    func testRerender_keepsAFarSelectionOnABuiltRow() {
        let panel = makePanel()
        _ = panel.updateCandidates(Self.cells)
        showForScrolling(panel)
        defer { panel.orderOut(nil) }
        for _ in 0 ..< 20 {
            panel.navigate(.nextCandidate)
        }

        let swapped = Self.cells.map { CandidateCellContent(text: $0.annotation ?? $0.text, annotation: $0.text) }
        panel.rerenderCandidates(swapped)

        XCTAssertEqual(panel.selectedIndex, 20)
        let selected = itemsByRow(in: panel)[20]
        XCTAssertEqual(selected?.isHighlighted, true)
        XCTAssertEqual(selected?.candidateLabelText, "hau20", "rebuilt from the new cells")
    }

    /// A user scroll — the wheel, a dragged thumb — is not a navigation call,
    /// and can move the viewport further than the buffer in one go. The rows
    /// it lands on are built by the time the scroll notification has been
    /// delivered, synchronously, with nothing built for the rows it skipped.
    func testUserScroll_pastTheBuffer_buildsTheRowsItLandsOn_andNothingBetween() {
        let panel = makePanel()
        _ = panel.updateCandidates(Self.cells)
        showForScrolling(panel)
        defer { panel.orderOut(nil) }

        scrollUserViewport(of: panel, toRow: 30)

        let built = builtRows(in: panel)
        XCTAssertTrue(built.contains(30), "the row at the top of the viewport")
        XCTAssertTrue(built.contains(28), "and the buffer above it")
        XCTAssertFalse(built.contains(27), "but not past the buffer")
        XCTAssertFalse((13 ..< 28).contains { built.contains($0) }, "the skipped rows stay unbuilt")
        assertDigitsMatchSlots(in: panel)
    }

    /// Navigation on a window that was never shown — the seam's unit tests do
    /// this — still lands the selection on a built row: the viewport height
    /// `rebuildRows` computed stands in for the clip view's, which is zero.
    func testNavigatingAnUnshownWindow_buildsTheSelectedRow() {
        let panel = makePanel()
        _ = panel.updateCandidates(Self.cells)

        for _ in 0 ..< 20 {
            panel.navigate(.nextCandidate)
        }

        XCTAssertEqual(panel.selectedIndex, 20)
        XCTAssertEqual(itemsByRow(in: panel)[20]?.isHighlighted, true)
    }

    /// Tahoe hides the two hairlines touching the selection pill. With a
    /// sparse separator set that lookup is by row, so the hidden pair must be
    /// the pair around the selection — not whichever two happen to sit at
    /// positions 38 and 39 of the built set, which with the gap left by the
    /// scroll would be rows further down.
    func testTahoe_hidesTheSeparatorsAroundASelectionInASparseList() {
        let panel = makePanel(style: .tahoe)
        _ = panel.updateCandidates(Self.cells)
        showForScrolling(panel)
        defer { panel.orderOut(nil) }
        // Scroll past the buffer, then page from there: the jump reads the
        // anchor the scroll left, so the selection lands a page below it.
        scrollUserViewport(of: panel, toRow: 30)
        panel.navigate(.pageDown)
        XCTAssertEqual(panel.selectedIndex, 39)
        XCTAssertFalse(builtRows(in: panel).contains(20), "the list is sparse")

        let itemHeight = Self.rowHeight - 1
        let hidden = TestFixtures.descendants(of: panel, as: CandidateSeparatorView.self)
            .filter { $0.alphaValue == 0 }
            .map { Int(($0.frame.origin.y - itemHeight) / Self.rowHeight) }
            .sorted()
        XCTAssertEqual(hidden, [38, 39], "the hairline above and below row 39")
    }

    // MARK: - Helpers

    private func makePanel(style: CandidateWindowStyle = .sequoia) -> VerticalCandidatePanel {
        VerticalCandidatePanel(style: style, metrics: TestFixtures.defaultCandidateMetrics)
    }

    /// Moves the clip view the way a wheel or a dragged thumb does — through
    /// the scroll view, not through the panel's navigation.
    private func scrollUserViewport(of panel: CandidateBasePanel, toRow row: Int) {
        guard let scrollView = TestFixtures.descendants(of: panel, as: NSScrollView.self).first else {
            return XCTFail("the vertical panel has no scroll view")
        }
        scrollView.contentView.scroll(to: NSPoint(x: 0, y: CGFloat(row) * Self.rowHeight))
        scrollView.reflectScrolledClipView(scrollView.contentView)
    }

    private func itemsByRow(in panel: CandidateBasePanel) -> [Int: CandidateItemView] {
        Dictionary(
            uniqueKeysWithValues: TestFixtures.candidateCells(in: panel).map { ($0.absoluteIndex, $0) },
        )
    }

    private func builtRows(in panel: CandidateBasePanel) -> [Int] {
        itemsByRow(in: panel).keys.sorted()
    }
}
