// Pins the NextWord prediction cell shape `ActionHandler.predictionSuggestions`
// emits per Candidate Display mode (§42, USER 2026-09-12: Hanji with Romanization lists both scripts of a
// prediction, Romanization Only the roman alone, Hanji–Romanization Pairing today's dual-script cell). Android
// parity: `NextWordController.buildPredictionWords`.

import KeyboardKit
@testable import TaigiKeyboard
import XCTest

final class ActionHandlerPredictionSuggestionsTests: XCTestCase {
    private func prediction(
        text: String,
        subtitle: String?,
        hanji: String,
        tl: String? = nil,
    ) -> RustEngineBridge.NextWordEnginePrediction {
        RustEngineBridge.NextWordEnginePrediction(
            text: text,
            subtitle: subtitle,
            hanji: hanji,
            tl: tl ?? text,
        )
    }

    private var predictions: [RustEngineBridge.NextWordEnginePrediction] {
        [
            prediction(text: "tsia̍h", subtitle: "食", hanji: "食"),
            // Homophones — same roman, different hanji.
            prediction(text: "tsia̍h", subtitle: "𤆬", hanji: "𤆬"),
            // Polyphonic Hanji — same hanji, different roman.
            prediction(text: "tîng", subtitle: "重", hanji: "重"),
            prediction(text: "tāng", subtitle: "重", hanji: "重"),
            // Hanji-only prediction (engine shaped no roman).
            prediction(text: "去", subtitle: nil, hanji: "去", tl: ""),
        ]
    }

    func testUnsplit_emitsOneDualScriptSuggestionPerPrediction() {
        let suggestions = ActionHandler.predictionSuggestions(predictions, splitCombinedCells: false)

        XCTAssertEqual(suggestions.count, predictions.count)
        XCTAssertEqual(suggestions[0].text, "tsia̍h")
        XCTAssertEqual(suggestions[0].subtitle, "食")
        XCTAssertNil(suggestions[4].subtitle)
        XCTAssertEqual(suggestions[4].text, "去")
        for suggestion in suggestions {
            XCTAssertNil(CandidateCellScript.marker(for: suggestion))
            XCTAssertEqual(suggestion.additionalInfo["isNextWord"], "true")
        }
        XCTAssertEqual(suggestions[3].additionalInfo["canonicalTl"], "tāng")
    }

    func testSplit_emitsHanjiThenRomanCells_dedupedPerScript() {
        let suggestions = ActionHandler.predictionSuggestions(predictions, splitCombinedCells: true)

        let shown = suggestions.map { "\(CandidateCellScript.marker(for: $0) ?? "-"):\($0.text)" }
        XCTAssertEqual(shown, [
            "hanji:食",
            "roman:tsia̍h",
            // 𤆬's roman cell reads like 食's → collapsed; its Hanji cell stays.
            "hanji:𤆬",
            "hanji:重",
            "roman:tîng",
            // 重/tāng: 重 already drawn, its own roman cell stays reachable.
            "roman:tāng",
            "hanji:去",
        ])
        for suggestion in suggestions {
            XCTAssertNil(suggestion.subtitle, "split cells are single-script")
        }
    }

    func testSplitCells_shareIdentity_andHanjiCellCarriesBracketRoman() {
        let suggestions = ActionHandler.predictionSuggestions([predictions[3]], splitCombinedCells: true)

        XCTAssertEqual(suggestions.count, 2)
        for suggestion in suggestions {
            XCTAssertEqual(suggestion.additionalInfo["isNextWord"], "true")
            XCTAssertEqual(suggestion.additionalInfo["hanzi"], "重")
            XCTAssertEqual(suggestion.additionalInfo["canonicalTl"], "tāng")
        }
        XCTAssertEqual(suggestions[0].additionalInfo[CandidateCellScript.bracketRomanKey], "tāng")
    }

    /// The tap request (D1) is read off the metadata, never the cell text the
    /// view rewrites (the swap puts the Hanji in `text`): the roman the engine
    /// resolves from (empty for a Hanji-only prediction), the Hanji, the
    /// canonical TL and a split cell's own script. What each request writes is
    /// the engine's (`engine/composing/tests/prediction_commit_resolution.rs`).
    /// Android parity: `PredictionWordsBuilderTest.prediction_pick_reads_the_word_not_the_cell_text`.
    @MainActor
    func testPredictionPick_readsTheMetadataNotTheCellText() {
        let unsplit = ActionHandler.predictionSuggestions(predictions, splitCombinedCells: false)
        let swapped = AutocompleteSuggestion(
            text: "重",
            title: "重",
            subtitle: "tāng",
            additionalInfo: unsplit[3].additionalInfo,
        )
        XCTAssertEqual(
            ActionHandler.predictionPick(for: swapped),
            RustEngineBridge.PredictionPick(script: .lead, roman: "tāng", hanji: "重", canonicalTl: "tāng"),
        )
        XCTAssertEqual(
            ActionHandler.predictionPick(for: unsplit[4]),
            RustEngineBridge.PredictionPick(script: .lead, roman: "", hanji: "去", canonicalTl: ""),
        )

        let split = ActionHandler.predictionSuggestions([predictions[3]], splitCombinedCells: true)
            .map(ActionHandler.predictionPick(for:))
        XCTAssertEqual(split.map(\.script), [.hanji, .roman])
        // The Hanji cell carries the roman its Annotate in Brackets commit appends.
        XCTAssertEqual(split.map(\.roman), ["tāng", "tāng"])
    }
}
