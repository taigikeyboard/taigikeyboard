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
        keySet: CandidateSlotKeySet = .bareKeys,
    ) -> ComposingKeyIntent {
        ComposingKeyIntent.intent(
            for: key,
            isComposing: isComposing,
            isShowingCandidates: isShowingCandidates,
            bindings: ComposingKeyBindings(slotKeySet: keySet),
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

    // MARK: - The bare keys

    func testTheNineBareKeys_pickSlotsZeroToEight_whileTheBarIsUp() {
        XCTAssertEqual(CandidateSlotKeySet.bareKeyRow.count, HorizontalPageLayout.pageSize, "one key per slot")
        for (slot, key) in CandidateSlotKeySet.bareKeyRow.enumerated() {
            XCTAssertEqual(intent(snapshot(key)), .selectCandidateSlot(slot), key)
        }
    }

    /// The set is the shipped one: a fresh `ComposingKeyBindings` reads it,
    /// and so does a store with nothing recorded.
    func testTheBareKeys_areTheShippedSet() throws {
        XCTAssertEqual(ComposingKeyBindings.default.slotKeySet, .bareKeys)
        XCTAssertEqual(try makeScratchSettingsStore().composingKeyBindings.slotKeySet, .bareKeys)
    }

    /// A bare digit never picks, under any set: it is the tone marker, even
    /// where no tone could follow (`tai5` + `2` → `tai52`, kept verbatim by
    /// the engine). One set of keys picks; a digit always means one thing.
    func testABareDigit_isAlwaysTheTone_underEverySet() {
        for keySet in CandidateSlotKeySet.allCases {
            XCTAssertEqual(intent(snapshot("2"), keySet: keySet), .input("2"), "\(keySet)")
            XCTAssertEqual(intent(snapshot("2"), isShowingCandidates: false, keySet: keySet), .input("2"))
        }
        XCTAssertEqual(
            intent(snapshot("2"), isComposing: false, isShowingCandidates: false), .passThrough,
            "outside a composition a digit is the host's",
        )
    }

    /// Caps Lock is a latched state, not a held chord: the letter still picks.
    /// Shift is a chord, and ⇧Q is the capital the composition takes as text.
    func testALetter_underCapsLock_stillPicks_butShiftedIsTheCapital() {
        XCTAssertEqual(intent(snapshot("Q", modifiers: .capsLock)), .selectCandidateSlot(0))
        XCTAssertEqual(intent(snapshot("Q", modifiers: .shift, keyCode: kVK_ANSI_Q)), .input("Q"))
    }

    /// The bare keys are the slot tier's only while the bar is up: with none,
    /// or under another set, `q` is a letter a custom-dictionary romanization
    /// may need, and `;` is the punctuation it types.
    func testABareKey_isItselfWhereverItIsNotASlotKey() {
        XCTAssertEqual(intent(snapshot("q"), isShowingCandidates: false), .input("q"))
        XCTAssertEqual(intent(snapshot("q"), isComposing: false, isShowingCandidates: false), .input("q"))
        XCTAssertEqual(intent(snapshot("q"), keySet: .control), .input("q"))
        XCTAssertEqual(intent(snapshot("q"), keySet: .option), .input("q"))
        XCTAssertEqual(intent(snapshot(";"), isShowingCandidates: false), .commitThenInsert(";"))
        XCTAssertEqual(intent(snapshot(";"), keySet: .control), .commitThenInsert(";"))
    }

    /// A syllable letter, and the punctuation typed straight after a word to
    /// end it, stay what they were: the set takes no key a composition needs.
    func testKeysOutsideTheSet_stayWhatTheyType() {
        XCTAssertEqual(intent(snapshot("a")), .input("a"))
        for punctuation in [",", ".", "'"] {
            XCTAssertEqual(intent(snapshot(punctuation)), .commitThenInsert(punctuation), punctuation)
        }
    }

    /// One set is live at a time: under the bare keys a shifted digit is the
    /// punctuation it types, and a bare `7` is the tone it always is.
    func testUnderTheBareKeys_aShiftedDigitIsPunctuation_andABareDigitATone() throws {
        XCTAssertEqual(try intent(shiftedDigitUS(7)), .commitThenInsert("&"))
        XCTAssertEqual(intent(snapshot(";")), .selectCandidateSlot(8))
        XCTAssertEqual(intent(snapshot("7")), .input("7"))
    }

    // MARK: - The shifted digits

    func testAShiftedDigit_picksTheSlot_underTheShiftSet_only() throws {
        XCTAssertEqual(try intent(shiftedDigitUS(1), keySet: .shift), .selectCandidateSlot(0))
        XCTAssertEqual(try intent(shiftedDigitUS(5), keySet: .shift), .selectCandidateSlot(4))
        XCTAssertEqual(try intent(shiftedDigitUS(9), keySet: .shift), .selectCandidateSlot(8))
        for keySet in CandidateSlotKeySet.allCases where keySet != .shift {
            XCTAssertEqual(try intent(shiftedDigitUS(5), keySet: keySet), .commitThenInsert("%"), "\(keySet)")
        }
    }

    /// The reason the chord costs nothing: punctuation is typed after a
    /// composition ends, and then `⇧2` is the `@` it always was.
    func testAShiftedDigit_isTheSymbolItTypes_withNoBarUp() throws {
        XCTAssertEqual(
            try intent(shiftedDigitUS(2), isShowingCandidates: false, keySet: .shift), .commitThenInsert("@"),
        )
        XCTAssertEqual(
            try intent(shiftedDigitUS(2), isComposing: false, isShowingCandidates: false, keySet: .shift),
            .passThrough,
        )
    }

    /// Shift alone: with another chording modifier the chord is the host's —
    /// `⇧⌘3` is the system's screenshot.
    func testAShiftedDigit_withAnotherChordingModifier_belongsToTheHost() throws {
        for extra in [NSEvent.ModifierFlags.command, .control, .option] {
            XCTAssertEqual(
                try intent(shiftedDigitUS(3, extra: extra), keySet: .shift), .commitThenPassThrough, "\(extra)",
            )
        }
    }

    func testAShiftedZero_addressesNoSlot() {
        XCTAssertEqual(intent(shiftedZeroUS(), keySet: .shift), .commitThenInsert(")"))
    }

    /// On a layout whose digits are the SHIFTED characters — AZERTY types `&`
    /// bare and `1` with Shift — the same number-row key is pressed, and
    /// `⇧&` still picks the first candidate.
    func testAShiftedDigit_onALayoutWhereTheDigitIsTheShiftedCharacter_stillPicks() {
        let azertyShiftOne = snapshot("1", modifiers: .shift, charactersIgnoringModifiers: "1", keyCode: kVK_ANSI_1)
        XCTAssertEqual(intent(azertyShiftOne, keySet: .shift), .selectCandidateSlot(0))
    }

    /// A digit the number row does not carry — the keypad's, which Shift
    /// leaves alone — is read off the characters instead.
    func testAShiftedKeypadDigit_isReadOffTheCharacters() {
        let shiftKeypadThree = snapshot("3", modifiers: [.shift, .numericPad], keyCode: kVK_ANSI_Keypad3)
        XCTAssertEqual(intent(shiftKeypadThree, keySet: .shift), .selectCandidateSlot(2))
    }

    /// Through a real event, which is where the key code comes from.
    func testAShiftedDigit_fromAnAppKitEvent_readsTheDigitOffTheKeyCode() throws {
        let event = try TestFixtures.keyDownEvent(
            characters: "#", modifiers: .shift, keyCode: UInt16(kVK_ANSI_3),
        )
        let key = KeyEventSnapshot(event)

        XCTAssertEqual(key.keyCode, UInt16(kVK_ANSI_3))
        XCTAssertEqual(intent(key, keySet: .shift), .selectCandidateSlot(2))
    }

    // MARK: - What the recorder would store

    /// A shifted digit is judged as the digit, the way `⌃3` already is — so
    /// the recorder refuses it as the slot chord it is, whichever set is
    /// live, rather than accepting the `#` it types and storing a row that
    /// could never be matched.
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
        XCTAssertEqual(
            CandidateSlotKeySet.allCases.map(\.menuLabel),
            ["q w d f z x v y ;", "⇧1 – ⇧9", "⌃1 – ⌃9", "⌥1 – ⌥9"],
        )
    }

    // MARK: - A global row left on a shifted digit

    /// Before the refusal, the recorder accepted `⇧3` as the `#` it types, so
    /// a global row can still hold one — and Carbon would dispatch it before
    /// the classifier saw the digit. The launch pass clears it.
    func testALaunchPass_clearsAGlobalRowLeftOnAShiftedDigit() throws {
        let saved = KeyboardShortcuts.getShortcut(for: .openLastSettingsPane)
        addTeardownBlock { KeyboardShortcuts.setShortcut(saved, for: .openLastSettingsPane) }
        KeyboardShortcuts.setShortcut(.init(.three, modifiers: [.shift]), for: .openLastSettingsPane)

        try ShortcutConflicts.resolveAcrossRegistries(in: makeScratchSettingsStore())

        XCTAssertNil(KeyboardShortcuts.getShortcut(for: .openLastSettingsPane))
    }
}
