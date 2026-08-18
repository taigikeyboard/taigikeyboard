// The expanded grid's geometry: spans, row breaks, vertical targeting.

@testable import TaigiInputMethodCore
import XCTest

/// Pins the grid the expandable window unfolds into. A column width of 10 and
/// six columns keep every trace mental-arithmetic, mirroring
/// `HorizontalPageLayoutTests`.
final class ExpandedGridLayoutTests: XCTestCase {
    private func grid(_ widths: [CGFloat]) -> ExpandedGridLayout {
        ExpandedGridLayout.compute(widths: widths, columnWidth: 10, columnCount: 6)
    }

    func testCompute_quantizesWidthsIntoColumnSpans() {
        // trace: spans = ceil(width/10) → [1, 2, 3]; 1+2+3 = 6 fills the row
        // exactly; the next item starts row 1.
        let layout = grid([10, 15, 25, 10])

        XCTAssertEqual(
            layout.rows.map { $0.map { [$0.candidateIndex, $0.columnStart, $0.columnSpan] } },
            [[[0, 0, 1], [1, 1, 2], [2, 3, 3]], [[3, 0, 1]]],
        )
    }

    func testCompute_oversizedItemIsClampedToAFullRow() {
        // trace: 200 → span ceil(20) clamps to 6 → its own full row.
        let layout = grid([10, 200, 10])

        XCTAssertEqual(
            layout.rows.map { $0.map(\.candidateIndex) },
            [[0], [1], [2]],
        )
        XCTAssertEqual(layout.rows[1][0].columnSpan, 6)
    }

    func testVerticalTarget_landsOnTheOverlappingCell() {
        // trace: row 0 = [idx0 span1 @0, idx1 span2 @1, idx2 span3 @3];
        // row 1 = [idx3 span3 @0, idx4 span3 @3].
        let layout = grid([10, 15, 25, 25, 25])

        // idx1 occupies columns 1..<3; idx3 (0..<3) overlaps → down lands on 3.
        XCTAssertEqual(layout.verticalTarget(from: 1, rowStep: 1), 3)
        // idx4 occupies 3..<6; back up, idx2 (3..<6) overlaps → 2.
        XCTAssertEqual(layout.verticalTarget(from: 4, rowStep: -1), 2)
    }

    func testVerticalTarget_atTheEdgeRow_isNilSoTheCallerCanCollapse() {
        let layout = grid([10, 15, 25, 25, 25])

        XCTAssertNil(layout.verticalTarget(from: 0, rowStep: -1), "no row above the first")
        XCTAssertNil(layout.verticalTarget(from: 4, rowStep: 1), "no row below the last")
    }

    func testVerticalTargetBackward_prefersTheNeighbourUnderTheBody() {
        // trace: row 0 = [idx0 span4 @0, idx1 span2 @4]; row 1 = [idx2 span1
        // @0, idx3 span1 @1, idx4 span1 @2, idx5 span3 @3]. From idx5 (columns
        // 3..<6) stepping up, idx0 (0..<4) overlaps first but starts left of
        // the origin's column and idx1 (4..<6) still begins inside the span →
        // the tiebreak steps right to idx1, the cell the highlight sits over.
        let layout = grid([40, 20, 10, 10, 10, 30])

        XCTAssertEqual(layout.verticalTarget(from: 5, rowStep: -1), 1)
    }
}
