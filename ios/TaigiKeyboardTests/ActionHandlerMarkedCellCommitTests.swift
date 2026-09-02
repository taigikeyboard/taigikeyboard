// Pins the §42 漢羅濫 marked-cell commit resolver (`ActionHandler.markedCellCommit`)
// and the shared auto-space verdict: a split cell's document text comes from the
// `cellScript` marker + info fields, never from `parseRomanAndHanzi`; a defective
// marker is declined by `CandidateCellScript.marker` so the render guard and the
// commit resolver fall back to the unmarked path TOGETHER (Android
// `resolveMarkedCellCommit` parity).

@testable import TaigiKeyboard
import KeyboardKit
import XCTest

final class ActionHandlerMarkedCellCommitTests: XCTestCase {
    func testHanjiCell_bracketsOff_commitsHanjiAlone_noRomanizationWritten() {
        let result = ActionHandler.markedCellCommit(
            cellScript: CandidateCellScript.hanji,
            cellText: "台語",
            roman: "tâi-gí",
            isOutputBothScripts: false,
        )
        XCTAssertEqual(result.text, "台語")
        XCTAssertFalse(result.wroteRomanization, "pure hanji commit must not trigger auto-space")
    }

    func testHanjiCell_bracketsOn_appendsRomanSidechannel_romanizationWritten() {
        let result = ActionHandler.markedCellCommit(
            cellScript: CandidateCellScript.hanji,
            cellText: "台語",
            roman: "tâi-gí",
            isOutputBothScripts: true,
        )
        XCTAssertEqual(result.text, "台語 (tâi-gí)", "括號標註 ON = today's swapped output")
        XCTAssertTrue(result.wroteRomanization, "the bracket form DID write romanization → auto-space fires")
    }

    func testRomanCell_bracketsIgnored_commitsBareRoman_andSpaces() {
        let result = ActionHandler.markedCellCommit(
            cellScript: CandidateCellScript.roman,
            cellText: "tâi-gí",
            roman: nil,
            isOutputBothScripts: true,
        )
        XCTAssertEqual(
            result.text,
            "tâi-gí",
            "roman cell commits the bare roman even with 括號標註 ON (desktop .alternate parity)",
        )
        XCTAssertTrue(result.wroteRomanization, "roman commit drives the auto-space gate")
    }

    /// 括號標註 ON without a roman to bracket commits the bare 漢字 — never
    /// `台語 ()`. Mirrors Android `hanjiCell_bracketsOn_missingRoman_commitsHanjiAlone`.
    func testHanjiCell_bracketsOn_missingRoman_commitsHanjiAlone() {
        for roman in [nil, ""] {
            let result = ActionHandler.markedCellCommit(
                cellScript: CandidateCellScript.hanji,
                cellText: "台語",
                roman: roman,
                isOutputBothScripts: true,
            )
            XCTAssertEqual(result.text, "台語", "no roman sidechannel → no empty brackets")
            XCTAssertFalse(result.wroteRomanization)
        }
    }

    // MARK: - Marker recognition (render guard + commit resolver share it)

    /// `CandidateCellScript.marker` is the ONE predicate deciding "this is a §42
    /// split cell". A wire defect resolves to nil so the render guard
    /// (`suggestionToHandle`) and this handler fall back to the unmarked
    /// mode-derived path together — a marker honoured by one and declined by the
    /// other would skip the swap rewrite and then re-parse the un-rewritten cell.
    func testMarker_recognizesOnlyKnownMarkersWithAPayload() {
        XCTAssertEqual(
            CandidateCellScript.marker(for: markedCell(CandidateCellScript.hanji, text: "台語")),
            CandidateCellScript.hanji,
        )
        XCTAssertEqual(
            CandidateCellScript.marker(for: markedCell(CandidateCellScript.roman, text: "tâi-gí")),
            CandidateCellScript.roman,
        )
        XCTAssertNil(
            CandidateCellScript.marker(for: markedCell("both", text: "tâi-gí")),
            "unknown marker value — not a split cell",
        )
        XCTAssertNil(
            CandidateCellScript.marker(for: markedCell(CandidateCellScript.hanji, text: "")),
            "marker without a payload to commit — not a split cell",
        )
        XCTAssertNil(
            CandidateCellScript.marker(for: AutocompleteSuggestion(text: "tâi-gí", title: "tâi-gí", subtitle: "台語")),
            "unmarked dual-script suggestion",
        )
    }

    // MARK: - Auto-space verdict (the §42 refactor's shared predicate)

    /// trace: `ActionHandler.rawPreeditWritesRomanization` — the layout is the
    /// whole question for a commit that writes the composition as typed.
    /// Mirrors Android `rawPreedit_writesRomanizationUnlessTheLayoutComposesBopomofo`.
    func testRawPreeditWritesRomanization_followsTheLayoutNotTheMode() {
        XCTAssertTrue(ActionHandler.rawPreeditWritesRomanization(isTPSLayout: false))
        XCTAssertFalse(ActionHandler.rawPreeditWritesRomanization(isTPSLayout: true))
    }

    /// Mirrors Android `shouldAppendAutoSpace_requiresSettingRomanizationAndNoHyphenTail`.
    func testShouldAppendAutoSpace_requiresSettingRomanizationAndNoHyphenTail() {
        XCTAssertTrue(ActionHandler.shouldAppendAutoSpace(
            isAutoSpaceEnabled: true,
            wroteRomanization: true,
            documentText: "tâi-gí",
        ))
        XCTAssertFalse(
            ActionHandler.shouldAppendAutoSpace(isAutoSpaceEnabled: false, wroteRomanization: true, documentText: "tâi-gí"),
            "setting off",
        )
        XCTAssertFalse(
            ActionHandler.shouldAppendAutoSpace(isAutoSpaceEnabled: true, wroteRomanization: false, documentText: "台語"),
            "pure hanji commit wrote no romanization",
        )
        XCTAssertFalse(
            ActionHandler.shouldAppendAutoSpace(isAutoSpaceEnabled: true, wroteRomanization: true, documentText: "tâi-"),
            "trailing hyphen = 連字 continuation, keep composing",
        )
    }

    // MARK: - Helpers

    private func markedCell(_ marker: String, text: String) -> AutocompleteSuggestion {
        AutocompleteSuggestion(
            text: text,
            title: text,
            subtitle: nil,
            additionalInfo: [CandidateCellScript.infoKey: marker],
        )
    }
}
