import KeyboardKit
@testable import TaigiKeyboard
import XCTest

/// Pins the toolbar keyboard button's tap rule and the one-handed storage values
/// (shared with Android `OneHandedModeTest`).
final class OneHandedModeTests: XCTestCase {
    func testRawValuesMatchAndroidStorage() {
        XCTAssertEqual(OneHandedMode.allCases.map(\.rawValue), ["off", "left", "right"])
        XCTAssertEqual(KeyboardToolbarAction.allCases.map(\.rawValue), ["dismiss", "left", "right"])
    }

    func testDismissActionAlwaysDismisses() {
        for mode in OneHandedMode.allCases {
            XCTAssertNil(KeyboardToolbarAction.dismiss.tapResult(current: mode))
        }
    }

    func testSideActionTogglesBetweenSideAndOff() {
        XCTAssertEqual(KeyboardToolbarAction.right.tapResult(current: .off), .right)
        XCTAssertEqual(KeyboardToolbarAction.right.tapResult(current: .right), .off)
        XCTAssertEqual(KeyboardToolbarAction.right.tapResult(current: .left), .right)
        XCTAssertEqual(KeyboardToolbarAction.left.tapResult(current: .off), .left)
        XCTAssertEqual(KeyboardToolbarAction.left.tapResult(current: .left), .off)
        XCTAssertEqual(KeyboardToolbarAction.left.tapResult(current: .right), .left)
    }

    func testActionForModeSkipsOff() {
        XCTAssertNil(KeyboardToolbarAction(mode: .off))
        XCTAssertEqual(KeyboardToolbarAction(mode: .left), .left)
        XCTAssertEqual(KeyboardToolbarAction(mode: .right), .right)
    }

    func testFlippedAndDockEdge() {
        XCTAssertEqual(OneHandedMode.left.flipped, .right)
        XCTAssertEqual(OneHandedMode.right.flipped, .left)
        XCTAssertEqual(OneHandedMode.off.flipped, .off)
        XCTAssertNil(OneHandedMode.off.dockEdge)
        XCTAssertEqual(OneHandedMode.left.dockEdge, .leading)
        XCTAssertEqual(OneHandedMode.right.dockEdge, .trailing)
    }
}
