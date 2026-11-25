// Pins the expanded-grid row packing: no row is wider than the grid, so a long
// sentence candidate can never push the control column off screen.

import KeyboardKit
@testable import TaigiKeyboard
import XCTest

final class ExpandedCandidateRowLayoutTests: XCTestCase {
    private func suggestions(_ texts: [String]) -> [AutocompleteSuggestion] {
        texts.map { AutocompleteSuggestion(text: $0, title: $0, subtitle: nil) }
    }

    private func rowWidth(_ row: [ExpandedCandidateRowLayout.RowItem], spacing: CGFloat) -> CGFloat {
        row.map(\.measuredWidth).reduce(0, +) + spacing * CGFloat(max(0, row.count - 1))
    }

    func testArrangeRows_overWideCandidate_clampedToAvailableWidth() {
        let widths: [String: CGFloat] = ["long": 900, "a": 60, "b": 60]
        let rows = ExpandedCandidateRowLayout.arrangeRows(
            suggestions: suggestions(["long", "a", "b"]),
            availableWidth: 300,
            itemSpacing: 4,
            measureCellWidth: { widths[$0.text]! },
        )

        XCTAssertEqual(rows.count, 2, "the long candidate owns its row, the short ones share the next")
        XCTAssertEqual(rows[0].map(\.suggestion.text), ["long"])
        XCTAssertEqual(rows[0][0].measuredWidth, 300, "over-wide cell is clamped to the grid width")
        XCTAssertEqual(rows[1].map(\.suggestion.text), ["a", "b"])
    }

    func testArrangeRows_everyRowFitsAvailableWidth() {
        let texts = (0 ..< 30).map { "c\($0)" }
        let rows = ExpandedCandidateRowLayout.arrangeRows(
            suggestions: suggestions(texts),
            availableWidth: 250,
            itemSpacing: 4,
            // Every 7th cell is over-wide; the rest are 44–96 pt.
            measureCellWidth: { suggestion in
                let n = Int(suggestion.text.dropFirst())!
                return n % 7 == 0 ? 1200 : CGFloat(44 + 13 * (n % 5))
            },
        )

        for (index, row) in rows.enumerated() {
            XCTAssertLessThanOrEqual(rowWidth(row, spacing: 4), 250, "row \(index) overflows the grid")
        }
        XCTAssertEqual(rows.flatMap(\.self).map(\.originalIndex), Array(0 ..< 30), "order and indices kept")
    }
}
