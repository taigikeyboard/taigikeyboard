// The wire config every request carries: what this platform pins, not what the user chose.

@testable import TaigiInputMethodCore
import XCTest

final class RustEngineBridgeAppConfigTests: XCTestCase {
    /// POJ `oo`→`o͘` / `nn`→`ⁿ` folding is a user setting on iOS and Android,
    /// whose on-screen keyboards have dedicated keys for both graphemes. A
    /// hardware keyboard has none, so macOS pins the fold on. Protobuf `bool`
    /// defaults to `false`, which means a refactor that drops an assignment
    /// leaves that grapheme untypable in POJ rather than failing loudly — this
    /// case is what notices.
    func testAppConfig_pinsBothPojDoubletapFoldsOn() {
        let config = RustEngineBridge.appConfig(.defaults)

        XCTAssertTrue(config.ooDoubletapEnabled)
        XCTAssertTrue(config.nnDoubletapEnabled)
    }

    func testAppConfig_identifiesThePlatformAsMacOS() {
        XCTAssertEqual(RustEngineBridge.appConfig(.defaults).platformID, .macos)
    }
}
