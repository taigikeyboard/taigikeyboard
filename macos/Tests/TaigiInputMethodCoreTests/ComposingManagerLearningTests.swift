// What a commit reports for learning, end to end through the engine.

@testable import TaigiInputMethodCore
import XCTest

/// Drives real compositions against the real engine and asserts on what the
/// manager reports: the picks it counts and the next-word handshakes. What the
/// engine learns from a handshake — the window, noise, sentence ends — and
/// stores is its own tests' (`engine/nextword/src/decide.rs`). Everything here needs the engine artefacts to be
/// current — the next-word intents are rejected outright by an engine built
/// before `PLATFORM_MACOS` existed.
@MainActor
final class ComposingManagerLearningTests: XCTestCase {
    /// Fresh per case: XCTest makes a new instance for every test method.
    private let usage = RecordingUsageRecorder()
    private let nextWord = RecordingNextWordPort()

    override func setUpWithError() throws {
        try super.setUpWithError()
        InstalledLexicon.installOnce()
    }

    // MARK: - Frequency

    func testCommitCandidate_countsTheWordUnderTheReadingItWasCommittedAs() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()
        let candidate = try composeAndTakeWholeBufferCandidate(manager, executing: executor)

        _ = manager.commitCandidate(candidate, executing: executor)

        XCTAssertEqual(
            usage.recorded,
            [expectedUsage(of: candidate)],
            "the pick is counted under the (display text, canonical TL) pair the ranker looks it up by",
        )
    }

    /// A Taiwanese word is the `(Hanji, canonical TL)` pair (Core Principle #7),
    /// and which SCRIPT it was written in is not part of that. Committing the
    /// other script must therefore land on the same row — otherwise mixed-script typing
    /// would quietly split every word's frequency in two, and neither half
    /// would rank.
    func testAlternateScriptCommit_learnsTheSameWordAsThePrimaryOne() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()
        let candidate = try composeAndTakeWholeBufferCandidate(manager, executing: executor)

        _ = manager.commitCandidate(candidate, script: .alternate, executing: executor)

        XCTAssertEqual(
            usage.recorded,
            [expectedUsage(of: candidate)],
            "the identity is the pair, never the rendering that reached the document",
        )
    }

    func testCommitCandidate_withRecordingOff_tellsTheEngineNotToCount() throws {
        let manager = try makeManager(
            settingsProvider: StubEngineSettingsProvider(frequencyRecording: false),
        )
        let executor = RecordingEffectExecutor()
        let candidate = try composeAndTakeWholeBufferCandidate(manager, executing: executor)

        _ = manager.commitCandidate(candidate, executing: executor)

        XCTAssertEqual(
            usage.recorded,
            [expectedUsage(of: candidate, frequencyRecording: false)],
            "the setting gates the count, not the boost — an off switch that still counted "
                + "would keep changing the ranking of everything typed while it was off; the "
                + "pick is still reported so a learned phrase taken whole is touched",
        )
    }

    // MARK: - Next word

    func testTwoCommitsInARow_reportBothUnderTheirIdentity() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        let first = try commitWholeBuffer("tai", manager, executing: executor)
        let second = try commitWholeBuffer("gi", manager, executing: executor)

        XCTAssertEqual(
            nextWord.handshakes,
            [
                .selected(text: first.displayText, roman: first.canonicalTl),
                .selected(text: second.displayText, roman: second.canonicalTl),
            ],
            "the manager reports each commit; the engine decides the pair between them",
        )
    }

    /// A mixed-script sentence mixes the scripts word by word, so a bigram will
    /// routinely have one half written in each. The pair must still be learnt
    /// under the identity, not under whichever rendering reached the document —
    /// otherwise `我 ê` learnt in mixed script would never predict `ê` again.
    func testABigramWithOneHalfInTheOtherScript_reportsTheSameIdentity() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        let first = try commitWholeBuffer("tai", manager, executing: executor)
        let second = try composeAndTakeWholeBufferCandidate(manager, executing: executor, "gi")
        _ = manager.commitCandidate(second, script: .alternate, executing: executor)

        XCTAssertEqual(
            nextWord.handshakes,
            [
                .selected(text: first.displayText, roman: first.canonicalTl),
                .selected(text: second.displayText, roman: second.canonicalTl),
            ],
            "the identity pair is reported, whichever script the document got",
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

        let (outcome, commit) = manager.commitCandidate(
            romanOnly, script: .alternate, executing: executor,
        )

        XCTAssertEqual(outcome, .ignored)
        XCTAssertNil(commit, "a commit that wrote nothing earns no auto space")
        XCTAssertEqual(
            usage.recorded, [],
            "a commit that wrote nothing teaches nothing",
        )
    }

    /// Sentence-end punctuation typed straight into the host is reported: the
    /// engine ends the context on it, which is what stops the last word of one
    /// sentence being learned as the predecessor of the first word of the next.
    func testAFullStopBetweenTwoCommits_isReportedBetweenThem() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        let first = try commitWholeBuffer("tai", manager, executing: executor)
        manager.noteCharacterTypedOutsideComposition("。")
        let second = try commitWholeBuffer("gi", manager, executing: executor)

        XCTAssertEqual(nextWord.reported, [first.displayText, "。", second.displayText])
    }

    func testACommaBetweenTwoCommits_isReportedBetweenThem() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        let first = try commitWholeBuffer("tai", manager, executing: executor)
        manager.noteCharacterTypedOutsideComposition("、")
        let second = try commitWholeBuffer("gi", manager, executing: executor)

        // A comma is noise, so the engine keeps the context across it.
        XCTAssertEqual(nextWord.reported, [first.displayText, "、", second.displayText])
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

        XCTAssertFalse(nextWord.reported.contains("x"), "\(nextWord.reported)")
    }

    func testANewSession_forgetsTheContextFromTheOldOne() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        let first = try commitWholeBuffer("tai", manager, executing: executor)
        // A session change is usually a change of application: what was typed in
        // the last one must not seed what is typed in the next.
        manager.startNewSession()
        let second = try commitWholeBuffer("gi", manager, executing: executor)

        XCTAssertEqual(nextWord.reported, [first.displayText, "∅", second.displayText])
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

        let first = try commitWholeBuffer("tai", manager, executing: executor)
        for character in "gi" {
            manager.append(String(character), executing: executor)
        }
        manager.commitComposition(thenInsert: "。", executing: executor)
        let second = try commitWholeBuffer("gi", manager, executing: executor)

        XCTAssertEqual(
            nextWord.reported,
            [first.displayText, "∅", second.displayText],
            "under-learning one pair is the safe half; pairing 台 with the word after the "
                + "full stop would be learning something the user never typed",
        )
    }

    // MARK: - Mirror

    /// The fetch response carries the authoritative composition state, and a
    /// fetch that reports the engine idle means the composition is gone. A fetch
    /// that dropped that answer would leave the mirror claiming a composition
    /// the engine no longer had.
    func testFetchCandidates_afterTheEngineWasResetUnderneathIt_updatesTheMirror() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()
        _ = try compose("taigi", manager, executing: executor)
        XCTAssertTrue(manager.isComposing)

        // Resets the engine to Idle without going through the manager, which is
        // what a generation change does in production.
        let other = try TestFixtures.makeComposingManager(
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
            usage: usage,
            nextWord: nextWord,
            startingGeneration: TestFixtures.generationCounter.next(),
        )
    }

    /// What committing `candidate` whole reports: the identity pair, its Hanji
    /// for the learned-phrase touch, and the recording setting.
    private func expectedUsage(of candidate: ContinuousCandidate, frequencyRecording: Bool = true) -> Usage {
        Usage(
            displayText: candidate.displayText,
            canonicalTl: candidate.canonicalTl,
            hanji: candidate.hanji,
            isFrequencyRecordingEnabled: frequencyRecording,
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
        // Two scripts, so `.alternate` has something to write: §34 puts the
        // one-script literal first on the desktop, and it declines the flip.
        return try XCTUnwrap(
            candidates.first {
                $0.hanji != nil && $0.consumedSpanEnd >= UInt32(manager.rawInput.utf8.count)
            },
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
