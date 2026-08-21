// Pins what each output setting writes into the document. PR5 is what lets a
// user reach the non-default combinations; the rendering is decided here.

@testable import TaigiInputMethodCore
import XCTest

final class CandidateDocumentTextTests: XCTestCase {
    private let word = TestFixtures.candidate(roman: "tâi-gí", hanji: "台語")

    /// The 直接送出漢字 and 直接送出羅馬字 keys override the user's own output
    /// preference for one word. Asserted against every settings combination,
    /// including a stale stored `isOutputBothScripts` — the toggle has no UI
    /// any more, but the engine still reads the key, and a key that says
    /// "Hanji" must never produce a bracketed pair.
    func testAForcedRendering_writesThatScriptUnderEveryOutputSetting() {
        for swapped in [true, false] {
            for bothScripts in [true, false] {
                let settings = TestFixtures.settings(swapped: swapped, bothScripts: bothScripts)

                XCTAssertEqual(
                    CandidateDocumentText.text(for: word, settings: settings, rendering: .hanji),
                    "台語",
                )
                XCTAssertEqual(
                    CandidateDocumentText.text(for: word, settings: settings, rendering: .romanization),
                    "tâi-gí",
                )
            }
        }
    }

    /// A romanization-only candidate has no Hanji to force, and the answer is
    /// "cannot" rather than a silent fall back to the romanization: the key
    /// names a script, and writing the other one is worse than doing nothing.
    func testTheHanjiRendering_cannotRenderARomanizationOnlyCandidate() {
        let romanOnly = TestFixtures.candidate(roman: "tâi-gí", hanji: nil)

        XCTAssertFalse(CandidateDocumentText.Rendering.hanji.canRender(romanOnly))
        XCTAssertTrue(CandidateDocumentText.Rendering.romanization.canRender(romanOnly))
        XCTAssertTrue(CandidateDocumentText.Rendering.settings.canRender(romanOnly))
        XCTAssertTrue(CandidateDocumentText.Rendering.hanji.canRender(word))
    }

    /// An empty Hanji means the same as an absent one, and a producer that
    /// emitted `""` must not make the key look available and then write
    /// nothing.
    func testAnEmptyHanji_countsAsAbsent() {
        XCTAssertFalse(
            CandidateDocumentText.Rendering.hanji
                .canRender(TestFixtures.candidate(roman: "tâi-gí", hanji: "")),
        )
    }

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
