// The chrome generation the candidate window draws, and the backdrop it builds.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The chrome follows the running OS with no setting in between, so the
/// contract worth pinning is the one at the end of that chain rather than the
/// version check itself: the view actually installed as the panel's background
/// is glass exactly where `NSGlassEffectView` exists.
@MainActor
final class CandidateWindowStyleTests: XCTestCase {
    /// The system style must reach a backdrop the OS can really draw. Asserted
    /// against the constructed view rather than against `systemStyle`'s own
    /// branch, so moving that threshold off macOS 26 fails here.
    func testTheBackdropForTheSystemStyle_isGlassOnlyWhereGlassExists() {
        let backdrop = CandidateBackdrop.make(style: .systemStyle)

        if #available(macOS 26, *) {
            XCTAssertTrue(backdrop.view is NSGlassEffectView)
            // Glass requires its content in a designated container, so unlike
            // vibrancy the two are different views.
            XCTAssertFalse(backdrop.view === backdrop.contentArea)
        } else {
            XCTAssertTrue(backdrop.view is NSVisualEffectView)
            XCTAssertTrue(backdrop.view === backdrop.contentArea)
        }
    }

    /// Sequoia draws vibrancy on every OS the app runs on, macOS 26 included —
    /// which is what lets the geometry tests pin `style: .sequoia` and assert
    /// fixed numbers wherever they run.
    func testTheSequoiaBackdrop_isVibrancyOnEveryOS() {
        let backdrop = CandidateBackdrop.make(style: .sequoia)

        XCTAssertTrue(backdrop.view is NSVisualEffectView)
        XCTAssertTrue(backdrop.view === backdrop.contentArea)
    }
}
