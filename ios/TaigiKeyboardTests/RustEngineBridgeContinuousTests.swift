@testable import TaigiKeyboard
import XCTest

/// v3.5.8 Phase 7A — iOS bridge surface tests for the continuous-input slice.
///
/// Scope (per pre-impl Codex consult, Q4 = A):
/// - 4 ContinuousRequest wrappers compile + round-trip via the FFI seam,
/// - `ContinuousFetchResult` tri-state semantics (nil / empty / non-empty)
///   on the `candidates` field,
/// - 3 Phase 4 NextWord effect cases decode through `synthComposing`
///   instead of being dropped to nil (the pre-7A behavior).
///
/// Functional verification of dispatch logic is in
/// `engine/composing/tests/dispatch_continuous.rs`; engine state machine
/// coverage is in `engine/composing/tests/continuous_phase.rs`. This file
/// is the iOS-side boundary check.
final class RustEngineBridgeContinuousTests: XCTestCase {
    override class func setUp() {
        super.setUp()
        RustEngineBridge.install()
    }

    /// Per-test envelope generation. Engine `EngineHandle::handle` resets
    /// state on mismatch BEFORE applying the request, so every test starts
    /// with a clean phase regardless of prior test residue.
    private static var nextEnvelopeGen: UInt64 = 200_000
    private var envelopeGen: UInt64 = 0
    private let toggles = ToneToggles(isDoubleTapOOEnabled: false, isDoubleTapNNEnabled: false)

    override func setUp() {
        super.setUp()
        Self.nextEnvelopeGen &+= 1
        envelopeGen = Self.nextEnvelopeGen
    }

    // MARK: - EnterContinuous

    func testEnterContinuous_FromComposing_PreservesRawAndStaysComposing() {
        _ = RustEngineBridge.composingStart(
            "tsua", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let enter = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        XCTAssertTrue(enter.isComposing)
        XCTAssertEqual(enter.rawInput, "tsua")
        // EnterContinuous is a phase swap — no document mutation.
        XCTAssertTrue(enter.effects.isEmpty, "expected no effects, got \(enter.effects)")
    }

    func testEnterContinuous_FromIdle_NoOp() {
        let enter = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        XCTAssertFalse(enter.isComposing)
        XCTAssertEqual(enter.rawInput, "")
        XCTAssertTrue(enter.effects.isEmpty)
    }

    // MARK: - FetchAtPos tri-state

    func testFetchAtPos_FromIdle_CandidatesIsNil() {
        // Phase::Idle → engine returns snapshot without `continuous` carrier.
        let result = RustEngineBridge.composingFetchAtPos(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        XCTAssertNil(result.candidates, "Idle phase must yield nil candidates (carrier absent)")
        XCTAssertFalse(result.transition.isComposing)
        // Read-only RPC emits no effects.
        XCTAssertTrue(result.transition.effects.isEmpty)
    }

    func testFetchAtPos_FromComposing_NotYetContinuous_CandidatesIsNil() {
        _ = RustEngineBridge.composingStart(
            "tsua", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        // Composing phase is NOT Continuous — handle_fetch_at_pos returns
        // snapshot without continuous carrier.
        let result = RustEngineBridge.composingFetchAtPos(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        XCTAssertNil(result.candidates, "Composing phase must yield nil candidates")
        XCTAssertTrue(result.transition.isComposing)
    }

    func testFetchAtPos_FromContinuous_CandidatesNonNil() {
        // Continuous phase → carrier is present (Some). Whether it contains
        // hits depends on whether the lexicon was installed with a syllable
        // inventory. Unit tests do not install lexicon, so the carrier is
        // present-but-empty (the tri-state distinction we want to lock).
        _ = RustEngineBridge.composingStart(
            "tsua", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let result = RustEngineBridge.composingFetchAtPos(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        XCTAssertNotNil(
            result.candidates,
            "Continuous phase must yield non-nil candidates carrier (engine sets continuous=Some)",
        )
        XCTAssertTrue(result.transition.isComposing)
        XCTAssertTrue(
            result.transition.effects.isEmpty,
            "FetchAtPos is read-only; effects must be empty",
        )
    }

    // MARK: - ResetContinuous emits NextWordClearForNewComposing

    func testResetContinuous_FromContinuous_EmitsNextWordClearForNewComposing() {
        _ = RustEngineBridge.composingStart(
            "tsua", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let reset = RustEngineBridge.composingResetContinuous(generation: envelopeGen)
        XCTAssertFalse(reset.isComposing, "ResetContinuous must exit to Idle")
        let hasClear = reset.effects.contains { effect in
            if case .nextWordClearForNewComposing = effect { return true }
            return false
        }
        XCTAssertTrue(
            hasClear,
            "Phase 4 contract: ResetContinuous emits NextWordClearForNewComposing; got \(reset.effects)",
        )
    }

    // MARK: - CommitContinuous final-commit emits NextWordWordSelected

    func testCommitContinuous_FullConsume_EmitsNextWordWordSelected() {
        _ = RustEngineBridge.composingStart(
            "tai", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        // consumed_bytes = "tai".utf8.count → final commit
        let commit = RustEngineBridge.composingCommitContinuous(
            displayText: "台",
            consumedBytes: UInt32("tai".utf8.count),
            syllableCount: 1,
            mode: .tl,
            toggles: toggles,
            generation: envelopeGen,
        )
        let hasSelected = commit.effects.contains { effect in
            if case .nextWordWordSelected = effect { return true }
            return false
        }
        XCTAssertTrue(
            hasSelected,
            "Phase 4 final-commit contract: CommitContinuous emits NextWordWordSelected; got \(commit.effects)",
        )
    }

    // MARK: - CommitContinuous mid-commit emits NextWordUpdateLastSelectedWord

    func testCommitContinuous_PartialConsume_EmitsNextWordUpdateLastSelectedWord() {
        // Multi-syllable buffer; consume only the leading syllable so the
        // engine stays in Continuous (mid-commit branch). Phase 4 contract
        // emits NextWordUpdateLastSelectedWord (NOT WordSelected) in this
        // path because the user is composing a sentence, not finalizing.
        _ = RustEngineBridge.composingStart(
            "taibak", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let commit = RustEngineBridge.composingCommitContinuous(
            displayText: "台",
            consumedBytes: UInt32("tai".utf8.count),
            syllableCount: 1,
            mode: .tl,
            toggles: toggles,
            generation: envelopeGen,
        )
        XCTAssertTrue(commit.isComposing, "Mid-commit must stay in Continuous phase")
        XCTAssertEqual(commit.rawInput, "bak", "Pending tail should remain after mid-commit")
        let hasUpdate = commit.effects.contains { effect in
            if case .nextWordUpdateLastSelectedWord = effect { return true }
            return false
        }
        XCTAssertTrue(
            hasUpdate,
            "Phase 4 mid-commit contract: emits NextWordUpdateLastSelectedWord; got \(commit.effects)",
        )
    }

    // MARK: - Effect mapping completeness (no nil drops)

    /// Phase 7A removed the three `nil` returns at the bottom of
    /// `synthComposing` (pre-Phase-7A behavior). Verify by triggering a
    /// reset path that the engine populates with NextWord effects, and
    /// confirming none are silently dropped.
    func testNextWordEffects_AreNeverDroppedToNil() {
        _ = RustEngineBridge.composingStart(
            "tai", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let reset = RustEngineBridge.composingResetContinuous(generation: envelopeGen)
        // Pre-Phase-7A this would silently filter NextWord effects out.
        // Now they must appear in the effect list — the count cannot be
        // less than the engine emits.
        let nextWordEffects = reset.effects.filter { effect in
            switch effect {
            case .nextWordUpdateLastSelectedWord,
                 .nextWordWordSelected,
                 .nextWordClearForNewComposing:
                return true
            default:
                return false
            }
        }
        XCTAssertFalse(
            nextWordEffects.isEmpty,
            "ResetContinuous must surface at least one NextWord effect post-Phase-7A",
        )
    }

    // MARK: - FetchAtPos read-only invariance

    func testFetchAtPos_DoesNotMutateBuffer() {
        _ = RustEngineBridge.composingStart(
            "tsua", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let before = RustEngineBridge.composingQueryState(generation: envelopeGen)
        _ = RustEngineBridge.composingFetchAtPos(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let after = RustEngineBridge.composingQueryState(generation: envelopeGen)
        XCTAssertEqual(before.rawInput, after.rawInput, "rawInput must not change across FetchAtPos")
        XCTAssertEqual(before.isComposing, after.isComposing, "isComposing must not change")
        XCTAssertEqual(
            before.selectedCandidateIndex,
            after.selectedCandidateIndex,
            "selectedCandidateIndex must not change",
        )
    }
}
