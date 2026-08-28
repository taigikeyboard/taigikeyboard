// The selection latch: the second way a bare digit comes to pick a candidate,
// and the only way a toneless typist ever gets one.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// `BareDigitSelectionTests` pins the grammar rule — a digit is romanization
/// exactly while the raw buffer ends in a letter — and with it the unlatched
/// half of every case here, since production defaults the latch off. That rule
/// alone never fires for someone who does not type tones, because a toneless
/// buffer always ends in a letter. These cases pin the way in that does: `↓`,
/// and nothing else.
final class SelectionLatchTests: XCTestCase {
    private func intent(
        _ characters: String,
        modifiers: NSEvent.ModifierFlags = [],
        isComposing: Bool = true,
        isShowingCandidates: Bool = true,
        keySet: CandidateSlotKeySet = .bareKeys,
        rawInput: String,
        isSelectionLatched: Bool,
    ) throws -> ComposingKeyIntent {
        let event = try TestFixtures.keyDownEvent(characters: characters, modifiers: modifiers)
        return ComposingKeyIntent.intent(
            for: KeyEventSnapshot(event),
            isComposing: isComposing,
            isShowingCandidates: isShowingCandidates,
            bindings: ComposingKeyBindings(slotKeySet: keySet),
            rawInput: rawInput,
            isSelectionLatched: isSelectionLatched,
        )
    }

    private func navigationSnapshot(_ key: NavigationKey) -> KeyEventSnapshot {
        KeyEventSnapshot(
            characters: nil,
            modifiers: [],
            isNamedSpecialKey: true,
            navigationKey: key,
        )
    }

    // MARK: - What the latch changes

    func testWhileLatched_aBareDigitSelectsOutOfATonelessBuffer() throws {
        // trace: raw "taigi" ends in a letter, so `canTypeToneDigit` is true and
        // the grammar tier would call `3` a tone. The latch is what overrides it.
        XCTAssertEqual(
            try intent("3", rawInput: "taigi", isSelectionLatched: true),
            .selectCandidateSlot(2),
        )
        XCTAssertEqual(
            try intent("1", rawInput: "tai", isSelectionLatched: true),
            .selectCandidateSlot(0),
        )
        XCTAssertEqual(
            try intent("9", rawInput: "hoogua", isSelectionLatched: true),
            .selectCandidateSlot(8),
        )
    }

    // MARK: - What the latch must NOT change

    func testTheLatchCannotSelectWithNoBarUp() throws {
        // The controller clears the latch when the bar comes down, but the
        // tier is gated on `isShowingCandidates` in its own right — so even a
        // stale `true`, passed in here deliberately, cannot commit from a
        // window the user cannot see.
        XCTAssertEqual(
            try intent(
                "2", isShowingCandidates: false, rawInput: "tai", isSelectionLatched: true,
            ),
            .input("2"),
        )
    }

    func testTheLatchDoesNotReachOutsideAComposition() throws {
        XCTAssertEqual(
            try intent(
                "2",
                isComposing: false,
                isShowingCandidates: false,
                rawInput: "",
                isSelectionLatched: true,
            ),
            .passThrough,
        )
    }

    func testZeroIsNotASlot_latchedOrNot() throws {
        // The slots are 1–9. `0` names none of them, and the latch must not
        // quietly give it one; while the tail is a letter it is still input.
        XCTAssertEqual(try intent("0", rawInput: "tai", isSelectionLatched: true), .input("0"))
        XCTAssertEqual(try intent("0", rawInput: "tai", isSelectionLatched: false), .input("0"))
    }

    func testTheChordTier_isUnaffectedByTheLatch() throws {
        for latched in [true, false] {
            XCTAssertEqual(
                try intent(
                    "3", modifiers: .control, keySet: .control, rawInput: "taigi",
                    isSelectionLatched: latched,
                ),
                .selectCandidateSlot(2),
                "the chord picks whatever the latch says — it is classified first",
            )
        }
    }

    func testLetters_areStillComposition_whileLatched() throws {
        // The way out of the latch is the next thing the user was going to type,
        // so a letter must still reach the composition rather than be swallowed.
        XCTAssertEqual(
            try intent("g", rawInput: "tai", isSelectionLatched: true),
            .input("g"),
        )
    }

    // MARK: - The transition rule

    func testOnlyDownEntersTheLatch() {
        XCTAssertTrue(
            ComposingKeyIntent.selectionLatch(after: .navigate(.down), wasLatched: false),
            "`↓` is the gesture that means 'into the list'",
        )
    }

    func testTheOtherWaysOfWalkingTheBar_leaveTheLatchAlone() {
        // Space, ⇧⇥, the paging keys and the other arrows are how a Taigi
        // typist LOOKS at the homophones before deciding which tone to add.
        // Latching on them would make the very next `5` commit a candidate.
        let looking: [CandidateNavigation] = [
            .left, .right, .up, .pageUp, .pageDown, .nextCandidate, .previousCandidate,
        ]
        for direction in looking {
            XCTAssertFalse(
                ComposingKeyIntent.selectionLatch(after: .navigate(direction), wasLatched: false),
                "\(direction) must not latch on its own",
            )
            XCTAssertTrue(
                ComposingKeyIntent.selectionLatch(after: .navigate(direction), wasLatched: true),
                "\(direction) must not drop a latch the user already asked for",
            )
        }
    }

    func testTypingAndEndingTheComposition_clearTheLatch() {
        let clearing: [ComposingKeyIntent] = [
            .input("g"),
            .deleteBackward,
            .commit,
            .cancel,
            .commitThenInsert(" "),
            .commitThenPassThrough,
        ]
        for intent in clearing {
            XCTAssertFalse(
                ComposingKeyIntent.selectionLatch(after: intent, wasLatched: true),
                "\(intent) is typing or finishing, not choosing",
            )
        }
    }

    /// Picking is choosing, so it keeps the latch — and it has to, because what
    /// a pick DID is not knowable here. A slot the page never filled commits
    /// nothing; a candidate that consumes part of the buffer nails a prefix and
    /// leaves the user owing a candidate for the rest. Where a pick really does
    /// finish the composition, the bar comes down and the controller clears the
    /// latch there (`testPickingAFullCandidate_endsSelectionModeWithTheBar`).
    func testPickingACandidate_keepsTheLatch() {
        for intent in [ComposingKeyIntent.commitHighlightedCandidate, .selectCandidateSlot(6)] {
            XCTAssertTrue(
                ComposingKeyIntent.selectionLatch(after: intent, wasLatched: true),
                "\(intent) is the user choosing, not typing",
            )
            XCTAssertFalse(
                ComposingKeyIntent.selectionLatch(after: intent, wasLatched: false),
                "\(intent) is not a way INTO selection mode either",
            )
        }
    }

    func testAHostKeyLeavesTheLatchAlone() {
        // A pass-through says nothing either way about the bar; the paths that
        // take the bar down are what clear it (`TaigiInputController
        // .dismissCandidates`).
        XCTAssertTrue(ComposingKeyIntent.selectionLatch(after: .passThrough, wasLatched: true))
        XCTAssertFalse(ComposingKeyIntent.selectionLatch(after: .passThrough, wasLatched: false))
    }

    // MARK: - The gesture is classified before it takes effect

    func testTheDownArrowIsStillANavigation_notSwallowedByTheLatch() {
        // Latching is a side effect of `↓`, never a replacement for what it
        // does: the window must still move.
        XCTAssertEqual(
            ComposingKeyIntent.intent(
                for: navigationSnapshot(.downArrow),
                isComposing: true,
                isShowingCandidates: true,
                isSelectionLatched: true,
            ),
            .navigate(.down),
        )
    }
}
