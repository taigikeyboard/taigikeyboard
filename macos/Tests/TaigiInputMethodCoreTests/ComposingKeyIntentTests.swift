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

    /// Control rewrites the characters of the digits it is chorded with —
    /// `⌃3` arrives as `\u{1B}` — and the fixed tier must not read that as
    /// an Escape and cancel the composition: a Control chord is the host's,
    /// under either scheme, bar up or not. (The `⌃1`…`⌃9` slot set went with
    /// the picker that chose it, 2026-09-08.)
    func testControlDigits_belongToTheHost_underEitherScheme() throws {
        let controlThree = try TestFixtures.keyDownEvent(
            characters: "\u{1B}",
            modifiers: .control,
            charactersIgnoringModifiers: "3",
        )

        for scheme in ToneInputScheme.allCases {
            for isShowingCandidates in [true, false] {
                XCTAssertEqual(
                    ComposingKeyIntent.intent(
                        for: KeyEventSnapshot(controlThree),
                        isComposing: true,
                        isShowingCandidates: isShowingCandidates,
                        bindings: ComposingKeyBindings(toneScheme: scheme),
                    ),
                    .commitThenPassThrough,
                    "⌃3 is the host's shortcut under \(scheme), bar \(isShowingCandidates ? "up" : "down")",
                )
            }
        }
    }

    // MARK: - Telex

    private func telexIntent(
        _ characters: String,
        isComposing: Bool = true,
        isShowingCandidates: Bool = false,
    ) throws -> ComposingKeyIntent {
        try ComposingKeyIntent.intent(
            for: KeyEventSnapshot(TestFixtures.keyDownEvent(characters: characters)),
            isComposing: isComposing,
            isShowingCandidates: isShowingCandidates,
            bindings: ComposingKeyBindings(toneScheme: .telex),
        )
    }

    /// A tone letter mid-composition is the engine's Telex key, in either
    /// case — `V` carries the same tone as `v`.
    func testTelex_aToneLetterWhileComposing_isATelexKey() throws {
        for key in ["v", "y", "d", "w", "x", "q", "f"] {
            XCTAssertEqual(try telexIntent(key), .telexKey(key), key)
        }
        XCTAssertEqual(try telexIntent("V"), .telexKey("V"))
    }

    /// Idle, a tone letter or `f` has no syllable to mark and passes to the
    /// host like an idle digit; `z` types an initial, so it starts one.
    func testTelex_idleKeys_passThroughExceptZ() throws {
        XCTAssertEqual(try telexIntent("v", isComposing: false), .passThrough)
        XCTAssertEqual(try telexIntent("f", isComposing: false), .passThrough)
        XCTAssertEqual(try telexIntent("z", isComposing: false), .telexKey("z"))
        XCTAssertEqual(try telexIntent("Z", isComposing: false), .telexKey("Z"))
    }

    /// The digits are the slot keys: with the bar up a digit picks; with no
    /// bar it is document text that ends the composition, never a tone.
    func testTelex_aDigit_picksWithTheBarUp_andIsDocumentTextWithout() throws {
        XCTAssertEqual(try telexIntent("3", isShowingCandidates: true), .selectCandidateSlot(2))
        XCTAssertEqual(try telexIntent("3"), .commitThenInsert("3"))
        XCTAssertEqual(try telexIntent("3", isComposing: false), .passThrough)
    }

    /// `q` is a tone key under Telex, not the first slot — even with the bar up.
    func testTelex_aBareLetter_isNotASlotKey() throws {
        XCTAssertEqual(try telexIntent("q", isShowingCandidates: true), .telexKey("q"))
    }

    /// The letters a syllable is spelled with are untouched by the scheme.
    func testTelex_syllableLetters_areStillInput() throws {
        for key in ["t", "a", "-", "c"] {
            XCTAssertEqual(try telexIntent(key), .input(key), key)
            XCTAssertEqual(try telexIntent(key, isComposing: false), .input(key), key)
        }
    }

    /// Under Standard nothing changed: a digit is the tone, `q` picks, and
    /// `v` is a letter the composition takes.
    func testStandard_isUnchangedByTheScheme() throws {
        let standard = ComposingKeyBindings(toneScheme: .standard)
        func intent(_ characters: String, isShowingCandidates: Bool = false) throws -> ComposingKeyIntent {
            try ComposingKeyIntent.intent(
                for: KeyEventSnapshot(TestFixtures.keyDownEvent(characters: characters)),
                isComposing: true, isShowingCandidates: isShowingCandidates, bindings: standard,
            )
        }
        XCTAssertEqual(try intent("3", isShowingCandidates: true), .input("3"))
        XCTAssertEqual(try intent("q", isShowingCandidates: true), .selectCandidateSlot(0))
        XCTAssertEqual(try intent("v"), .input("v"))
        XCTAssertEqual(try intent("z"), .input("z"))
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

    /// S33: with the window switched off there is never a candidate to take,
    /// so every candidate key ends the composition as typed — Return and
    /// Space alike, and the paging keys with them, so no candidate key is
    /// ever swallowed. The window is never up in that state, so the "bar up"
    /// meanings are not reachable and are not asserted.
    func testCandidateKeys_commitTheTypedText_whenTheWindowIsOff() throws {
        let windowOff = ComposingKeyBindings(isCandidateWindowEnabled: false)
        let cases: [(name: String, characters: String)] = [
            ("Return", "\r"),
            ("Space", " "),
            ("Tab", "\t"),
            ("Page forward", "]"),
            ("Page backward", "["),
        ]

        for testCase in cases {
            let event = try TestFixtures.keyDownEvent(characters: testCase.characters)
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: true, bindings: windowOff),
                .commit,
                "\(testCase.name) writes the romanization as typed when there is no window to act on",
            )
            XCTAssertEqual(
                ComposingKeyIntent.intent(for: KeyEventSnapshot(event), isComposing: false, bindings: windowOff),
                .passThrough,
                "\(testCase.name) is the host's with no composition, window or not",
            )
        }
        // The negative control: with the window ON and no bar up, the same
        // keys keep their shipped meanings (`testReturn_endsTheCompositionWithNoBarUp`).
        let returnEvent = try TestFixtures.keyDownEvent(characters: "\r")
        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(returnEvent), isComposing: true),
            .commitThenPassThrough,
        )
        let spaceEvent = try TestFixtures.keyDownEvent(characters: " ")
        XCTAssertEqual(
            ComposingKeyIntent.intent(for: KeyEventSnapshot(spaceEvent), isComposing: true),
            .commitThenInsert(" "),
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
