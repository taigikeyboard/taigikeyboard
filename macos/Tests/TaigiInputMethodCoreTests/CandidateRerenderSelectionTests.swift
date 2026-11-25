// An in-place repaint that names the cell the selection follows to.

@testable import TaigiInputMethodCore
import XCTest

/// What these pin: a list fetched again under new settings (the
/// Hanji/romanization swap with romanized words as candidates) may put the
/// selected word on another cell, so the repaint takes the cell desktop-core
/// followed it to (`CandidateListUpdate.selectedIndex`) — in every layout —
/// while a repaint naming none keeps the selection on its index.
@MainActor
final class CandidateRerenderSelectionTests: XCTestCase {
    private static let cells: [CandidateCellContent] = (0 ..< 12).map {
        CandidateCellContent(text: "候\($0)", annotation: "hau\($0)")
    }

    func testARerenderNamingACell_selectsIt() {
        for panel in TestFixtures.candidatePanels() {
            let label = String(describing: type(of: panel))
            _ = panel.updateCandidates(Self.cells)
            panel.navigate(.nextCandidate)
            panel.navigate(.nextCandidate)
            panel.navigate(.nextCandidate)

            panel.rerender(Array(Self.cells.dropLast()), selecting: 1)

            XCTAssertEqual(panel.selectedIndex, 1, "\(label): the followed cell")
            panel.clear()
        }
    }

    func testARerenderNamingNoCell_keepsTheIndex() {
        for panel in TestFixtures.candidatePanels() {
            let label = String(describing: type(of: panel))
            _ = panel.updateCandidates(Self.cells)
            panel.navigate(.nextCandidate)
            let kept = panel.selectedIndex

            panel.rerender(Self.cells)

            XCTAssertEqual(panel.selectedIndex, kept, "\(label): the same index")
            panel.clear()
        }
    }
}
