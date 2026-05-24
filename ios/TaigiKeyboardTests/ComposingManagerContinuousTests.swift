@testable import TaigiKeyboard
import XCTest

/// v3.5.8 Phase 7B — `ComposingManager` Continuous-input wrapper tests.
///
/// Scope (per Codex consult 2026-05-10, session 019e11d7):
/// - Auto-promotion: every raw-input mutation (`startComposing` /
///   `appendCharacter` / `appendHyphen` / `replaceLastCharacter`) issues a
///   synchronous `EnterContinuous` so the engine reaches `Phase::Continuous`.
///   Verified via `RustEngineBridge.composingFetchAtPos` returning a
///   non-nil candidates carrier (the only public observable distinguishing
///   `Phase::Composing` from `Phase::Continuous`).
/// - `fetchContinuousCandidates()` is read-only — does not mutate `rawInput`
///   or `isComposing` (mirrors `RustEngineBridgeContinuousTests`).
/// - `commitContinuous` mid-commit stays in Continuous (pending tail remains
///   addressable); full-commit exits to Idle and clears mirror.
/// - `resetContinuous` from Continuous returns to Idle and emits the abort
///   effect trio (`ClearPreeditWithoutCommit` + `ResetAutocomplete` +
///   `NextWordClearForNewComposing`).
///
/// Boundary coverage: `RustEngineBridgeContinuousTests` pins the FFI
/// contract; this file pins the `ComposingManager` observable-mirror +
/// effect-dispatch wrapper. UI integration (AutocompleteService /
/// ActionHandler tap decode) is verified by Codex post-impl + manual
/// dogfood — the wrappers are thin enough that mocking the keyboard
/// extension context for unit tests is not cost-justified.
final class ComposingManagerContinuousTests: XCTestCase {
    private final class DelegateSpy: ComposingDelegate {
        var effects: [RustEngineBridge.ComposingTransition.Effect] = []

        func execute(_ effect: RustEngineBridge.ComposingTransition.Effect) {
            effects.append(effect)
        }
    }

    private var manager: ComposingManager!
    private var spy: DelegateSpy!
    private let toggles = ToneToggles(isDoubleTapOOEnabled: false, isDoubleTapNNEnabled: false)

    override class func setUp() {
        super.setUp()
        RustEngineBridge.install()
    }

    override func setUp() {
        super.setUp()
        manager = ComposingManager()
        spy = DelegateSpy()
        manager.delegate = spy
        // Force engine to Idle before each test. The `EngineHandle` singleton
        // persists across tests; `manager.reset()` issues `composingReset`
        // which the engine accepts unconditionally and returns to Idle.
        manager.reset()
        spy.effects.removeAll()
    }

    override func tearDown() {
        manager = nil
        spy = nil
        super.tearDown()
    }

    // MARK: - Auto-promotion to Continuous

    /// Pins the Codex Fork A modification (synchronous promotion). After
    /// `appendCharacter`, the engine MUST be in `Phase::Continuous` —
    /// observable via `FetchAtPos.candidates` going from nil to non-nil.
    func testAppendCharacter_PromotesToContinuous() {
        manager.appendCharacter("t")
        manager.appendCharacter("s")
        manager.appendCharacter("u")
        manager.appendCharacter("a")

        let result = RustEngineBridge.composingFetchAtPos(
            mode: .tl,
            toggles: toggles,
            generation: 1, // ComposingManager's default currentGeneration
        )
        XCTAssertNotNil(
            result.candidates,
            "appendCharacter must auto-promote to Phase::Continuous (FetchAtPos returns Some)",
        )
    }

    func testStartComposing_PromotesToContinuous() {
        manager.startComposing(with: "tsua")

        let result = RustEngineBridge.composingFetchAtPos(
            mode: .tl,
            toggles: toggles,
            generation: 1,
        )
        XCTAssertNotNil(
            result.candidates,
            "startComposing must auto-promote to Phase::Continuous",
        )
    }

    func testAppendHyphen_PromotesToContinuous() {
        manager.startComposing(with: "tai")
        manager.appendHyphen()

        let result = RustEngineBridge.composingFetchAtPos(
            mode: .tl,
            toggles: toggles,
            generation: 1,
        )
        XCTAssertNotNil(
            result.candidates,
            "appendHyphen must keep engine in Phase::Continuous",
        )
    }

    func testReplaceLastCharacter_PromotesToContinuous() {
        manager.startComposing(with: "tsuab")
        manager.replaceLastCharacter(with: "c")

        let result = RustEngineBridge.composingFetchAtPos(
            mode: .tl,
            toggles: toggles,
            generation: 1,
        )
        XCTAssertNotNil(
            result.candidates,
            "replaceLastCharacter must keep engine in Phase::Continuous",
        )
    }

    // MARK: - fetchContinuousCandidates

    /// `fetchContinuousCandidates` is a two-phase wrapper over
    /// `composingFetchAtPos` (Phase 9.3b): a neutral fetch learns candidate
    /// `displayText` keys, then `user_frequency.db` is batch-queried and a
    /// populated fetch re-ranks. Unit tests skip phase 2 because (a) the
    /// lexicon FST is not installed in this process so phase 1 returns an
    /// empty candidate carrier, and (b) `CompositionRoot.userFrequencyService`
    /// is unopened in this test harness so the cold-start guard short-circuits
    /// even if phase 1 ever produced hits. The bridge wire shape for the
    /// populated path is pinned in `RustEngineBridgeContinuousTests`; the
    /// boost arithmetic is pinned in `engine/lexicon/tests/user_freq_plumb.rs`.
    ///
    /// On Idle the engine returns nil candidates → wrapper exposes []. The
    /// caller (AutocompleteService) cannot distinguish "not Continuous" from
    /// "Continuous but no FST hits" — both fall through to the lexicon path.
    func testFetchContinuousCandidates_FromIdle_ReturnsEmpty() {
        let candidates = manager.fetchContinuousCandidates()
        XCTAssertTrue(
            candidates.isEmpty,
            "fetchContinuousCandidates from Idle must return [] (carrier-nil flattens)",
        )
    }

    /// FetchAtPos is read-only by contract. Wrapper must not flip
    /// `isComposing` or alter `rawInput` even though `apply()` runs on the
    /// returned snapshot to mirror engine state.
    func testFetchContinuousCandidates_DoesNotMutateBuffer() {
        manager.startComposing(with: "tsua")
        let rawBefore = manager.rawInput
        let isComposingBefore = manager.isComposing
        spy.effects.removeAll()

        _ = manager.fetchContinuousCandidates()

        XCTAssertEqual(manager.rawInput, rawBefore, "rawInput must not change across fetch")
        XCTAssertEqual(manager.isComposing, isComposingBefore, "isComposing must not change")
        XCTAssertTrue(spy.effects.isEmpty, "FetchAtPos contract: zero effects emitted")
    }

    // MARK: - commitContinuous

    /// Mid-commit (consumed_bytes < pending.utf8.count): engine consumes
    /// the leading bytes, leaves the tail in `Phase::Continuous`. Wrapper
    /// inlines `applyAsSelfCommit` to inspect `transition.effects` and
    /// returns `(didCommit: true, didFinalCommit: false)` so the caller
    /// (ActionHandler) records frequency without inserting a final-commit
    /// trailing space.
    func testCommitContinuous_PartialConsume_StaysInContinuous() {
        manager.startComposing(with: "taibak")
        spy.effects.removeAll()

        let outcome = manager.commitContinuous(
            displayText: "台",
            consumedBytes: UInt32("tai".utf8.count),
            syllableCount: 1,
        )

        XCTAssertTrue(manager.isComposing, "Mid-commit keeps Continuous active")
        XCTAssertEqual(manager.rawInput, "bak", "Pending tail remains after partial consume")
        XCTAssertTrue(outcome.didCommit, "Mid-commit emits CommitTextReplacingPreedit")
        XCTAssertFalse(outcome.didFinalCommit, "Mid-commit does not exit Continuous")
    }

    /// Full-commit (consumed_bytes >= pending.utf8.count): engine exits to
    /// Idle, wrapper clears mirror, returns `(true, true)`.
    func testCommitContinuous_FullConsume_ExitsToIdle() {
        manager.startComposing(with: "tai")
        spy.effects.removeAll()

        let outcome = manager.commitContinuous(
            displayText: "台",
            consumedBytes: UInt32("tai".utf8.count),
            syllableCount: 1,
        )

        XCTAssertFalse(manager.isComposing, "Full-commit must exit to Idle")
        XCTAssertEqual(manager.rawInput, "")
        XCTAssertEqual(manager.selectedCandidateIndex, -1)
        XCTAssertTrue(outcome.didCommit, "Full-commit emits CommitTextReplacingPreedit")
        XCTAssertTrue(outcome.didFinalCommit, "Full-commit exits Continuous → didFinalCommit")
    }

    /// Codex PR #257 r3214932308 regression: when the platform bumps
    /// `currentGeneration` between the user's last edit and a stale tap
    /// landing on a Continuous suggestion, the engine silently resets to
    /// Idle in `engine/composing/src/handle.rs:61-65` BEFORE applying the
    /// CommitContinuous intent. The intent then no-ops (phase mismatch),
    /// emitting zero effects. The wrapper MUST return `(false, false)`
    /// so the caller does not record frequency / insert a stray space
    /// for text that was never written.
    func testCommitContinuous_StaleAfterGenerationBump_ReturnsFalseFalse() {
        manager.startComposing(with: "tai")
        spy.effects.removeAll()
        // Simulate input-context switch firing `textWillChange` and bumping
        // generation while the autocomplete context still holds the stale
        // Continuous candidate.
        manager.bumpGeneration()

        let outcome = manager.commitContinuous(
            displayText: "台",
            consumedBytes: UInt32("tai".utf8.count),
            syllableCount: 1,
        )

        XCTAssertFalse(
            outcome.didCommit,
            "Stale-generation tap must not be reported as committed (engine noops)",
        )
        XCTAssertFalse(
            outcome.didFinalCommit,
            "didFinalCommit must imply didCommit; both stay false on noop",
        )
        let emittedCommitText = spy.effects.contains { effect in
            if case .commitTextReplacingPreedit = effect { return true }
            return false
        }
        XCTAssertFalse(
            emittedCommitText,
            "Engine must not emit CommitTextReplacingPreedit on stale-generation noop",
        )
    }

    // MARK: - resetContinuous

    /// `resetContinuous` from Continuous emits the engine-defined abort
    /// trio. Order is asserted at the engine layer
    /// (`engine/composing/tests/continuous_phase.rs`); the wrapper just
    /// fans the effects to the delegate.
    func testResetContinuous_FromContinuous_EmitsAbortTrio() {
        manager.startComposing(with: "tsua")
        spy.effects.removeAll()

        manager.resetContinuous()

        XCTAssertFalse(manager.isComposing, "Reset must exit to Idle")
        XCTAssertEqual(manager.rawInput, "")
        // Abort trio order pinned by engine: ClearPreeditWithoutCommit ->
        // ResetAutocomplete -> NextWordClearForNewComposing.
        XCTAssertEqual(spy.effects, [
            .clearPreeditWithoutCommit,
            .resetAutocomplete,
            .nextWordClearForNewComposing,
        ])
    }

    func testResetContinuous_FromIdle_IsNoop() {
        manager.resetContinuous()

        XCTAssertFalse(manager.isComposing)
        XCTAssertTrue(spy.effects.isEmpty, "Reset from Idle emits no effects")
    }
}
