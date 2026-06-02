@testable import TaigiKeyboard
import XCTest

/// Pins the attaching-punctuation set for the auto-space "smart punctuation"
/// swap. INVARIANT_AUTO_SPACE_PUNCTUATION_SWAP — mirrors the Android
/// `AutoSpacePunctuationTest`; the two sets must stay identical.
final class AutoSpacePunctuationTests: XCTestCase {
    func testINVARIANT_attaching_sentenceEnd() {
        for ch in ["。", "！", "？", ".", "!", "?"] {
            XCTAssertTrue(AutoSpacePunctuation.isAttaching(ch), "sentence-end '\(ch)' must attach")
        }
    }

    func testINVARIANT_attaching_clauseSeparators() {
        for ch in ["，", ",", "、", "；", ";", "：", ":"] {
            XCTAssertTrue(AutoSpacePunctuation.isAttaching(ch), "clause '\(ch)' must attach")
        }
    }

    func testINVARIANT_attaching_closingBracketsAndQuotes() {
        for ch in [")", "）", "]", "】", "」", "』"] {
            XCTAssertTrue(AutoSpacePunctuation.isAttaching(ch), "closing '\(ch)' must attach")
        }
    }

    func testINVARIANT_notAttaching_openingBracketsAndQuotes() {
        for ch in ["(", "（", "[", "【", "「", "『"] {
            XCTAssertFalse(AutoSpacePunctuation.isAttaching(ch), "opening '\(ch)' must NOT attach")
        }
    }

    func testINVARIANT_notAttaching_asciiStraightQuotes() {
        for ch in ["\"", "'"] {
            XCTAssertFalse(AutoSpacePunctuation.isAttaching(ch), "straight quote '\(ch)' must NOT attach")
        }
    }

    func testINVARIANT_notAttaching_lettersAndDigits() {
        for ch in ["a", "A", "5", "-", "我", "â"] {
            XCTAssertFalse(AutoSpacePunctuation.isAttaching(ch), "'\(ch)' must NOT attach")
        }
    }

    func testINVARIANT_notAttaching_multiCharOrEmpty() {
        XCTAssertFalse(AutoSpacePunctuation.isAttaching(""))
        XCTAssertFalse(AutoSpacePunctuation.isAttaching("?!"))
        XCTAssertFalse(AutoSpacePunctuation.isAttaching("? "))
    }
}
