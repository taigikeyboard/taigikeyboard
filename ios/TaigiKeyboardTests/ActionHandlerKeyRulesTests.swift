// Pins the pure key rules ActionHandler shares with Android TextInputKeyHandler.kt:
// the double-space period, the action-field Return, and single-shot Shift consumption.

@testable import TaigiKeyboard
import UIKit
import XCTest

final class ActionHandlerKeyRulesTests: XCTestCase {
    func testDoubleSpacePeriod_afterLetterNumberOrMark_writesPeriod() {
        // trace: last scalar before the space — "a" Ll, "7" Nd, "á" Ll,
        // "o͘" ends in U+0358 Mn, "台" Lo, "𠢕" U+20895 Lo (supplementary plane).
        for text in ["a ", "7 ", "á ", "o\u{0358} ", "台 ", "\u{20895} ", "x\u{20895} "] {
            XCTAssertEqual(
                ActionHandler.doubleSpacePeriodReplacement(textBeforeCaret: text, isFullWidthPunctuation: false),
                ". ",
                text,
            )
            XCTAssertEqual(
                ActionHandler.doubleSpacePeriodReplacement(textBeforeCaret: text, isFullWidthPunctuation: true),
                "。",
                text,
            )
        }
    }

    func testDoubleSpacePeriod_afterPunctuationWhitespaceOrEmoji_keepsSpace() {
        for text in [". ", "。 ", "? ", "  ", "\t ", "\n ", "\u{1F600} ", " ", "", "a", "ab"] {
            XCTAssertNil(
                ActionHandler.doubleSpacePeriodReplacement(textBeforeCaret: text, isFullWidthPunctuation: false),
                text,
            )
        }
        XCTAssertNil(ActionHandler.doubleSpacePeriodReplacement(textBeforeCaret: nil, isFullWidthPunctuation: false))
    }

    func testActionReturnKey_actionLabels_areActions() {
        let actions: [UIReturnKeyType] = [
            .go, .google, .join, .next, .route, .search, .send, .yahoo, .done, .emergencyCall, .continue,
        ]
        for type in actions {
            XCTAssertTrue(ActionHandler.isActionReturnKey(type), "\(type.rawValue)")
        }
    }

    func testActionReturnKey_defaultOrUnknown_isNewline() {
        XCTAssertFalse(ActionHandler.isActionReturnKey(.default))
        XCTAssertFalse(ActionHandler.isActionReturnKey(nil))
    }

    func testConsumesSingleShift_casedLettersOnly() {
        // U+10400 DESERET CAPITAL LONG I — a scalar, as Android reads a code point.
        for char in ["a", "Z", "á", "o\u{0358}", "\u{10400}"] {
            XCTAssertTrue(ActionHandler.consumesSingleShift(char), char)
        }
        for char in ["-", "2", "8", "ㄅ", "ˋ", "", "."] {
            XCTAssertFalse(ActionHandler.consumesSingleShift(char), char)
        }
    }
}
