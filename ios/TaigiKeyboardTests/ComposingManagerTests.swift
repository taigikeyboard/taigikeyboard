@testable import TaigiKeyboard
import XCTest

/// Tests for `ComposingManager` — the iOS platform wrapper over
/// `ComposingState`. Focus:
/// - Published mirror (isComposing / rawInput / composingText),
/// - Delegate call order via the platform-neutral `Effect` enum,
/// - Public API routing to the right intents.
///
/// Pure-state behavior (phase transitions, effect ordering per intent) is
/// covered more densely in `ComposingStateTests`; this file verifies the
/// wrapper wires the engine's transitions to published state + delegate
/// correctly.
final class ComposingManagerTests: XCTestCase {
    /// Spy that records every `execute(_:)` call so tests can assert the
    /// wrapper fans out effects in the order the engine emitted them.
    private final class DelegateSpy: ComposingDelegate {
        var effects: [RustEngineBridge.ComposingTransition.Effect] = []

        func execute(_ effect: RustEngineBridge.ComposingTransition.Effect) {
            effects.append(effect)
        }
    }

    private var manager: ComposingManager!
    private var spy: DelegateSpy!

    /// Strictly-increasing generation seed, one bump per test (process-wide,
    /// XCTest runs methods serially within a process).
    private static var generationSeed: UInt64 = 0

    override func setUp() {
        super.setUp()
        manager = ComposingManager()
        spy = DelegateSpy()
        manager.delegate = spy

        // Test isolation. The composing engine is a PROCESS-WIDE Rust singleton
        // (`EngineHandle::instance()`, engine/composing/src/handle.rs:39); a fresh
        // `ComposingManager()` does NOT reset it. The singleton only drops to a
        // clean `EngineState::default()` on a generation MISMATCH (handle.rs:61-65)
        // — `Intent::Reset` alone is phase-scoped and narrower. Every fresh manager
        // starts at generation 1, so once the first test sets `last_generation = 1`,
        // no later test mismatches and the singleton leaks the prior test's
        // composing / next-word state (e.g. rawInput "ab" → next test sees "aba",
        // or stray `nextWordClearForNewComposing` effects). Bump to a strictly
        // increasing per-test generation so each test's first engine request forces
        // the full reset. Bump count is bounded by tests-per-process (trivial).
        Self.generationSeed += 1
        for _ in 0 ..< Self.generationSeed {
            manager.bumpGeneration()
        }
    }

    override func tearDown() {
        manager = nil
        spy = nil
        super.tearDown()
    }

    // MARK: - Initial State

    func testInitialState_isIdle() {
        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "")
        XCTAssertEqual(manager.composingText, "")
        XCTAssertTrue(spy.effects.isEmpty)
    }

    // MARK: - startComposing / appendCharacter

    func testStartComposing_entersComposingAndFiresUpdateThenRefresh() {
        manager.startComposing(with: "a")

        XCTAssertTrue(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "a")
        XCTAssertEqual(spy.effects, [
            .updatePreedit(manager.composingText),
            .refreshCandidates,
        ])
    }

    func testAppendCharacter_whenIdle_startsComposing() {
        manager.appendCharacter("a")

        XCTAssertTrue(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "a")
        XCTAssertEqual(spy.effects, [
            .updatePreedit(manager.composingText),
            .refreshCandidates,
        ])
    }

    func testAppendCharacter_whenComposing_appendsToRawInput() {
        manager.startComposing(with: "a")
        spy.effects.removeAll()

        manager.appendCharacter("b")

        XCTAssertEqual(manager.rawInput, "ab")
        XCTAssertEqual(spy.effects, [
            .updatePreedit(manager.composingText),
            .refreshCandidates,
        ])
    }

    // MARK: - replaceLastCharacter

    func testReplaceLastCharacter_replacesTailAndKeepsComposing() {
        manager.startComposing(with: "ab")
        spy.effects.removeAll()

        manager.replaceLastCharacter(with: "c")

        XCTAssertEqual(manager.rawInput, "ac")
        XCTAssertTrue(manager.isComposing)
        // At wrapper level we only verify the effect list matches what the
        // engine emitted.
        XCTAssertEqual(spy.effects, [
            .updatePreedit(manager.composingText),
            .refreshCandidates,
        ])
    }

    func testReplaceLastCharacter_whenIdle_isNoop() {
        manager.replaceLastCharacter(with: "x")

        XCTAssertFalse(manager.isComposing)
        XCTAssertTrue(spy.effects.isEmpty)
    }

    // MARK: - appendHyphen

    func testAppendHyphen_behavesAsAppendCharacter() {
        manager.startComposing(with: "a")
        spy.effects.removeAll()

        manager.appendHyphen()

        XCTAssertEqual(manager.rawInput, "a-")
    }

    // MARK: - deleteBackward

    func testDeleteBackward_whenMultipleChars_shortensRaw() {
        manager.startComposing(with: "ab")
        spy.effects.removeAll()

        manager.deleteBackward()

        XCTAssertTrue(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "a")
        XCTAssertEqual(spy.effects, [
            .updatePreedit(manager.composingText),
            .refreshCandidates,
        ])
    }

    func testDeleteBackward_whenSingleChar_exitsWithOrderedEffects() {
        manager.startComposing(with: "a")
        spy.effects.removeAll()

        manager.deleteBackward()

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "")
        // v3.5.8 Continuous: the single composing char lives in the PREEDIT,
        // never the document, so deleting it clears the preedit + exits to Idle
        // with NO DeleteBackwardFromDocument; the terminal effect is
        // NextWordClearForNewComposing (matches engine/composing/tests/
        // continuous_phase.rs::delete_backward_under_continuous_with_empty_state_exits_to_idle).
        XCTAssertEqual(spy.effects, [
            .clearPreeditWithoutCommit,
            .clearCandidates,
            .nextWordClearForNewComposing,
        ])
    }

    func testDeleteBackward_whenIdle_isNoop() {
        manager.deleteBackward()

        XCTAssertFalse(manager.isComposing)
        XCTAssertTrue(spy.effects.isEmpty)
    }

    // MARK: - commitComposition

    func testCommitComposition_insertsDerivedTextAndExits() {
        manager.startComposing(with: "hello")
        let derived = manager.composingText
        spy.effects.removeAll()

        manager.commitComposition()

        XCTAssertFalse(manager.isComposing)
        // startComposing composes in Phase::Continuous (R12);
        // `commitComposition` routes through CommitRaw, which under Continuous
        // commits `derived_display(pending)` and fires the terminal
        // NextWordWordSelected (records the association — Model B; matches
        // engine/composing/tests/continuous_phase.rs::commit_raw_under_continuous_commits_derived_display_and_fires_nextword
        // and testCommitRawInput below). roman carries the raw buffer "hello".
        XCTAssertEqual(spy.effects, [
            .commitTextReplacingPreedit(derived),
            .clearCandidates,
            .resetCandidateContext,
            .nextWordWordSelected(text: derived, roman: "hello", triggerPrediction: true, preceding: []),
        ])
    }

    func testCommitComposition_whenIdle_isNoop() {
        manager.commitComposition()
        XCTAssertTrue(spy.effects.isEmpty)
    }

    // MARK: - commitPreeditThenInsertExternal (emoji / clipboard path)

    // INVARIANT_COMPOSING_EXTERNAL_INSERT_COMMITS_PREEDIT_ATOMICALLY (behavioral-invariants.md §13)
    func testCommitPreeditThenInsertExternal_whenComposing_commitsAtomicallyWithExternalText() {
        // Pins the iOS side of the parity fix with Android
        // `MediaInputManager.sendEmojiKeyPress`. Wrapper must emit a single
        // `.commitTextReplacingPreedit` carrying the derived preedit + the
        // external text — never a bare `.updatePreedit("")` or clear pair.
        manager.startComposing(with: "hello")
        let derived = manager.composingText
        spy.effects.removeAll()

        manager.commitPreeditThenInsertExternal("😀")

        XCTAssertFalse(manager.isComposing)
        // Under Continuous the pending derived + external commit atomically,
        // then NextWordClearForNewComposing (matches engine/composing/tests/
        // continuous_phase.rs::commit_preedit_then_insert_external_under_continuous_combines_pending_and_external).
        XCTAssertEqual(spy.effects, [
            .commitTextReplacingPreedit(derived + "😀"),
            .clearCandidates,
            .resetCandidateContext,
            .nextWordClearForNewComposing,
        ])
    }

    func testCommitPreeditThenInsertExternal_whenIdle_insertsExternalTextOnly() {
        manager.commitPreeditThenInsertExternal("😀")

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(spy.effects, [.commitTextReplacingPreedit("😀")])
    }

    func testCommitPreeditThenInsertExternal_withEmptyText_whenComposing_isNoop() {
        manager.startComposing(with: "abc")
        spy.effects.removeAll()

        manager.commitPreeditThenInsertExternal("")

        XCTAssertTrue(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "abc")
        XCTAssertTrue(spy.effects.isEmpty)
    }

    // MARK: - reset

    /// INVARIANT_composing_clear_preedit_does_not_commit — the wrapper must
    /// emit `.clearPreeditWithoutCommit` (never `.commitTextReplacingPreedit`)
    /// on `.reset`, mirroring the Android binding's pre-zero before
    /// `finishComposingText()`. UIKit-side proxy integration is not covered
    /// here — this project has no UI test harness for `UITextDocumentProxy`.
    func testReset_whenComposing_returnsToIdleWithoutInserting() {
        manager.startComposing(with: "abc")
        spy.effects.removeAll()

        manager.reset()

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "")
        XCTAssertEqual(manager.composingText, "")
        // Under Continuous, reset exits to Idle with the composing clear pair +
        // a terminal NextWordClearForNewComposing (matches engine/composing/tests/
        // continuous_phase.rs::reset_under_continuous_emits_nextword_clear_in_addition_to_composing_pair).
        // The clear-not-commit invariant below is unaffected.
        XCTAssertEqual(spy.effects, [
            .clearPreeditWithoutCommit,
            .clearCandidates,
            .nextWordClearForNewComposing,
        ])
        XCTAssertFalse(spy.effects.contains { effect in
            if case .commitTextReplacingPreedit = effect {
                return true
            }
            return false
        })
    }

    /// INVARIANT_composing_reset_when_idle_is_noop — also pinned at the pure
    /// engine layer in `ComposingStateTests.testReset_whenIdle_emitsEmptyEffects`.
    func testReset_whenIdle_emitsNoEffects() {
        manager.reset()

        XCTAssertFalse(manager.isComposing)
        XCTAssertTrue(spy.effects.isEmpty)
    }
}
