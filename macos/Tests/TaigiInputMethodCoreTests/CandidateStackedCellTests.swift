// The stacked arrangement: two scripts on top of each other, inside the cell.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The row-shaped layouts stack their cells so a row fits more candidates. The
/// cell has to hold both lines at every text size — its height comes from the
/// panel's frame, not from the labels, so a line box wider or taller than the
/// metrics predicted would render clipped rather than break a constraint.
@MainActor
final class CandidateStackedCellTests: XCTestCase {
    private static let cell = CandidateCellContent(text: "候選", annotation: "hāu-suán")

    func testRowShapedLayoutsStack_andTheColumnShapedOneStaysInline() {
        XCTAssertEqual(CandidateLayout.horizontal.cellArrangement, .stacked)
        XCTAssertEqual(CandidateLayout.expandable.cellArrangement, .stacked)
        XCTAssertEqual(
            CandidateLayout.vertical.cellArrangement, .inline,
            "the vertical layout aligns every annotation on one x, which needs them inline",
        )
    }

    func testStackedWidth_isTheWiderScriptNotBothTogether() {
        for textSize in CandidateTextSizeChoice.allCases {
            let inline = CandidateMetrics(textSize: textSize, windowSize: .medium)
            let stacked = inline.arranged(.stacked)

            XCTAssertGreaterThanOrEqual(
                stacked.measureWidth(Self.cell),
                2 * stacked.horizontalPadding + stacked.annotationTextWidth(Self.cell.annotation),
                "\(textSize): the wider line is the romanization here, and it must fit",
            )
            XCTAssertLessThan(
                stacked.measureWidth(Self.cell), inline.measureWidth(Self.cell),
                "\(textSize): stacking is what buys the row its extra candidates",
            )
            XCTAssertGreaterThan(
                stacked.itemHeight, inline.itemHeight,
                "\(textSize): and costs the row a second line's height",
            )
        }
    }

    /// The empty-annotation case renders a blank second line rather than
    /// collapsing: a page of cells of two different heights would not line up.
    func testStackedCell_keepsBothLinesInsideItsFrameAtEverySize() {
        for textSize in CandidateTextSizeChoice.allCases {
            for windowSize in CandidateWindowSizeChoice.allCases {
                for content in [Self.cell, CandidateCellContent(text: "候", annotation: nil)] {
                    let metrics = CandidateMetrics(
                        textSize: textSize, windowSize: windowSize, cellArrangement: .stacked,
                    )
                    let view = CandidateItemView(style: .sequoia, metrics: metrics)
                    view.configure(content)
                    view.frame = NSRect(
                        x: 0, y: 0,
                        width: metrics.measureWidth(content), height: metrics.itemHeight,
                    )
                    view.layoutSubtreeIfNeeded()

                    let labels = view.subviews.compactMap { $0 as? NSTextField }
                    XCTAssertEqual(labels.count, 2, "a stacked cell draws both scripts")
                    for label in labels {
                        XCTAssertTrue(
                            view.bounds.contains(label.frame),
                            "\(textSize)/\(windowSize): \(label.stringValue) at \(label.frame) "
                                + "must fit the cell's \(view.bounds)",
                        )
                    }
                    XCTAssertNotEqual(
                        labels[0].frame.minY, labels[1].frame.minY,
                        "\(textSize)/\(windowSize): the two scripts sit on separate lines",
                    )
                }
            }
        }
    }
}
