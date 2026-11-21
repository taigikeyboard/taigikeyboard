// Pins the pure key rules ActionHandler shares with Android TextInputKeyHandler.kt:
// the action-field Return, single-shot Shift consumption and which keys keep
// the auto-space arm.

import KeyboardKit
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

    func testKeepsAutoSpaceArm_shiftAndPageKeysOnly() {
        // trace: `keepsAutoSpaceArm` — `.shift` and every `.keyboardType`
        // write nothing; a character, space or backspace writes the document.
        let keeps: [KeyboardAction] = [
            .shift(.lowercased), .keyboardType(.numeric),
            .keyboardType(.symbolic), .keyboardType(.alphabetic), .keyboardType(.emojis),
        ]
        for action in keeps {
            XCTAssertTrue(ActionHandler.keepsAutoSpaceArm(action), "\(action)")
        }
        let writes: [KeyboardAction] = [.character("…"), .character("?"), .space, .backspace]
        for action in writes {
            XCTAssertFalse(ActionHandler.keepsAutoSpaceArm(action), "\(action)")
        }
    }
}
