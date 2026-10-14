@testable import TaigiKeyboard
import UIKit
import XCTest

/// Pins the idle Return-key icon per `UIReturnKeyType`, grouped as Android
/// `KeyContent.kt` `resolveEnterVisual` (done / arrow / search / send / newline).
final class ReturnKeySymbolNameTests: XCTestCase {
    func testEveryReturnKeyType() {
        let rows: [(UIReturnKeyType?, String)] = [
            (nil, "arrow.turn.down.left"),
            (.default, "arrow.turn.down.left"),
            (.done, "checkmark"),
            (.go, "arrow.right"),
            (.next, "arrow.right"),
            (.join, "arrow.right"),
            (.route, "arrow.right"),
            (.continue, "arrow.right"),
            (.emergencyCall, "arrow.right"),
            (.search, "magnifyingglass"),
            (.google, "magnifyingglass"),
            (.yahoo, "magnifyingglass"),
            (.send, "paperplane"),
        ]
        for (returnKeyType, expected) in rows {
            XCTAssertEqual(
                ButtonImageProvider.returnKeySymbolName(for: returnKeyType),
                expected,
                String(describing: returnKeyType),
            )
        }
    }
}
