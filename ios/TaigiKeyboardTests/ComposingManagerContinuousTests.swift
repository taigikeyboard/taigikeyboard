@testable import TaigiKeyboard
import XCTest

/// v3.5.8 Phase 7B — `ComposingManager` Continuous-input wrapper tests.
///
/// Scope (per Codex consult 2026-05-10, session 019e11d7):
/// - One engine call per keystroke: every raw-input mutation
///   (`startComposing` / `appendCharacter` / `appendHyphen` /
///   `tpsKey`) leaves a composition `FetchAtPos` answers with
///   a candidates carrier — no separate promotion request (R12).
/// - `fetchContinuousCandidates()` is read-only — does not mutate `rawInput`
///   or `isComposing` (mirrors `RustEngineBridgeContinuousTests`).
/// - `commitContinuous` mid-commit stays in Continuous (pending tail remains
///   addressable); full-commit exits to Idle and clears mirror.
/// - `reset` from a composition returns to Idle and emits the abort
///   effects (`ClearPreeditWithoutCommit` + `ClearCandidates`; the engine drops
///   the next-word handshake, no envelope is set).
///
/// Boundary coverage: `RustEngineBridgeContinuousTests` pins the FFI
/// contract; this file pins the `ComposingManager` observable-mirror +
/// effect-dispatch wrapper. UI integration (TaigiAutocompleteService /
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
    private let settings = StubEngineSettings()

    /// The 台 / `tâi` pick over the leading `tai` of the pending buffer.
    private let taiPick = RustEngineBridge.ContinuousPick(
        script: .lead,
        roman: "tâi",
        canonicalText: "台",
        associationTl: "tâi",
        hanji: "台",
        consumedBytes: UInt32("tai".utf8.count),
        syllableCount: 1,
    )

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

    // MARK: - One call per keystroke

    /// After `appendCharacter` alone the engine holds a composition —
    /// observable via `FetchAtPos.candidates` being non-nil (R12: no
    /// separate promotion request).
    func testAppendCharacter_ComposesInOneCall() {
        manager.appendCharacter("t")
        manager.appendCharacter("s")
        manager.appendCharacter("u")
        manager.appendCharacter("a")

        let result = RustEngineBridge.composingFetchAtPos(
            settings: settings,
            generation: 1, // ComposingManager's default currentGeneration
            nowMs: 0,
        )
        XCTAssertNotNil(
            result.candidates,
            "appendCharacter alone must leave a composition FetchAtPos reads",
        )
    }

    func testStartComposing_ComposesInOneCall() {
        manager.startComposing(with: "tsua")

        let result = RustEngineBridge.composingFetchAtPos(
            settings: settings,
            generation: 1,
            nowMs: 0,
        )
        XCTAssertNotNil(
            result.candidates,
            "startComposing alone must leave a composition FetchAtPos reads",
        )
    }

    func testAppendHyphen_ComposesInOneCall() {
        manager.startComposing(with: "tai")
        manager.appendHyphen()

        let result = RustEngineBridge.composingFetchAtPos(
            settings: settings,
            generation: 1,
            nowMs: 0,
        )
        XCTAssertNotNil(
            result.candidates,
            "appendHyphen must keep engine in Phase::Continuous",
        )
    }

    func testTpsKey_ComposesInOneCall() {
        manager.tpsKey("ㄗ")
        manager.tpsKey("ㄧ")

        var tpsSettings = settings
        tpsSettings.inputMode = .tps
        let result = RustEngineBridge.composingFetchAtPos(
            settings: tpsSettings,
            generation: 1,
            nowMs: 0,
        )
        XCTAssertNotNil(
            result.candidates,
            "tpsKey must keep engine in Phase::Continuous",
        )
    }

    // MARK: - fetchContinuousCandidates

    /// `fetchContinuousCandidates` is one `composingFetchAtPos`: the engine
    /// reads the user's data itself (user-data-engine-roadmap P7b). In this
    /// process the lexicon FST is not installed, so the carrier is empty, and
    /// the user data is never opened, so ranking is neutral. The bridge wire
    /// shape is pinned in `RustEngineBridgeContinuousTests`; the ranking with
    /// the user's data in `engine/dispatch/tests/user_data_reads.rs`.
    ///
    /// On Idle the engine returns nil candidates → wrapper exposes []. The
    /// caller (TaigiAutocompleteService) cannot distinguish "not Continuous" from
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
    /// the leading bytes, leaves the tail in `Phase::Continuous` and answers
    /// `.nailed`, so the caller (ActionHandler) inserts no final-commit
    /// trailing space.
    func testCommitContinuous_PartialConsume_StaysInContinuous() {
        manager.startComposing(with: "taibak")
        spy.effects.removeAll()

        let outcome = manager.commitContinuous(taiPick)

        XCTAssertTrue(manager.isComposing, "Mid-commit keeps Continuous active")
        XCTAssertEqual(manager.rawInput, "bak", "Pending tail remains after partial consume")
        XCTAssertEqual(outcome, .nailed, "Mid-commit nails the segment and keeps composing (Model B)")
    }

    /// Full-commit (consumed_bytes >= pending.utf8.count): engine exits to
    /// Idle, wrapper clears mirror, answers `.finalized`.
    func testCommitContinuous_FullConsume_ExitsToIdle() {
        manager.startComposing(with: "tai")
        spy.effects.removeAll()

        let outcome = manager.commitContinuous(taiPick)

        XCTAssertFalse(manager.isComposing, "Full-commit must exit to Idle")
        XCTAssertEqual(manager.rawInput, "")
        guard case .finalized = outcome else {
            return XCTFail("Full-commit exits Continuous → .finalized, got \(outcome)")
        }
    }

    /// Codex PR #257 r3214932308 regression: when the platform bumps
    /// `currentGeneration` between the user's last edit and a stale tap
    /// landing on a Continuous suggestion, the engine silently resets to
    /// Idle in `engine/composing/src/handle.rs:61-65` BEFORE applying the
    /// CommitContinuous intent. The intent then no-ops (phase mismatch),
    /// emitting zero effects, and answers `.ignored`, so the caller inserts
    /// no stray space and the engine counts nothing for text that was never
    /// written.
    func testCommitContinuous_StaleAfterGenerationBump_IsIgnored() {
        manager.startComposing(with: "tai")
        spy.effects.removeAll()
        // Simulate input-context switch firing `textWillChange` and bumping
        // generation while the autocomplete context still holds the stale
        // Continuous candidate.
        manager.bumpGeneration()

        let outcome = manager.commitContinuous(taiPick)

        XCTAssertEqual(outcome, .ignored, "Stale-generation tap must not be reported as committed (engine noops)")
        let emittedCommitText = spy.effects.contains { effect in
            if case .commitTextReplacingPreedit = effect {
                return true
            }
            return false
        }
        XCTAssertFalse(
            emittedCommitText,
            "Engine must not emit CommitTextReplacingPreedit on stale-generation noop",
        )
    }

    // MARK: - reset

    /// `reset` from a composition emits the engine-defined abort
    /// effects. Order is asserted at the engine layer
    /// (`engine/composing/tests/continuous_phase.rs`); the wrapper just
    /// fans the effects to the delegate.
    func testReset_FromComposition_EmitsAbortEffects() {
        manager.startComposing(with: "tsua")
        spy.effects.removeAll()

        manager.reset()

        XCTAssertFalse(manager.isComposing, "Reset must exit to Idle")
        XCTAssertEqual(manager.rawInput, "")
        // Abort order pinned by engine: ClearPreeditWithoutCommit ->
        // ClearCandidates (the next-word handshake is dropped: no envelope).
        XCTAssertEqual(spy.effects, [
            .clearPreeditWithoutCommit,
            .clearCandidates,
        ])
    }

    func testReset_FromIdle_IsNoop() {
        manager.reset()

        XCTAssertFalse(manager.isComposing)
        XCTAssertTrue(spy.effects.isEmpty, "Reset from Idle emits no effects")
    }
}
