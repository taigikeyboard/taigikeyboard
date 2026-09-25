// What the size choice resolves to, and the cell arithmetic built on it.

@testable import TaigiInputMethodCore
import XCTest

/// The candidate window's size metrics: the ladder, the scaling, and the
/// width arithmetic every layout packs and aligns by.
final class CandidateMetricsTests: XCTestCase {
    private let defaultMetrics = TestFixtures.defaultCandidateMetrics

    // MARK: - Ladder

    /// Every step, pinned literally. One knob scales the text AND the air:
    /// the paddings are upstream's times `chromeRatio` 0.7 times the text
    /// scale, so the window keeps one proportion at every size.
    ///
    /// trace, scale = size / 16, chrome = 0.7 * scale:
    /// 13 — ann 14*.8125=11.4→11, gap 7*.8125=5.7→6, h 9*.56875=5.1→5,
    ///      v 12*.56875=6.8→7, tahoe 8*.56875=4.55→5, item 13+7=20;
    /// 15 — ann 13.1→13, gap 6.6→7, h 5.9→6, v 7.9→8, tahoe 5.25→5, item 23;
    /// 17 — ann 14.9→15, gap 7.4→7, h 6.7→7, v 8.9→9, tahoe 5.95→6, item 26;
    /// 20 — ann 17.5→18, gap 8.75→9, h 7.9→8, v 10.5→11, tahoe 7, item 31;
    /// 23 — ann 20.1→20, gap 10.1→10, h 9.06→9, v 12.1→12, tahoe 8.05→8,
    ///      item 35.
    func testEveryStep_resolvesToItsTracedValues() {
        let resolved = CandidateSizeChoice.allCases.map { size in
            let metrics = CandidateMetrics(size: size)
            return [
                metrics.candidateFontSize, metrics.annotationFontSize, metrics.candidateAnnotationGap,
                metrics.horizontalPadding, metrics.verticalPadding, metrics.tahoeSeparatorInset,
                metrics.itemHeight,
            ]
        }

        XCTAssertEqual(resolved, [
            [13, 11, 6, 5, 7, 5, 20],
            [15, 13, 7, 6, 8, 5, 23],
            [17, 15, 7, 7, 9, 6, 26],
            [20, 18, 9, 8, 11, 7, 31],
            [23, 20, 10, 9, 12, 8, 35],
        ])
    }

    /// A fresh install renders at Standard — a step smaller than the two-knob
    /// ladder's default of 20pt text in a 30pt row (USER 2026-09-23: the
    /// standard size was still too big).
    func testDefault_isTheStandardStep() {
        XCTAssertEqual(defaultMetrics.size, .standard)
        XCTAssertEqual(defaultMetrics.candidateFontSize, 17)
        XCTAssertEqual(defaultMetrics.itemHeight, 26)
    }

    /// The ladder is declared smallest-first, which is the order the slider
    /// runs in.
    func testLadder_risesWithEveryStep() {
        XCTAssertEqual(CandidateSizeChoice.allCases.map(\.candidateFontSize), [13, 15, 17, 20, 23])
    }

    /// The symbol scaler is anchored at the 16pt reference, which no step
    /// sits on any more: the chevron and page arrows scale with the text.
    func testSymbolMetrics_scaleWithTheText() {
        XCTAssertEqual(CandidateMetrics(size: .large).scaledSymbolMetric(8), 10)
        XCTAssertEqual(CandidateMetrics(size: .extraSmall).scaledSymbolMetric(8), 7)
    }

    // MARK: - One knob

    /// Every step moves the text and the air together — no step may grow one
    /// while shrinking the other, and none may collide with its neighbour.
    func testEachStep_growsTheTextAndTheAirTogether() {
        for (smaller, larger) in zip(CandidateSizeChoice.allCases, CandidateSizeChoice.allCases.dropFirst()) {
            let small = CandidateMetrics(size: smaller)
            let large = CandidateMetrics(size: larger)
            let label = "\(smaller) → \(larger)"

            XCTAssertGreaterThan(large.candidateFontSize, small.candidateFontSize, label)
            XCTAssertGreaterThanOrEqual(large.annotationFontSize, small.annotationFontSize, label)
            XCTAssertGreaterThanOrEqual(large.horizontalPadding, small.horizontalPadding, label)
            XCTAssertGreaterThanOrEqual(large.verticalPadding, small.verticalPadding, label)
            XCTAssertGreaterThan(large.itemHeight, small.itemHeight, label)
        }
    }

    /// Scaled values land on whole points, the way upstream rounds them
    /// (`MacishCandidateItemView.updateFontSize`) — a fractional padding would
    /// put every cell edge on a half pixel.
    func testScaledValues_areWholePoints() {
        for size in CandidateSizeChoice.allCases {
            let metrics = CandidateMetrics(size: size)
            for value in [
                metrics.annotationFontSize, metrics.candidateAnnotationGap,
                metrics.horizontalPadding, metrics.verticalPadding, metrics.tahoeSeparatorInset,
                metrics.scaledSymbolMetric(11), metrics.scaledSymbolMetric(8),
            ] {
                XCTAssertEqual(value, value.rounded(), "\(size) is fractional")
            }
        }
    }

    // MARK: - Panel-cache invalidation

    /// `CandidatePanel` decides whether to rebuild every cached panel by
    /// comparing the metrics it built them at against the current ones, so
    /// each step must resolve to its own value: two steps that compared equal
    /// would leave a panel rendering at the size the user just moved away from.
    func testEveryStep_resolvesToDistinctMetrics() {
        let resolved = CandidateSizeChoice.allCases.map { CandidateMetrics(size: $0) }

        XCTAssertEqual(Set(resolved.map(\.itemHeight)).count, resolved.count)
        for (index, metrics) in resolved.enumerated() {
            XCTAssertFalse(resolved[..<index].contains(metrics), "\(metrics.size) collides")
        }
    }

    // MARK: - Content-resolved height

    /// A stacked layout showing a list with no annotated cell — Romanization Only, or
    /// Hanji with Romanization where every cell is one script — has nothing to stack, and
    /// renders one line tall: the inline metrics, capsule and inset included
    /// (USER 2026-09-02: no second-line air under Romanization Only).
    func testStackedMetrics_withNoAnnotatedCell_resolveToTheOneLineHeight() {
        let stacked = defaultMetrics.arranged(.stacked)

        XCTAssertEqual(stacked.forContent(hasAnnotations: false), defaultMetrics.arranged(.inline))
    }

    /// One annotated cell in the list keeps the two-line height for the whole
    /// list — Hanji–Romanization Pairing's mixed lists (a §34 literal beside Hanji rows) line up.
    func testStackedMetrics_withAnAnnotatedCell_keepTheTwoLineHeight() {
        let stacked = defaultMetrics.arranged(.stacked)

        XCTAssertEqual(stacked.forContent(hasAnnotations: true), stacked)
    }

    /// The inline arrangement is already one line either way.
    func testInlineMetrics_areUnchangedByTheContent() {
        for hasAnnotations in [false, true] {
            XCTAssertEqual(defaultMetrics.forContent(hasAnnotations: hasAnnotations), defaultMetrics)
        }
    }

    // MARK: - Tahoe shape policy

    /// A window of one-line cells keeps upstream's capsule: at 20-35pt those
    /// cells are the size range macOS itself capsules, and the selection sits
    /// a hairline inside it.
    ///
    /// trace: Standard inline → itemHeight 26, container 13, inset 2, highlight 11.
    func testInlineArrangement_keepsTheCapsuleAndItsHairlineInset() {
        let inline = defaultMetrics.arranged(.inline)

        XCTAssertEqual(inline.tahoeContainerCornerRadius, inline.itemHeight / 2)
        XCTAssertEqual(inline.tahoeHighlightInset, 2)
    }

    /// A window of two-line cells rounds to a fixed rectangle instead: the
    /// capsule formula reads as a stadium once a cell is twice a control tall
    /// (USER 2026-08-25). The shape is the arrangement's, not the size's: how
    /// big the window is drawn may not round it differently.
    ///
    /// trace: container 16, inset 4, highlight 12 at every step.
    func testStackedArrangement_roundsToAFixedRectangleAtEveryStep() {
        for size in CandidateSizeChoice.allCases {
            let metrics = CandidateMetrics(size: size, cellArrangement: .stacked)
            let label = "\(size)"

            XCTAssertEqual(metrics.tahoeContainerCornerRadius, 16, label)
            XCTAssertEqual(metrics.tahoeHighlightInset, 4, label)
            XCTAssertEqual(metrics.tahoeHighlightCornerRadius, 12, label)
            XCTAssertLessThan(
                metrics.tahoeContainerCornerRadius, metrics.itemHeight / 2,
                "\(label): a stacked cell that still resolved to a capsule would not have "
                    + "been fixed",
            )
        }
    }

    /// macOS 26 asks nested shapes to be concentric — the inner radius is the
    /// outer one less the padding between them.
    func testHighlightRadius_isTheContainersLessTheInsetEverywhere() {
        for size in CandidateSizeChoice.allCases {
            for arrangement in [CandidateCellArrangement.inline, .stacked] {
                let metrics = CandidateMetrics(size: size, cellArrangement: arrangement)

                XCTAssertEqual(
                    metrics.tahoeContainerCornerRadius - metrics.tahoeHighlightCornerRadius,
                    metrics.tahoeHighlightInset,
                    "\(size)/\(arrangement)",
                )
            }
        }
    }

    /// The one clamp both the window and the selection are drawn through: a
    /// radius past half the shorter side would round a shape wider than the box
    /// it is rounding, which a FIXED radius has no other guard against.
    func testCornerRadiusClamp_holdsARadiusToWhatTheBoxCanRound() {
        let radius = defaultMetrics.arranged(.stacked).tahoeContainerCornerRadius

        // Roomy in both axes: the radius is what the caller asked for.
        XCTAssertEqual(
            CandidateMetrics.cornerRadius(radius, fitting: CGSize(width: 200, height: 57)), radius,
        )
        // Either axis alone can be the binding one.
        XCTAssertEqual(
            CandidateMetrics.cornerRadius(radius, fitting: CGSize(width: 10, height: 57)), 5,
        )
        XCTAssertEqual(
            CandidateMetrics.cornerRadius(radius, fitting: CGSize(width: 200, height: 24)), 12,
        )
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

        XCTAssertEqual(measured, ceil(TestFixtures.defaultFontWidth(of: text, size: defaultMetrics.annotationFontSize)))
        XCTAssertNotEqual(measured, ceil(TestFixtures.defaultFontWidth(of: text, size: defaultMetrics.candidateFontSize)))
    }

    /// The digit slot, the gap and each padding are charged once — a cell that
    /// double-counted any of them would push a column off every page.
    @MainActor
    func testMeasuredWidth_isThePaddingsPlusBothColumnsExactlyOnce() throws {
        let cell = CandidateCellContent(text: "tâi-gí khí-puânn", annotation: "台語齒盤")
        let annotation = try XCTUnwrap(cell.annotation)

        let expected = defaultMetrics.horizontalPadding
            + defaultMetrics.indexWidth
            + defaultMetrics.indexCandidateGap
            + defaultMetrics.measurePrimaryWidth(cell.text)
            + defaultMetrics.candidateAnnotationGap
            + ceil(TestFixtures.defaultFontWidth(of: annotation, size: defaultMetrics.annotationFontSize))
            + defaultMetrics.horizontalPadding

        XCTAssertEqual(defaultMetrics.measureWidth(cell), expected, accuracy: 0.01)
    }

    /// `baseWidth` is the packing budget's unit, so it must stay the width of
    /// the narrowest cell there is — one full-width glyph, no annotation.
    @MainActor
    func testBaseWidth_isTheNarrowestPrimaryOnlyCell() {
        for size in CandidateSizeChoice.allCases {
            let metrics = CandidateMetrics(size: size)

            XCTAssertEqual(
                metrics.baseWidth,
                metrics.measureWidth(CandidateCellContent(text: "永", annotation: nil)),
                accuracy: 0.01,
                "\(size)",
            )
        }
    }

    /// The one-glyph floor is measured at the candidate font, so it has to
    /// grow with the size — a floor cached across sizes would leave a large
    /// window packing columns sized for a small one.
    @MainActor
    func testBaseWidth_growsWithTheSize() {
        let widths = CandidateSizeChoice.allCases.map { CandidateMetrics(size: $0).baseWidth }

        XCTAssertEqual(widths, widths.sorted())
        XCTAssertEqual(Set(widths).count, widths.count)
    }

    /// The digit hint scales with the size, like the other text-anchored
    /// distances — a hint that stayed 10pt beside 23pt candidates would read
    /// as a speck. At the smallest step it is upstream's own 8pt.
    @MainActor
    func testIndexColumn_scalesWithTheSize() {
        let small = CandidateMetrics(size: .small)
        let large = CandidateMetrics(size: .extraLarge)

        XCTAssertGreaterThan(large.indexFontSize, small.indexFontSize)
        XCTAssertGreaterThan(large.indexWidth, small.indexWidth)
        XCTAssertEqual(CandidateMetrics(size: .extraSmall).indexFontSize, 8)
        // The slot holds the WIDEST form the key can take — `⌥9`, not `9` —
        // so the column keeps one width as the live key changes.
        let widest = CandidateIndexLabel.widestLabelForms
            .map { ceil(($0 as NSString).size(withAttributes: [.font: small.indexFont]).width) }
            .max() ?? 0
        XCTAssertEqual(small.indexWidth, widest + 2)
        XCTAssertGreaterThan(
            small.indexWidth,
            ceil(("9" as NSString).size(withAttributes: [.font: small.indexFont]).width) + 2,
            "a bare digit alone would leave no room for the chord",
        )
        XCTAssertEqual(small.indexColumnWidth, small.indexWidth + small.indexCandidateGap)
    }

    /// The slot is charged to the narrowest cell there is, because every cell
    /// reserves it whether or not a digit is drawn in it — a packer that
    /// budgeted without it would fit a column the window cannot render.
    @MainActor
    func testBaseWidth_chargesTheDigitSlot() {
        for arrangement in [CandidateCellArrangement.inline, .stacked] {
            let metrics = defaultMetrics.arranged(arrangement)

            XCTAssertEqual(
                metrics.baseWidth,
                2 * metrics.horizontalPadding + metrics.indexColumnWidth + metrics.primaryColumnFloor,
                accuracy: 0.01,
                "\(arrangement)",
            )
        }
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
