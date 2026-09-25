// The full-width punctuation policy: the map's rows.

@testable import TaigiInputMethodCore
import XCTest

/// The pure policy — what maps and what never does. When it applies at all is
/// the Hanji/romanization swap mode, read by the controller and covered by
/// `FullWidthPunctuationControllerTests`
/// (`testRomanFirstMode_passesPunctuationThrough`).
final class FullWidthPunctuationTests: XCTestCase {
    // MARK: - Which width is written

    /// The four cells of the contract: the bare key follows the mode, the
    /// width-flip chord types the other width — and is always the input
    /// method's to write, since the host would read the chord as a shortcut.
    func testDocumentPunctuation_flipTypesTheOtherWidthAndTheBareKeyTheModes() {
        XCTAssertEqual(FullWidthPunctuation.documentPunctuation(",", isFullWidthMode: true, isWidthFlip: false), "，")
        XCTAssertEqual(FullWidthPunctuation.documentPunctuation(",", isFullWidthMode: true, isWidthFlip: true), ",")
        XCTAssertNil(FullWidthPunctuation.documentPunctuation(",", isFullWidthMode: false, isWidthFlip: false))
        XCTAssertEqual(FullWidthPunctuation.documentPunctuation(",", isFullWidthMode: false, isWidthFlip: true), "，")
        // A key the map does not carry is the host's under the mode; the
        // classifier never reports it as a flip.
        XCTAssertNil(FullWidthPunctuation.documentPunctuation("5", isFullWidthMode: true, isWidthFlip: false))
    }

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
            "@": "＠", "#": "＃", "$": "＄", "%": "％", "^": "＾", "&": "＆", "*": "＊",
            "_": "＿", "+": "＋",
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
