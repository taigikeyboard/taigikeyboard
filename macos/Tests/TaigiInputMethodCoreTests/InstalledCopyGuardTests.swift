// Executable spec for the guard that keeps a stray copy from stealing IMK events.

import XCTest

@testable import TaigiInputMethodCore

/// If this guard wrongly accepts a build product, two processes with the same
/// bundle ID race for one IMKServer connection name and the loser silently
/// receives no key events. If it wrongly rejects the installed copy, the input
/// method exits at launch.
final class InstalledCopyGuardTests: XCTestCase {
    func testIsInputMethodsCopy_installLocations_areAccepted() {
        let acceptedPaths = [
            "/Users/someone/Library/Input Methods/TaigiKeyboard.app",
            "/Library/Input Methods/TaigiKeyboard.app",
            "/Users/someone/Library/Input Methods/./TaigiKeyboard.app",
        ]

        for path in acceptedPaths {
            XCTAssertTrue(
                AppDelegate.isInputMethodsCopy(URL(fileURLWithPath: path)),
                "\(path) is an install location and must be accepted",
            )
        }
    }

    func testIsInputMethodsCopy_otherLocations_areRejected() {
        let rejectedPaths = [
            "/Users/someone/repo/macos/.build/bundle/TaigiKeyboard.app",
            "/Applications/TaigiKeyboard.app",
            "/Users/someone/Library/Input Methods/nested/TaigiKeyboard.app",
        ]

        for path in rejectedPaths {
            XCTAssertFalse(
                AppDelegate.isInputMethodsCopy(URL(fileURLWithPath: path)),
                "\(path) is not an install location and must be rejected",
            )
        }
    }
}
