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

    /// A scalar none of the bundled faces carries, so a cell containing it is
    /// laid out in a cascaded fallback face rather than the chosen one. Pinned
    /// by `testTheCascadeProbe_isUncoveredByEveryBundledFace` — every Taigi
    /// glyph this project renders IS covered, so the probe has to come from
    /// outside the repertoire for the fallback case to be exercised at all.
    private static let cascadeProbe = "😀"

    func testTheCascadeProbe_isUncoveredByEveryBundledFace() {
        XCTAssertEqual(TestFixtures.unregisterableFontFiles, [])
        let probe = Self.cascadeProbe.unicodeScalars.first!

        for choice in CandidateFontChoice.allCases where choice != .system {
            XCTAssertFalse(
                choice.font(ofSize: 20).coveredCharacterSet.contains(probe),
                "\(choice) covers the probe — the cascade case below would not cascade",
            )
        }
    }

    /// A stacked cell's height is fixed at construction, so it has to be the
    /// CHOSEN face's line box — including for text the face has no glyph for,
    /// which the text system lays out in a cascaded fallback of its own metrics.
    func testStackedCell_holdsBothLinesInEveryFace() {
        XCTAssertEqual(TestFixtures.unregisterableFontFiles, [])
        let contents = [
            Self.cell,
            // The cascade cases: a fallback face on each line in turn.
            CandidateCellContent(text: Self.cascadeProbe, annotation: "hāu-suán"),
            CandidateCellContent(text: "候選", annotation: Self.cascadeProbe),
        ]
        for choice in CandidateFontChoice.allCases {
            let metrics = CandidateMetrics(
                textSize: .medium, windowSize: .medium, fontChoice: choice,
                cellArrangement: .stacked,
            )
            for content in contents {
                let view = CandidateItemView(style: .sequoia, metrics: metrics)
                view.configure(content)
                view.frame = NSRect(
                    x: 0, y: 0,
                    width: metrics.measureWidth(content), height: metrics.itemHeight,
                )
                view.layoutSubtreeIfNeeded()

                for label in view.subviews.compactMap({ $0 as? NSTextField }) {
                    XCTAssertTrue(
                        view.bounds.contains(label.frame),
                        "\(choice)/\(content.text): \(label.frame) must fit \(view.bounds)",
                    )
                }
            }
        }
    }

    /// The Tahoe highlight renders at the metrics' shape policy: inset inside
    /// the cell, and rounded concentrically with the window rather than as a
    /// capsule of its own height — which is what a two-line cell used to draw
    /// (USER 2026-08-25: the digit hint looked like it fell outside the
    /// selection).
    func testStackedTahoeCell_drawsTheConcentricHighlightNotACapsule() throws {
        let metrics = TestFixtures.defaultCandidateMetrics.arranged(.stacked)
        let view = CandidateItemView(style: .tahoe, metrics: metrics)
        view.configure(Self.cell)
        view.frame = NSRect(
            x: 0, y: 0, width: metrics.measureWidth(Self.cell), height: metrics.itemHeight,
        )
        view.isHighlighted = true
        view.layoutSubtreeIfNeeded()

        let highlight = try XCTUnwrap(view.subviews.first { !($0 is NSTextField) })
        XCTAssertEqual(highlight.frame, view.bounds.insetBy(dx: 4, dy: 4))
        XCTAssertEqual(highlight.layer?.cornerRadius, metrics.tahoeHighlightCornerRadius)
    }

    /// A cell narrower than twice the radius rounds to what it can hold: the
    /// concentric radius is a fixed number, so the cell it lands in is what
    /// bounds it (`CandidateMetrics.cornerRadius(_:fitting:)`).
    func testNarrowTahoeCell_roundsToWhatItCanHold() throws {
        let metrics = TestFixtures.defaultCandidateMetrics.arranged(.stacked)
        let view = CandidateItemView(style: .tahoe, metrics: metrics)
        view.configure(Self.cell)
        // Narrower than the layouts ever pack, so the width is the binding axis.
        view.frame = NSRect(x: 0, y: 0, width: 20, height: metrics.itemHeight)
        view.isHighlighted = true
        view.layoutSubtreeIfNeeded()

        let highlight = try XCTUnwrap(view.subviews.first { !($0 is NSTextField) })
        XCTAssertEqual(highlight.layer?.cornerRadius, (20 - 2 * 4) / 2)
        XCTAssertLessThan(
            try XCTUnwrap(highlight.layer?.cornerRadius), metrics.tahoeHighlightCornerRadius,
        )
    }

    // MARK: - One-script lists render one line tall

    private static let oneScriptCells = [
        CandidateCellContent(text: "候選", annotation: nil),
        CandidateCellContent(text: "hāu-suán", annotation: nil),
    ]
    private static let twoScriptCells = [cell, CandidateCellContent(text: "候", annotation: nil)]

    /// The row-shaped layouts drop to the one-line height when no cell in the
    /// list carries an annotation — 羅馬字, or 漢羅合用's one-script cells — and
    /// come back to two lines the moment one does (USER 2026-09-02). Through
    /// the two entry points the shared panel hands cells over by, `layout`
    /// and `rerender`, so a mode change under an open window reflows it. The
    /// vertical layout is inline already and is left as it was.
    func testStackedPanels_renderAListWithNoAnnotationOneLineTall_andReflowOnRerender() {
        let caret = CGRect(x: 120, y: 400, width: 1, height: 18)
        for panel in TestFixtures.candidatePanels() {
            let configured = panel.configuredMetrics
            let oneLine = configured.arranged(.inline).itemHeight
            let label = String(describing: type(of: panel))

            let size = panel.layout(Self.oneScriptCells, forCaret: caret)

            XCTAssertEqual(panel.metrics.itemHeight, oneLine, label)
            if configured.cellArrangement == .stacked {
                XCTAssertEqual(size.height, oneLine, "\(label): the window is one row of one-line cells")
            }
            for cell in TestFixtures.candidateCells(in: panel) {
                XCTAssertEqual(cell.frame.height, oneLine, label)
            }

            panel.rerender(Self.twoScriptCells)

            XCTAssertEqual(panel.metrics, configured, "\(label): an annotated cell brings the configured height back")
            for cell in TestFixtures.candidateCells(in: panel) {
                XCTAssertEqual(cell.frame.height, configured.itemHeight, label)
            }

            panel.rerender(Self.oneScriptCells)

            XCTAssertEqual(panel.metrics.itemHeight, oneLine, "\(label): and a one-script list drops it again")
            panel.clear()
        }
    }

    /// The configured metrics — what the panel cache compares against — never
    /// move with the content: a one-script list must not make
    /// `CandidatePanel.panel(for:)` rebuild the window on the next keystroke.
    func testContentResolution_leavesTheConfiguredMetricsAlone() {
        let panel = HorizontalCandidatePanel(
            style: .sequoia, metrics: TestFixtures.defaultCandidateMetrics.arranged(.stacked),
        )

        _ = panel.layout(Self.oneScriptCells, forCaret: .zero)

        XCTAssertEqual(panel.configuredMetrics, TestFixtures.defaultCandidateMetrics.arranged(.stacked))
        XCTAssertNotEqual(panel.metrics, panel.configuredMetrics)
        panel.clear()
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
                    XCTAssertEqual(
                        labels.count, 3, "a stacked cell draws the digit and both scripts",
                    )
                    for label in labels {
                        XCTAssertTrue(
                            view.bounds.contains(label.frame),
                            "\(textSize)/\(windowSize): \(label.stringValue) at \(label.frame) "
                                + "must fit the cell's \(view.bounds)",
                        )
                    }
                    // Indices 1 and 2: the digit hint is the cell's FIRST text
                    // field, and it shares neither line — the two scripts are
                    // the pair this asserts about.
                    XCTAssertNotEqual(
                        labels[1].frame.minY, labels[2].frame.minY,
                        "\(textSize)/\(windowSize): the two scripts sit on separate lines",
                    )
                }
            }
        }
    }
}
