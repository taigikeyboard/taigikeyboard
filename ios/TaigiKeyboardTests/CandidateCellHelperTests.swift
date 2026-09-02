// Pins the candidate cell arm order (TPS → romanOnly → combined → swapped → default)
// for `CandidateCellHelper.displayTitle` / `displaySubtitle` — the platform half of
// 候選詞顯示 = 羅馬字 (the engine half collapses same-roman rows) and of
// 候選詞顯示 = 漢羅濫 (§42 split cells: every cell is single-script, split upstream
// in `TaigiAutocompleteService.buildContinuousSuggestions`), plus the content-level
// subtitle-space flag and the marked-cell no-op in `suggestionToHandle`.

@testable import TaigiKeyboard
import KeyboardKit
import XCTest

final class CandidateCellHelperTests: XCTestCase {
    private let dual = AutocompleteSuggestion(text: "tâi-gí", title: "tâi-gí", subtitle: "台語")
    private let romanOnlyRow = AutocompleteSuggestion(text: "tâi", title: "tâi", subtitle: nil)

    /// §42 split cells as `buildContinuousSuggestions` emits them under 濫:
    /// single-script `text`, `subtitle = nil`, `cellScript` marker.
    private let markedHanjiCell = AutocompleteSuggestion(
        text: "台語",
        title: "台語",
        subtitle: nil,
        additionalInfo: ["isContinuous": "true", "cellScript": "hanji", "roman": "tâi-gí"],
    )
    private let markedRomanCell = AutocompleteSuggestion(
        text: "tâi-gí",
        title: "tâi-gí",
        subtitle: nil,
        additionalInfo: ["isContinuous": "true", "cellScript": "roman", "hanji": "台語"],
    )

    // MARK: - romanOnly arm

    func testDisplayTitle_romanOnly_showsRomanRegardlessOfSwap() {
        for swapped in [false, true] {
            let title = CandidateCellHelper.displayTitle(
                for: dual,
                isTranslateSwapped: swapped,
                isTPSLayout: false,
                orMapsToER: false,
                candidateDisplayMode: .romanOnly,
            )
            XCTAssertEqual(title, "tâi-gí", "romanOnly title = engine roman (swapped=\(swapped))")
        }
    }

    func testDisplaySubtitle_romanOnly_isNil() {
        for swapped in [false, true] {
            let subtitle = CandidateCellHelper.displaySubtitle(
                for: dual,
                isTranslateSwapped: swapped,
                isTPSLayout: false,
                candidateDisplayMode: .romanOnly,
            )
            XCTAssertNil(subtitle, "romanOnly never shows hanji (swapped=\(swapped))")
        }
    }

    func testDisplayTitle_romanOnly_hanjiLessRow_showsRoman() {
        let title = CandidateCellHelper.displayTitle(
            for: romanOnlyRow,
            isTranslateSwapped: false,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .romanOnly,
        )
        XCTAssertEqual(title, "tâi")
    }

    // MARK: - combined arm (漢羅濫 §42 split cells)

    func testDisplayTitle_combined_markedCells_renderOwnScriptAlone() {
        for swapped in [false, true] {
            let hanjiTitle = CandidateCellHelper.displayTitle(
                for: markedHanjiCell,
                isTranslateSwapped: swapped,
                isTPSLayout: false,
                orMapsToER: false,
                candidateDisplayMode: .combined,
            )
            let romanTitle = CandidateCellHelper.displayTitle(
                for: markedRomanCell,
                isTranslateSwapped: swapped,
                isTPSLayout: false,
                orMapsToER: false,
                candidateDisplayMode: .combined,
            )
            XCTAssertEqual(hanjiTitle, "台語", "hanji cell = its own script (swapped=\(swapped))")
            XCTAssertEqual(romanTitle, "tâi-gí", "roman cell = its own script (swapped=\(swapped))")
        }
    }

    func testDisplaySubtitle_combined_isNil() {
        for suggestion in [markedHanjiCell, markedRomanCell, dual] {
            let subtitle = CandidateCellHelper.displaySubtitle(
                for: suggestion,
                isTranslateSwapped: false,
                isTPSLayout: false,
                candidateDisplayMode: .combined,
            )
            XCTAssertNil(subtitle, "濫 cells never show a subtitle")
        }
    }

    /// Un-split rows (NextWord predictions) keep the dual-script shape and
    /// render hanji-led single-script under 濫 — no split, no commit change.
    func testDisplayTitle_combined_unsplitRow_isHanjiLed() {
        let title = CandidateCellHelper.displayTitle(
            for: dual,
            isTranslateSwapped: false,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .combined,
        )
        XCTAssertEqual(title, "台語", "un-split row under 濫 leads with the hanji")
    }

    func testDisplayTitle_combined_hanjiLessRow_showsRomanAlone() {
        let title = CandidateCellHelper.displayTitle(
            for: romanOnlyRow,
            isTranslateSwapped: true,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .combined,
        )
        XCTAssertEqual(title, "tâi", "no hanji → roman alone")
    }

    func testTPSLayout_ignoresCombined_showsHanjiOnly() {
        let title = CandidateCellHelper.displayTitle(
            for: dual,
            isTranslateSwapped: false,
            isTPSLayout: true,
            orMapsToER: false,
            candidateDisplayMode: .combined,
        )
        XCTAssertEqual(title, "台語", "TPS: title = hanji even under combined")
    }

    /// A 濫 split cell is single-script, so the width path is the default
    /// measure — the cell's `text` at the title font, no joined label.
    func testMeasuredCellWidth_combined_measuresSingleScriptAtTitleFont() {
        let titleFontSize: CGFloat = 17
        let width = CandidateCellHelper.measuredCellWidth(
            for: markedHanjiCell,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .combined,
            titleFontSize: titleFontSize,
            subtitleFontSize: 13,
        )
        let textWidth = ("台語" as NSString)
            .size(withAttributes: [.font: KeyboardFonts.globalUIFont(size: titleFontSize)])
            .width
        XCTAssertEqual(
            width,
            max(CandidateCellHelper.minimumCellWidth, textWidth + 20),
            accuracy: 0.5,
            "濫 width = single text at title font + padding",
        )
    }

    // MARK: - suggestionToHandle marked no-op (§42)

    /// A marked cell already carries exactly the script it commits: the swap
    /// rewrite must NOT replace its text even in swapped mode. Hostile fixture
    /// carries a non-empty subtitle to prove the guard precedes the swap arm.
    func testSuggestionToHandle_markedCell_isNoOp_evenWhenSwapped() {
        let hostile = AutocompleteSuggestion(
            text: "台語",
            title: "台語",
            subtitle: "tâi-gí",
            additionalInfo: ["isContinuous": "true", "cellScript": "hanji", "roman": "tâi-gí"],
        )
        let handled = CandidateCellHelper.suggestionToHandle(
            for: hostile,
            isTranslateSwapped: true,
            isTPSLayout: false,
            orMapsToER: false,
        )
        XCTAssertEqual(handled.text, "台語", "marked cell text must survive the swap rewrite")
        XCTAssertEqual(handled.subtitle, "tâi-gí", "marked cell passes through unmodified")
        XCTAssertEqual(handled.additionalInfo["cellScript"], "hanji")
    }

    /// The guard also precedes the TPS fallback rewrite (defensive — TPS never
    /// splits, but a marked cell reaching a TPS-layout render must not be
    /// re-rendered through `tlNumericToTPS`).
    func testSuggestionToHandle_markedCell_isNoOp_underTPSLayout() {
        let handled = CandidateCellHelper.suggestionToHandle(
            for: markedRomanCell,
            isTranslateSwapped: false,
            isTPSLayout: true,
            orMapsToER: false,
        )
        XCTAssertEqual(handled.text, "tâi-gí", "marked cell text must survive the TPS fallback")
        XCTAssertNil(handled.subtitle)
    }

    // MARK: - TPS precedes romanOnly (setting ignored)

    func testTPSLayout_ignoresRomanOnly_showsHanjiOnly() {
        let title = CandidateCellHelper.displayTitle(
            for: dual,
            isTranslateSwapped: false,
            isTPSLayout: true,
            orMapsToER: false,
            candidateDisplayMode: .romanOnly,
        )
        let subtitle = CandidateCellHelper.displaySubtitle(
            for: dual,
            isTranslateSwapped: false,
            isTPSLayout: true,
            candidateDisplayMode: .romanOnly,
        )
        XCTAssertEqual(title, "台語", "TPS: title = hanji even under romanOnly")
        XCTAssertNil(subtitle, "TPS never shows a subtitle")
    }

    // MARK: - sideBySide keeps today's arms byte-for-byte

    func testSideBySide_default_romanTitleHanjiSubtitle() {
        let title = CandidateCellHelper.displayTitle(
            for: dual,
            isTranslateSwapped: false,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .sideBySide,
        )
        let subtitle = CandidateCellHelper.displaySubtitle(
            for: dual,
            isTranslateSwapped: false,
            isTPSLayout: false,
            candidateDisplayMode: .sideBySide,
        )
        XCTAssertEqual(title, "tâi-gí")
        XCTAssertEqual(subtitle, "台語")
    }

    func testSideBySide_swapped_hanjiTitleRomanSubtitle() {
        let title = CandidateCellHelper.displayTitle(
            for: dual,
            isTranslateSwapped: true,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .sideBySide,
        )
        let subtitle = CandidateCellHelper.displaySubtitle(
            for: dual,
            isTranslateSwapped: true,
            isTPSLayout: false,
            candidateDisplayMode: .sideBySide,
        )
        XCTAssertEqual(title, "台語")
        XCTAssertEqual(subtitle, "tâi-gí")
    }

    // MARK: - contentHasSubtitles (§42 one-line content)

    func testContentHasSubtitles_sideBySideDualList_isTrue() {
        XCTAssertTrue(CandidateCellHelper.contentHasSubtitles(
            [dual],
            isTranslateSwapped: false,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .sideBySide,
        ))
    }

    /// A mixed 並排 list (one hanji-less literal among two-line cells) keeps
    /// the flag TRUE — the spacer stays so rows line up.
    func testContentHasSubtitles_mixedSideBySideList_isTrue() {
        XCTAssertTrue(CandidateCellHelper.contentHasSubtitles(
            [romanOnlyRow, dual],
            isTranslateSwapped: false,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .sideBySide,
        ))
    }

    func testContentHasSubtitles_combinedSplitList_isFalse() {
        XCTAssertFalse(CandidateCellHelper.contentHasSubtitles(
            [markedHanjiCell, markedRomanCell],
            isTranslateSwapped: true,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .combined,
        ), "濫 content is one line tall — no reserved subtitle space")
    }

    func testContentHasSubtitles_romanOnlyList_isFalse() {
        XCTAssertFalse(CandidateCellHelper.contentHasSubtitles(
            [dual, romanOnlyRow],
            isTranslateSwapped: false,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .romanOnly,
        ))
    }

    /// Swapped 並排 puts the roman in the subtitle slot; a hanji-less row's
    /// subtitle equals its title and the cell skips it — the flag must apply
    /// the same predicate.
    func testContentHasSubtitles_swappedHanjiLessOnlyList_isFalse() {
        XCTAssertFalse(CandidateCellHelper.contentHasSubtitles(
            [romanOnlyRow],
            isTranslateSwapped: true,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .sideBySide,
        ))
    }

    func testContentHasSubtitles_tpsList_isFalse() {
        XCTAssertFalse(CandidateCellHelper.contentHasSubtitles(
            [dual],
            isTranslateSwapped: false,
            isTPSLayout: true,
            orMapsToER: false,
            candidateDisplayMode: .sideBySide,
        ))
    }
}
