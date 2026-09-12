// Executable spec for the tone scheme: which keys type a tone, and the slot
// set that follows from it.

@testable import TaigiInputMethodCore
import XCTest

/// What these pin: the slot key set is DERIVED from the scheme, and the eight
/// Telex keys are exactly the eight letters the Standard slot row holds — the
/// same keys, swapped, so neither scheme can take a key a syllable needs.
final class ToneInputSchemeTests: XCTestCase {
    func testStandard_isTheShippedScheme() {
        XCTAssertEqual(ComposingKeyBindings.default.toneScheme, .standard)
        XCTAssertEqual(ToneInputScheme.allCases.map(\.rawValue), ["standard", "telex"], "the stored spellings")
    }

    func testTheSlotKeySet_followsTheScheme() {
        XCTAssertEqual(ToneInputScheme.standard.slotKeySet, .bareKeys)
        XCTAssertEqual(ToneInputScheme.telex.slotKeySet, .digits)
        XCTAssertEqual(ComposingKeyBindings(toneScheme: .telex).slotKeySet, .digits)
    }

    /// The eight Telex keys are the eight letters of the Standard slot row:
    /// what one scheme types tones with, the other picks with.
    func testTheTelexKeys_areTheLettersOfTheBareSlotRow() {
        let slotLetters = Set(CandidateSlotKeySet.bareKeyRow.compactMap(\.first).filter(\.isLetter))
        XCTAssertEqual(ToneInputScheme.telexKeys, slotLetters)
        XCTAssertEqual(ToneInputScheme.telexKeys, ["v", "y", "d", "w", "x", "q", "z", "f"])
    }

    /// Mirrors the engine's `TELEX_KEYS` — the engine test of the same
    /// shape walks the alphabet too, so a key added on one side fails here.
    func testIsTelexKey_namesTheEightKeys_inEitherCase_andNothingElse() {
        for letter in "abcdefghijklmnopqrstuvwxyz" {
            let expected = ToneInputScheme.telexKeys.contains(letter)
            XCTAssertEqual(ToneInputScheme.isTelexKey(letter), expected, "\(letter)")
            XCTAssertEqual(ToneInputScheme.isTelexKey(Character(letter.uppercased())), expected, "\(letter.uppercased())")
        }
        for other: Character in ["1", "-", ";", " ", "ｖ", "ｚ"] {
            XCTAssertFalse(ToneInputScheme.isTelexKey(other), "\(other)")
        }
    }

    /// Only `z` types an initial; a tone letter or `f` has nothing to attach
    /// to when idle.
    func testOnlyZ_startsAComposition() {
        XCTAssertTrue(ToneInputScheme.startsComposition("z"))
        XCTAssertTrue(ToneInputScheme.startsComposition("Z"))
        for key in ToneInputScheme.telexKeys where key != "z" {
            XCTAssertFalse(ToneInputScheme.startsComposition(key), "\(key)")
        }
    }
}
