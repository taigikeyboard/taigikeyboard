// Pins the pure key rules ActionHandler shares with Android TextInputKeyHandler.kt:
// the action-field Return and single-shot Shift consumption.

@testable import TaigiKeyboard
import UIKit
import XCTest

final class ActionHandlerKeyRulesTests: XCTestCase {
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
