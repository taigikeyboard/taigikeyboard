@testable import TaigiKeyboard
import XCTest

/// Pure-engine tests for the G5 NextWordController split. Covers the
/// Phase 0 invariants referenced from `docs/architecture/behavioral-invariants.md`
/// and `docs/architecture/nextword-engine-boundary.md` §10.
///
/// Executor-side invariants (Timer leak, late prediction discarded) are
/// deferred to G9 where a platform harness runs them.
final class NextWordEngineTests: XCTestCase {
    // MARK: - Fixtures

    private func settings(
        inputMode: InputMode = .tl,
        isTranslateSwapped: Bool = false,
        isAssociationRecordingEnabled: Bool = true,
    ) -> NextWordEngineSettings {
        NextWordEngineSettings(
            inputMode: inputMode,
            isTranslateSwapped: isTranslateSwapped,
            isAssociationRecordingEnabled: isAssociationRecordingEnabled,
        )
    }

    private func input(
        nowMs: Int64 = 10000,
        settings: NextWordEngineSettings? = nil,
    ) -> NextWordDecisionInput {
        NextWordDecisionInput(nowMs: nowMs, settings: settings ?? self.settings())
    }

    private func stateWithLastSelection(
        word: String = "早",
        roman: String = "tsá",
        timeMs: Int64 = 0,
        isShowing: Bool = false,
        generation: UInt64 = 0,
    ) -> NextWordPersistedState {
        NextWordPersistedState(
            lastSelectedWord: word,
            lastSelectedRoman: roman,
            lastSelectionTimeMs: timeMs,
            isShowing: isShowing,
            currentGeneration: generation,
        )
    }

    // MARK: - INVARIANT_nextword_association_window_strict_lt_10s

    func testAssociationWindow_atBoundary9999_returnsTrue() {
        let state = stateWithLastSelection(timeMs: 0)
        XCTAssertTrue(NextWordEngine.shouldRecordAssociation(state: state, nowMs: 9999))
    }

    func testAssociationWindow_atBoundary10000_returnsFalse() {
        let state = stateWithLastSelection(timeMs: 0)
        XCTAssertFalse(NextWordEngine.shouldRecordAssociation(state: state, nowMs: 10000))
    }

    func testAssociationWindow_negativeDelta_returnsFalse() {
        // New invariant: clock-skew / wrapping scenarios do NOT record.
        // Pre-split controller returned true here (unsigned delta bug).
        let state = stateWithLastSelection(timeMs: 1_000_000)
        XCTAssertFalse(NextWordEngine.shouldRecordAssociation(state: state, nowMs: 500_000))
    }

    func testAssociationWindow_withoutLastSelection_returnsFalse() {
        let state = NextWordPersistedState.initial
        XCTAssertFalse(NextWordEngine.shouldRecordAssociation(state: state, nowMs: 1000))
    }

    // MARK: - INVARIANT_nextword_backspace_does_not_record

    func testBackspace_neverEmitsRecordEffects() {
        let state = stateWithLastSelection(timeMs: 100)
        let outcome = NextWordEngine.decide(
            intent: .backspace(lastChar: "安"),
            state: state,
            input: input(nowMs: 200),
        )

        for effect in outcome.effects {
            switch effect {
            case .recordAssociation, .recordCompoundAssociations:
                XCTFail("backspace must never emit record effects; got \(effect)")
            default:
                continue
            }
        }
        XCTAssertEqual(outcome.effects.count, 1)
        guard case let .queryPredictions(word, roman, generation) = outcome.effects[0] else {
            return XCTFail("expected queryPredictions as sole effect")
        }
        XCTAssertEqual(word, "安")
        XCTAssertEqual(roman, "")
        XCTAssertEqual(generation, state.currentGeneration &+ 1)
    }

    // MARK: - INVARIANT_nextword_sentence_end_resets_context

    func testWordSelected_sentenceEndPunctuation_resetsAndCancelsTimer() {
        let state = stateWithLastSelection(isShowing: true, generation: 42)
        let outcome = NextWordEngine.decide(
            intent: .wordSelected(text: "。", roman: "", requireRomanMode: false, triggerPrediction: true),
            state: state,
            input: input(),
        )

        XCTAssertEqual(outcome.newState.lastSelectedWord, nil)
        XCTAssertEqual(outcome.newState.lastSelectedRoman, nil)
        XCTAssertEqual(outcome.newState.lastSelectionTimeMs, 0)
        XCTAssertEqual(outcome.newState.isShowing, false)
        XCTAssertEqual(outcome.newState.currentGeneration, 43)

        XCTAssertTrue(outcome.effects.contains(.cancelContextTimeout))
        XCTAssertTrue(outcome.effects.contains(.clearPredictionsUI(generation: 43)))
    }

    func testWordSelected_sentenceEndPunctuation_whenNotShowing_skipsUIClear() {
        let state = stateWithLastSelection(isShowing: false, generation: 0)
        let outcome = NextWordEngine.decide(
            intent: .wordSelected(text: "!", roman: "", requireRomanMode: false, triggerPrediction: true),
            state: state,
            input: input(),
        )
        XCTAssertEqual(outcome.effects, [.cancelContextTimeout])
    }

    // MARK: - INVARIANT_nextword_compound_pairs_are_sequential

    func testCompoundAssociationPairs_ordersSequentially() {
        let pairs = NextWordEngine.compoundAssociationPairs(
            displayText: "a-b-c",
            roman: "x-y-z",
        )
        XCTAssertEqual(pairs, [
            NextWordAssociationPair(prev: "a", prevTl: "x", next: "b", nextTl: "y"),
            NextWordAssociationPair(prev: "b", prevTl: "y", next: "c", nextTl: "z"),
        ])
    }

    func testCompoundAssociationPairs_singleWord_returnsEmpty() {
        XCTAssertEqual(NextWordEngine.compoundAssociationPairs(displayText: "abc", roman: "xyz"), [])
    }

    func testCompoundAssociationPairs_missingRomanParts_fillsEmpty() {
        let pairs = NextWordEngine.compoundAssociationPairs(displayText: "a-b-c", roman: "x")
        XCTAssertEqual(pairs, [
            NextWordAssociationPair(prev: "a", prevTl: "x", next: "b", nextTl: ""),
            NextWordAssociationPair(prev: "b", prevTl: "", next: "c", nextTl: ""),
        ])
    }

    // MARK: - INVARIANT_nextword_generation_bumps_on_invalidating_intents

    func testGenerationBumps_onWordSelected() {
        let state = NextWordPersistedState.initial
        let outcome = NextWordEngine.decide(
            intent: .wordSelected(text: "早", roman: "tsá", requireRomanMode: false, triggerPrediction: true),
            state: state,
            input: input(),
        )
        XCTAssertEqual(outcome.newState.currentGeneration, state.currentGeneration &+ 1)
    }

    func testGenerationBumps_onBackspace() {
        let state = stateWithLastSelection(generation: 7)
        let outcome = NextWordEngine.decide(
            intent: .backspace(lastChar: "x"),
            state: state,
            input: input(),
        )
        XCTAssertEqual(outcome.newState.currentGeneration, 8)
    }

    func testGenerationBumps_onClearForNewComposing() {
        let state = stateWithLastSelection(isShowing: true, generation: 3)
        let outcome = NextWordEngine.decide(
            intent: .clearForNewComposing,
            state: state,
            input: input(),
        )
        XCTAssertEqual(outcome.newState.currentGeneration, 4)
        XCTAssertEqual(outcome.newState.isShowing, false)
        // Association state preserved — only UI hidden.
        XCTAssertEqual(outcome.newState.lastSelectedWord, state.lastSelectedWord)
    }

    func testClearForNewComposing_whenNotShowing_bumpsGenerationWithoutEffects() {
        let state = stateWithLastSelection(isShowing: false, generation: 5)
        let outcome = NextWordEngine.decide(
            intent: .clearForNewComposing,
            state: state,
            input: input(),
        )
        XCTAssertEqual(outcome.newState.currentGeneration, 6)
        XCTAssertEqual(outcome.effects, [])
    }

    func testGenerationBumps_onContextTimeout() {
        let state = stateWithLastSelection(isShowing: true, generation: 99)
        let outcome = NextWordEngine.decide(
            intent: .contextTimeoutFired,
            state: state,
            input: input(),
        )
        XCTAssertEqual(outcome.newState.currentGeneration, 100)
        XCTAssertNil(outcome.newState.lastSelectedWord)
    }

    func testGenerationBumps_onResetFull() {
        let state = stateWithLastSelection(isShowing: true, generation: 12)
        let outcome = NextWordEngine.decide(
            intent: .resetFull,
            state: state,
            input: input(),
        )
        XCTAssertEqual(outcome.newState.currentGeneration, 13)
    }

    // MARK: - INVARIANT_nextword_prediction_filter_hides_empty_tl_in_roman_mode

    func testFilterPredictions_romanMode_dropsEmptyTl() {
        let raw = [
            RawNextWordPrediction(hanzi: "好", tl: "hó", score: 10),
            RawNextWordPrediction(hanzi: "安", tl: "", score: 9),
        ]
        let result = NextWordEngine.filterPredictions(
            raw,
            settings: settings(inputMode: .tl, isTranslateSwapped: false),
        )
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result.first?.hanzi, "好")
    }

    func testFilterPredictions_hanziMode_keepsEmptyTl() {
        let raw = [
            RawNextWordPrediction(hanzi: "好", tl: "hó", score: 10),
            RawNextWordPrediction(hanzi: "安", tl: "", score: 9),
        ]
        let result = NextWordEngine.filterPredictions(
            raw,
            settings: settings(inputMode: .tl, isTranslateSwapped: true),
        )
        XCTAssertEqual(result.count, 2)
    }

    // MARK: - wordSelected records association when inside window

    func testWordSelected_recordsAssociationWhenInsideWindow() {
        let state = stateWithLastSelection(word: "早", roman: "tsá", timeMs: 0)
        let outcome = NextWordEngine.decide(
            intent: .wordSelected(text: "安", roman: "an", requireRomanMode: false, triggerPrediction: true),
            state: state,
            input: input(nowMs: 5000),
        )

        let expected = NextWordAssociationPair(prev: "早", prevTl: "tsá", next: "安", nextTl: "an")
        XCTAssertTrue(outcome.effects.contains(.recordAssociation(expected)))
    }

    func testWordSelected_requireRomanModeInSwappedMode_noOps() {
        let state = NextWordPersistedState.initial
        let outcome = NextWordEngine.decide(
            intent: .wordSelected(text: "abc", roman: "abc", requireRomanMode: true, triggerPrediction: true),
            state: state,
            input: input(settings: settings(isTranslateSwapped: true)),
        )
        XCTAssertEqual(outcome.effects, [])
        XCTAssertEqual(outcome.newState, state)
    }

    func testWordSelected_triggerPredictionFalse_recordsButSkipsQuery() {
        // Space-key path: record + update state + reschedule timer, but do NOT
        // emit `queryPredictions`.
        let state = NextWordPersistedState.initial
        let outcome = NextWordEngine.decide(
            intent: .wordSelected(text: "早", roman: "tsá", requireRomanMode: false, triggerPrediction: false),
            state: state,
            input: input(),
        )

        XCTAssertEqual(outcome.newState.lastSelectedWord, "早")
        XCTAssertTrue(outcome.effects.contains(.rescheduleContextTimeout(after: NextWordEngine.contextTimeoutSeconds)))
        for effect in outcome.effects {
            if case .queryPredictions = effect {
                XCTFail("triggerPrediction=false must not emit queryPredictions")
            }
        }
    }

    func testWordSelected_noiseText_skipsSilently() {
        let state = NextWordPersistedState.initial
        let outcome = NextWordEngine.decide(
            intent: .wordSelected(text: ",", roman: "", requireRomanMode: false, triggerPrediction: true),
            state: state,
            input: input(),
        )
        XCTAssertEqual(outcome.effects, [])
        XCTAssertEqual(outcome.newState, state)
    }

    // MARK: - resetFull behavior

    func testResetFull_zeroesStateAndCancelsTimer() {
        let state = stateWithLastSelection(isShowing: true, generation: 4)
        let outcome = NextWordEngine.decide(
            intent: .resetFull,
            state: state,
            input: input(),
        )
        XCTAssertNil(outcome.newState.lastSelectedWord)
        XCTAssertEqual(outcome.newState.lastSelectionTimeMs, 0)
        XCTAssertFalse(outcome.newState.isShowing)
        XCTAssertTrue(outcome.effects.contains(.cancelContextTimeout))
    }
}
