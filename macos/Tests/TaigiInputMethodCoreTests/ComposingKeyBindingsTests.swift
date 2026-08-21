// Executable spec for the part of the key contract the user chooses.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// `ComposingKeyIntentTests` pins the contract as it ships. This one pins what
/// each binding changes about it, and — just as importantly — what it does not:
/// a setting that quietly took a typing key away would be worse than no setting.
final class ComposingKeyBindingsTests: XCTestCase {
    // MARK: - Defaults

    func testDefaultBindings_areTheShippedContract() {
        let bindings = ComposingKeyBindings.default

        XCTAssertEqual(bindings.returnKey, .commitLiteral)
        XCTAssertEqual(bindings.spaceKey, .confirmHighlighted)
        XCTAssertEqual(bindings.tabCycle, .disabled)
        XCTAssertEqual(bindings.slotModifier, .control)
        XCTAssertEqual(
            bindings.bracketPaging,
            .enabled,
            "the brackets page out of the box, the way the system Zhuyin input method's do",
        )
    }

    // MARK: - Brackets

    func testBrackets_pageTheBar_underTheDefaultBinding() throws {
        let cases: [(String, ComposingKeyIntent)] = [
            ("[", .navigate(.pageUp)),
            ("]", .navigate(.pageDown)),
        ]

        for (characters, expected) in cases {
            XCTAssertEqual(
                try intent(characters, isShowingCandidates: true),
                expected,
                "'\(characters)' pages the candidate window, as it does in the system Zhuyin input method",
            )
        }
    }

    func testBrackets_areDocumentText_whenNoBarIsUp() throws {
        XCTAssertEqual(
            try intent("[", isShowingCandidates: false),
            .commitThenInsert("["),
            "with no candidates to page through, a bracket is a bracket",
        )
        XCTAssertEqual(
            try intent("[", isComposing: false, isShowingCandidates: false),
            .passThrough,
        )
    }

    func testBrackets_areDocumentText_whenPagingIsTurnedOff() throws {
        let bindings = ComposingKeyBindings(bracketPaging: .disabled)

        for characters in ["[", "]"] {
            XCTAssertEqual(
                try intent(characters, isShowingCandidates: true, bindings: bindings),
                .commitThenInsert(characters),
                "a user who needs to type brackets mid-composition turns the binding off",
            )
            XCTAssertEqual(
                try intent(characters, isComposing: false, isShowingCandidates: false, bindings: bindings),
                .passThrough,
            )
        }
    }

    func testShiftedBrackets_stayDocumentText_whateverTheBinding() throws {
        for characters in ["{", "}"] {
            XCTAssertEqual(
                try intent(characters, modifiers: .shift, isShowingCandidates: true),
                .commitThenInsert(characters),
                "'\(characters)' is its own character, and the binding names the bracket keys by character",
            )
        }
    }

    // MARK: - Return

    func testReturn_commitsTheHighlightedCandidate_whenBound() throws {
        let bindings = ComposingKeyBindings(returnKey: .confirmHighlighted)

        XCTAssertEqual(
            try intent("\r", isShowingCandidates: true, bindings: bindings),
            .commitHighlightedCandidate,
        )
        XCTAssertEqual(
            try intent("\u{3}", isShowingCandidates: true, bindings: bindings),
            .commitHighlightedCandidate,
            "the keypad's Enter is the same key to the user",
        )
    }

    func testShiftReturn_keepsCommittingTheLiteral_whenReturnIsBoundToTheCandidate() throws {
        let bindings = ComposingKeyBindings(returnKey: .confirmHighlighted)

        XCTAssertEqual(
            try intent("\r", modifiers: .shift, isShowingCandidates: true, bindings: bindings),
            .commit,
            "without this escape hatch there would be no key left that keeps what was typed",
        )
    }

    func testReturn_commitsTheLiteral_withNoBarUp_underEitherBinding() throws {
        for returnKey in ReturnKeyBehavior.allCases {
            XCTAssertEqual(
                try intent("\r", isShowingCandidates: false, bindings: ComposingKeyBindings(returnKey: returnKey)),
                .commit,
                "there is no highlighted candidate to commit instead",
            )
        }
    }

    func testReturn_staysTheHostsKey_withNoComposition_underEitherBinding() throws {
        for returnKey in ReturnKeyBehavior.allCases {
            XCTAssertEqual(
                try intent(
                    "\r",
                    isComposing: false,
                    isShowingCandidates: false,
                    bindings: ComposingKeyBindings(returnKey: returnKey),
                ),
                .passThrough,
                "no binding may take Return away from a user who is not composing",
            )
        }
    }

    func testShiftKeypadEnter_alsoCommitsTheLiteral_whenReturnIsBoundToTheCandidate() throws {
        XCTAssertEqual(
            try intent(
                "\u{3}",
                modifiers: .shift,
                isShowingCandidates: true,
                bindings: ComposingKeyBindings(returnKey: .confirmHighlighted),
            ),
            .commit,
            "the escape hatch is the same key on the keypad",
        )
    }

    // MARK: - Space

    func testSpace_walksTheBar_whenBoundToNextCandidate() throws {
        let bindings = ComposingKeyBindings(spaceKey: .nextCandidate)

        XCTAssertEqual(
            try intent(" ", isShowingCandidates: true, bindings: bindings),
            .navigate(.nextCandidate),
            "the semantic direction, not `.right` — a vertical window pages on `.right`",
        )
        XCTAssertEqual(
            try intent(" ", isShowingCandidates: false, bindings: bindings),
            .commitThenInsert(" "),
            "with no bar up Space is the document's space under either binding",
        )
    }

    // MARK: - Tab

    func testTab_walksTheBar_whenCyclingIsTurnedOn() throws {
        let bindings = ComposingKeyBindings(tabCycle: .enabled)

        XCTAssertEqual(
            try intent("\t", isShowingCandidates: true, bindings: bindings),
            .navigate(.nextCandidate),
        )
        XCTAssertEqual(
            try intent("\u{19}", isShowingCandidates: true, bindings: bindings),
            .navigate(.previousCandidate),
            "AppKit sends ⇧Tab as U+0019, its own character rather than Tab plus a flag",
        )
    }

    func testTab_staysTheHostFocusKey_whenCyclingIsOff() throws {
        for characters in ["\t", "\u{19}"] {
            XCTAssertEqual(
                try intent(characters, isShowingCandidates: true),
                .commitThenPassThrough,
                "'\(characters)' moves the host's focus until the user says otherwise",
            )
        }
    }

    func testTab_staysTheHostFocusKey_whenNoBarIsUp_evenWhenCyclingIsOn() throws {
        XCTAssertEqual(
            try intent("\t", isShowingCandidates: false, bindings: ComposingKeyBindings(tabCycle: .enabled)),
            .commitThenPassThrough,
            "there is nothing to cycle through",
        )
    }

    // MARK: - Candidate slot modifier

    func testOptionDigits_selectCandidates_whenOptionIsTheBoundModifier() throws {
        let bindings = ComposingKeyBindings(slotModifier: .option)
        // Option rewrites the digits it is chorded with the way Control does,
        // which is why the slot is read from the unmodified characters.
        let optionThree = try TestFixtures.keyDownEvent(
            characters: "£",
            modifiers: .option,
            charactersIgnoringModifiers: "3",
        )

        XCTAssertEqual(
            ComposingKeyIntent.intent(
                for: KeyEventSnapshot(optionThree),
                isComposing: true,
                isShowingCandidates: true,
                bindings: bindings,
            ),
            .selectCandidateSlot(2),
        )
    }

    func testTheUnboundModifier_keepsHandingItsDigitChordsToTheHost() throws {
        let cases: [(name: String, bound: CandidateSlotModifier, typed: NSEvent.ModifierFlags)] = [
            ("Option bound, ⌃3 typed", .option, .control),
            ("Control bound, ⌥3 typed", .control, .option),
        ]

        for testCase in cases {
            let event = try TestFixtures.keyDownEvent(
                characters: "\u{1B}",
                modifiers: testCase.typed,
                charactersIgnoringModifiers: "3",
            )

            XCTAssertEqual(
                ComposingKeyIntent.intent(
                    for: KeyEventSnapshot(event),
                    isComposing: true,
                    isShowingCandidates: true,
                    bindings: ComposingKeyBindings(slotModifier: testCase.bound),
                ),
                .commitThenPassThrough,
                "\(testCase.name): the modifier the user did not choose is still the host's",
            )
        }
    }

    func testOptionDigits_withAnotherChordingModifier_belongToTheHost() throws {
        for extraModifiers in [NSEvent.ModifierFlags.command, .control, .shift] {
            let event = try TestFixtures.keyDownEvent(
                characters: "£",
                modifiers: extraModifiers.union(.option),
                charactersIgnoringModifiers: "3",
            )

            XCTAssertEqual(
                ComposingKeyIntent.intent(
                    for: KeyEventSnapshot(event),
                    isComposing: true,
                    isShowingCandidates: true,
                    bindings: ComposingKeyBindings(slotModifier: .option),
                ),
                .commitThenPassThrough,
                "the chord binds ⌥1…⌥9 and nothing built on top of them",
            )
        }
    }

    func testOptionDigits_selectCandidates_whateverElseAppKitReports() throws {
        for extraModifiers in [NSEvent.ModifierFlags.capsLock, .numericPad, [.numericPad, .function]] {
            let event = try TestFixtures.keyDownEvent(
                characters: "£",
                modifiers: extraModifiers.union(.option),
                charactersIgnoringModifiers: "3",
            )

            XCTAssertEqual(
                ComposingKeyIntent.intent(
                    for: KeyEventSnapshot(event),
                    isComposing: true,
                    isShowingCandidates: true,
                    bindings: ComposingKeyBindings(slotModifier: .option),
                ),
                .selectCandidateSlot(2),
                "⌥3 with \(extraModifiers) also held is still ⌥3",
            )
        }
    }

    // MARK: - What no binding may take away

    /// Every setting is a chance to bind a key the user needs to type with. The
    /// keys romanization is built from must survive every combination.
    func testTypingKeys_surviveEveryBindingCombination() throws {
        for bindings in Self.everyBindingCombination {
            for characters in ["t", "a", "-"] {
                XCTAssertEqual(
                    try intent(characters, isShowingCandidates: true, bindings: bindings),
                    .input(characters),
                    "'\(characters)' builds a syllable — no binding may take it away",
                )
            }
            XCTAssertEqual(
                try intent("5", isShowingCandidates: true, bindings: bindings),
                .input("5"),
                "a bare digit is the numeric tone of `tai5` under every binding",
            )
            XCTAssertEqual(
                try intent("\u{8}", isShowingCandidates: true, bindings: bindings),
                .deleteBackward,
            )
            XCTAssertEqual(
                try intent("\u{1B}", isShowingCandidates: true, bindings: bindings),
                .cancel,
                "Escape is the way out of a composition, whatever else is bound",
            )
        }
    }

    /// The 32 combinations of the five settings, which is small enough to walk
    /// exhaustively rather than sample.
    private static let everyBindingCombination: [ComposingKeyBindings] = {
        var combinations: [ComposingKeyBindings] = []
        for returnKey in ReturnKeyBehavior.allCases {
            for spaceKey in SpaceKeyBehavior.allCases {
                for bracketPaging in BracketPagingBehavior.allCases {
                    for tabCycle in TabCycleBehavior.allCases {
                        for slotModifier in CandidateSlotModifier.allCases {
                            combinations.append(ComposingKeyBindings(
                                returnKey: returnKey,
                                spaceKey: spaceKey,
                                bracketPaging: bracketPaging,
                                tabCycle: tabCycle,
                                slotModifier: slotModifier,
                            ))
                        }
                    }
                }
            }
        }
        return combinations
    }()

    private func intent(
        _ characters: String,
        modifiers: NSEvent.ModifierFlags = [],
        isComposing: Bool = true,
        isShowingCandidates: Bool,
        bindings: ComposingKeyBindings = .default,
    ) throws -> ComposingKeyIntent {
        let event = try TestFixtures.keyDownEvent(characters: characters, modifiers: modifiers)
        return ComposingKeyIntent.intent(
            for: KeyEventSnapshot(event),
            isComposing: isComposing,
            isShowingCandidates: isShowingCandidates,
            bindings: bindings,
        )
    }
}
