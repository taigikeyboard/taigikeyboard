// Resolving a window's cell index back to the candidate and script it commits.

@testable import TaigiInputMethodCore
import XCTest

/// `CandidateSource` holds one fetch and its presentation together; `resolve`
/// is the only road from a presenter's absolute index back to the
/// `(candidate, script)` a commit needs.
@MainActor
final class CandidateSourceTests: XCTestCase {
    /// Under 合用 a Hanji candidate is two adjacent cells sharing ONE
    /// candidate: the 漢字 cell commits `.primary`, the 羅馬字 cell
    /// `.alternate`, and `flip` — what Space asks for — answers the other
    /// script of the same candidate. Past the list there is nothing to commit.
    func testResolve_underCombined_answersEachCellsOwnScriptAndFlipTheOther() throws {
        let taigi = TestFixtures.candidate(roman: "tâi-gí", hanji: "台語", consumedSpanEnd: 5)
        let source = try makeSource(candidates: [taigi], displayMode: .combined)

        XCTAssertEqual(source.cells.map(\.text), ["台語", "tâi-gí"])
        for (cellIndex, script) in [(0, CandidateScript.primary), (1, .alternate)] {
            let own = try XCTUnwrap(source.resolve(cellIndex: cellIndex, flip: false))
            XCTAssertEqual(own.candidate, taigi, "cell \(cellIndex)")
            XCTAssertEqual(own.script, script, "cell \(cellIndex)")
            XCTAssertEqual(
                try XCTUnwrap(source.resolve(cellIndex: cellIndex, flip: true)).script,
                script.flipped,
                "cell \(cellIndex)",
            )
        }
        XCTAssertNil(source.resolve(cellIndex: 2, flip: false), "past the list — nothing to commit")
    }

    /// The empty source is what a dismissed bar leaves behind: nothing shows,
    /// nothing resolves.
    func testEmpty_showsNothingAndResolvesNothing() {
        XCTAssertTrue(CandidateSource.empty.isEmpty)
        XCTAssertNil(CandidateSource.empty.resolve(cellIndex: 0, flip: false))
    }

    private func makeSource(
        candidates: [ContinuousCandidate],
        displayMode: CandidateDisplayMode,
    ) throws -> CandidateSource {
        try CandidateSource(
            candidates: candidates,
            manager: TestFixtures.makeComposingManager(
                settingsProvider: StubEngineSettingsProvider(candidateDisplayMode: displayMode),
                startingGeneration: TestFixtures.generationCounter.next(),
            ),
        )
    }
}
