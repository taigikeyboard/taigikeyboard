@testable import TaigiKeyboard
import XCTest

/// Tests for ComposingManager: published state transitions + ComposingDelegate call order.
///
/// Focus:
/// - State contract (isComposing / rawInput / selectedCandidateIndex)
/// - Delegate call ORDER — this is the behavioral contract with KeyboardViewController
/// - Idle ↔ composing transitions
///
/// Avoids asserting `composingText` for strings whose tone conversion depends on
/// SharedSettings.inputMode; uses inputs that pass through unchanged
/// (e.g. "abc", "gua" with no tone digit).
final class ComposingManagerTests: XCTestCase {
    // MARK: - Spy

    /// Records ComposingDelegate calls in order for contract verification.
    private final class DelegateSpy: ComposingDelegate {
        enum Event: Equatable {
            case insertText(String)
            case deleteBackward
            case setMarkedText(String)
            case clearMarkedText
            case resetAutocomplete
            case performAutocomplete
            case resetAutocompleteContext
        }

        var events: [Event] = []

        func insertText(_ text: String) {
            events.append(.insertText(text))
        }

        func deleteBackward() {
            events.append(.deleteBackward)
        }

        func setMarkedText(_ text: String) {
            events.append(.setMarkedText(text))
        }

        func clearMarkedText() {
            events.append(.clearMarkedText)
        }

        func resetAutocomplete() {
            events.append(.resetAutocomplete)
        }

        func performAutocomplete() {
            events.append(.performAutocomplete)
        }

        func resetAutocompleteContext() {
            events.append(.resetAutocompleteContext)
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
        XCTAssertEqual(manager.selectedCandidateIndex, 0)
        XCTAssertEqual(spy.events, [])
    }

    // MARK: - startComposing

    func testStartComposing_entersComposingAndNotifiesDelegate() {
        manager.startComposing(with: "a")

        XCTAssertTrue(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "a")
        XCTAssertEqual(manager.selectedCandidateIndex, 0)

        // Contract: setMarkedText (from sync) must happen before performAutocomplete
        XCTAssertEqual(spy.events, [
            .setMarkedText(manager.composingText),
            .performAutocomplete,
        ])
    }

    // MARK: - appendCharacter

    func testAppendCharacter_whenIdle_startsComposing() {
        manager.appendCharacter("a")

        XCTAssertTrue(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "a")
        XCTAssertEqual(spy.events, [
            .setMarkedText(manager.composingText),
            .performAutocomplete,
        ])
    }

    func testAppendCharacter_whenComposing_appendsAndResetsSelectedIndex() {
        manager.startComposing(with: "a")
        manager.selectedCandidateIndex = 3
        spy.events.removeAll()

        manager.appendCharacter("b")

        XCTAssertEqual(manager.rawInput, "ab")
        XCTAssertEqual(manager.selectedCandidateIndex, 0)
        XCTAssertEqual(spy.events, [
            .setMarkedText(manager.composingText),
            .performAutocomplete,
        ])
    }

    // MARK: - replaceLastCharacter

    func testReplaceLastCharacter_replacesTailWithoutResettingSelectedIndex() {
        manager.startComposing(with: "ab")
        manager.selectedCandidateIndex = 2
        spy.events.removeAll()

        manager.replaceLastCharacter(with: "c")

        XCTAssertEqual(manager.rawInput, "ac")
        // Contract: replaceLast must NOT reset selectedCandidateIndex
        // (append/start do reset; replace is a correction, keeps selection)
        XCTAssertEqual(manager.selectedCandidateIndex, 2)
    }

    func testReplaceLastCharacter_whenIdle_isNoop() {
        manager.replaceLastCharacter(with: "x")

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "")
        XCTAssertEqual(spy.events, [])
    }

    func testReplaceLastCharacter_whenEmptyRaw_isNoop() {
        // Force composing state with empty raw is not reachable via public API;
        // this guard documents the internal guard.
        manager.replaceLastCharacter(with: "x")
        XCTAssertEqual(spy.events, [])
    }

    // MARK: - appendHyphen

    func testAppendHyphen_behavesAsAppendCharacter() {
        manager.startComposing(with: "a")
        spy.events.removeAll()

        manager.appendHyphen()

        XCTAssertEqual(manager.rawInput, "a-")
    }

    // MARK: - deleteBackward

    func testDeleteBackward_whenComposingWithMultipleChars_shortensRaw() {
        manager.startComposing(with: "ab")
        spy.events.removeAll()

        manager.deleteBackward()

        XCTAssertTrue(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "a")
        XCTAssertEqual(spy.events, [
            .setMarkedText(manager.composingText),
            .performAutocomplete,
        ])
    }

    func testDeleteBackward_whenComposingWithSingleChar_exitsAndDelegatesDelete() {
        manager.startComposing(with: "a")
        spy.events.removeAll()

        manager.deleteBackward()

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "")
        XCTAssertEqual(manager.selectedCandidateIndex, -1)

        // Contract ordering: idle transition (clearMarkedText + resetAutocomplete)
        // must happen BEFORE delegate?.deleteBackward() — the markedText must be
        // cleared before the backing text mutates.
        XCTAssertEqual(spy.events, [
            .clearMarkedText,
            .resetAutocomplete,
            .deleteBackward,
        ])
    }

    func testDeleteBackward_whenIdle_isNoop() {
        manager.deleteBackward()

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(spy.events, [])
    }

    // MARK: - commitComposition

    func testCommitComposition_insertsComposingTextAndExits() {
        manager.startComposing(with: "a")
        let committedText = manager.composingText
        manager.selectedCandidateIndex = 2
        spy.events.removeAll()

        manager.commitComposition()

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "")
        XCTAssertEqual(manager.selectedCandidateIndex, -1)

        // Contract ordering: state → idle (clearMarkedText + resetAutocomplete)
        // → insertText → resetAutocompleteContext
        XCTAssertEqual(spy.events, [
            .clearMarkedText,
            .resetAutocomplete,
            .insertText(committedText),
            .resetAutocompleteContext,
        ])
    }

    func testCommitComposition_whenIdle_isNoop() {
        manager.commitComposition()
        XCTAssertEqual(spy.events, [])
    }

    // MARK: - commitRawInput

    func testCommitRawInput_insertsRawStringBypassingConversion() {
        manager.startComposing(with: "Hello")
        spy.events.removeAll()

        manager.commitRawInput()

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.selectedCandidateIndex, -1)

        XCTAssertEqual(spy.events, [
            .clearMarkedText,
            .resetAutocomplete,
            .insertText("Hello"),
            .resetAutocompleteContext,
        ])
    }

    func testCommitRawInput_whenIdle_isNoop() {
        manager.commitRawInput()
        XCTAssertEqual(spy.events, [])
    }

    // MARK: - selectSuggestion

    func testSelectSuggestion_whenComposing_commitsSuggestionTextAndExits() {
        manager.startComposing(with: "a")
        spy.events.removeAll()

        manager.selectSuggestion(text: "picked")

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "")
        XCTAssertEqual(manager.selectedCandidateIndex, -1)

        // Contract ordering: clearMarkedText → insertText → resetAutocomplete →
        // resetAutocompleteContext. NOTE: this ordering differs from commitComposition
        // (which goes clearMarkedText → resetAutocomplete → insertText) because
        // selectSuggestion writes state to .idle directly and only calls the delegate
        // reset hooks afterward. Preserving this ordering is deliberate.
        XCTAssertEqual(spy.events, [
            .clearMarkedText,
            .insertText("picked"),
            .resetAutocomplete,
            .resetAutocompleteContext,
        ])
    }

    func testSelectSuggestion_whenIdle_isNoop() {
        manager.selectSuggestion(text: "picked")

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(spy.events, [])
    }

    // MARK: - confirmSelectedCandidate

    func testConfirmSelectedCandidate_whenIndexValid_selectsSuggestionAndReturnsTrue() {
        manager.startComposing(with: "a")
        manager.selectedCandidateIndex = 1
        spy.events.removeAll()

        let confirmed = manager.confirmSelectedCandidate(availableTexts: ["zero", "one"])

        XCTAssertTrue(confirmed)
        XCTAssertFalse(manager.isComposing)
        // Should pick "one" (index 1)
        XCTAssertTrue(spy.events.contains(.insertText("one")))
    }

    func testConfirmSelectedCandidate_whenIndexOutOfRange_returnsFalseAndDoesNothing() {
        manager.startComposing(with: "a")
        manager.selectedCandidateIndex = 5
        spy.events.removeAll()

        let confirmed = manager.confirmSelectedCandidate(availableTexts: ["only"])

        XCTAssertFalse(confirmed)
        XCTAssertTrue(manager.isComposing)
        XCTAssertEqual(spy.events, [])
    }

    func testConfirmSelectedCandidate_whenIndexNegative_returnsFalse() {
        manager.startComposing(with: "a")
        manager.selectedCandidateIndex = -1
        spy.events.removeAll()

        let confirmed = manager.confirmSelectedCandidate(availableTexts: ["only"])

        XCTAssertFalse(confirmed)
        XCTAssertEqual(spy.events, [])
    }

    func testConfirmSelectedCandidate_whenIdle_returnsFalse() {
        let confirmed = manager.confirmSelectedCandidate(availableTexts: ["only"])

        XCTAssertFalse(confirmed)
        XCTAssertEqual(spy.events, [])
    }

    // MARK: - reset

    func testReset_whenComposing_returnsToIdleWithoutInserting() {
        manager.startComposing(with: "abc")
        manager.selectedCandidateIndex = 2
        spy.events.removeAll()

        manager.reset()

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "")
        XCTAssertEqual(manager.composingText, "")
        XCTAssertEqual(manager.selectedCandidateIndex, -1)

        // No text should have been inserted or deleted
        XCTAssertFalse(spy.events.contains { event in
            if case .insertText = event { return true }
            if case .deleteBackward = event { return true }
            return false
        })
        // Should notify clearMarkedText + resetAutocomplete
        XCTAssertEqual(spy.events, [
            .clearMarkedText,
            .resetAutocomplete,
        ])
    }

    func testReset_whenIdle_isIdempotent() {
        spy.events.removeAll()
        manager.reset()

        XCTAssertFalse(manager.isComposing)
        // Idle → idle does not re-trigger state-change side effects (state didSet
        // still fires, so delegate clear hooks still run — this documents the current
        // behavior).
        // We assert only that no insert/delete happened.
        XCTAssertFalse(spy.events.contains { event in
            if case .insertText = event { return true }
            if case .deleteBackward = event { return true }
            return false
        })
    }
}
