// The §34 literal cell's own decorations: no key, and a faint fill of its own.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// What these pin: with 顯示當咧拍的字 on, cell 0 is what the user is currently
/// typing rather than a candidate the engine offered — it is marked as such,
/// in whichever layout is up, and being marked is what centres its text across
/// the whole cell (it names no key, so it does not step around the key column
/// the other cells align on). It draws NO fill of its own: a tint was tried on
/// 2026-09-09 and taken back out the same day (USER: 「背景底色強調效果不好,
/// 恢復第一個位置的背景底色」).
@MainActor
final class CandidateLiteralCellTests: XCTestCase {
    private static let cells: [CandidateCellContent] = (0 ..< 12).map {
        CandidateCellContent(text: "候\($0)", annotation: "hau\($0)")
    }

    func testOnlyTheLeadCell_isMarkedAsTheLiteral() {
        for panel in TestFixtures.candidatePanels() {
            let label = String(describing: type(of: panel))
            panel.leadCellIsUnkeyed = true
            _ = panel.updateCandidates(Self.cells)

            let marked = TestFixtures.candidateCells(in: panel).filter(\.isLiteralCell)
            XCTAssertEqual(marked.map(\.absoluteIndex), [0], "\(label): only cell 0 is the literal")
            panel.clear()
        }
    }

    func testWithoutTheLiteral_noCellIsMarked() {
        for panel in TestFixtures.candidatePanels() {
            let label = String(describing: type(of: panel))
            _ = panel.updateCandidates(Self.cells)

            XCTAssertTrue(
                TestFixtures.candidateCells(in: panel).allSatisfy { !$0.isLiteralCell },
                "\(label): with the setting off every cell is an ordinary candidate",
            )
            panel.clear()
        }
    }

    /// A list re-presented without the literal leaves no cell marked — the
    /// mark rides the same repaint as the keys, and it is what places the
    /// text
    /// (`CandidateBasePanel.refreshCellDecorations`), so it cannot outlive the
    /// list it was read from.
    func testARerenderedList_dropsTheMarkWithTheLiteral() {
        let panel = HorizontalCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        panel.leadCellIsUnkeyed = true
        _ = panel.updateCandidates(Self.cells)
        XCTAssertEqual(
            TestFixtures.candidateCells(in: panel).filter(\.isLiteralCell).count, 1,
        )

        panel.leadCellIsUnkeyed = false
        panel.rerender(Self.cells)

        XCTAssertTrue(
            TestFixtures.candidateCells(in: panel).allSatisfy { !$0.isLiteralCell },
            "the list no longer leads with the literal, so no cell keeps its fill",
        )
    }

    /// The literal cell names no key, so its text centres across the WHOLE
    /// cell — the cell its fill covers — rather than in the area an ordinary
    /// cell leaves once the key column has its slot (USER 2026-09-09). An
    /// ordinary cell keeps the column, which is what lines a row's candidates
    /// up with each other.
    func testTheLiteralCell_centresItsTextAcrossTheWholeCell() {
        let metrics = TestFixtures.defaultCandidateMetrics.arranged(.stacked)
        let content = CandidateCellContent(text: "tâi", annotation: nil)
        let frame = NSRect(x: 0, y: 0, width: metrics.measureWidth(content), height: metrics.itemHeight)

        let literal = CandidateItemView(style: .sequoia, metrics: metrics)
        literal.isLiteralCell = true
        literal.configure(content)
        literal.frame = frame
        literal.layoutSubtreeIfNeeded()

        let ordinary = CandidateItemView(style: .sequoia, metrics: metrics)
        ordinary.configure(content)
        ordinary.setIndexLabel("q")
        ordinary.frame = frame
        ordinary.layoutSubtreeIfNeeded()

        // Index 1 is the candidate itself; index 0 is the key column.
        let literalText = literal.subviews.compactMap { $0 as? NSTextField }[1].frame
        let ordinaryText = ordinary.subviews.compactMap { $0 as? NSTextField }[1].frame

        XCTAssertEqual(
            literalText.midX, literal.bounds.midX, accuracy: 0.5,
            "the literal cell's text is centred in the cell its fill covers",
        )
        XCTAssertEqual(
            literalText.midY, literal.bounds.midY, accuracy: 0.5,
            "vertically too — a one-script cell gives the empty line's height back",
        )
        XCTAssertGreaterThan(
            ordinaryText.midX, literalText.midX,
            "an ordinary cell still centres to the right of the key column",
        )

        // And back: a recycled cell that stops being the literal takes the key
        // column into account again, with no constraint left over from before.
        literal.isLiteralCell = false
        literal.setIndexLabel("q")
        literal.layoutSubtreeIfNeeded()
        XCTAssertEqual(
            literal.subviews.compactMap { $0 as? NSTextField }[1].frame, ordinaryText,
        )
    }
}
