@testable import TaigiKeyboard
import XCTest

/// Pure-state tests for the `ComposingState` engine — Foundation-only, no
/// delegate / no `@Published` fan-out. Named per G4-design §8 so the Phase
/// 0 behavioral invariants doc can reference the test cases directly.
///
/// Inputs avoid tone-marked display assertions (those are covered by
/// `ToneConverterTests`); most checks focus on phase transitions, effect
/// ordering, and `selectedCandidateIndex` invariants.
final class ComposingStateTests: XCTestCase {
    private let tl = InputMode.tl
    private let togglesOff = ToneToggles(isDoubleTapOOEnabled: false, isDoubleTapNNEnabled: false)

    // MARK: - Initial State

    func testInitialState_isIdle_withSelectedIndexMinusOne() {
        let state = ComposingState()
        XCTAssertFalse(state.isComposing)
        XCTAssertEqual(state.rawInput, "")
        XCTAssertEqual(state.selectedCandidateIndex, -1)
    }

    // MARK: - start / append effect ordering

    func testStart_emitsUpdatePreeditThenPerformAutocomplete() {
        var state = ComposingState()
        let t = state.apply(.start("a"), mode: tl, toneToggles: togglesOff)

        XCTAssertTrue(state.isComposing)
        XCTAssertEqual(state.rawInput, "a")
        XCTAssertEqual(state.selectedCandidateIndex, 0)
        XCTAssertEqual(t.newSelectedIndex, 0)
        XCTAssertEqual(t.effects, [.updatePreedit("a"), .performAutocomplete])
    }

    func testAppend_whenIdle_behavesAsStart() {
        var state = ComposingState()
        let t = state.apply(.append("a"), mode: tl, toneToggles: togglesOff)
        XCTAssertEqual(t.effects, [.updatePreedit("a"), .performAutocomplete])
        XCTAssertEqual(t.newSelectedIndex, 0)
    }

    func testAppend_whenComposing_appendsToRawAndResetsSelectedIndex() {
        var state = ComposingState()
        _ = state.apply(.start("a"), mode: tl, toneToggles: togglesOff)
        let t = state.apply(.append("b"), mode: tl, toneToggles: togglesOff)

        XCTAssertEqual(state.rawInput, "ab")
        XCTAssertEqual(t.newSelectedIndex, 0)
        XCTAssertEqual(t.effects, [.updatePreedit("ab"), .performAutocomplete])
    }

    func testAppendHyphen_appendsLiteralHyphen() {
        var state = ComposingState()
        _ = state.apply(.start("a"), mode: tl, toneToggles: togglesOff)
        let t = state.apply(.appendHyphen, mode: tl, toneToggles: togglesOff)

        XCTAssertEqual(state.rawInput, "a-")
        XCTAssertEqual(t.effects, [.updatePreedit("a-"), .performAutocomplete])
    }

    // MARK: - INVARIANT_composing_replace_last_preserves_selected_index

    func testReplaceLast_doesNotResetSelectedIndex() {
        // Externally bump the index to 3 (simulating a candidate-bar tap),
        // then verify replaceLast preserves it — unlike append, which would
        // snap back to 0.
        var state = ComposingState()
        _ = state.apply(.start("ab"), mode: tl, toneToggles: togglesOff)
        state.setSelectedCandidateIndex(3)
        let t = state.apply(.replaceLast("c"), mode: tl, toneToggles: togglesOff)

        XCTAssertEqual(state.rawInput, "ac")
        XCTAssertEqual(state.selectedCandidateIndex, 3)
        XCTAssertEqual(t.newSelectedIndex, 3)
    }

    func testAppend_resetsSelectedIndexBackToZero() {
        var state = ComposingState()
        _ = state.apply(.start("a"), mode: tl, toneToggles: togglesOff)
        state.setSelectedCandidateIndex(3)
        let t = state.apply(.append("b"), mode: tl, toneToggles: togglesOff)

        XCTAssertEqual(state.selectedCandidateIndex, 0)
        XCTAssertEqual(t.newSelectedIndex, 0)
    }

    func testReplaceLast_whenIdle_isNoop() {
        var state = ComposingState()
        let t = state.apply(.replaceLast("x"), mode: tl, toneToggles: togglesOff)
        XCTAssertFalse(state.isComposing)
        XCTAssertEqual(t.effects, [])
    }

    // MARK: - INVARIANT_composing_delete_order

    func testDeleteBackward_whenRawLengthOne_emitsClearResetDeleteInOrder() {
        var state = ComposingState()
        _ = state.apply(.start("a"), mode: tl, toneToggles: togglesOff)
        let t = state.apply(.deleteBackward, mode: tl, toneToggles: togglesOff)

        XCTAssertFalse(state.isComposing)
        XCTAssertEqual(state.selectedCandidateIndex, -1)
        XCTAssertEqual(t.newSelectedIndex, -1)
        XCTAssertEqual(t.effects, [
            .clearPreeditWithoutCommit,
            .resetAutocomplete,
            .deleteBackwardFromDocument,
        ])
    }

    func testDeleteBackward_whenRawLengthGreaterThanOne_shortensAndKeepsComposing() {
        var state = ComposingState()
        _ = state.apply(.start("ab"), mode: tl, toneToggles: togglesOff)
        let t = state.apply(.deleteBackward, mode: tl, toneToggles: togglesOff)

        XCTAssertTrue(state.isComposing)
        XCTAssertEqual(state.rawInput, "a")
        XCTAssertEqual(t.effects, [.updatePreedit("a"), .performAutocomplete])
    }

    /// Regression for Codex P2 r3106434856: non-empty backspace must snap the
    /// engine's selectedCandidateIndex back to 0 so a later replaceLast can't
    /// resurrect a stale pre-delete selection.
    func testDeleteBackward_nonEmpty_resetsEngineSelectedIndexSoReplaceLastCannotResurrect() {
        var state = ComposingState()
        _ = state.apply(.start("ab"), mode: tl, toneToggles: togglesOff)
        state.setSelectedCandidateIndex(3)

        let deleteT = state.apply(.deleteBackward, mode: tl, toneToggles: togglesOff)
        XCTAssertEqual(state.selectedCandidateIndex, 0)
        XCTAssertEqual(deleteT.newSelectedIndex, 0)

        let replaceT = state.apply(.replaceLast("c"), mode: tl, toneToggles: togglesOff)
        XCTAssertEqual(state.selectedCandidateIndex, 0,
                       "replaceLast must preserve the post-delete 0, not the pre-delete 3")
        XCTAssertEqual(replaceT.newSelectedIndex, 0)
    }

    func testDeleteBackward_whenIdle_isNoop() {
        var state = ComposingState()
        let t = state.apply(.deleteBackward, mode: tl, toneToggles: togglesOff)
        XCTAssertEqual(t.effects, [])
        XCTAssertEqual(t.newSelectedIndex, -1)
    }

    // MARK: - INVARIANT_composing_commit_captures_text_before_idle

    func testCommitDerived_emitsAtomicCommitThenResetThenContextReset() {
        var state = ComposingState()
        _ = state.apply(.start("hello"), mode: .english, toneToggles: togglesOff)
        let t = state.apply(.commitDerived, mode: .english, toneToggles: togglesOff)

        XCTAssertFalse(state.isComposing)
        XCTAssertEqual(state.rawInput, "")
        XCTAssertEqual(state.selectedCandidateIndex, -1)
        XCTAssertEqual(t.effects, [
            .commitTextReplacingPreedit("hello"),
            .resetAutocomplete,
            .resetAutocompleteContext,
        ])
    }

    func testCommitDerived_whenIdle_isNoop() {
        var state = ComposingState()
        let t = state.apply(.commitDerived, mode: tl, toneToggles: togglesOff)
        XCTAssertEqual(t.effects, [])
    }

    func testCommitRaw_emitsRawInputAsCommit() {
        var state = ComposingState()
        _ = state.apply(.start("gua2"), mode: tl, toneToggles: togglesOff)
        let t = state.apply(.commitRaw, mode: tl, toneToggles: togglesOff)

        XCTAssertEqual(t.effects, [
            .commitTextReplacingPreedit("gua2"),
            .resetAutocomplete,
            .resetAutocompleteContext,
        ])
    }

    // MARK: - INVARIANT_composing_select_suggestion_is_atomic_commit

    func testSelectSuggestion_emitsAtomicCommitOnlyNoClearPreedit() {
        var state = ComposingState()
        _ = state.apply(.start("a"), mode: tl, toneToggles: togglesOff)
        let t = state.apply(.selectSuggestion("picked"), mode: tl, toneToggles: togglesOff)

        XCTAssertFalse(state.isComposing)
        XCTAssertEqual(state.selectedCandidateIndex, -1)
        XCTAssertEqual(t.newSelectedIndex, -1)
        XCTAssertEqual(t.effects, [
            .commitTextReplacingPreedit("picked"),
            .resetAutocomplete,
            .resetAutocompleteContext,
        ])
        XCTAssertFalse(t.effects.contains(.clearPreeditWithoutCommit),
                       "selectSuggestion must not emit a separate clear-then-insert pair")
    }

    func testSelectSuggestion_whenIdle_isNoop() {
        var state = ComposingState()
        let t = state.apply(.selectSuggestion("picked"), mode: tl, toneToggles: togglesOff)
        XCTAssertEqual(t.effects, [])
    }

    // MARK: - reset

    func testReset_whenComposing_exitsWithClearAndAutocompleteReset() {
        var state = ComposingState()
        _ = state.apply(.start("abc"), mode: tl, toneToggles: togglesOff)
        let t = state.apply(.reset, mode: tl, toneToggles: togglesOff)

        XCTAssertFalse(state.isComposing)
        XCTAssertEqual(state.rawInput, "")
        XCTAssertEqual(state.selectedCandidateIndex, -1)
        XCTAssertEqual(t.effects, [.clearPreeditWithoutCommit, .resetAutocomplete])
    }

    // MARK: - INVARIANT_composing_idle_to_idle_is_noop

    func testReset_whenIdle_emitsEmptyEffects() {
        var state = ComposingState()
        let t = state.apply(.reset, mode: tl, toneToggles: togglesOff)
        XCTAssertEqual(t.effects, [])
        XCTAssertEqual(t.newSelectedIndex, -1)
        XCTAssertFalse(state.isComposing)
    }

    // MARK: - INVARIANT_composing_idle_has_no_selected_candidate

    func testEveryIdleProducingIntent_setsNewSelectedIndexToMinusOne() {
        func selectedIndexOnIdle(applying intent: (inout ComposingState) -> ComposingTransition) -> Int {
            var state = ComposingState()
            _ = state.apply(.start("a"), mode: tl, toneToggles: togglesOff)
            return intent(&state).newSelectedIndex
        }

        XCTAssertEqual(
            selectedIndexOnIdle { $0.apply(.deleteBackward, mode: tl, toneToggles: togglesOff) },
            -1,
        )
        XCTAssertEqual(
            selectedIndexOnIdle { $0.apply(.commitDerived, mode: .english, toneToggles: togglesOff) },
            -1,
        )
        XCTAssertEqual(
            selectedIndexOnIdle { $0.apply(.commitRaw, mode: tl, toneToggles: togglesOff) },
            -1,
        )
        XCTAssertEqual(
            selectedIndexOnIdle { $0.apply(.selectSuggestion("x"), mode: tl, toneToggles: togglesOff) },
            -1,
        )
        XCTAssertEqual(
            selectedIndexOnIdle { $0.apply(.reset, mode: tl, toneToggles: togglesOff) },
            -1,
        )
    }
}
