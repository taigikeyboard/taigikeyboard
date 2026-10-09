// Pins the Continuous tap request (R5): the script and roman the engine resolves the document
// text from come off the suggestion's metadata, never its view-rewritten `text`. What each
// request writes is the engine's (`engine/composing/src/commit_text.rs` truth tables).
// Mirrors Android `ContinuousPickTest`.

import KeyboardKit
@testable import TaigiKeyboard
import XCTest

final class ActionHandlerContinuousPickTests: XCTestCase {
    override class func setUp() {
        super.setUp()
        // The TPS cell rendering (`suggestionToHandle`) is an engine call.
        RustEngineBridge.install()
    }

    private func candidate(roman: String, hanji: String?) -> RustEngineBridge.ContinuousCandidate {
        RustEngineBridge.ContinuousCandidate(
            consumedSpanEnd: 7,
            syllableCount: 2,
            displayText: hanji ?? roman,
            roman: roman,
            hanji: hanji,
            canonicalTl: "tâi-gí",
        )
    }

    /// The suggestions as the strip builds them, then as the view hands them to the handler.
    private func handed(
        _ candidates: [RustEngineBridge.ContinuousCandidate],
        splitCombinedCells: Bool = false,
        isHanjiFirst: Bool = false,
        isTPSLayout: Bool = false,
    ) -> [AutocompleteSuggestion] {
        TaigiAutocompleteService()
            .buildContinuousSuggestions(from: candidates, splitCombinedCells: splitCombinedCells)
            .map {
                CandidateCellHelper.suggestionToHandle(
                    for: $0,
                    isHanjiFirst: isHanjiFirst,
                    isTPSLayout: isTPSLayout,
                    orMapsToER: false,
                )
            }
    }

    func testUnsplitCell_commitsTheLead_withTheCandidateMetadata() throws {
        let suggestion = try XCTUnwrap(handed([candidate(roman: "tâi-gí", hanji: "台語")]).first)
        let pick = try XCTUnwrap(ActionHandler.continuousPick(for: suggestion))
        XCTAssertEqual(pick, RustEngineBridge.ContinuousPick(
            script: .lead,
            roman: "tâi-gí",
            canonicalText: "台語",
            associationTl: "tâi-gí",
            hanji: "台語",
            consumedBytes: 7,
            syllableCount: 2,
        ))
    }

    /// The swap and the TPS layout rewrite the cell's `text` (`suggestionToHandle`); the pick
    /// still carries the candidate's own romanization. The old path parsed the rewritten cell,
    /// which under TPS was the `tlNumericToTPS` rendering of the display romanization (R2d).
    func testRewrittenCell_stillSendsTheCandidateRoman() throws {
        let swapped = try XCTUnwrap(handed([candidate(roman: "tâi-gí", hanji: "台語")], isHanjiFirst: true).first)
        XCTAssertEqual(swapped.text, "台語", "precondition: the swap rewrote the cell")
        let hanjilessTPS = try XCTUnwrap(handed([candidate(roman: "tâi-gí", hanji: nil)], isTPSLayout: true).first)
        XCTAssertNotEqual(hanjilessTPS.text, "tâi-gí", "precondition: TPS rewrote the cell")
        for suggestion in [swapped, hanjilessTPS] {
            let pick = try XCTUnwrap(ActionHandler.continuousPick(for: suggestion))
            XCTAssertEqual(pick.script, .lead)
            XCTAssertEqual(pick.roman, "tâi-gí", "cell text \(suggestion.text)")
        }
        XCTAssertNil(try XCTUnwrap(ActionHandler.continuousPick(for: hanjilessTPS)).hanji)
    }

    /// §42 split cells commit the script their marker names; both cells carry the same
    /// identity and roman.
    func testSplitCells_commitTheirOwnScript() throws {
        let cells = handed([candidate(roman: "tâi-gí", hanji: "台語")], splitCombinedCells: true)
        XCTAssertEqual(cells.count, 2)
        let hanjiCell = try XCTUnwrap(ActionHandler.continuousPick(for: cells[0]))
        let romanCell = try XCTUnwrap(ActionHandler.continuousPick(for: cells[1]))
        XCTAssertEqual(hanjiCell.script, .hanji)
        XCTAssertEqual(romanCell.script, .roman)
        for pick in [hanjiCell, romanCell] {
            XCTAssertEqual(pick.roman, "tâi-gí")
            XCTAssertEqual(pick.hanji, "台語")
            XCTAssertEqual(pick.canonicalText, "台語", "identity never moves")
        }
    }

    /// A marker `CandidateCellScript.marker` declines (unknown value, empty text) commits the
    /// lead, as the render guard shows it.
    func testDefectiveMarker_commitsTheLead() throws {
        let suggestion = try XCTUnwrap(handed([candidate(roman: "tâi-gí", hanji: "台語")]).first)
        var info = suggestion.additionalInfo
        info[CandidateCellScript.infoKey] = "both"
        let defective = AutocompleteSuggestion(text: suggestion.text, title: suggestion.title, subtitle: suggestion.subtitle, additionalInfo: info)
        XCTAssertEqual(ActionHandler.commitScript(for: defective), .lead)
    }

    /// Strict-required metadata: a suggestion missing any of it is dropped, never committed
    /// as bare cell text.
    func testMissingMetadata_dropsTheTap() throws {
        let suggestion = try XCTUnwrap(handed([candidate(roman: "tâi-gí", hanji: "台語")]).first)
        for key in ["displayText", "roman", "consumedBytes", "syllableCount"] {
            var info = suggestion.additionalInfo
            info[key] = nil
            let stripped = AutocompleteSuggestion(text: suggestion.text, title: suggestion.title, subtitle: suggestion.subtitle, additionalInfo: info)
            XCTAssertNil(ActionHandler.continuousPick(for: stripped), "missing \(key)")
        }
    }

    func testRequest_carriesScriptRomanAndIdentity() {
        let request = RustEngineBridge.ContinuousPick(
            script: .hanji,
            roman: "tâi-gí",
            canonicalText: "台語",
            associationTl: "tâi-gí",
            hanji: "台語",
            consumedBytes: 7,
            syllableCount: 2,
        ).request
        XCTAssertEqual(request.script, .hanji)
        XCTAssertEqual(request.roman, "tâi-gí")
        XCTAssertEqual(request.canonicalText, "台語")
        XCTAssertEqual(request.associationTl, "tâi-gí")
        XCTAssertEqual(request.hanji, "台語")
        XCTAssertEqual(request.consumedBytes, 7)
        XCTAssertEqual(request.syllableCount, 2)

        let hanjiless = RustEngineBridge.ContinuousPick(
            script: .lead,
            roman: "tâi",
            canonicalText: "tâi",
            associationTl: "tâi",
            hanji: nil,
            consumedBytes: 3,
            syllableCount: 1,
        ).request
        XCTAssertFalse(hanjiless.hasHanji, "§50: a hanji-less pick sends no hanji")
    }
}
