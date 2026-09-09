// The keys that pick a candidate out of the nine slots: the bare letters
// under Standard, the bare digits under Telex.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// What these pin: a key `CandidateSlotKeySet` names for a slot is the key the
/// classifier picks that slot on — and only that key, only with the bar up,
/// only under the scheme that puts it there. The snapshots are built by hand
/// where a layout is being described: `⇧3` types `#` on a US layout.
@MainActor
final class CandidateSlotKeyTests: XCTestCase {
    private func snapshot(
        _ characters: String,
        modifiers: NSEvent.ModifierFlags = [],
        charactersIgnoringModifiers: String? = nil,
        keyCode: UInt16? = nil,
    ) -> KeyEventSnapshot {
        KeyEventSnapshot(
            characters: characters,
            modifiers: modifiers,
            isNamedSpecialKey: false,
            charactersIgnoringModifiers: charactersIgnoringModifiers,
            keyCode: keyCode,
        )
    }

    private func intent(
        _ key: KeyEventSnapshot,
        isComposing: Bool = true,
        isShowingCandidates: Bool = true,
        scheme: ToneInputScheme = .standard,
    ) -> ComposingKeyIntent {
        ComposingKeyIntent.intent(
            for: key,
            isComposing: isComposing,
            isShowingCandidates: isShowingCandidates,
            bindings: ComposingKeyBindings(toneScheme: scheme),
        )
    }

    /// `⇧3` as a US layout reports it: `#` in both character fields, and the
    /// digit only in the key code.
    private func shiftedDigitUS(_ digit: Int) -> KeyEventSnapshot {
        let symbols = ["!", "@", "#", "$", "%", "^", "&", "*", "("]
        return snapshot(
            symbols[digit - 1], modifiers: .shift,
            keyCode: ComposingKeyChord.numberRowKeyCodes[digit - 1],
        )
    }

    // MARK: - The bare keys, under Standard

    func testTheNineBareKeys_pickSlotsZeroToEight_whileTheBarIsUp() {
        XCTAssertEqual(CandidateSlotKeySet.bareKeyRow.count, HorizontalPageLayout.pageSize, "one key per slot")
        for (slot, key) in CandidateSlotKeySet.bareKeyRow.enumerated() {
            XCTAssertEqual(intent(snapshot(key)), .selectCandidateSlot(slot, flip: false), key)
        }
    }

    /// The set is the shipped one: a fresh `ComposingKeyBindings` reads it,
    /// and so does a store with nothing recorded.
    func testTheBareKeys_areTheShippedSet() throws {
        XCTAssertEqual(ComposingKeyBindings.default.slotKeySet, .bareKeys)
        XCTAssertEqual(try makeScratchSettingsStore().composingKeyBindings.slotKeySet, .bareKeys)
    }

    /// A bare digit never picks under Standard: it is the tone marker, even
    /// where no tone could follow (`tai5` + `2` → `tai52`, kept verbatim by
    /// the engine). One set of keys picks; a digit always means one thing.
    func testABareDigit_isAlwaysTheTone_underStandard() {
        XCTAssertEqual(intent(snapshot("2")), .input("2"))
        XCTAssertEqual(intent(snapshot("2"), isShowingCandidates: false), .input("2"))
        XCTAssertEqual(
            intent(snapshot("2"), isComposing: false, isShowingCandidates: false), .passThrough,
            "outside a composition a digit is the host's",
        )
    }

    /// Caps Lock is a latched state, not a held chord: the letter still picks.
    /// Shift is the 漢羅 chord on the same key: ⇧Q commits slot 0 in the
    /// other script (USER 2026-09-10), so the capital no longer reaches the
    /// composition while the bar is up.
    func testALetter_underCapsLock_stillPicks_andShiftedFlipsTheScript() {
        XCTAssertEqual(intent(snapshot("Q", modifiers: .capsLock)), .selectCandidateSlot(0, flip: false))
        XCTAssertEqual(intent(snapshot("Q", modifiers: .shift)), .selectCandidateSlot(0, flip: true))
    }

    // MARK: - ⇧ on a slot key: the 漢羅 commit aimed at the slot

    /// `⇧;` as a US layout reports it: `:` in both character fields, and the
    /// `;` only in the key code.
    private func shiftedSemicolonUS() -> KeyEventSnapshot {
        snapshot(":", modifiers: .shift, keyCode: ComposingKeyChord.semicolonKeyCode)
    }

    /// Every bare key flips its slot under ⇧ — the letters by their capital,
    /// `;` by its key code since a US layout types `:` for it.
    func testEveryBareKey_underShift_flipsItsSlot() {
        for (slot, key) in CandidateSlotKeySet.bareKeyRow.dropLast().enumerated() {
            XCTAssertEqual(
                intent(snapshot(key.uppercased(), modifiers: .shift)),
                .selectCandidateSlot(slot, flip: true), key,
            )
        }
        XCTAssertEqual(intent(shiftedSemicolonUS()), .selectCandidateSlot(8, flip: true))
    }

    /// A shifted digit flips its slot under Telex, where the digits are the
    /// slot keys — read off the key code, since a US layout types `&` for
    /// `⇧7`. Under Standard the digits are tones, and the shifted key is
    /// still the punctuation it types.
    func testAShiftedDigit_flipsItsSlot_underTelex_andTypesUnderStandard() {
        XCTAssertEqual(intent(shiftedDigitUS(7), scheme: .telex), .selectCandidateSlot(6, flip: true))
        XCTAssertEqual(intent(shiftedDigitUS(5), scheme: .telex), .selectCandidateSlot(4, flip: true))
        XCTAssertEqual(intent(shiftedDigitUS(7), scheme: .standard), .commitThenInsert("&"))
        XCTAssertEqual(intent(shiftedSemicolonUS(), scheme: .telex), .commitThenInsert(":"))
    }

    /// Only exactly ⇧ flips: any host chord beside it is the host's, and a
    /// symbol reached without the key — a layout with its own `&` — is the
    /// punctuation it types.
    func testShiftWithAHostChord_orWithoutTheKey_doesNotFlip() {
        XCTAssertEqual(
            intent(snapshot("Q", modifiers: [.shift, .control])), .commitThenPassThrough,
        )
        XCTAssertEqual(intent(snapshot("&", modifiers: .shift), scheme: .telex), .commitThenInsert("&"))
    }

    /// With no bar up there is no slot to flip: ⇧Q is the capital the
    /// composition takes as text again, and `⇧;` the `:` it types.
    func testAShiftedSlotKey_isItselfWhereverTheBarIsDown() {
        XCTAssertEqual(intent(snapshot("Q", modifiers: .shift), isShowingCandidates: false), .input("Q"))
        XCTAssertEqual(intent(shiftedSemicolonUS(), isShowingCandidates: false), .commitThenInsert(":"))
    }

    /// The bare keys are the slot tier's only while the bar is up: with none,
    /// `q` is a letter a custom-dictionary romanization may need, and `;` is
    /// the punctuation it types.
    func testABareKey_isItselfWhereverItIsNotASlotKey() {
        XCTAssertEqual(intent(snapshot("q"), isShowingCandidates: false), .input("q"))
        XCTAssertEqual(intent(snapshot("q"), isComposing: false, isShowingCandidates: false), .input("q"))
        XCTAssertEqual(intent(snapshot(";"), isShowingCandidates: false), .commitThenInsert(";"))
    }

    /// A syllable letter, and the punctuation typed straight after a word to
    /// end it, stay what they were: the set takes no key a composition needs.
    func testKeysOutsideTheSet_stayWhatTheyType() {
        XCTAssertEqual(intent(snapshot("a")), .input("a"))
        for punctuation in [",", ".", "'"] {
            XCTAssertEqual(intent(snapshot(punctuation)), .commitThenInsert(punctuation), punctuation)
        }
    }

    // MARK: - The digits, under Telex

    func testTheNineDigits_pickSlotsZeroToEight_whileTheBarIsUp_underTelex() {
        XCTAssertEqual(ToneInputScheme.telex.slotKeySet, .digits)
        for slot in 0 ..< HorizontalPageLayout.pageSize {
            XCTAssertEqual(intent(snapshot(String(slot + 1)), scheme: .telex), .selectCandidateSlot(slot, flip: false))
        }
        XCTAssertEqual(intent(snapshot("0"), scheme: .telex), .commitThenInsert("0"), "`0` names no slot")
    }

    /// The letters are the scheme's tone keys under Telex, not slot keys —
    /// and `;` is back to being the punctuation it types.
    func testTheBareKeys_doNotPick_underTelex() {
        XCTAssertEqual(intent(snapshot("q"), scheme: .telex), .telexKey("q"))
        XCTAssertEqual(intent(snapshot(";"), scheme: .telex), .commitThenInsert(";"))
    }

    /// The bar is what makes a digit a pick: with none up, a digit
    /// mid-composition is not a tone under Telex either, so it ends the
    /// composition and lands in the document like punctuation.
    func testADigit_withNoBarUp_isDocumentText_underTelex() {
        XCTAssertEqual(intent(snapshot("3"), isShowingCandidates: false, scheme: .telex), .commitThenInsert("3"))
        XCTAssertEqual(
            intent(snapshot("3"), isComposing: false, isShowingCandidates: false, scheme: .telex), .passThrough,
        )
    }

    /// Caps Lock and the number pad say how a key was reached, not which key
    /// it is: a keypad `3` still picks the third candidate.
    func testAKeypadDigit_stillPicks_underTelex() {
        for extra in [NSEvent.ModifierFlags.numericPad, [.numericPad, .function], .capsLock] {
            XCTAssertEqual(
                intent(snapshot("3", modifiers: extra), scheme: .telex),
                .selectCandidateSlot(2, flip: false), "\(extra)",
            )
        }
    }

    /// Both sets are bare keys: any chording modifier makes the key miss the
    /// slot and reach the host as the chord it is.
    func testAChordedSlotKey_belongsToTheHost_underEitherScheme() {
        for modifier in [NSEvent.ModifierFlags.command, .control, .option] {
            XCTAssertEqual(
                intent(snapshot("3", modifiers: modifier, charactersIgnoringModifiers: "3"), scheme: .telex),
                .commitThenPassThrough, "\(modifier)3 under Telex",
            )
            XCTAssertEqual(
                intent(snapshot("q", modifiers: modifier, charactersIgnoringModifiers: "q")),
                .commitThenPassThrough, "\(modifier)Q under Standard",
            )
        }
    }

    // MARK: - What the recorder would store

    /// Every slot key of every set is a typing key to the gate, so no chord
    /// can ever sit on one — under either scheme, whichever is live when it
    /// is recorded. This is what lets the bindings skip a pass against the
    /// slot tier.
    func testEverySlotKey_ofEverySet_isRefusedAsAChord() {
        for keySet in CandidateSlotKeySet.allCases {
            for slot in 0 ..< HorizontalPageLayout.pageSize {
                let key = keySet.label(forSlot: slot)
                XCTAssertEqual(
                    ComposingKeyChord.make(key: key, modifiers: []),
                    .failure(.typesRomanization), "\(keySet) slot \(slot) (`\(key)`)",
                )
            }
        }
    }

    /// A shifted number-row key is refused as the digit it is, read off the
    /// key code since a US layout types `#` for it — the same answer the
    /// Carbon bridge gives when handed the unmodified `3` with Shift, so a
    /// press no composing row can hold is one no global row bridges to.
    func testAShiftedDigit_isRefusedAsTheDigitItIs_onBothPaths() throws {
        XCTAssertEqual(ComposingKeyChord.make(shiftedDigitUS(3)), .failure(.typesRomanization))
        XCTAssertEqual(ComposingKeyChord.make(key: "3", modifiers: .shift), .failure(.typesRomanization))
        // A symbol reached without the number row — a keypad or another
        // layout's own `#` key — is still the character it types.
        XCTAssertEqual(try ComposingKeyChord.make(snapshot("#", modifiers: .shift)).get().key, "#")
    }

    /// `⇧;` is refused the same way: it is the ninth slot key's 漢羅 chord,
    /// and a US layout types `:` for it. A `:` reached without that key
    /// still records.
    func testAShiftedSemicolon_isRefusedAsTheSlotKeyItIs() throws {
        XCTAssertEqual(ComposingKeyChord.make(shiftedSemicolonUS()), .failure(.typesRomanization))
        XCTAssertEqual(try ComposingKeyChord.make(snapshot(":", modifiers: .shift)).get().key, ":")
    }

    /// Every other shifted key keeps the character it types, which is what
    /// its stored chords already hold.
    func testOtherShiftedKeys_stillRecordTheCharacterTheyType() throws {
        let chord = try ComposingKeyChord.make(
            snapshot("{", modifiers: .shift, charactersIgnoringModifiers: "{"),
        ).get()

        XCTAssertEqual(chord.key, "{")
        XCTAssertEqual(chord.modifiers, .shift)
    }

    // MARK: - The labels

    func testTheLabels_nameTheKeysThemselves() {
        XCTAssertEqual((0 ..< 9).map { CandidateSlotKeySet.bareKeys.label(forSlot: $0) }, CandidateSlotKeySet.bareKeyRow)
        XCTAssertEqual(
            (0 ..< 9).map { CandidateSlotKeySet.digits.label(forSlot: $0) },
            ["1", "2", "3", "4", "5", "6", "7", "8", "9"],
        )
    }
}
