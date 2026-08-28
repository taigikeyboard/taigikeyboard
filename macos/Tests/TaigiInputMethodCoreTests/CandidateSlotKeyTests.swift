// The keys that pick a candidate out of the nine slots: the set the user
// chooses, and the shifted digits every set shares.

import AppKit
import Carbon.HIToolbox
import KeyboardShortcuts
@testable import TaigiInputMethodCore
import XCTest

/// What these pin: a key `CandidateSlotKeySet` names for a slot is the key the
/// classifier picks that slot on — and only that key, only with the bar up. The
/// snapshots are built by hand where a layout is being described: `⇧3` types
/// `#` on a US layout and `⇧&` types `1` on AZERTY, and in both the number-row
/// key code is what the chord is read from.
@MainActor
final class CandidateSlotKeyTests: XCTestCase {
    private func snapshot(
        _ characters: String,
        modifiers: NSEvent.ModifierFlags = [],
        charactersIgnoringModifiers: String? = nil,
        keyCode: Int? = nil,
    ) -> KeyEventSnapshot {
        KeyEventSnapshot(
            characters: characters,
            modifiers: modifiers,
            isNamedSpecialKey: false,
            charactersIgnoringModifiers: charactersIgnoringModifiers,
            keyCode: keyCode.map(UInt16.init),
        )
    }


    private func intent(
        _ key: KeyEventSnapshot,
        isComposing: Bool = true,
        isShowingCandidates: Bool = true,
        keySet: CandidateSlotKeySet = .letters,
        rawInput: String = "tai",
    ) -> ComposingKeyIntent {
        ComposingKeyIntent.intent(
            for: key,
            isComposing: isComposing,
            isShowingCandidates: isShowingCandidates,
            bindings: ComposingKeyBindings(slotKeySet: keySet),
            rawInput: rawInput,
        )
    }

    /// `⇧3` as a US layout reports it: `#` in both character fields, the
    /// digit only in the key code.
    private func shiftedDigitUS(_ digit: Int, extra: NSEvent.ModifierFlags = []) throws -> KeyEventSnapshot {
        try KeyEventSnapshot(TestFixtures.shiftedDigitKeyDownEvent(slot: digit - 1, modifiers: extra.union(.shift)))
    }

    /// `⇧0`, which names no slot: `)` on a US layout.
    private func shiftedZeroUS() -> KeyEventSnapshot {
        snapshot(")", modifiers: .shift, keyCode: kVK_ANSI_0)
    }

    // MARK: - The letters

    func testTheSixLetters_pickSlotsZeroToFive_whileTheBarIsUp() {
        for (slot, letter) in CandidateSlotKeySet.letterKeys.enumerated() {
            XCTAssertEqual(intent(snapshot(letter)), .selectCandidateSlot(slot), letter)
        }
    }

    /// The set is the shipped one: a fresh `ComposingKeyBindings` reads it,
    /// and so does a store with nothing recorded.
    func testTheLetters_areTheShippedSet() throws {
        XCTAssertEqual(ComposingKeyBindings.default.slotKeySet, .letters)
        XCTAssertEqual(try makeScratchSettingsStore().composingKeyBindings.slotKeySet, .letters)
    }

    func testTheLetters_pickWhetherOrNotAToneCouldStillBeTyped() {
        XCTAssertEqual(intent(snapshot("q"), rawInput: "tai"), .selectCandidateSlot(0))
        XCTAssertEqual(intent(snapshot("q"), rawInput: "tai5"), .selectCandidateSlot(0))
    }

    /// Caps Lock is a latched state, not a held chord: the letter still picks.
    /// Shift is a chord, and ⇧Q is the capital the composition takes as text.
    func testALetter_underCapsLock_stillPicks_butShiftedIsTheCapital() {
        XCTAssertEqual(intent(snapshot("Q", modifiers: .capsLock)), .selectCandidateSlot(0))
        XCTAssertEqual(intent(snapshot("Q", modifiers: .shift, keyCode: kVK_ANSI_Q)), .input("Q"))
    }

    /// The letters are the slot tier's only while the bar is up: with none,
    /// or under another set, `q` is a letter a custom-dictionary romanization
    /// may need.
    func testALetter_isInputWhereverItIsNotASlotKey() {
        XCTAssertEqual(intent(snapshot("q"), isShowingCandidates: false), .input("q"))
        XCTAssertEqual(intent(snapshot("q"), isComposing: false, isShowingCandidates: false), .input("q"))
        XCTAssertEqual(intent(snapshot("q"), keySet: .control), .input("q"))
        XCTAssertEqual(intent(snapshot("q"), keySet: .option), .input("q"))
    }

    /// The two free letters the set leaves out, and a syllable letter, stay
    /// input.
    func testTheLettersOutsideTheSet_stayInput() {
        for letter in ["v", "y", "a"] {
            XCTAssertEqual(intent(snapshot(letter)), .input(letter), letter)
        }
    }

    /// Slots 6…8 have no letter; the shifted digits are how the set reaches
    /// them, and a bare `7` is still the tone it was.
    func testTheLastThreeSlots_underTheLetters_takeTheShiftedDigit() throws {
        XCTAssertEqual(try intent(shiftedDigitUS(7)), .selectCandidateSlot(6))
        XCTAssertEqual(intent(snapshot("7"), rawInput: "tai"), .input("7"))
    }

    // MARK: - The shifted digits

    func testAShiftedDigit_picksTheSlot_underEverySet() throws {
        for keySet in CandidateSlotKeySet.allCases {
            XCTAssertEqual(try intent(shiftedDigitUS(1), keySet: keySet), .selectCandidateSlot(0))
            XCTAssertEqual(try intent(shiftedDigitUS(5), keySet: keySet), .selectCandidateSlot(4))
            XCTAssertEqual(try intent(shiftedDigitUS(9), keySet: keySet), .selectCandidateSlot(8))
        }
    }

    /// Like the modifier chords, it picks whether or not a tone could still
    /// follow — that is what makes it the path for a toneless composition.
    func testAShiftedDigit_picksOnBothSidesOfTheToneRule() throws {
        XCTAssertEqual(try intent(shiftedDigitUS(2), rawInput: "tai"), .selectCandidateSlot(1))
        XCTAssertEqual(try intent(shiftedDigitUS(2), rawInput: "tai5"), .selectCandidateSlot(1))
    }

    /// The reason the chord costs nothing: punctuation is typed after a
    /// composition ends, and then `⇧2` is the `@` it always was.
    func testAShiftedDigit_isTheSymbolItTypes_withNoBarUp() throws {
        XCTAssertEqual(
            try intent(shiftedDigitUS(2), isShowingCandidates: false), .commitThenInsert("@"),
        )
        XCTAssertEqual(
            try intent(shiftedDigitUS(2), isComposing: false, isShowingCandidates: false),
            .passThrough,
        )
    }

    /// Shift alone: with another chording modifier the chord is the host's —
    /// `⇧⌘3` is the system's screenshot.
    func testAShiftedDigit_withAnotherChordingModifier_belongsToTheHost() throws {
        for extra in [NSEvent.ModifierFlags.command, .control, .option] {
            XCTAssertEqual(
                try intent(shiftedDigitUS(3, extra: extra)), .commitThenPassThrough, "\(extra)",
            )
        }
    }

    func testAShiftedZero_addressesNoSlot() {
        XCTAssertEqual(intent(shiftedZeroUS()), .commitThenInsert(")"))
    }

    /// On a layout whose digits are the SHIFTED characters — AZERTY types `&`
    /// bare and `1` with Shift — the same number-row key is pressed, and
    /// `⇧&` still picks the first candidate.
    func testAShiftedDigit_onALayoutWhereTheDigitIsTheShiftedCharacter_stillPicks() {
        let azertyShiftOne = snapshot("1", modifiers: .shift, charactersIgnoringModifiers: "1", keyCode: kVK_ANSI_1)
        XCTAssertEqual(intent(azertyShiftOne), .selectCandidateSlot(0))
    }

    /// A digit the number row does not carry — the keypad's, which Shift
    /// leaves alone — is read off the characters instead.
    func testAShiftedKeypadDigit_isReadOffTheCharacters() {
        let shiftKeypadThree = snapshot("3", modifiers: [.shift, .numericPad], keyCode: kVK_ANSI_Keypad3)
        XCTAssertEqual(intent(shiftKeypadThree), .selectCandidateSlot(2))
    }

    /// Through a real event, which is where the key code comes from.
    func testAShiftedDigit_fromAnAppKitEvent_readsTheDigitOffTheKeyCode() throws {
        let event = try TestFixtures.keyDownEvent(
            characters: "#", modifiers: .shift, keyCode: UInt16(kVK_ANSI_3),
        )
        let key = KeyEventSnapshot(event)

        XCTAssertEqual(key.keyCode, UInt16(kVK_ANSI_3))
        XCTAssertEqual(intent(key), .selectCandidateSlot(2))
    }

    // MARK: - What the recorder would store

    /// A shifted digit is judged as the digit, the way `⌃3` already is — so
    /// the recorder refuses it as the slot chord it is, rather than accepting
    /// the `#` it types and storing a row the slot tier would never let fire.
    func testAShiftedDigit_isRefusedAsASlotChord_notRecordedAsTheSymbolItTypes() throws {
        XCTAssertEqual(
            try ComposingKeyChord.make(shiftedDigitUS(3)),
            .failure(.candidateSlotChord),
        )
        XCTAssertEqual(
            try ComposingKeyChord.make(shiftedZeroUS()).get().key, ")",
            "⇧0 names no slot, so it records as the `)` it types",
        )
    }

    /// Digits only: every other shifted key keeps the character it types,
    /// which is what its stored chords already hold.
    func testOtherShiftedKeys_stillRecordTheCharacterTheyType() throws {
        let chord = try ComposingKeyChord.make(
            snapshot("{", modifiers: .shift, charactersIgnoringModifiers: "{", keyCode: kVK_ANSI_LeftBracket),
        ).get()

        XCTAssertEqual(chord.key, "{")
        XCTAssertEqual(chord.modifiers, .shift)
    }

    // MARK: - The picker

    func testTheMenuLabels_nameTheKeysThemselves() {
        XCTAssertEqual(CandidateSlotKeySet.allCases.map(\.menuLabel), ["q w d f z x", "⌃1 – ⌃9", "⌥1 – ⌥9"])
    }

    // MARK: - A global row left on a shifted digit

    /// Before the fixed tier, the recorder accepted `⇧3` as the `#` it types,
    /// so a global row can still hold one — and Carbon would dispatch it
    /// before the classifier saw the digit. The launch pass clears it.
    func testALaunchPass_clearsAGlobalRowLeftOnAShiftedDigit() throws {
        let saved = KeyboardShortcuts.getShortcut(for: .openLastSettingsPane)
        addTeardownBlock { KeyboardShortcuts.setShortcut(saved, for: .openLastSettingsPane) }
        KeyboardShortcuts.setShortcut(.init(.three, modifiers: [.shift]), for: .openLastSettingsPane)

        ShortcutConflicts.resolveAcrossRegistries(in: try makeScratchSettingsStore())

        XCTAssertNil(KeyboardShortcuts.getShortcut(for: .openLastSettingsPane))
    }
}
