// The full-width punctuation policy: the map's rows.

@testable import TaigiInputMethodCore
import XCTest

/// The pure policy — what maps and what never does. When it applies at all is
/// the 漢羅對調 mode, read by the controller and covered by
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
            "'": "、",
        ]

        for (typed, fullWidth) in expected {
            XCTAssertEqual(FullWidthPunctuation.mapped(typed), fullWidth, "for \(typed)")
        }
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
        // `AutoSpacePunctuation` excludes it from the attaching set.
        XCTAssertNil(FullWidthPunctuation.mapped("\""))
        XCTAssertNil(FullWidthPunctuation.mapped(" "))
    }

    func testMultiCharacterText_neverMaps() {
        XCTAssertNil(FullWidthPunctuation.mapped("?!"))
        XCTAssertNil(FullWidthPunctuation.mapped(""))
    }
}
