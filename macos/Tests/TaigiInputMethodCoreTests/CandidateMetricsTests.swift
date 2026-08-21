// What the two size choices resolve to, and the cell arithmetic built on them.

@testable import TaigiInputMethodCore
import XCTest

/// The candidate window's size metrics: the ladders, the scaling, and the
/// width arithmetic every layout packs and aligns by.
final class CandidateMetricsTests: XCTestCase {
    /// The smallest pair of choices — no longer the port's original geometry:
    /// the whitespace rebalance put the whole chrome ladder at or below
    /// upstream's air (USER 2026-08-21).
    private let originalMetrics = CandidateMetrics(textSize: .small, windowSize: .small)
    private let defaultMetrics = TestFixtures.defaultCandidateMetrics

    // MARK: - Ladders

    /// The smallest tier, pinned literally. The 16pt reference font survives
    /// (the symbol scaler's identity anchor); the paddings sit BELOW the
    /// port's originals since the whitespace rebalance.
    ///
    /// trace: chrome 0.7 → h 9*0.7=6.3→6, v 12*0.7=8.4→8, tahoe 8*0.7=5.6→6;
    /// itemHeight 16+8=24.
    func testSmallestChoices_resolveToTheirLiterals() {
        XCTAssertEqual(originalMetrics.candidateFontSize, 16)
        XCTAssertEqual(originalMetrics.annotationFontSize, 14)
        XCTAssertEqual(originalMetrics.candidateAnnotationGap, 7)
        XCTAssertEqual(originalMetrics.horizontalPadding, 6)
        XCTAssertEqual(originalMetrics.verticalPadding, 8)
        XCTAssertEqual(originalMetrics.tahoeSeparatorInset, 6)
        XCTAssertEqual(originalMetrics.itemHeight, 24)
        // At the reference font the symbol scaler is the identity, so the
        // chevron and page arrows keep the point sizes the port shipped with.
        XCTAssertEqual(originalMetrics.scaledSymbolMetric(11), 11)
        XCTAssertEqual(originalMetrics.scaledSymbolMetric(8), 8)
    }

    /// The default spends its points on the glyphs (USER 2026-08-21: bigger
    /// text, less whitespace): the font is well above the 16pt reference while
    /// the paddings sit below upstream's 9/12 originals.
    ///
    /// trace: 中/中 → font 20, ann 14*1.25=17.5→18, gap 7*1.25=8.75→9;
    /// chrome 0.85 → h 9*0.85=7.65→8, v 12*0.85=10.2→10; itemHeight 30.
    func testDefaultChoices_areTextForward() {
        XCTAssertEqual(defaultMetrics.candidateFontSize, 20)
        XCTAssertEqual(defaultMetrics.annotationFontSize, 18)
        XCTAssertEqual(defaultMetrics.candidateAnnotationGap, 9)
        XCTAssertEqual(defaultMetrics.horizontalPadding, 8)
        XCTAssertEqual(defaultMetrics.verticalPadding, 10)
        XCTAssertEqual(defaultMetrics.itemHeight, 30)
    }

    /// The ladders are declared smallest-first, which is the order the pickers
    /// list them in. The largest chrome step is upstream MacishType's original
    /// air — nothing renders roomier than the port did.
    func testLadders_riseWithEveryStep() {
        XCTAssertEqual(CandidateTextSizeChoice.allCases.map(\.candidateFontSize), [16, 20, 23])
        XCTAssertEqual(CandidateWindowSizeChoice.allCases.map(\.chromeScale), [0.7, 0.85, 1.0])
    }

    // MARK: - Which knob owns which metric

    /// The text knob owns the fonts and the gap between the two scripts; the
    /// paddings are the window knob's. Mixing them would make one knob move
    /// the other's geometry.
    func testTextChoice_scalesTheFontsAndTheGapButNotThePaddings() {
        let larger = CandidateMetrics(textSize: .large, windowSize: .small)

        XCTAssertGreaterThan(larger.candidateFontSize, originalMetrics.candidateFontSize)
        XCTAssertGreaterThan(larger.annotationFontSize, originalMetrics.annotationFontSize)
        XCTAssertGreaterThan(larger.candidateAnnotationGap, originalMetrics.candidateAnnotationGap)
        XCTAssertGreaterThan(larger.scaledSymbolMetric(11), originalMetrics.scaledSymbolMetric(11))
        XCTAssertEqual(larger.horizontalPadding, originalMetrics.horizontalPadding)
        XCTAssertEqual(larger.verticalPadding, originalMetrics.verticalPadding)
        XCTAssertEqual(larger.tahoeSeparatorInset, originalMetrics.tahoeSeparatorInset)
    }

    func testWindowChoice_scalesThePaddingsButNotTheFonts() {
        let larger = CandidateMetrics(textSize: .small, windowSize: .large)

        XCTAssertGreaterThan(larger.horizontalPadding, originalMetrics.horizontalPadding)
        XCTAssertGreaterThan(larger.verticalPadding, originalMetrics.verticalPadding)
        XCTAssertGreaterThan(larger.tahoeSeparatorInset, originalMetrics.tahoeSeparatorInset)
        XCTAssertEqual(larger.candidateFontSize, originalMetrics.candidateFontSize)
        XCTAssertEqual(larger.annotationFontSize, originalMetrics.annotationFontSize)
        XCTAssertEqual(larger.candidateAnnotationGap, originalMetrics.candidateAnnotationGap)
        XCTAssertEqual(larger.scaledSymbolMetric(11), originalMetrics.scaledSymbolMetric(11))
    }

    /// Scaled values land on whole points, the way upstream rounds them
    /// (`MacishCandidateItemView.updateFontSize`) — a fractional padding would
    /// put every cell edge on a half pixel.
    func testScaledValues_areWholePoints() {
        for textSize in CandidateTextSizeChoice.allCases {
            for windowSize in CandidateWindowSizeChoice.allCases {
                let metrics = CandidateMetrics(textSize: textSize, windowSize: windowSize)
                for value in [
                    metrics.annotationFontSize, metrics.candidateAnnotationGap,
                    metrics.horizontalPadding, metrics.verticalPadding, metrics.tahoeSeparatorInset,
                    metrics.scaledSymbolMetric(11), metrics.scaledSymbolMetric(8),
                ] {
                    XCTAssertEqual(value, value.rounded(), "\(textSize)/\(windowSize) is fractional")
                }
            }
        }
    }

    // MARK: - Panel-cache invalidation

    /// `CandidatePanel` decides whether to rebuild every cached panel by
    /// comparing the metrics it built them at against the current ones, so
    /// each pair of choices must resolve to its own value: two choices that
    /// compared equal would leave a panel rendering at the size the user just
    /// moved away from.
    func testEveryChoicePair_resolvesToDistinctMetrics() {
        var seen: [CandidateMetrics] = []
        for textSize in CandidateTextSizeChoice.allCases {
            for windowSize in CandidateWindowSizeChoice.allCases {
                let metrics = CandidateMetrics(textSize: textSize, windowSize: windowSize)
                XCTAssertFalse(seen.contains(metrics), "\(textSize)/\(windowSize) collides")
                seen.append(metrics)
            }
        }
    }

    // MARK: - Measurement

    @MainActor
    func testAnnotation_widensTheCell() {
        let bare = defaultMetrics.measureWidth(CandidateCellContent(text: "tâi-gí", annotation: nil))
        let annotated = defaultMetrics.measureWidth(
            CandidateCellContent(text: "tâi-gí", annotation: "台語"),
        )

        XCTAssertGreaterThan(annotated, bare)
    }

    /// The gap is charged only when there is something to separate — an absent
    /// annotation must cost the cell nothing at all.
    @MainActor
    func testAbsentAnnotation_costsNoWidth() {
        XCTAssertEqual(defaultMetrics.annotationWidth(nil), 0)
        XCTAssertEqual(defaultMetrics.annotationWidth(""), 0)
    }

    /// The annotation is measured at ITS font, not the candidate's — measuring
    /// at the candidate font would over-reserve on every two-script cell.
    @MainActor
    func testAnnotation_isMeasuredAtTheAnnotationFont() {
        let text = "台語"
        let measured = defaultMetrics.annotationWidth(text) - defaultMetrics.candidateAnnotationGap

        XCTAssertEqual(measured, ceil(TestFixtures.systemFontWidth(of: text, size: defaultMetrics.annotationFontSize)))
        XCTAssertNotEqual(measured, ceil(TestFixtures.systemFontWidth(of: text, size: defaultMetrics.candidateFontSize)))
    }

    /// The gap and each padding are charged once — a cell that double-counted
    /// any of them would push a column off every page.
    @MainActor
    func testMeasuredWidth_isThePaddingsPlusBothColumnsExactlyOnce() throws {
        let cell = CandidateCellContent(text: "tâi-gí khí-puânn", annotation: "台語齒盤")
        let annotation = try XCTUnwrap(cell.annotation)

        let expected = defaultMetrics.horizontalPadding
            + defaultMetrics.measurePrimaryWidth(cell.text)
            + defaultMetrics.candidateAnnotationGap
            + ceil(TestFixtures.systemFontWidth(of: annotation, size: defaultMetrics.annotationFontSize))
            + defaultMetrics.horizontalPadding

        XCTAssertEqual(defaultMetrics.measureWidth(cell), expected, accuracy: 0.01)
    }

    /// `baseWidth` is the packing budget's unit, so it must stay the width of
    /// the narrowest cell there is — one full-width glyph, no annotation.
    @MainActor
    func testBaseWidth_isTheNarrowestPrimaryOnlyCell() {
        for textSize in CandidateTextSizeChoice.allCases {
            let metrics = CandidateMetrics(textSize: textSize, windowSize: .medium)

            XCTAssertEqual(
                metrics.baseWidth,
                metrics.measureWidth(CandidateCellContent(text: "永", annotation: nil)),
                accuracy: 0.01,
                "\(textSize)",
            )
        }
    }

    /// The one-glyph floor is measured at the candidate font, so it has to
    /// grow with the text choice — a floor cached across sizes would leave a
    /// large window packing columns sized for a small one.
    @MainActor
    func testBaseWidth_growsWithBothChoices() {
        let original = originalMetrics.baseWidth

        XCTAssertGreaterThan(
            CandidateMetrics(textSize: .large, windowSize: .small).baseWidth, original,
        )
        XCTAssertGreaterThan(
            CandidateMetrics(textSize: .small, windowSize: .large).baseWidth, original,
        )
    }

    /// The aligned column can never ask for more than the cell holds — the
    /// vertical layout clamps to this before handing rows their column width.
    @MainActor
    func testMaximumPrimaryColumnWidth_leavesTheChromeAndTrailingTheirRoom() {
        let metrics = defaultMetrics
        let cellWidth = metrics.baseWidth * 3
        let trailing = metrics.horizontalPadding

        let cap = metrics.maximumPrimaryColumnWidth(inCellWidth: cellWidth, trailingInset: trailing)

        // trace: baseWidth = horizontalPadding + one-glyph floor +
        // horizontalPadding, so everything outside the candidate column is
        // `baseWidth - floor`, and the cap is what a cell has left after it.
        XCTAssertEqual(
            cap, cellWidth - (metrics.baseWidth - metrics.primaryColumnFloor), accuracy: 0.01,
        )
        // A cell too narrow for even one glyph still floors at one, rather than
        // answering with a negative column.
        XCTAssertEqual(
            metrics.maximumPrimaryColumnWidth(inCellWidth: 1, trailingInset: trailing),
            metrics.candidateFontSize,
        )
    }
}
