// Pins the NextWord prediction cell shape `ActionHandler.predictionSuggestions`
// emits per 候選詞顯示 mode (§42, USER 2026-09-12: 濫 lists both scripts of a
// prediction, 羅馬字 the roman alone, 並排 today's dual-script cell). Android
// parity: `NextWordHandler.buildPredictionWords`.

import KeyboardKit
@testable import TaigiKeyboard
import XCTest

final class ActionHandlerPredictionSuggestionsTests: XCTestCase {
    private func prediction(
        text: String,
        subtitle: String?,
        hanzi: String,
        tl: String? = nil,
    ) -> RustEngineBridge.NextWordEnginePrediction {
        RustEngineBridge.NextWordEnginePrediction(
            text: text,
            subtitle: subtitle,
            hanzi: hanzi,
            tl: tl ?? text,
            score: 1.0,
        )
    }

    private var predictions: [RustEngineBridge.NextWordEnginePrediction] {
        [
            prediction(text: "tsia̍h", subtitle: "食", hanzi: "食"),
            // 同音異字 — same roman, different hanji.
            prediction(text: "tsia̍h", subtitle: "𤆬", hanzi: "𤆬"),
            // 一字多音 — same hanji, different roman.
            prediction(text: "tîng", subtitle: "重", hanzi: "重"),
            prediction(text: "tāng", subtitle: "重", hanzi: "重"),
            // Hanji-only prediction (engine shaped no roman).
            prediction(text: "去", subtitle: nil, hanzi: "去", tl: ""),
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
            // 𤆬's roman cell reads like 食's → collapsed; its 漢字 cell stays.
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
            XCTAssertEqual(suggestion.additionalInfo["tl"], "tāng")
            XCTAssertEqual(suggestion.additionalInfo["canonicalTl"], "tāng")
            XCTAssertEqual(suggestion.additionalInfo["displayText"], "重")
        }
        XCTAssertEqual(suggestions[0].additionalInfo[CandidateCellScript.bracketRomanKey], "tāng")
    }
}
