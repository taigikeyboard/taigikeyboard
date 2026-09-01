// Pins the candidate cell arm order (TPS → romanOnly → combined → swapped → default)
// for `CandidateCellHelper.displayTitle` / `displaySubtitle` — the platform half of
// 候選詞顯示 = 羅馬字 (the engine half collapses same-roman rows) and the whole of
// 候選詞顯示 = 漢羅合用 (one `漢字 羅馬字` label, measured at the title font).

@testable import TaigiKeyboard
import KeyboardKit
import XCTest

final class CandidateCellHelperTests: XCTestCase {
    private let dual = AutocompleteSuggestion(text: "tâi-gí", title: "tâi-gí", subtitle: "台語")
    private let romanOnlyRow = AutocompleteSuggestion(text: "tâi", title: "tâi", subtitle: nil)

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

    // MARK: - combined arm (漢羅合用)

    func testDisplayTitle_combined_hanjiThenRomanRegardlessOfSwap() {
        for swapped in [false, true] {
            let title = CandidateCellHelper.displayTitle(
                for: dual,
                isTranslateSwapped: swapped,
                isTPSLayout: false,
                orMapsToER: false,
                candidateDisplayMode: .combined,
            )
            XCTAssertEqual(title, "台語 tâi-gí", "combined = hanji, one space, roman (swapped=\(swapped))")
        }
    }

    func testDisplaySubtitle_combined_isNil() {
        for swapped in [false, true] {
            let subtitle = CandidateCellHelper.displaySubtitle(
                for: dual,
                isTranslateSwapped: swapped,
                isTPSLayout: false,
                candidateDisplayMode: .combined,
            )
            XCTAssertNil(subtitle, "combined folds both scripts into the title (swapped=\(swapped))")
        }
    }

    func testDisplayTitle_combined_hanjiLessRow_showsRomanAlone() {
        let title = CandidateCellHelper.displayTitle(
            for: romanOnlyRow,
            isTranslateSwapped: true,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .combined,
        )
        XCTAssertEqual(title, "tâi", "no hanji → no separator, roman alone")
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

    /// The combined label is wider than either script alone, so the width path
    /// must measure the joined string at the title font — not max(title, subtitle).
    func testMeasuredCellWidth_combined_measuresJoinedLabelAtTitleFont() {
        let titleFontSize: CGFloat = 17
        let combinedWidth = CandidateCellHelper.measuredCellWidth(
            for: dual,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .combined,
            titleFontSize: titleFontSize,
            subtitleFontSize: 13,
        )
        let sideBySideWidth = CandidateCellHelper.measuredCellWidth(
            for: dual,
            isTPSLayout: false,
            orMapsToER: false,
            candidateDisplayMode: .sideBySide,
            titleFontSize: titleFontSize,
            subtitleFontSize: 13,
        )
        let joinedLabelWidth = ("台語 tâi-gí" as NSString)
            .size(withAttributes: [.font: KeyboardFonts.globalUIFont(size: titleFontSize)])
            .width

        XCTAssertGreaterThanOrEqual(combinedWidth, joinedLabelWidth, "cell must fit the joined label")
        XCTAssertGreaterThan(combinedWidth, sideBySideWidth, "joined label is wider than max(title, subtitle)")
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
}
