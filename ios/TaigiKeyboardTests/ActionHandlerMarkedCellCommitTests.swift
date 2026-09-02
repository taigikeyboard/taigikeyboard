// Pins the §42 漢羅濫 marked-cell commit resolver (`ActionHandler.markedCellCommit`):
// a split cell's document text comes from the `cellScript` marker + info fields,
// never from `parseRomanAndHanzi`, and the returned `wroteRomanization` flag is
// what the auto-space gate follows (the committed script, not the mode).

@testable import TaigiKeyboard
import XCTest

final class ActionHandlerMarkedCellCommitTests: XCTestCase {
    func testHanjiCell_bracketsOff_commitsHanjiAlone_noRomanizationWritten() {
        let result = ActionHandler.markedCellCommit(
            cellScript: "hanji",
            cellText: "台語",
            roman: "tâi-gí",
            isOutputBothScripts: false,
        )
        XCTAssertEqual(result.docText, "台語")
        XCTAssertFalse(result.wroteRomanization, "hanji commit must not trigger auto-space")
    }

    func testHanjiCell_bracketsOn_appendsRomanSidechannel() {
        let result = ActionHandler.markedCellCommit(
            cellScript: "hanji",
            cellText: "台語",
            roman: "tâi-gí",
            isOutputBothScripts: true,
        )
        XCTAssertEqual(result.docText, "台語 (tâi-gí)", "括號標註 ON = today's swapped output")
        XCTAssertFalse(result.wroteRomanization)
    }

    func testHanjiCell_bracketsOn_missingRoman_fallsBackToHanjiAlone() {
        let result = ActionHandler.markedCellCommit(
            cellScript: "hanji",
            cellText: "台語",
            roman: nil,
            isOutputBothScripts: true,
        )
        XCTAssertEqual(result.docText, "台語", "no roman sidechannel → no empty brackets")
        XCTAssertFalse(result.wroteRomanization)
    }

    func testRomanCell_bracketsIgnored_commitsBareRoman_andSpaces() {
        let result = ActionHandler.markedCellCommit(
            cellScript: "roman",
            cellText: "tâi-gí",
            roman: nil,
            isOutputBothScripts: true,
        )
        XCTAssertEqual(
            result.docText,
            "tâi-gí",
            "roman cell commits the bare roman even with 括號標註 ON (desktop .alternate parity)",
        )
        XCTAssertTrue(result.wroteRomanization, "roman commit drives the auto-space gate")
    }
}
