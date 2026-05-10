@testable import TaigiKeyboard
import XCTest

/// Tests for `ComposingManager` — the iOS platform wrapper over
/// `ComposingState`. Focus:
/// - Published mirror (isComposing / rawInput / composingText / selectedCandidateIndex),
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

    override func setUp() {
        super.setUp()
        manager = ComposingManager()
        spy = DelegateSpy()
        manager.delegate = spy
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
        XCTAssertEqual(manager.selectedCandidateIndex, -1)
        XCTAssertTrue(spy.effects.isEmpty)
    }

    // MARK: - startComposing / appendCharacter

    func testStartComposing_entersComposingAndFiresUpdateThenPerform() {
        manager.startComposing(with: "a")

        XCTAssertTrue(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "a")
        XCTAssertEqual(manager.selectedCandidateIndex, 0)
        XCTAssertEqual(spy.effects, [
            .updatePreedit(manager.composingText),
            .performAutocomplete,
        ])
    }

    func testAppendCharacter_whenIdle_startsComposing() {
        manager.appendCharacter("a")

        XCTAssertTrue(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "a")
        XCTAssertEqual(spy.effects, [
            .updatePreedit(manager.composingText),
            .performAutocomplete,
        ])
    }

    func testAppendCharacter_whenComposing_appendsAndResetsSelectedIndex() {
        manager.startComposing(with: "a")
        spy.effects.removeAll()

        manager.appendCharacter("b")

        XCTAssertEqual(manager.rawInput, "ab")
        XCTAssertEqual(manager.selectedCandidateIndex, 0)
        XCTAssertEqual(spy.effects, [
            .updatePreedit(manager.composingText),
            .performAutocomplete,
        ])
    }

    // MARK: - replaceLastCharacter

    func testReplaceLastCharacter_replacesTailAndKeepsComposing() {
        manager.startComposing(with: "ab")
        spy.effects.removeAll()

        manager.replaceLastCharacter(with: "c")

        XCTAssertEqual(manager.rawInput, "ac")
        XCTAssertTrue(manager.isComposing)
        // Contract asserted in ComposingStateTests: selectedCandidateIndex
        // is preserved across replaceLast. At wrapper level we only verify
        // the effect list matches what the engine emitted.
        XCTAssertEqual(spy.effects, [
            .updatePreedit(manager.composingText),
            .performAutocomplete,
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
            .performAutocomplete,
        ])
    }

    func testDeleteBackward_whenSingleChar_exitsWithOrderedEffects() {
        manager.startComposing(with: "a")
        spy.effects.removeAll()

        manager.deleteBackward()

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "")
        XCTAssertEqual(manager.selectedCandidateIndex, -1)
        XCTAssertEqual(spy.effects, [
            .clearPreeditWithoutCommit,
            .resetAutocomplete,
            .deleteBackwardFromDocument,
        ])
    }

    func testDeleteBackward_whenIdle_isNoop() {
        manager.deleteBackward()

        XCTAssertFalse(manager.isComposing)
        XCTAssertTrue(spy.effects.isEmpty)
    }

    // MARK: - commitComposition / commitRawInput

    func testCommitComposition_insertsDerivedTextAndExits() {
        manager.startComposing(with: "hello")
        let derived = manager.composingText
        spy.effects.removeAll()

        manager.commitComposition()

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.selectedCandidateIndex, -1)
        // v3.5.8 Phase 7B: startComposing auto-promotes to Phase::Continuous,
        // and `commitComposition` was rerouted through SelectSuggestion to
        // commit cleanly across all phases. SelectSuggestion under Continuous
        // emits the abort trio + the commit + NextWordClearForNewComposing
        // (engine/composing/tests/continuous_phase.rs::select_suggestion_under_continuous_commits_text_and_exits).
        XCTAssertEqual(spy.effects, [
            .commitTextReplacingPreedit(derived),
            .resetAutocomplete,
            .resetAutocompleteContext,
            .nextWordClearForNewComposing,
        ])
    }

    func testCommitComposition_whenIdle_isNoop() {
        manager.commitComposition()
        XCTAssertTrue(spy.effects.isEmpty)
    }

    func testCommitRawInput_insertsRawStringBypassingConversion() {
        manager.startComposing(with: "Hello")
        spy.effects.removeAll()

        manager.commitRawInput()

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.selectedCandidateIndex, -1)
        // v3.5.8 Phase 7B: same Continuous-aware reroute as commitComposition.
        // SelectSuggestion under Continuous emits 4 effects (3-effect Composing
        // path + NextWordClearForNewComposing for the Continuous abort).
        XCTAssertEqual(spy.effects, [
            .commitTextReplacingPreedit("Hello"),
            .resetAutocomplete,
            .resetAutocompleteContext,
            .nextWordClearForNewComposing,
        ])
    }

    func testCommitRawInput_whenIdle_isNoop() {
        manager.commitRawInput()
        XCTAssertTrue(spy.effects.isEmpty)
    }

    // MARK: - selectSuggestion / confirmSelectedCandidate

    func testSelectSuggestion_whenComposing_commitsAtomically() {
        manager.startComposing(with: "a")
        spy.effects.removeAll()

        manager.selectSuggestion(text: "picked")

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.selectedCandidateIndex, -1)
        XCTAssertEqual(spy.effects, [
            .commitTextReplacingPreedit("picked"),
            .resetAutocomplete,
            .resetAutocompleteContext,
        ])
    }

    func testSelectSuggestion_whenIdle_isNoop() {
        manager.selectSuggestion(text: "picked")

        XCTAssertFalse(manager.isComposing)
        XCTAssertTrue(spy.effects.isEmpty)
    }

    // MARK: - commitPreeditThenInsertExternal (emoji / clipboard path)

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
        XCTAssertEqual(manager.selectedCandidateIndex, -1)
        XCTAssertEqual(spy.effects, [
            .commitTextReplacingPreedit(derived + "😀"),
            .resetAutocomplete,
            .resetAutocompleteContext,
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

    func testConfirmSelectedCandidate_whenIndexValid_selectsAndReturnsTrue() {
        manager.startComposing(with: "a")
        spy.effects.removeAll()

        // selectedCandidateIndex defaults to 0 after startComposing, so
        // availableTexts[0] is what confirm picks.
        let confirmed = manager.confirmSelectedCandidate(availableTexts: ["zero", "one"])

        XCTAssertTrue(confirmed)
        XCTAssertFalse(manager.isComposing)
        XCTAssertTrue(spy.effects.contains(.commitTextReplacingPreedit("zero")))
    }

    func testConfirmSelectedCandidate_whenIndexOutOfRange_returnsFalseAndDoesNothing() {
        manager.startComposing(with: "a")
        manager.setSelectedCandidateIndex(5) // past the end of availableTexts
        spy.effects.removeAll()

        let confirmed = manager.confirmSelectedCandidate(availableTexts: ["only"])

        XCTAssertFalse(confirmed)
        XCTAssertTrue(manager.isComposing)
        XCTAssertTrue(spy.effects.isEmpty)
    }

    func testSetSelectedCandidateIndex_updatesWrapperAndEngineTogether() {
        manager.startComposing(with: "a")
        manager.setSelectedCandidateIndex(2)
        spy.effects.removeAll()

        // replaceLast preserves the current index, so the engine's 2 must
        // round-trip through the wrapper mirror.
        manager.replaceLastCharacter(with: "b")

        XCTAssertEqual(manager.selectedCandidateIndex, 2)
    }

    func testConfirmSelectedCandidate_whenIdle_returnsFalse() {
        let confirmed = manager.confirmSelectedCandidate(availableTexts: ["only"])

        XCTAssertFalse(confirmed)
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
        XCTAssertEqual(manager.selectedCandidateIndex, -1)
        XCTAssertEqual(spy.effects, [
            .clearPreeditWithoutCommit,
            .resetAutocomplete,
        ])
        XCTAssertFalse(spy.effects.contains { effect in
            if case .commitTextReplacingPreedit = effect { return true }
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
