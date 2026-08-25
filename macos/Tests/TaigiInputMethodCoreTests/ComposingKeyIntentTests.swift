// Executable spec for the input method's key contract.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// Every key classified as anything but `.passThrough` is a key the host never
/// receives, so this table is also the list of things the user can no longer do
/// in their app while typing. It is asserted in both composition states because
/// most keys mean different things in each.
final class ComposingKeyIntentTests: XCTestCase {
    func testRomanizationCharacters_areComposingInput_inBothStates() throws {
        for characters in ["t", "A", "-"] {
            let event = try TestFixtures.keyDownEvent(characters: characters)
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: false),
                .input(characters),
                "'\(characters)' must be able to start a composition",
            )
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true),
                .input(characters),
                "'\(characters)' must extend a running composition",
            )
        }
    }

    func testDigits_areToneMarkersOnlyWhileComposing() throws {
        let event = try TestFixtures.keyDownEvent(characters: "5")

        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true),
            .input("5"),
            "a digit typed after romanization is the numeric tone of `tai5`",
        )
        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: false),
            .passThrough,
            "a bare digit is a digit — starting a composition with it would make numbers untypable",
        )
    }

    func testCompositionControlKeys_belongToTheHostWhenThereIsNoComposition() throws {
        let cases: [(name: String, characters: String, composing: ComposingKeyIntent)] = [
            // With no bar up there is no candidate to take, so Return is the
            // host's paragraph break — after the composition it follows.
            ("Return", "\r", .commitThenPassThrough),
            ("Escape", "\u{1B}", .cancel),
            ("Backspace", "\u{8}", .deleteBackward),
            ("Delete", "\u{7F}", .deleteBackward),
            ("Space", " ", .commitThenInsert(" ")),
            ("Period", ".", .commitThenInsert(".")),
        ]

        for testCase in cases {
            let event = try TestFixtures.keyDownEvent(characters: testCase.characters)
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true),
                testCase.composing,
                "\(testCase.name) acts on the composition while one is running",
            )
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: false),
                .passThrough,
                "\(testCase.name) belongs to the host when there is no composition to act on",
            )
        }
    }

    func testHostOwnedEvents_fallThrough_evenMidComposition() throws {
        let cases: [(name: String, characters: String, modifiers: NSEvent.ModifierFlags)] = [
            ("Command chord", "s", .command),
            ("Control chord", "a", .control),
            ("Option chord", "a", .option),
            ("Tab", "\t", []),
            ("Left arrow", "\u{F702}", []),
            ("F1", "\u{F704}", []),
            ("Line separator", "\u{2028}", []),
        ]

        for testCase in cases {
            let event = try TestFixtures.keyDownEvent(characters: testCase.characters, modifiers: testCase.modifiers)
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true),
                .commitThenPassThrough,
                """
                \(testCase.name) is the host's even mid-composition, but the composition must be \
                finished first — the host is about to act on the document it sits in
                """,
            )
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: false),
                .passThrough,
                "\(testCase.name) with no composition running needs no finishing step",
            )
        }
    }

    func testNonRomanizationLetters_areDocumentTextRatherThanEngineInput() throws {
        let event = try TestFixtures.keyDownEvent(characters: "字")

        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true),
            .commitThenInsert("字"),
            "a character the engine cannot parse ends the composition and goes to the document",
        )
        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: false),
            .passThrough,
        )
    }

    func testEmptyCharacters_fallThrough() throws {
        XCTAssertEqual(
            try ComposingKeyIntent.intent(for: KeyEventSnapshot(TestFixtures.keyDownEvent(characters: "")), isComposing: false),
            .passThrough,
            "an event carrying no characters has nothing to compose",
        )
    }

    func testNonAsciiDigits_areDocumentTextRatherThanTones() throws {
        let event = try TestFixtures.keyDownEvent(characters: "\u{FF15}") // full-width ５

        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true),
            .commitThenInsert("\u{FF15}"),
            "the engine's tone markers are ASCII digits — a full-width numeral is document text",
        )
    }

    // MARK: - Candidate keys

    /// The six keys the window binds, handed through as raw directions — what
    /// each one DOES belongs to the window's layout, not to this table.
    func testNavigationKeys_driveTheBarWhileItIsUp() {
        let cases: [(NavigationKey, ComposingKeyIntent)] = [
            (.leftArrow, .navigate(.left)),
            (.rightArrow, .navigate(.right)),
            (.upArrow, .navigate(.up)),
            (.downArrow, .navigate(.down)),
            (.pageUp, .navigate(.pageUp)),
            (.pageDown, .navigate(.pageDown)),
        ]

        for (key, expected) in cases {
            XCTAssertEqual(
                ComposingKeyIntent.intent(
                    for: navigationSnapshot(key),
                    isComposing: true,
                    isShowingCandidates: true,
                ),
                expected,
                "\(key) must drive the candidate bar while it is on screen",
            )
        }
    }

    func testNavigationKeys_belongToTheHostWhenNoBarIsUp() {
        for key in [NavigationKey.leftArrow, .downArrow, .pageUp] {
            XCTAssertEqual(
                ComposingKeyIntent.intent(
                    for: navigationSnapshot(key),
                    isComposing: true,
                    isShowingCandidates: false,
                ),
                .commitThenPassThrough,
                "an arrow with no candidates on screen moves the host's caret, after the "
                    + "composition has been written where the user typed it",
            )
        }
    }

    func testShiftedArrow_staysTheHostSelectionKey_evenWithTheBarUp() {
        XCTAssertEqual(
            ComposingKeyIntent.intent(
                for: navigationSnapshot(.leftArrow, modifiers: .shift),
                isComposing: true,
                isShowingCandidates: true,
            ),
            .commitThenPassThrough,
            "⇧← extends a selection; binding it to the bar would take that away for no gain",
        )
    }

    /// Control rewrites the characters of the digits it is chorded with, so this
    /// is the case the whole `charactersIgnoringModifiers` field exists for:
    /// classified from `characters`, `⌃3` arrives as `\u{1B}` and cancels the
    /// composition instead of picking the third candidate.
    func testControlDigits_selectCandidates_despiteArrivingAsControlCharacters() throws {
        let controlThree = try TestFixtures.keyDownEvent(
            characters: "\u{1B}",
            modifiers: .control,
            charactersIgnoringModifiers: "3",
        )

        XCTAssertEqual(
            ComposingKeyIntent.intent(
                for: KeyEventSnapshot(controlThree),
                isComposing: true,
                isShowingCandidates: true,
            ),
            .selectCandidateSlot(2),
            "⌃3 selects the third candidate of the visible page, counting slots from zero",
        )
    }

    func testControlDigits_belongToTheHostWhenNoBarIsUp() throws {
        let controlThree = try TestFixtures.keyDownEvent(
            characters: "\u{1B}",
            modifiers: .control,
            charactersIgnoringModifiers: "3",
        )

        XCTAssertEqual(
            ComposingKeyIntent.intent(
                for: KeyEventSnapshot(controlThree),
                isComposing: true,
                isShowingCandidates: false,
            ),
            .commitThenPassThrough,
            "with nothing to select, a Control chord is the host's shortcut again",
        )
    }

    /// Caps Lock does not change what a digit key means, and the number pad sets
    /// `.numericPad` (plus `.function` on some keyboards). Testing for an exact
    /// modifier set would make `⌃3` select on the top row and quietly commit the
    /// composition on the keypad.
    func testControlDigits_selectCandidates_whateverElseAppKitReports() throws {
        for extraModifiers in [NSEvent.ModifierFlags.capsLock, .numericPad, [.numericPad, .function]] {
            let event = try TestFixtures.keyDownEvent(
                characters: "\u{1B}",
                modifiers: extraModifiers.union(.control),
                charactersIgnoringModifiers: "3",
            )

            XCTAssertEqual(
                ComposingKeyIntent.intent(
                    for: KeyEventSnapshot(event),
                    isComposing: true,
                    isShowingCandidates: true,
                ),
                .selectCandidateSlot(2),
                "⌃3 with \(extraModifiers) also held is still ⌃3",
            )
        }
    }

    func testControlDigits_withAnotherChordingModifier_belongToTheHost() throws {
        for extraModifiers in [NSEvent.ModifierFlags.command, .option, .shift] {
            let event = try TestFixtures.keyDownEvent(
                characters: "\u{1B}",
                modifiers: extraModifiers.union(.control),
                charactersIgnoringModifiers: "3",
            )

            XCTAssertEqual(
                ComposingKeyIntent.intent(
                    for: KeyEventSnapshot(event),
                    isComposing: true,
                    isShowingCandidates: true,
                ),
                .commitThenPassThrough,
                "⌃⌘3 and friends are the host's — the bar binds ⌃1…⌃9 and nothing built on top of them",
            )
        }
    }

    func testControlZero_isNotACandidateChord() throws {
        let controlZero = try TestFixtures.keyDownEvent(
            characters: "\u{0}",
            modifiers: .control,
            charactersIgnoringModifiers: "0",
        )

        XCTAssertEqual(
            ComposingKeyIntent.intent(
                for: KeyEventSnapshot(controlZero),
                isComposing: true,
                isShowingCandidates: true,
            ),
            .commitThenPassThrough,
            "the bar holds nine candidates, addressed by ⌃1 to ⌃9 — ⌃0 addresses nothing",
        )
    }

    /// Space writes the highlighted candidate in the OTHER script — the 漢羅
    /// key (`ComposingAction.commitAlternateScript`). It walked the candidates
    /// until 2026-08-25, which every layout's arrows already did.
    func testSpace_commitsTheOtherScriptOnlyWhileTheBarIsUp() throws {
        let event = try TestFixtures.keyDownEvent(characters: " ")

        XCTAssertEqual(
            ComposingKeyIntent.intent(
                for: KeyEventSnapshot(event),
                isComposing: true,
                isShowingCandidates: true,
            ),
            .commitAlternateScript,
        )
        XCTAssertEqual(
            ComposingKeyIntent.intent(
                for: KeyEventSnapshot(event),
                isComposing: true,
                isShowingCandidates: false,
            ),
            .commitThenInsert(" "),
            "with no bar up there is no candidate to re-render, so Space is the "
                + "document's space again — which is how a 漢羅 sentence gets its spaces",
        )
    }

    /// Return takes the candidate and ⇧Return takes what was typed — the
    /// Zhuyin pairing, and the reason the literal commit is one of the two
    /// actions that may never be left unbound.
    func testReturn_commitsTheCandidate_andShiftReturnTheLiteral() throws {
        let plain = try TestFixtures.keyDownEvent(characters: "\r")
        let shifted = try TestFixtures.keyDownEvent(characters: "\r", modifiers: .shift)

        XCTAssertEqual(
            ComposingKeyIntent.intent(
                for: KeyEventSnapshot(plain),
                isComposing: true,
                isShowingCandidates: true,
            ),
            .commitHighlightedCandidate,
        )
        XCTAssertEqual(
            ComposingKeyIntent.intent(
                for: KeyEventSnapshot(shifted),
                isComposing: true,
                isShowingCandidates: true,
            ),
            .commit,
            "⇧Return is the only key that keeps what was typed rather than what was suggested",
        )
    }

    /// With no bar up there is no candidate to take, so Return ends the
    /// composition whichever action holds it.
    func testReturn_endsTheCompositionWithNoBarUp() throws {
        let event = try TestFixtures.keyDownEvent(characters: "\r")

        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true),
            .commitThenPassThrough,
            "the host gets its paragraph break, after the text it follows",
        )
    }

    /// The mapping from AppKit's own key names, which the snapshot is what
    /// isolates: everything above is asserted against `NavigationKey` directly,
    /// so without this the six keys could all be extracted as nil.
    func testArrowEvents_areRecognizedAsNavigationKeys() throws {
        let cases: [(String, NavigationKey)] = try [
            (String(XCTUnwrap(UnicodeScalar(NSLeftArrowFunctionKey))), .leftArrow),
            (String(XCTUnwrap(UnicodeScalar(NSRightArrowFunctionKey))), .rightArrow),
            (String(XCTUnwrap(UnicodeScalar(NSUpArrowFunctionKey))), .upArrow),
            (String(XCTUnwrap(UnicodeScalar(NSDownArrowFunctionKey))), .downArrow),
            (String(XCTUnwrap(UnicodeScalar(NSPageUpFunctionKey))), .pageUp),
            (String(XCTUnwrap(UnicodeScalar(NSPageDownFunctionKey))), .pageDown),
        ]

        for (characters, expected) in cases {
            let event = try TestFixtures.keyDownEvent(characters: characters)
            XCTAssertEqual(KeyEventSnapshot(event).navigationKey, expected)
        }
    }

    /// AppKit encodes the arrows in the same private-use range as the function
    /// keys, and only the ones bound above are navigation.
    func testFunctionKeyEvents_areNotNavigationKeys() throws {
        let event = try TestFixtures.keyDownEvent(
            characters: String(XCTUnwrap(UnicodeScalar(NSF5FunctionKey))),
        )

        XCTAssertNil(KeyEventSnapshot(event).navigationKey)
    }

    // MARK: - Document text

    /// The pass-through path reports document text to the engine so a full stop
    /// can end a learning context. What it must NOT report is a key the host
    /// acts on — Escape and Return are pass-through too, and neither is a
    /// character anyone typed into a document.
    func testIsDocumentText_acceptsPrintableCharactersAndRejectsKeysTheHostActsOn() {
        for text in ["。", "、", "!", "?", " ", "台", "x"] {
            XCTAssertTrue(
                ComposingKeyIntent.isDocumentText(textSnapshot(text)),
                "'\(text)' is text the host puts into its document",
            )
        }
        for text in ["\u{1B}", "\r", "\u{8}", "\u{7F}"] {
            XCTAssertFalse(
                ComposingKeyIntent.isDocumentText(textSnapshot(text)),
                "a control character is a command, not document text",
            )
        }
    }

    /// `⌘.` and a typed `.` carry the same character. One of them inserts
    /// nothing, and reporting it as document text would end a learning context
    /// on a keystroke that never reached the document.
    func testIsDocumentText_rejectsAHostChordCarryingAPrintableCharacter() {
        for modifier in [NSEvent.ModifierFlags.command, .control, .option] {
            XCTAssertFalse(
                ComposingKeyIntent.isDocumentText(textSnapshot(".", modifiers: modifier)),
                "a chord is a host command however printable its character is",
            )
        }
        XCTAssertTrue(
            ComposingKeyIntent.isDocumentText(textSnapshot(".", modifiers: .shift)),
            "Shift is how the character was typed, not a command",
        )
    }

    func testIsDocumentText_rejectsArrowsAndFunctionKeys() throws {
        let leftArrow = try String(XCTUnwrap(UnicodeScalar(NSLeftArrowFunctionKey)))
        let functionKey = try String(XCTUnwrap(UnicodeScalar(NSF5FunctionKey)))

        XCTAssertFalse(ComposingKeyIntent.isDocumentText(textSnapshot(leftArrow)))
        XCTAssertFalse(ComposingKeyIntent.isDocumentText(textSnapshot(functionKey)))
        XCTAssertFalse(
            ComposingKeyIntent.isDocumentText(textSnapshot("\u{2028}", isNamedSpecialKey: true)),
            "a line separator is a named key AppKit gives us, not typed text",
        )
    }

    func testIsDocumentText_rejectsNothingAtAll() {
        XCTAssertFalse(ComposingKeyIntent.isDocumentText(textSnapshot(nil)))
        XCTAssertFalse(ComposingKeyIntent.isDocumentText(textSnapshot("")))
    }

    private func textSnapshot(
        _ characters: String?,
        modifiers: NSEvent.ModifierFlags = [],
        isNamedSpecialKey: Bool = false,
    ) -> KeyEventSnapshot {
        KeyEventSnapshot(
            characters: characters,
            modifiers: modifiers,
            isNamedSpecialKey: isNamedSpecialKey,
        )
    }

    private func navigationSnapshot(
        _ key: NavigationKey,
        modifiers: NSEvent.ModifierFlags = [],
    ) -> KeyEventSnapshot {
        KeyEventSnapshot(
            characters: nil,
            modifiers: modifiers,
            isNamedSpecialKey: true,
            navigationKey: key,
        )
    }
}
