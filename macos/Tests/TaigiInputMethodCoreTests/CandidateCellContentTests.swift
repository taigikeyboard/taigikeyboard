// Which script a cell leads with, and how wide the cell that holds both is.

@testable import TaigiInputMethodCore
import XCTest

/// The display mapping: a candidate is a `(漢字, 羅馬字)` pair, and the cell
/// shows both — the swap setting decides which one leads.
final class CandidateCellContentTests: XCTestCase {
    private func candidate(roman: String, hanji: String?) -> ContinuousCandidate {
        ContinuousCandidate(
            consumedSpanStart: 0,
            consumedSpanEnd: 5,
            syllableCount: 2,
            displayText: hanji ?? roman,
            score: 1,
            form: 0,
            mode: .unspecified,
            roman: roman,
            hanji: hanji,
            canonicalTl: roman,
        )
    }

    private func settings(swapped: Bool, bothScripts: Bool = false) -> EngineSettings {
        let defaults = EngineSettings.defaults
        return EngineSettings(
            inputMode: defaults.inputMode,
            isTranslateSwapped: swapped,
            isOutputBothScripts: bothScripts,
            isLiteralRomanCandidateEnabled: defaults.isLiteralRomanCandidateEnabled,
            isFrequencyRecordingEnabled: defaults.isFrequencyRecordingEnabled,
            isAssociationRecordingEnabled: defaults.isAssociationRecordingEnabled,
            isCustomDictEnabled: defaults.isCustomDictEnabled,
            dictionarySources: defaults.dictionarySources,
        )
    }

    // MARK: - Mapping

    func testUnswapped_leadsWithRomanizationAndAnnotatesWithHanji() {
        let cell = CandidateCellContent.cell(
            for: candidate(roman: "tâi-gí", hanji: "台語"),
            settings: settings(swapped: false),
        )

        XCTAssertEqual(cell.text, "tâi-gí")
        XCTAssertEqual(cell.annotation, "台語")
    }

    func testSwapped_leadsWithHanjiAndAnnotatesWithRomanization() {
        let cell = CandidateCellContent.cell(
            for: candidate(roman: "tâi-gí", hanji: "台語"),
            settings: settings(swapped: true),
        )

        XCTAssertEqual(cell.text, "台語")
        XCTAssertEqual(cell.annotation, "tâi-gí")
    }

    /// A romanization-only candidate has no second script in EITHER direction —
    /// the same case `CandidateDocumentText` answers with the bare romanization.
    func testRomanizationOnlyCandidate_hasNoAnnotationInEitherDirection() {
        for swapped in [false, true] {
            let cell = CandidateCellContent.cell(
                for: candidate(roman: "guá", hanji: nil),
                settings: settings(swapped: swapped),
            )

            XCTAssertEqual(cell.text, "guá")
            XCTAssertNil(cell.annotation, "swapped=\(swapped) invented a second script")
        }
    }

    /// A producer that emits `""` for "no Hanji" means what omitting it means;
    /// the layout must not reserve annotation width for the difference.
    func testEmptyHanji_readsTheSameAsAnAbsentOne() {
        let cell = CandidateCellContent.cell(
            for: candidate(roman: "guá", hanji: ""),
            settings: settings(swapped: false),
        )

        XCTAssertEqual(cell, CandidateCellContent(text: "guá", annotation: nil))
    }

    func testEmptyAnnotationString_normalizesToNil() {
        XCTAssertNil(CandidateCellContent(text: "guá", annotation: "").annotation)
    }

    /// The cell and the commit resolve the same settings snapshot, so the cell
    /// always LEADS with the script the document gets first — including the
    /// both-scripts rendering, where the document additionally brackets the
    /// other one and the cell keeps it in its own column.
    func testCellPrimary_leadsWithWhateverTheDocumentLeadsWith() {
        let word = candidate(roman: "tâi-gí", hanji: "台語")

        for swapped in [false, true] {
            for bothScripts in [false, true] {
                let settings = settings(swapped: swapped, bothScripts: bothScripts)
                let cell = CandidateCellContent.cell(for: word, settings: settings)
                let document = CandidateDocumentText.text(for: word, settings: settings)

                XCTAssertTrue(
                    document.hasPrefix(cell.text),
                    "swapped=\(swapped) bothScripts=\(bothScripts): document \"\(document)\" "
                        + "does not lead with the cell's \"\(cell.text)\"",
                )
            }
        }
    }

    /// With both scripts off — the shipped default — the two are the same
    /// string, which is what the end-to-end commit tests assert on.
    func testCellPrimary_isExactlyTheDocumentTextUnderTheDefaults() {
        let word = candidate(roman: "tâi-gí", hanji: "台語")

        for swapped in [false, true] {
            let settings = settings(swapped: swapped)

            XCTAssertEqual(
                CandidateCellContent.cell(for: word, settings: settings).text,
                CandidateDocumentText.text(for: word, settings: settings),
            )
        }
    }

    // MARK: - Measurement

    @MainActor
    func testAnnotation_widensTheCell() {
        let bare = CandidateItemView.measureWidth(CandidateCellContent(text: "tâi-gí", annotation: nil))
        let annotated = CandidateItemView.measureWidth(
            CandidateCellContent(text: "tâi-gí", annotation: "台語"),
        )

        XCTAssertGreaterThan(annotated, bare)
    }

    /// The gap is charged only when there is something to separate — an absent
    /// annotation must cost the cell nothing at all.
    @MainActor
    func testAbsentAnnotation_costsNoWidth() {
        XCTAssertEqual(CandidateItemView.annotationWidth(nil), 0)
        XCTAssertEqual(CandidateItemView.annotationWidth(""), 0)
    }

    /// The annotation is measured at ITS font, not the candidate's — measuring
    /// at 16pt would over-reserve on every two-script cell.
    @MainActor
    func testAnnotation_isMeasuredAtTheAnnotationFont() {
        let text = "台語"
        let measured = CandidateItemView.annotationWidth(text)
            - CandidateItemView.Metrics.candidateAnnotationGap

        XCTAssertEqual(measured, ceil(width(of: text, size: CandidateItemView.Metrics.annotationFontSize)))
        XCTAssertNotEqual(measured, ceil(width(of: text, size: CandidateItemView.Metrics.candidateFontSize)))
    }

    /// The gap and each padding are charged once — a cell that double-counted
    /// any of them would push a column off every page.
    @MainActor
    func testMeasuredWidth_isThePaddingsPlusBothColumnsExactlyOnce() throws {
        let cell = CandidateCellContent(text: "tâi-gí khí-puânn", annotation: "台語齒盤")
        let annotation = try XCTUnwrap(cell.annotation)

        let expected = CandidateItemView.Metrics.horizontalPadding
            + CandidateItemView.measurePrimaryWidth(cell.text)
            + CandidateItemView.Metrics.candidateAnnotationGap
            + ceil(width(of: annotation, size: CandidateItemView.Metrics.annotationFontSize))
            + CandidateItemView.Metrics.horizontalPadding

        XCTAssertEqual(CandidateItemView.measureWidth(cell), expected, accuracy: 0.01)
    }

    /// `baseWidth` is the packing budget's unit, so it must stay the width of
    /// the narrowest cell there is — one full-width glyph, no annotation.
    @MainActor
    func testBaseWidth_isTheNarrowestPrimaryOnlyCell() {
        XCTAssertEqual(
            CandidateItemView.baseWidth,
            CandidateItemView.measureWidth(CandidateCellContent(text: "永", annotation: nil)),
            accuracy: 0.01,
        )
    }

    // MARK: - Cell reconfiguration

    /// Cells are recycled across pages and across the vertical layout's
    /// renumbering, so an annotation that goes away must give its width back —
    /// and coming back must take it again.
    @MainActor
    func testCellReconfiguration_tracksTheAnnotationBothWays() {
        let item = CandidateItemView(style: .sequoia)
        let annotated = CandidateCellContent(text: "tâi-gí", annotation: "台語")
        let bare = CandidateCellContent(text: "guá", annotation: nil)

        item.configure(annotated)
        let withAnnotation = item.fittingSize.width
        item.configure(bare)
        let withoutAnnotation = item.fittingSize.width
        item.configure(annotated)
        let withAnnotationAgain = item.fittingSize.width

        XCTAssertLessThan(withoutAnnotation, withAnnotation)
        XCTAssertEqual(withAnnotationAgain, withAnnotation, accuracy: 0.01)
    }

    /// The candidate column can be widened for column alignment, and never
    /// shrinks below the one-glyph floor.
    @MainActor
    func testPrimaryColumnWidth_widensTheCellAndFloorsAtOneGlyph() {
        let item = CandidateItemView(style: .sequoia)
        item.configure(CandidateCellContent(text: "guá", annotation: "我"))
        let natural = item.fittingSize.width

        item.setPrimaryColumnWidth(natural + 40)

        XCTAssertGreaterThan(item.fittingSize.width, natural)

        item.setPrimaryColumnWidth(0)

        XCTAssertEqual(item.fittingSize.width, natural, accuracy: 0.01)
    }

    // MARK: - Clamped cells

    /// A candidate longer than the cell it is clamped into truncates rather
    /// than breaking the cell's geometry: both the horizontal packer's row
    /// limit and the vertical window's column cap hand a cell less width than
    /// its text wants, and neither may push the label past the cell's edge.
    @MainActor
    func testCellClampedNarrowerThanItsText_keepsItsLabelsInside() {
        let item = CandidateItemView(style: .sequoia)
        let cell = CandidateCellContent(
            text: "tâi-gí khí-puânn tsin hó-sè", annotation: "台語齒盤真好勢",
        )
        item.configure(cell)
        // Deliberately narrower than `measureWidth` wants — the clamp the
        // packer applies to an oversized candidate. Hosted in a container the
        // way the panels host their cells, because a detached view never runs
        // the layout pass that applies the frame to its subviews.
        let clampedWidth = CandidateItemView.baseWidth * 2
        let container = FlippedContainerView(frame: NSRect(
            x: 0, y: 0, width: clampedWidth, height: CandidateItemView.Metrics.itemHeight,
        ))
        container.addSubview(item)
        item.frame = container.bounds
        container.layoutSubtreeIfNeeded()

        let labels = item.subviews.compactMap { $0 as? NSTextField }
        XCTAssertEqual(labels.count, 2, "the cell lays out a candidate and an annotation")
        for label in labels {
            // Non-zero first, so a layout pass that never ran cannot pass this
            // case by leaving every frame at the origin.
            XCTAssertGreaterThan(label.frame.width, 0, "\"\(label.stringValue)\" was never laid out")
            XCTAssertLessThanOrEqual(
                label.frame.maxX, clampedWidth + 0.01,
                "\"\(label.stringValue)\" runs past the clamped cell's edge",
            )
        }
    }

    /// The aligned column can never ask for more than the cell holds — the
    /// vertical layout clamps to this before handing rows their column width.
    @MainActor
    func testMaximumPrimaryColumnWidth_leavesTheChromeAndTrailingTheirRoom() {
        let cellWidth = CandidateItemView.baseWidth * 3
        let trailing = CandidateItemView.Metrics.horizontalPadding

        let cap = CandidateItemView.maximumPrimaryColumnWidth(
            inCellWidth: cellWidth, trailingInset: trailing,
        )

        // trace: baseWidth = horizontalPadding + one-glyph floor +
        // horizontalPadding, so everything outside the candidate column is
        // `baseWidth - floor`, and the cap is what a cell has left after it.
        let oneGlyphFloor = max(
            CandidateItemView.Metrics.candidateFontSize,
            CandidateItemView.measurePrimaryWidth("永"),
        )
        XCTAssertEqual(
            cap, cellWidth - (CandidateItemView.baseWidth - oneGlyphFloor), accuracy: 0.01,
        )
        // A cell too narrow for even one glyph still floors at one, rather than
        // answering with a negative column.
        XCTAssertEqual(
            CandidateItemView.maximumPrimaryColumnWidth(inCellWidth: 1, trailingInset: trailing),
            CandidateItemView.Metrics.candidateFontSize,
        )
    }

    @MainActor
    private func width(of text: String, size: CGFloat) -> CGFloat {
        (text as NSString).size(withAttributes: [.font: NSFont.systemFont(ofSize: size)]).width
    }
}
