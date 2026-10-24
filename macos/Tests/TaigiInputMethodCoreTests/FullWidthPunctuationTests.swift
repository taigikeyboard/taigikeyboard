// The full-width punctuation policy: the map's rows.

@testable import TaigiInputMethodCore
import XCTest

/// The pure policy — what maps and what never does. When it applies at all is
/// the Hanji/romanization swap mode, read by the controller and covered by
/// `FullWidthPunctuationControllerTests`
/// (`testRomanFirstMode_passesPunctuationThrough`).
final class FullWidthPunctuationTests: XCTestCase {
    // MARK: - The map

    func testEveryMappedPair_followsTheMOETable() {
        let expected: [String: String] = [
            ",": "，", ".": "。", "?": "？", "!": "！",
            ";": "；", ":": "：",
            "(": "（", ")": "）",
            "[": "「", "]": "」",
            "{": "『", "}": "』",
            "<": "《", ">": "》",
            "'": "、", "\\": "、",
            "~": "～", "@": "＠", "#": "＃", "$": "＄", "%": "％", "^": "＾", "&": "＆", "*": "＊",
            "_": "＿", "+": "＋",
        ]

        for (typed, fullWidth) in expected {
            XCTAssertEqual(FullWidthPunctuation.mapped(typed), fullWidth, "for \(typed)")
        }
    }

    func testTilde_mapsButItsControlChordStaysTheHosts() {
        // trace: `~` is in `map` and in `hostChordKeys` → mapped, no chord.
        XCTAssertEqual(FullWidthPunctuation.mapped("~"), "～")
        XCTAssertFalse(FullWidthPunctuation.isPunctuationChordKey("~"))
        XCTAssertTrue(FullWidthPunctuation.isPunctuationChordKey(","))
        XCTAssertFalse(FullWidthPunctuation.isPunctuationChordKey("5"))
    }

    func testToneDigitsSyllableCharactersAndTheHyphen_neverMap() {
        // Digits carry TL/POJ tones (`tai5`), the hyphen separates syllables,
        // and letters spell the romanization — all must stay half-width.
        for typed in ["0", "1", "5", "9", "-", "a", "g", "A"] {
            XCTAssertNil(FullWidthPunctuation.mapped(typed), "\(typed) must stay half-width")
        }
    }

    func testTheAmbiguousStraightQuoteAndSpace_neverMap() {
        // The straight double quote opens and closes with the same glyph, so a
        // one-to-one map cannot pick a side — the same reason
        // the engine's auto-space attaching set excludes it.
        XCTAssertNil(FullWidthPunctuation.mapped("\""))
        XCTAssertNil(FullWidthPunctuation.mapped(" "))
    }

    func testMultiCharacterText_neverMaps() {
        XCTAssertNil(FullWidthPunctuation.mapped("?!"))
        XCTAssertNil(FullWidthPunctuation.mapped(""))
    }
}
