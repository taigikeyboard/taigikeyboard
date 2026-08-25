// What a commit teaches the two learning stores, end to end through the engine.

@testable import TaigiInputMethodCore
import XCTest

/// Drives real compositions against the real engine and asserts on the rows
/// that end up in SQLite. Everything here needs the engine artefacts to be
/// current — the next-word intents are rejected outright by an engine built
/// before `PLATFORM_MACOS` existed.
@MainActor
final class ComposingManagerLearningTests: XCTestCase {
    private var stores: UserDataStores!

    override func setUpWithError() throws {
        try super.setUpWithError()
        InstalledLexicon.installOnce()
        stores = try TestFixtures.makeUserDataStores()
    }

    override func tearDown() {
        stores = nil
        super.tearDown()
    }

    // MARK: - Frequency

    func testCommitCandidate_countsTheWordUnderTheReadingItWasCommittedAs() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()
        let candidate = try composeAndTakeWholeBufferCandidate(manager, executing: executor)

        _ = manager.commitCandidate(candidate, executing: executor)

        let rows = try XCTUnwrap(stores.frequency.rows(forWords: [candidate.displayText]))
        XCTAssertEqual(
            rows.map { FrequencyRow(word: $0.word, tl: $0.tl, count: $0.count, lastUsedMillis: 0) },
            [FrequencyRow(
                word: candidate.displayText,
                tl: candidate.canonicalTl,
                count: 1,
                lastUsedMillis: 0,
            )],
            "the row is keyed by the (display text, canonical TL) pair the ranker looks it up by",
        )
    }

    /// A Taiwanese word is the `(漢字, canonical TL)` pair (Core Principle #7),
    /// and which SCRIPT it was written in is not part of that. Committing the
    /// other script must therefore land on the same row — otherwise 漢羅 typing
    /// would quietly split every word's frequency in two, and neither half
    /// would rank.
    func testAlternateScriptCommit_learnsTheSameWordAsThePrimaryOne() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()
        let candidate = try composeAndTakeWholeBufferCandidate(manager, executing: executor)

        _ = manager.commitCandidate(candidate, script: .alternate, executing: executor)

        let rows = try XCTUnwrap(stores.frequency.rows(forWords: [candidate.displayText]))
        XCTAssertEqual(
            rows.map { FrequencyRow(word: $0.word, tl: $0.tl, count: $0.count, lastUsedMillis: 0) },
            [FrequencyRow(
                word: candidate.displayText,
                tl: candidate.canonicalTl,
                count: 1,
                lastUsedMillis: 0,
            )],
            "the identity is the pair, never the rendering that reached the document",
        )
    }

    func testCommitCandidate_withRecordingOff_learnsNothing() throws {
        let manager = try makeManager(
            settingsProvider: StubEngineSettingsProvider(frequencyRecording: false),
        )
        let executor = RecordingEffectExecutor()
        let candidate = try composeAndTakeWholeBufferCandidate(manager, executing: executor)

        _ = manager.commitCandidate(candidate, executing: executor)

        XCTAssertEqual(
            try XCTUnwrap(stores.frequency.rows(forWords: [candidate.displayText])),
            [],
            "the setting gates the write, not the boost — an off switch that still recorded "
                + "would keep changing the ranking of everything typed while it was off",
        )
    }

    /// The boost is the whole point of the second fetch: a candidate the user
    /// keeps choosing has to climb past the ones it used to trail.
    ///
    /// Measured against a rival rather than against index 0, because index 0 is
    /// not part of the ranking: the walker prepends its own best segmentation
    /// there explicitly and it takes no part in the sort
    /// (`engine/composing/src/continuous.rs:110-116`). Asserting "the boosted
    /// candidate is first now" would be asserting that a boost can displace
    /// something the sort never touches.
    func testFetchCandidates_aRepeatedlyCommittedCandidateOvertakesTheOneAboveIt() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        let before = try compose("taigi", manager, executing: executor)
        let ranked = Array(before.dropFirst())
        // The last two rather than the first two: candidates covering more of
        // the buffer outrank shorter ones by a margin no usage count is meant
        // to close, so a pair straddling that boundary would be asking the
        // boost to do something it must not. Two neighbours in the tail cover
        // the same span, which is where usage is the deciding term.
        XCTAssertGreaterThanOrEqual(ranked.count, 2, "the fixture needs at least two ranked candidates")
        let rival = try XCTUnwrap(ranked.dropLast().last)
        let promoted = try XCTUnwrap(ranked.last)
        XCTAssertEqual(
            rival.consumedSpanEnd,
            promoted.consumedSpanEnd,
            "the pair has to cover the same span, or the comparison is about coverage not usage",
        )

        for _ in 0 ..< 20 {
            stores.frequency.record(word: promoted.displayText, tl: promoted.canonicalTl)
        }

        manager.cancelComposition(executing: executor)
        let after = try compose("taigi", manager, executing: executor)

        XCTAssertLessThan(
            try XCTUnwrap(after.firstIndex(matching: promoted), "a boosted candidate stays in the list"),
            try XCTUnwrap(after.firstIndex(matching: rival)),
            "usage the user built up has to reach the ranking — if this fails the second "
                + "fetch is not carrying the frequency rows",
        )
        XCTAssertEqual(
            after.first?.displayText,
            before.first?.displayText,
            "the walker's own pick leads the list whatever the user has learned",
        )
    }

    // MARK: - Association

    func testTwoCommitsInARow_learnTheBigramBetweenThem() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        let first = try commitWholeBuffer("tai", manager, executing: executor)
        let second = try commitWholeBuffer("gi", manager, executing: executor)

        let rows = try XCTUnwrap(stores.association.allRows())
        XCTAssertEqual(
            rows.map { "\($0.pair.previous)→\($0.pair.next)" },
            ["\(first.displayText)→\(second.displayText)"],
            "the engine decided this pair was worth learning; the manager's job is to store it",
        )
    }

    /// A 漢羅 sentence mixes the scripts word by word, so a bigram will
    /// routinely have one half written in each. The pair must still be learnt
    /// under the identity, not under whichever rendering reached the document —
    /// otherwise `我 ê` learnt in mixed script would never predict `ê` again.
    func testABigramWithOneHalfInTheOtherScript_learnsTheSamePair() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        let first = try commitWholeBuffer("tai", manager, executing: executor)
        let second = try composeAndTakeWholeBufferCandidate(manager, executing: executor, "gi")
        _ = manager.commitCandidate(second, script: .alternate, executing: executor)

        let rows = try XCTUnwrap(stores.association.allRows())
        XCTAssertEqual(
            rows.map { "\($0.pair.previous)→\($0.pair.next)" },
            ["\(first.displayText)→\(second.displayText)"],
            "the pair is the identity pair, whichever script the document got",
        )
    }

    /// `.alternate` on a candidate that has no second script is answered here
    /// and nowhere else: nothing reaches the engine, nothing reaches the
    /// document, and nothing is learnt. One decision point — a caller-side
    /// pre-check plus a fallback here would be two rules for one case.
    func testAlternateOnASingleScriptCandidate_commitsNothing() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()
        let candidate = try composeAndTakeWholeBufferCandidate(manager, executing: executor)
        let romanOnly = TestFixtures.candidate(
            roman: candidate.roman,
            hanji: nil,
            displayText: candidate.displayText,
            canonicalTl: candidate.canonicalTl,
            consumedSpanEnd: candidate.consumedSpanEnd,
            syllableCount: candidate.syllableCount,
        )

        let (outcome, committedText) = manager.commitCandidate(
            romanOnly, script: .alternate, executing: executor,
        )

        XCTAssertEqual(outcome, .ignored)
        XCTAssertNil(committedText)
        XCTAssertEqual(
            try XCTUnwrap(stores.frequency.rows(forWords: [romanOnly.displayText])), [],
            "a commit that wrote nothing teaches nothing",
        )
    }

    func testWithAssociationRecordingOff_twoCommitsLearnNothing() throws {
        let manager = try makeManager(
            settingsProvider: StubEngineSettingsProvider(associationRecording: false),
        )
        let executor = RecordingEffectExecutor()

        _ = try commitWholeBuffer("tai", manager, executing: executor)
        _ = try commitWholeBuffer("gi", manager, executing: executor)

        XCTAssertEqual(try XCTUnwrap(stores.association.allRows()), [])
    }

    /// Sentence-end punctuation typed straight into the host ends the context,
    /// which is what stops the last word of one sentence being learned as the
    /// predecessor of the first word of the next.
    func testAFullStopBetweenTwoCommits_breaksTheBigram() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        _ = try commitWholeBuffer("tai", manager, executing: executor)
        manager.noteCharacterTypedOutsideComposition("。")
        _ = try commitWholeBuffer("gi", manager, executing: executor)

        XCTAssertEqual(
            try XCTUnwrap(stores.association.allRows()),
            [],
            "the context was reset, so the second commit had no predecessor to pair with",
        )
    }

    func testACommaBetweenTwoCommits_leavesTheBigramIntact() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        _ = try commitWholeBuffer("tai", manager, executing: executor)
        manager.noteCharacterTypedOutsideComposition("、")
        _ = try commitWholeBuffer("gi", manager, executing: executor)

        XCTAssertEqual(
            try XCTUnwrap(stores.association.allRows()).count,
            1,
            "a comma is noise, not the end of a sentence — the context survives it",
        )
    }

    func testALetterTypedOutsideAComposition_isNotTreatedAsAWord() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        _ = try commitWholeBuffer("tai", manager, executing: executor)
        // A letter reaching the host without a composition is not a Taiwanese
        // word, and letting it become the context would pair the next commit
        // with a stray keystroke.
        manager.noteCharacterTypedOutsideComposition("x")
        _ = try commitWholeBuffer("gi", manager, executing: executor)

        let rows = try XCTUnwrap(stores.association.allRows())
        XCTAssertEqual(rows.count, 1)
        XCTAssertNotEqual(rows.first?.pair.previous, "x")
    }

    func testANewSession_forgetsTheContextFromTheOldOne() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        _ = try commitWholeBuffer("tai", manager, executing: executor)
        // A session change is usually a change of application: what was typed in
        // the last one must not seed what is typed in the next.
        manager.startNewSession()
        _ = try commitWholeBuffer("gi", manager, executing: executor)

        XCTAssertEqual(try XCTUnwrap(stores.association.allRows()), [])
    }

    /// Punctuation typed while a composition is running does NOT go through the
    /// pass-through path: it commits the composition and rides along with it in
    /// one document write (`ComposingKeyIntent.commitThenInsert`). The engine
    /// describes that commit to the learner only as "clear for new composing"
    /// and never says which word was committed
    /// (`engine/composing/src/transition.rs:769-780`), so carrying on with the
    /// old context would pair the NEXT commit with the word before this one —
    /// a bigram that skips a word.
    func testPunctuationCommittedMidComposition_dropsTheContextRatherThanSkippingAWord() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        _ = try commitWholeBuffer("tai", manager, executing: executor)
        for character in "gi" {
            manager.append(String(character), executing: executor)
        }
        manager.commitComposition(thenInsert: "。", executing: executor)
        _ = try commitWholeBuffer("gi", manager, executing: executor)

        XCTAssertEqual(
            try XCTUnwrap(stores.association.allRows()),
            [],
            "under-learning one pair is the safe half; pairing 台 with the word after the "
                + "full stop would be learning something the user never typed",
        )
    }

    // MARK: - Mirror

    /// The fetch response carries the authoritative composition state, and a
    /// fetch that reports the engine idle means the composition is gone. Before
    /// the two-phase fetch landed, that answer was dropped and the mirror went
    /// on claiming a composition the engine no longer had.
    func testFetchCandidates_afterTheEngineWasResetUnderneathIt_updatesTheMirror() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()
        _ = try compose("taigi", manager, executing: executor)
        XCTAssertTrue(manager.isComposing)

        // Resets the engine to Idle without going through the manager, which is
        // what a generation change does in production.
        let other = try TestFixtures.makeComposingManager(
            stores: stores,
            startingGeneration: TestFixtures.generationCounter.next(),
        )
        other.append("t", executing: RecordingEffectExecutor())

        XCTAssertEqual(manager.fetchCandidates(), .notComposing)
        XCTAssertFalse(
            manager.isComposing,
            "the mirror has to follow the engine's answer, or the controller keeps routing keys "
                + "into a composition that no longer exists",
        )
    }

    // MARK: - Helpers

    private func makeManager(
        settingsProvider: EngineSettingsProvider = StubEngineSettingsProvider(),
    ) throws -> ComposingManager {
        try TestFixtures.makeComposingManager(
            settingsProvider: settingsProvider,
            stores: stores,
            startingGeneration: TestFixtures.generationCounter.next(),
        )
    }

    /// Types `romanization` one character at a time — the production path — and
    /// returns the candidates the engine offers for it.
    @discardableResult
    private func compose(
        _ romanization: String,
        _ manager: ComposingManager,
        executing executor: ComposingEffectExecutor,
    ) throws -> [ContinuousCandidate] {
        for character in romanization {
            manager.append(String(character), executing: executor)
        }
        guard case let .found(candidates) = manager.fetchCandidates() else {
            XCTFail("the engine offered no candidates for '\(romanization)'")
            return []
        }
        return candidates
    }

    private func composeAndTakeWholeBufferCandidate(
        _ manager: ComposingManager,
        executing executor: ComposingEffectExecutor,
        _ romanization: String = "taigi",
    ) throws -> ContinuousCandidate {
        let candidates = try compose(romanization, manager, executing: executor)
        return try XCTUnwrap(
            candidates.first { $0.consumedSpanEnd >= UInt32(manager.rawInput.utf8.count) },
            "no candidate consumes the whole buffer, so nothing here would be a final commit",
        )
    }

    /// Composes `romanization` and commits the candidate that consumes all of
    /// it, which is what makes the commit final and emits the learning
    /// handshake.
    @discardableResult
    private func commitWholeBuffer(
        _ romanization: String,
        _ manager: ComposingManager,
        executing executor: ComposingEffectExecutor,
    ) throws -> ContinuousCandidate {
        let candidate = try composeAndTakeWholeBufferCandidate(
            manager,
            executing: executor,
            romanization,
        )
        _ = manager.commitCandidate(candidate, executing: executor)
        return candidate
    }
}

private extension [ContinuousCandidate] {
    /// Where `candidate` sits in this list, matched on what identifies it rather
    /// than on the whole value: `score` is exactly what a boost changes, so a
    /// plain equality search would report a promoted candidate as missing.
    func firstIndex(matching candidate: ContinuousCandidate) -> Int? {
        firstIndex {
            $0.displayText == candidate.displayText
                && $0.canonicalTl == candidate.canonicalTl
                && $0.consumedSpanEnd == candidate.consumedSpanEnd
        }
    }
}
