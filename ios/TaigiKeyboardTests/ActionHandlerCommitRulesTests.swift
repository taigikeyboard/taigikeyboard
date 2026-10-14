// Pins the commit rules the platform still owns: the §42 split-cell marker the
// render guard and the tap's `CommitScript` share (a defective marker is declined
// by `CandidateCellScript.marker`, so both fall back to the lead together), and
// the auto-space verdict of the commits the engine does not resolve (an English
// word, the raw preedit). Android parity: `AutoSpaceVerdictTest`.

import KeyboardKit
@testable import TaigiKeyboard
import XCTest

final class ActionHandlerCommitRulesTests: XCTestCase {
    // MARK: - Marker recognition (render guard + commit script share it)

    /// `CandidateCellScript.marker` is the ONE predicate deciding "this is a §42
    /// split cell". A wire defect resolves to nil so the render guard
    /// (`suggestionToHandle`) and the tap (`commitScript(for:)`) fall back to the
    /// lead together — a marker honoured by one and declined by the other would
    /// skip the swap rewrite and then commit a script the cell does not show.
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
