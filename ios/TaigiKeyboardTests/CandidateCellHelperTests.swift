// Pins the candidate cell arm order (TPS → romanOnly → swapped → default) for
// `CandidateCellHelper.displayTitle` / `displaySubtitle` — the platform half of
// 候選詞顯示 = 羅馬字 (the engine half collapses same-roman rows).

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
