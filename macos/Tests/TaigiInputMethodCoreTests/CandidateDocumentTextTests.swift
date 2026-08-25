// Pins what each output setting writes into the document.

@testable import TaigiInputMethodCore
import XCTest

final class CandidateDocumentTextTests: XCTestCase {
    private let word = TestFixtures.candidate(roman: "tâi-gí", hanji: "台語")

    func testDefaults_writeTheRomanization() {
        let text = CandidateDocumentText.text(
            for: word,
            settings: TestFixtures.settings(swapped: false, bothScripts: false),
        )

        XCTAssertEqual(text, "tâi-gí")
    }

    func testSwapped_writesTheHanji() {
        let text = CandidateDocumentText.text(
            for: word,
            settings: TestFixtures.settings(swapped: true, bothScripts: false),
        )

        XCTAssertEqual(text, "台語")
    }

    func testBothScripts_bracketsTheHanjiAfterTheRomanization() {
        let text = CandidateDocumentText.text(
            for: word,
            settings: TestFixtures.settings(swapped: false, bothScripts: true),
        )

        XCTAssertEqual(text, "tâi-gí (台語)")
    }

    func testBothScriptsSwapped_bracketsTheRomanizationAfterTheHanji() {
        let text = CandidateDocumentText.text(
            for: word,
            settings: TestFixtures.settings(swapped: true, bothScripts: true),
        )

        XCTAssertEqual(
            text,
            "台語 (tâi-gí)",
            "the swap decides which script leads; both-scripts only decides that both appear",
        )
    }

    func testRomanizationOnlyCandidate_writesTheRomanizationUnderEverySetting() {
        let romanOnly = TestFixtures.candidate(roman: "tâi-gí", hanji: nil)

        for swapped in [true, false] {
            for bothScripts in [true, false] {
                XCTAssertEqual(
                    CandidateDocumentText.text(
                        for: romanOnly,
                        settings: TestFixtures.settings(swapped: swapped, bothScripts: bothScripts),
                    ),
                    "tâi-gí",
                    "no hanji exists to swap to or bracket (swapped: \(swapped), both: \(bothScripts))",
                )
            }
        }
    }

    func testPresentButEmptyHanji_isTreatedAsAbsent() {
        let defective = TestFixtures.candidate(roman: "tâi-gí", hanji: "")

        XCTAssertEqual(
            CandidateDocumentText.text(for: defective, settings: TestFixtures.settings(swapped: false, bothScripts: true)),
            "tâi-gí",
            "a wire defect must not render as an empty bracket in the user's document",
        )
        XCTAssertEqual(
            CandidateDocumentText.text(for: defective, settings: TestFixtures.settings(swapped: true, bothScripts: false)),
            "tâi-gí",
            "swapping to an empty hanji would commit nothing at all",
        )
    }

    // MARK: - The other script (the 漢羅 key)

    /// Space writes whichever script Return does not — that is the whole
    /// gesture, and it is why the key needs no mode of its own.
    func testAlternate_isWhicheverScriptThePrimaryIsNot() {
        for swapped in [false, true] {
            let settings = TestFixtures.settings(swapped: swapped, bothScripts: false)
            let primary = CandidateDocumentText.text(for: word, settings: settings)
            let alternate = CandidateDocumentText.alternateText(for: word, settings: settings)

            XCTAssertEqual(alternate, swapped ? "tâi-gí" : "台語", "swapped: \(swapped)")
            XCTAssertNotEqual(alternate, primary)
        }
    }

    /// The bracket setting says how to write a candidate that shows BOTH
    /// scripts. Space is the request for one of them by itself, so it is not
    /// read — a bracketed pair here would write the script the user already had.
    func testAlternate_ignoresTheBracketSetting() {
        for swapped in [false, true] {
            XCTAssertEqual(
                CandidateDocumentText.alternateText(
                    for: word,
                    settings: TestFixtures.settings(swapped: swapped, bothScripts: true),
                ),
                swapped ? "tâi-gí" : "台語",
                "swapped: \(swapped)",
            )
        }
    }

    /// A romanization-only candidate — the §34 literal candidate, an
    /// out-of-vocabulary name — has no second script, and saying so is what
    /// lets Space decline the key rather than write the romanization twice.
    func testAlternate_isAbsentWhenTheCandidateHasOneScript() {
        for hanji in [nil, ""] {
            let romanOnly = TestFixtures.candidate(roman: "Tsng-kiô", hanji: hanji)
            for swapped in [false, true] {
                XCTAssertNil(
                    CandidateDocumentText.alternateText(
                        for: romanOnly,
                        settings: TestFixtures.settings(swapped: swapped, bothScripts: false),
                    ),
                    "hanji: \(String(describing: hanji)), swapped: \(swapped)",
                )
            }
        }
    }

    /// Space writes exactly the script the bar shows under the primary one.
    /// The user is looking at the offer before they take it, and after the
    /// derivation through `CandidateCellContent` that is structural rather than
    /// two rules that happen to agree.
    func testAlternate_isTheScriptTheCellShowsBesideThePrimary() {
        for swapped in [false, true] {
            let settings = TestFixtures.settings(swapped: swapped, bothScripts: false)

            XCTAssertEqual(
                CandidateDocumentText.alternateText(for: word, settings: settings),
                CandidateCellContent.cell(for: word, settings: settings).annotation,
                "swapped: \(swapped)",
            )
        }
    }

    /// Verbatim, hyphens included. 298 dictionary entries write the 輕聲 `--`
    /// into the hanji field — MOE orthography, pinned by §21/S8 — and the 漢羅
    /// mixed entries carry the romanized half's own hyphen. Stripping either
    /// here would be this layer second-guessing the dictionary.
    func testAlternate_writesTheFieldVerbatim() {
        let hanjiSettings = TestFixtures.settings(swapped: false, bothScripts: false)
        for (roman, hanji) in [("kau--lâng", "交--人"), ("âng-kì-kì", "紅kì-kì")] {
            XCTAssertEqual(
                CandidateDocumentText.alternateText(
                    for: TestFixtures.candidate(roman: roman, hanji: hanji),
                    settings: hanjiSettings,
                ),
                hanji,
            )
        }
    }
}
