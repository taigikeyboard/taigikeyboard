// The §34 literal cell's own decorations: no key, and a faint fill of its own.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// What these pin: with 顯示當咧拍的字 on, cell 0 is what the user is currently
/// typing rather than a candidate the engine offered, so it draws a fill of its
/// own (USER 2026-09-09) — and only cell 0 does, in whichever layout is up.
/// The selection still wins: a highlighted literal cell is drawn like any other
/// highlighted cell, which is what keeps the tint from reading as a second
/// selection.
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
            XCTAssertEqual(marked.map(\.absoluteIndex), [0], "\(label): only cell 0 carries the fill")
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
    /// mark rides the same repaint as the keys
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

    /// The fill resolves DIFFERENTLY in the two appearances — a fixed tint
    /// would read wrong in one of them — and the selection's fill wins over
    /// it. Pinned on the Tahoe cell, whose fill is a view of its own.
    func testTheFill_resolvesPerAppearance_andTheSelectionWins() throws {
        var tintPerAppearance: [CGColor] = []
        for appearance in [NSAppearance(named: .aqua), NSAppearance(named: .darkAqua)] {
            let view = CandidateItemView(
                style: .tahoe, metrics: TestFixtures.defaultCandidateMetrics.arranged(.stacked),
            )
            view.appearance = try XCTUnwrap(appearance)
            view.frame = NSRect(x: 0, y: 0, width: 80, height: 40)
            view.configure(Self.cells[0])

            view.isLiteralCell = true
            view.layoutSubtreeIfNeeded()
            let fill = try XCTUnwrap(view.subviews.first { !($0 is NSTextField) })
            let tint = try XCTUnwrap(fill.layer?.backgroundColor)
            tintPerAppearance.append(tint)
            XCTAssertFalse(fill.isHidden, "the literal cell draws a fill of its own")
            XCTAssertEqual(fill.frame, view.bounds.insetBy(dx: 4, dy: 4), "at the highlight's shape")

            view.isHighlighted = true
            view.layoutSubtreeIfNeeded()
            XCTAssertFalse(fill.isHidden)
            XCTAssertNotEqual(
                fill.layer?.backgroundColor, tint, "the selection paints over the tint",
            )

            view.isHighlighted = false
            view.isLiteralCell = false
            XCTAssertTrue(fill.isHidden, "an ordinary unselected cell draws none")
        }

        XCTAssertEqual(tintPerAppearance.count, 2)
        XCTAssertNotEqual(
            tintPerAppearance[0], tintPerAppearance[1],
            "the tint follows the appearance rather than being one fixed colour",
        )
    }
}
