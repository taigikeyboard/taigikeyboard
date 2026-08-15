// Drives the candidate query and commit against the real engine and the real
// dictionary, which is the only place the byte-span contract can be checked.

import XCTest

@testable import TaigiInputMethodCore

/// The Rust composing state is one per process, so each case gets a generation
/// nobody else uses — see `GenerationCounter`.
@MainActor
final class ComposingManagerCandidateTests: XCTestCase {
    override func setUp() {
        super.setUp()
        InstalledLexicon.installOnce()
    }

    private func makeManager() -> ComposingManager {
        ComposingManager(startingGeneration: TestFixtures.generationCounter.next())
    }

    /// `taigi` segments into two syllables, so its candidates include both a
    /// whole-buffer word and shorter ones — which is what makes the difference
    /// between a nail and a final commit observable.
    private func composeTaigi(
        _ manager: ComposingManager,
        executing executor: RecordingEffectExecutor,
    ) {
        for character in ["t", "a", "i", "g", "i"] {
            manager.append(character, executing: executor)
        }
        executor.clearEffects()
    }

    // MARK: - Fetching

    func testFetchCandidates_whileComposing_findsCandidates() throws {
        let manager = makeManager()
        composeTaigi(manager, executing: RecordingEffectExecutor())

        guard case let .found(candidates) = manager.fetchCandidates() else {
            return XCTFail("a composition the engine is holding must answer with a candidate list")
        }

        XCTAssertFalse(candidates.isEmpty, "the dictionary has entries for taigi")
        XCTAssertTrue(
            candidates.allSatisfy { $0.consumedSpanEnd > 0 },
            "a candidate that consumes nothing could never be committed",
        )
    }

    func testFetchCandidates_whileIdle_reportsNotComposing() {
        let manager = makeManager()

        XCTAssertEqual(
            manager.fetchCandidates(),
            .notComposing,
            "an idle engine has no continuous phase to read candidates from",
        )
    }

    /// The query is read-only, but "read-only" is only observable through what
    /// the engine does NEXT: a query that reset the composition — by bumping the
    /// generation, say — would leave the following keystroke starting a brand
    /// new one, and the mirror alone would not show it.
    func testFetchCandidates_leavesTheCompositionIntactForTheNextKeystroke() throws {
        let manager = makeManager()
        let executor = RecordingEffectExecutor()
        composeTaigi(manager, executing: executor)
        let rawBefore = manager.rawInput

        _ = manager.fetchCandidates()
        _ = manager.fetchCandidates()
        manager.append("a", executing: executor)

        XCTAssertEqual(
            manager.rawInput,
            rawBefore + "a",
            "the buffer must carry on from where it was, not restart from the new character",
        )
        XCTAssertEqual(
            executor.effects.preeditTexts.last,
            manager.displayText,
            "the host must be looking at the same composition the mirror describes",
        )
    }

    // MARK: - Committing

    func testCommitCandidate_consumingTheWholeBuffer_writesTheDocumentAndEnds() throws {
        let manager = makeManager()
        let executor = RecordingEffectExecutor()
        composeTaigi(manager, executing: executor)
        let candidate = try XCTUnwrap(
            wholeBufferCandidate(from: manager),
            "taigi must offer at least one candidate spanning the whole buffer",
        )

        let outcome = manager.commitCandidate(candidate, executing: executor)

        XCTAssertEqual(outcome, .finalized)
        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(
            executor.committedTexts,
            [CandidateDocumentText.text(for: candidate, settings: .defaults)],
            "one document mutation, carrying the rendering the settings asked for — "
                + "asserting only the count would pass with the canonical key sent by mistake",
        )
        XCTAssertTrue(manager.displayText.isEmpty, "an ended composition has nothing left to render")
    }

    /// The formatter has its own unit tests; this is the wiring — that the
    /// manager passes the rendering to the bridge and the canonical key
    /// separately, rather than sending one string for both.
    func testCommitCandidate_swappedOutput_writesTheHanjiRatherThanTheRomanization() throws {
        let manager = ComposingManager(
            settingsProvider: StubEngineSettingsProvider(swapped: true),
            startingGeneration: TestFixtures.generationCounter.next(),
        )
        let executor = RecordingEffectExecutor()
        composeTaigi(manager, executing: executor)
        let candidate = try XCTUnwrap(
            wholeBufferCandidate(from: manager, requiringHanji: true),
            "the swap only differs from the default on a candidate that has a hanji",
        )
        let hanji = try XCTUnwrap(candidate.hanji)

        _ = manager.commitCandidate(candidate, executing: executor)

        XCTAssertEqual(executor.committedTexts, [hanji])
    }

    func testCommitCandidate_consumingPartOfTheBuffer_nailsItAndKeepsComposing() throws {
        let manager = makeManager()
        let executor = RecordingEffectExecutor()
        composeTaigi(manager, executing: executor)
        let candidate = try XCTUnwrap(
            partialCandidate(from: manager),
            "taigi must offer at least one candidate shorter than the whole buffer",
        )

        let outcome = manager.commitCandidate(candidate, executing: executor)

        XCTAssertEqual(outcome, .nailed)
        XCTAssertTrue(manager.isComposing)
        XCTAssertTrue(
            executor.committedTexts.isEmpty,
            "Model B keeps a nailed segment in the marked region — nothing reaches the document yet",
        )
    }

    func testCommitCandidate_nailed_leavesRawInputAsTheTailAndDisplayTextWhole() throws {
        let manager = makeManager()
        let executor = RecordingEffectExecutor()
        composeTaigi(manager, executing: executor)
        let candidate = try XCTUnwrap(partialCandidate(from: manager))
        let nailedText = CandidateDocumentText.text(for: candidate, settings: .defaults)
        executor.clearEffects()

        _ = manager.commitCandidate(candidate, executing: executor)

        XCTAssertEqual(
            manager.rawInput.utf8.count,
            "taigi".utf8.count - Int(candidate.consumedSpanEnd),
            "the nailed segment's bytes leave the raw buffer; the rest stays pending",
        )
        XCTAssertEqual(
            manager.displayText,
            executor.effects.preeditTexts.last,
            "the mirror must be the composition the host was just told to render, "
                + "not the value it happened to hold before the nail",
        )
        XCTAssertTrue(
            manager.displayText.hasPrefix(nailedText),
            "the marked region keeps the nailed segment in front of the pending tail — "
                + "this is what the candidate panel anchors to, so rawInput cannot stand in for it",
        )
    }

    func testCommitCandidate_afterTheCompositionEnded_isIgnored() throws {
        let manager = makeManager()
        let executor = RecordingEffectExecutor()
        composeTaigi(manager, executing: executor)
        let candidate = try XCTUnwrap(wholeBufferCandidate(from: manager))
        manager.cancelComposition(executing: executor)
        executor.clearEffects()

        let outcome = manager.commitCandidate(candidate, executing: executor)

        XCTAssertEqual(
            outcome,
            .ignored,
            "a stale candidate must not read as a successful commit — the mirror alone would say it ended",
        )
        XCTAssertTrue(executor.effects.isEmpty)
    }

    // MARK: - Candidate selection helpers

    /// The first candidate whose span satisfies `spanMatches`, measured against
    /// the pending buffer. Which candidate a real dictionary offers is not
    /// something a case should hardcode — what each case needs is a span of a
    /// particular SHAPE, and it fails loudly through `XCTUnwrap` if the
    /// dictionary stops offering one.
    private func candidate(
        from manager: ComposingManager,
        requiringHanji: Bool = false,
        spanMatches: (_ consumedSpanEnd: UInt32, _ pendingBytes: UInt32) -> Bool,
    ) -> ContinuousCandidate? {
        let pendingBytes = UInt32(manager.rawInput.utf8.count)
        guard case let .found(candidates) = manager.fetchCandidates() else { return nil }
        return candidates.first { candidate in
            spanMatches(candidate.consumedSpanEnd, pendingBytes)
                && (!requiringHanji || candidate.hanji?.isEmpty == false)
        }
    }

    private func wholeBufferCandidate(
        from manager: ComposingManager,
        requiringHanji: Bool = false,
    ) -> ContinuousCandidate? {
        candidate(from: manager, requiringHanji: requiringHanji) { $0 >= $1 }
    }

    private func partialCandidate(from manager: ComposingManager) -> ContinuousCandidate? {
        candidate(from: manager) { $0 > 0 && $0 < $1 }
    }
}
