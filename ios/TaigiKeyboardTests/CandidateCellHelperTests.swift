// Pins the candidate cell arm order (TPS → romanOnly → combined → swapped → default)
// for `CandidateCellHelper.displayTitle` / `displaySubtitle` — the platform half of
// Candidate Display = Romanization Only (the engine half collapses same-roman rows) and of
// Candidate Display = Hanji with Romanization (§42 split cells: every cell is single-script, split upstream
// in `TaigiAutocompleteService.buildContinuousSuggestions`), plus the content-level
// subtitle-space flag and the marked-cell no-op in `suggestionToHandle`.

import KeyboardKit
@testable import TaigiKeyboard
import XCTest

final class CandidateCellHelperTests: XCTestCase {
    private let dual = AutocompleteSuggestion(text: "tâi-gí", title: "tâi-gí", subtitle: "台語")
    /// A hanji-less row; `tpsText` is what the engine sends for it under the TPS layout
    /// (`CandidateMessage.tps_text`, tâi → tai5 → ㄉ + ㄞ + ˊ).
    private let romanOnlyRow = AutocompleteSuggestion(
        text: "tâi",
        title: "tâi",
        subtitle: nil,
        additionalInfo: [CandidateCellHelper.tpsTextKey: "ㄉㄞˊ"],
    )

    /// §42 split cells as `buildContinuousSuggestions` emits them under Hanji with Romanization:
    /// single-script `text`, `subtitle = nil`, `cellScript` marker.
    private let markedHanjiCell = AutocompleteSuggestion(
        text: "台語",
        title: "台語",
        subtitle: nil,
        additionalInfo: [
            "isContinuous": "true",
            CandidateCellScript.infoKey: CandidateCellScript.hanji,
            CandidateCellScript.bracketRomanKey: "tâi-gí",
        ],
    )
    private let markedRomanCell = AutocompleteSuggestion(
        text: "tâi-gí",
        title: "tâi-gí",
        subtitle: nil,
        additionalInfo: ["isContinuous": "true", CandidateCellScript.infoKey: CandidateCellScript.roman],
    )

    // MARK: - romanOnly arm

    func testDisplayTitle_romanOnly_showsRomanRegardlessOfSwap() {
        for swapped in [false, true] {
            let title = CandidateCellHelper.displayTitle(
                for: dual,
                isHanjiFirst: swapped,
                isTPSLayout: false,
                candidateDisplayMode: .romanOnly,
            )
            XCTAssertEqual(title, "tâi-gí", "romanOnly title = engine roman (swapped=\(swapped))")
        }
    }

    func testDisplaySubtitle_romanOnly_isNil() {
        for swapped in [false, true] {
            let subtitle = CandidateCellHelper.displaySubtitle(
                for: dual,
                isHanjiFirst: swapped,
                isTPSLayout: false,
                candidateDisplayMode: .romanOnly,
            )
            XCTAssertNil(subtitle, "romanOnly never shows hanji (swapped=\(swapped))")
        }
    }

    func testDisplayTitle_romanOnly_hanjiLessRow_showsRoman() {
        let title = CandidateCellHelper.displayTitle(
            for: romanOnlyRow,
            isHanjiFirst: false,
            isTPSLayout: false,
            candidateDisplayMode: .romanOnly,
        )
        XCTAssertEqual(title, "tâi")
    }

    // MARK: - combined arm (Hanji with Romanization §42 split cells)

    func testDisplaySubtitle_combined_isNil() {
        for suggestion in [markedHanjiCell, markedRomanCell, dual] {
            let subtitle = CandidateCellHelper.displaySubtitle(
                for: suggestion,
                isHanjiFirst: false,
                isTPSLayout: false,
                candidateDisplayMode: .combined,
            )
            XCTAssertNil(subtitle, "濫 cells never show a subtitle")
        }
    }

    /// Un-split rows (NextWord predictions) keep the dual-script shape and
    /// render hanji-led single-script under Hanji with Romanization — no split, no commit change.
    func testDisplayTitle_combined_unsplitRow_isHanjiLed() {
        let title = CandidateCellHelper.displayTitle(
            for: dual,
            isHanjiFirst: false,
            isTPSLayout: false,
            candidateDisplayMode: .combined,
        )
        XCTAssertEqual(title, "台語", "un-split row under 濫 leads with the hanji")
    }

    /// The Hanji with Romanization arm ignores the swap flag for EVERY row shape — marked split
    /// cells render their own script, un-split rows stay hanji-led. Mirrors
    /// Android `test_INVARIANT_combined_marked_cells_are_single_script` +
    /// `combined_unmarkedRow_rendersHanjiLedSingleScript`, and is the property
    /// `measuredCellWidth`'s combined arm relies on when it measures the title
    /// with a literal `isHanjiFirst: false`.
    func testDisplayTitle_combined_ignoresSwapFlag() {
        for swapped in [false, true] {
            let cases: [(AutocompleteSuggestion, String)] = [
                (markedHanjiCell, "台語"),
                (markedRomanCell, "tâi-gí"),
                (dual, "台語"),
            ]
            for (suggestion, expected) in cases {
                XCTAssertEqual(
                    CandidateCellHelper.displayTitle(
                        for: suggestion,
                        isHanjiFirst: swapped,
                        isTPSLayout: false,
                        candidateDisplayMode: .combined,
                    ),
                    expected,
                    "濫 title is swap-invariant (swapped=\(swapped))",
                )
            }
        }
    }

    func testDisplayTitle_combined_hanjiLessRow_showsRomanAlone() {
        let title = CandidateCellHelper.displayTitle(
            for: romanOnlyRow,
            isHanjiFirst: true,
            isTPSLayout: false,
            candidateDisplayMode: .combined,
        )
        XCTAssertEqual(title, "tâi", "no hanji → roman alone")
    }

    func testTPSLayout_ignoresCombined_showsHanjiOnly() {
        let title = CandidateCellHelper.displayTitle(
            for: dual,
            isHanjiFirst: false,
            isTPSLayout: true,
            candidateDisplayMode: .combined,
        )
        XCTAssertEqual(title, "台語", "TPS: title = hanji even under combined")
    }

    /// A Hanji with Romanization split cell is single-script, so the width path is the default
    /// measure — the cell's `text` at the title font, no joined label.
    func testMeasuredCellWidth_combined_measuresSingleScriptAtTitleFont() {
        let titleFontSize: CGFloat = 17
        let width = CandidateCellHelper.measuredCellWidth(
            for: markedHanjiCell,
            isTPSLayout: false,
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

    /// An un-split Hanji with Romanization row (NextWord prediction) renders hanji-led single-line
    /// at the TITLE font — width measures the rendered title, not the two-line
    /// `max(text@title, subtitle@subtitle)` rule.
    func testMeasuredCellWidth_combined_unsplitRow_measuresHanjiLedTitleFont() {
        let titleFontSize: CGFloat = 17
        let width = CandidateCellHelper.measuredCellWidth(
            for: dual,
            isTPSLayout: false,
            candidateDisplayMode: .combined,
            titleFontSize: titleFontSize,
            subtitleFontSize: 13,
        )
        let titleWidth = ("台語" as NSString)
            .size(withAttributes: [.font: KeyboardFonts.globalUIFont(size: titleFontSize)])
            .width
        XCTAssertEqual(
            width,
            max(CandidateCellHelper.minimumCellWidth, titleWidth + 20),
            accuracy: 0.5,
            "un-split 濫 row width = rendered hanji-led title at title font",
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
            additionalInfo: [
                "isContinuous": "true",
                CandidateCellScript.infoKey: CandidateCellScript.hanji,
                CandidateCellScript.bracketRomanKey: "tâi-gí",
            ],
        )
        let handled = CandidateCellHelper.suggestionToHandle(
            for: hostile,
            isHanjiFirst: true,
            isTPSLayout: false,
        )
        XCTAssertEqual(handled.text, "台語", "marked cell text must survive the swap rewrite")
        XCTAssertEqual(handled.subtitle, "tâi-gí", "marked cell passes through unmodified")
        XCTAssertEqual(handled.additionalInfo[CandidateCellScript.infoKey], CandidateCellScript.hanji)
    }

    /// R5 P2 (parity, USER 2026-09-30): a hanji-less TPS cell shows its display romanization
    /// rendered as TPS — the engine's `tps_text`, what picking it writes (engine
    /// `commit_text.rs`) and what Android's cell shows.
    func testTPSLayout_hanjilessCell_showsItsRomanizationAsTPS() {
        let title = CandidateCellHelper.displayTitle(
            for: romanOnlyRow,
            isHanjiFirst: false,
            isTPSLayout: true,
            candidateDisplayMode: .sideBySide,
        )
        XCTAssertEqual(title, "ㄉㄞˊ", "the engine's tps_text")
        let handled = CandidateCellHelper.suggestionToHandle(
            for: romanOnlyRow,
            isHanjiFirst: false,
            isTPSLayout: true,
        )
        XCTAssertEqual(handled.text, title, "the rendered cell and the handed cell agree")
    }

    /// The guard also precedes the TPS fallback rewrite (defensive — TPS never
    /// splits, but a marked cell reaching a TPS-layout render must not be
    /// rewritten to its TPS text).
    func testSuggestionToHandle_markedCell_isNoOp_underTPSLayout() {
        let handled = CandidateCellHelper.suggestionToHandle(
            for: markedRomanCell,
            isHanjiFirst: false,
            isTPSLayout: true,
        )
        XCTAssertEqual(handled.text, "tâi-gí", "marked cell text must survive the TPS fallback")
        XCTAssertNil(handled.subtitle)
    }

    // MARK: - TPS precedes romanOnly (setting ignored)

    func testTPSLayout_ignoresRomanOnly_showsHanjiOnly() {
        let title = CandidateCellHelper.displayTitle(
            for: dual,
            isHanjiFirst: false,
            isTPSLayout: true,
            candidateDisplayMode: .romanOnly,
        )
        let subtitle = CandidateCellHelper.displaySubtitle(
            for: dual,
            isHanjiFirst: false,
            isTPSLayout: true,
            candidateDisplayMode: .romanOnly,
        )
        XCTAssertEqual(title, "台語", "TPS: title = hanji even under romanOnly")
        XCTAssertNil(subtitle, "TPS never shows a subtitle")
    }

    // MARK: - sideBySide keeps today's arms byte-for-byte

    // INVARIANT_CANDIDATE_CELL_SHOWS_BOTH_SCRIPTS (behavioral-invariants.md §42)
    func testSideBySide_default_romanTitleHanjiSubtitle() {
        let title = CandidateCellHelper.displayTitle(
            for: dual,
            isHanjiFirst: false,
            isTPSLayout: false,
            candidateDisplayMode: .sideBySide,
        )
        let subtitle = CandidateCellHelper.displaySubtitle(
            for: dual,
            isHanjiFirst: false,
            isTPSLayout: false,
            candidateDisplayMode: .sideBySide,
        )
        XCTAssertEqual(title, "tâi-gí")
        XCTAssertEqual(subtitle, "台語")
    }

    func testSideBySide_swapped_hanjiTitleRomanSubtitle() {
        let title = CandidateCellHelper.displayTitle(
            for: dual,
            isHanjiFirst: true,
            isTPSLayout: false,
            candidateDisplayMode: .sideBySide,
        )
        let subtitle = CandidateCellHelper.displaySubtitle(
            for: dual,
            isHanjiFirst: true,
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
            isHanjiFirst: false,
            isTPSLayout: false,
            candidateDisplayMode: .sideBySide,
        ))
    }

    /// A mixed Hanji–Romanization Pairing list (one hanji-less literal among two-line cells) keeps
    /// the flag TRUE — the spacer stays so rows line up.
    func testContentHasSubtitles_mixedSideBySideList_isTrue() {
        XCTAssertTrue(CandidateCellHelper.contentHasSubtitles(
            [romanOnlyRow, dual],
            isHanjiFirst: false,
            isTPSLayout: false,
            candidateDisplayMode: .sideBySide,
        ))
    }

    func testContentHasSubtitles_combinedSplitList_isFalse() {
        XCTAssertFalse(CandidateCellHelper.contentHasSubtitles(
            [markedHanjiCell, markedRomanCell],
            isHanjiFirst: true,
            isTPSLayout: false,
            candidateDisplayMode: .combined,
        ), "濫 content is one line tall — no reserved subtitle space")
    }

    func testContentHasSubtitles_romanOnlyList_isFalse() {
        XCTAssertFalse(CandidateCellHelper.contentHasSubtitles(
            [dual, romanOnlyRow],
            isHanjiFirst: false,
            isTPSLayout: false,
            candidateDisplayMode: .romanOnly,
        ))
    }

    /// Swapped Hanji–Romanization Pairing puts the roman in the subtitle slot; a hanji-less row's
    /// subtitle equals its title and the cell skips it — the flag must apply
    /// the same predicate.
    func testContentHasSubtitles_swappedHanjiLessOnlyList_isFalse() {
        XCTAssertFalse(CandidateCellHelper.contentHasSubtitles(
            [romanOnlyRow],
            isHanjiFirst: true,
            isTPSLayout: false,
            candidateDisplayMode: .sideBySide,
        ))
    }

    func testContentHasSubtitles_tpsList_isFalse() {
        XCTAssertFalse(CandidateCellHelper.contentHasSubtitles(
            [dual],
            isHanjiFirst: false,
            isTPSLayout: true,
            candidateDisplayMode: .sideBySide,
        ))
    }
}
