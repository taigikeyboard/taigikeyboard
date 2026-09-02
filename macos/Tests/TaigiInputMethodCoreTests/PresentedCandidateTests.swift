// How the fetched list becomes the window's list: one cell per candidate, or
// two under 漢羅合用, each mapped back to what it commits.

@testable import TaigiInputMethodCore
import XCTest

/// The presentation rules per display mode, and the 合用 pair order and
/// romanization dedupe that give the window its cells.
final class PresentedCandidateTests: XCTestCase {
    private let taigi = TestFixtures.candidate(roman: "tâi-gí", hanji: "台語", consumedSpanEnd: 5)
    private let literal = TestFixtures.candidate(roman: "tâi-gí", hanji: nil, consumedSpanEnd: 5)

    // MARK: - One cell per candidate

    /// 並排 and 羅馬字 show one cell per candidate — byte-identical to the cell
    /// `CandidateCellContent.cell(for:settings:)` has always built — committing
    /// the primary script, with the cell's position naming the candidate.
    func testOneCellDisplays_presentEveryCandidateOnce_asTheExistingCell() {
        let candidates = [taigi, literal, TestFixtures.candidate(roman: "tâi", hanji: "台", consumedSpanEnd: 3)]
        let settingsUnderTest = [
            TestFixtures.settings(swapped: false),
            TestFixtures.settings(swapped: true),
            TestFixtures.settings(swapped: false, bothScripts: true),
            TestFixtures.settings(swapped: false, candidateDisplayMode: .romanOnly),
            TestFixtures.settings(swapped: true, candidateDisplayMode: .romanOnly),
        ]
        for settings in settingsUnderTest {
            let presented = PresentedCandidate.presentation(of: candidates, settings: settings)

            XCTAssertEqual(
                presented,
                candidates.enumerated().map { index, candidate in
                    PresentedCandidate(
                        candidateIndex: index,
                        script: .primary,
                        cell: CandidateCellContent.cell(for: candidate, settings: settings),
                    )
                },
                "\(settings.candidateDisplayMode) swapped=\(settings.isTranslateSwapped)",
            )
        }
    }

    func testEmptyList_presentsNothing() {
        for mode in CandidateDisplayMode.allCases {
            XCTAssertEqual(
                PresentedCandidate.presentation(
                    of: [], settings: TestFixtures.settings(candidateDisplayMode: mode),
                ),
                [],
            )
        }
    }

    // MARK: - 漢羅合用

    private let combined = TestFixtures.settings(swapped: true, candidateDisplayMode: .combined)

    /// A candidate with both scripts is two adjacent one-script cells, Hanji
    /// first: the Hanji cell commits `.primary` (the Hanji, under the forced
    /// swap), the romanization cell `.alternate` — and both point at the same
    /// candidate. Neither carries an annotation.
    func testCombined_splitsAHanjiCandidateIntoTwoAdjacentCells() {
        let presented = PresentedCandidate.presentation(of: [taigi], settings: combined)

        XCTAssertEqual(presented, [
            PresentedCandidate(
                candidateIndex: 0, script: .primary,
                cell: CandidateCellContent(text: "台語", annotation: nil),
            ),
            PresentedCandidate(
                candidateIndex: 0, script: .alternate,
                cell: CandidateCellContent(text: "tâi-gí", annotation: nil),
            ),
        ])
    }

    /// The stored swap flag has no say under 合用 — the pair order is the
    /// mode's, not the flag's.
    func testCombined_ignoresTheSwapFlag() {
        let stored = TestFixtures.settings(swapped: false, candidateDisplayMode: .combined)

        XCTAssertEqual(
            PresentedCandidate.presentation(of: [taigi], settings: stored),
            PresentedCandidate.presentation(of: [taigi], settings: combined),
        )
    }

    /// No Hanji, no Hanji cell: the romanization alone, committing `.primary`
    /// — which for a Hanji-less candidate IS the romanization.
    func testCombined_presentsAHanjiLessCandidateAsOneRomanizationCell() {
        for hanji in [nil, ""] {
            let candidate = TestFixtures.candidate(roman: "guá", hanji: hanji, consumedSpanEnd: 3)

            XCTAssertEqual(
                PresentedCandidate.presentation(of: [candidate], settings: combined),
                [PresentedCandidate(
                    candidateIndex: 0, script: .primary,
                    cell: CandidateCellContent(text: "guá", annotation: nil),
                )],
                "hanji=\(String(describing: hanji))",
            )
        }
    }

    /// Two Hanji under one reading share one romanization cell — a second
    /// `tsia̍h` would commit exactly what the first does. The survivor sits
    /// after the FIRST Hanji, in fetched order, and the second Hanji follows on
    /// its own.
    func testCombined_dedupesARomanizationCellSharedByTwoHanji() {
        let eat = TestFixtures.candidate(roman: "tsia̍h", hanji: "食", consumedSpanEnd: 5)
        let lead = TestFixtures.candidate(roman: "tsia̍h", hanji: "𤆬", consumedSpanEnd: 5)

        let presented = PresentedCandidate.presentation(of: [eat, lead], settings: combined)

        XCTAssertEqual(presented.map(\.cell.text), ["食", "tsia̍h", "𤆬"])
        XCTAssertEqual(presented.map(\.candidateIndex), [0, 0, 1])
        XCTAssertEqual(presented.map(\.script), [.primary, .alternate, .primary])
    }

    /// The §34 literal at slot 0 is the same offer as 台's romanization cell,
    /// so the latter is absorbed: `tâi` once, then 台 — never `tâi 台 tâi`.
    func testCombined_theLiteralAtSlotZero_absorbsTheDictionaryRomanizationCell() {
        let literal = TestFixtures.candidate(roman: "tâi", hanji: nil, consumedSpanEnd: 3)
        let tai = TestFixtures.candidate(roman: "tâi", hanji: "台", consumedSpanEnd: 3)

        let presented = PresentedCandidate.presentation(of: [literal, tai], settings: combined)

        XCTAssertEqual(presented.map(\.cell.text), ["tâi", "台"])
        XCTAssertEqual(presented.map(\.candidateIndex), [0, 1])
        XCTAssertEqual(presented.map(\.script), [.primary, .primary])
    }

    /// The same text over a DIFFERENT span is a different commit — it consumes
    /// a different stretch of the buffer — so both cells stay.
    func testCombined_keepsSameTextRomanizationCellsOverDifferentSpans() {
        let short = TestFixtures.candidate(roman: "tâi", hanji: "台", consumedSpanEnd: 3)
        let long = TestFixtures.candidate(roman: "tâi", hanji: "臺", consumedSpanEnd: 4)

        let presented = PresentedCandidate.presentation(of: [short, long], settings: combined)

        XCTAssertEqual(presented.map(\.cell.text), ["台", "tâi", "臺", "tâi"])
        XCTAssertEqual(presented.map(\.candidateIndex), [0, 0, 1, 1])
    }

    /// Hanji cells are never deduplicated: 重 tîng and 重 tāng are two
    /// morphemes (Core Principle #7), and the adjacent romanization is what
    /// tells the two 重 apart.
    func testCombined_neverDedupesHanjiCells() {
        let repeat_ = TestFixtures.candidate(roman: "tîng", hanji: "重", consumedSpanEnd: 5)
        let heavy = TestFixtures.candidate(roman: "tāng", hanji: "重", consumedSpanEnd: 5)

        let presented = PresentedCandidate.presentation(of: [repeat_, heavy], settings: combined)

        XCTAssertEqual(presented.map(\.cell.text), ["重", "tîng", "重", "tāng"])
        XCTAssertEqual(presented.map(\.candidateIndex), [0, 0, 1, 1])
    }

    /// The commit each cell resolves to, end to end through the document
    /// renderer: Return on the Hanji cell writes the Hanji, Return on the
    /// romanization cell the romanization, and Space — the flip — the other
    /// one of the SAME candidate in each case.
    func testCombined_eachCellCommitsItsOwnScript_andSpaceTheOther() {
        let presented = PresentedCandidate.presentation(of: [taigi], settings: combined)

        func written(_ script: CandidateScript) -> String? {
            switch script {
            case .primary: CandidateDocumentText.text(for: taigi, settings: combined)
            case .alternate: CandidateDocumentText.resolvedAlternate(for: taigi, settings: combined)?.text
            }
        }

        XCTAssertEqual(presented.map { written($0.script) }, ["台語", "tâi-gí"])
        XCTAssertEqual(presented.map { written($0.script.flipped) }, ["tâi-gí", "台語"])
    }
}
