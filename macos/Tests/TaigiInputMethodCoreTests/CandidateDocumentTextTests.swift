// Pins what each output setting writes into the document. PR5 is what lets a
// user reach the non-default combinations; the rendering is decided here.

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
}
