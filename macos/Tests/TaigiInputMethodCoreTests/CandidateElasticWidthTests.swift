// A long candidate widens the window instead of truncating inside a fixed one.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The bug these pin: every layout used to cap its cells at a fixed multiple of
/// the slot width — nine slots for the horizontal page and the expandable row,
/// six for the vertical column — so a candidate longer than the cap rendered
/// with an ellipsis while the screen still had room. The cap is now the screen's
/// own width, resolved per show; the panels here have never been shown, so they
/// lay out against the fallback budget, which is what `maximumWindowWidth` reads
/// back as.
@MainActor
final class CandidateElasticWidthTests: XCTestCase {
    /// Long enough to pass any per-layout slot-multiple cap — the nine-slot
    /// row budget and the vertical column's old six — while still fitting the
    /// fallback screen budget an unshown panel lays out against.
    private static let longCandidate = CandidateCellContent(
        text: String(repeating: "候", count: 9),
        annotation: String(repeating: "hau", count: 9),
    )
    private static let shortCandidate = CandidateCellContent(text: "候", annotation: "hau")

    func testEveryLayout_growsForACandidateTooLongForTheSlotWidths() {
        for panel in makePanels() {
            let narrow = panel.updateCandidates([Self.shortCandidate])
            let wide = panel.updateCandidates([Self.longCandidate])

            XCTAssertGreaterThan(
                wide.width, narrow.width,
                "\(type(of: panel)): a long candidate must widen the window, not truncate in it",
            )
            XCTAssertGreaterThanOrEqual(
                wide.width,
                panel.metrics.measureWidth(Self.longCandidate),
                "\(type(of: panel)): the window must be wide enough to render the whole candidate",
            )
        }
    }

    func testEveryLayout_staysInsideTheWidthBudget() {
        // 200 candidates each far wider than any screen: the budget, not the
        // text, has to be what decides the window's width here.
        let hugeCandidate = CandidateCellContent(
            text: String(repeating: "候", count: 400),
            annotation: nil,
        )
        for panel in makePanels() {
            let size = panel.updateCandidates(Array(repeating: hugeCandidate, count: 200))

            XCTAssertLessThanOrEqual(
                size.width, panel.maximumWindowWidth,
                "\(type(of: panel)): past the budget the text truncates — the window does not run off the screen",
            )
        }
    }

    /// The expandable layout's second geometry: the grid quantizes cells into
    /// columns, so its columns have to widen too — a cell spanning a whole row
    /// of them is what holds a long candidate.
    func testExpandableGrid_columnsWidenForALongCandidate() {
        let panel = ExpandableCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics,
        )
        let filler = (0 ..< 40).map { CandidateCellContent(text: "候\($0)", annotation: "hau\($0)") }
        _ = panel.updateCandidates([Self.longCandidate] + filler)

        panel.navigate(.down)
        XCTAssertEqual(panel.displayMode, .expanded, "`↓` opens the grid")

        let gridWidth = panel.expandedGridWidthForTesting
        XCTAssertGreaterThanOrEqual(
            gridWidth,
            panel.metrics.measureWidth(Self.longCandidate),
            "a grid row must be able to render the longest candidate whole",
        )
        XCTAssertLessThanOrEqual(
            gridWidth, panel.maximumWindowWidth,
            "and must still fit the width budget",
        )
    }

    private func makePanels() -> [CandidateBasePanel] {
        [
            HorizontalCandidatePanel(style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics),
            VerticalCandidatePanel(style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics),
            ExpandableCandidatePanel(style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics),
        ]
    }
}
