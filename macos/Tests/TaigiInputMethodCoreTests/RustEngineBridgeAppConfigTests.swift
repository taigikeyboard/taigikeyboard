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

    /// The display mode rides the BASE config, which every request family
    /// starts from — the composing dispatcher collapses same-romanization rows
    /// on it and the next-word filter reads it too. `.unspecified` is the
    /// wire default and means side-by-side, so a dropped assignment would
    /// silently leave a romanization-only install with duplicate cells; both
    /// derived configs are checked so neither can lose it on the way.
    func testAppConfig_carriesTheCandidateDisplayMode_onEveryDerivedConfig() {
        let sideBySide = TestFixtures.settings(candidateDisplayMode: .sideBySide)
        let romanOnly = TestFixtures.settings(candidateDisplayMode: .romanOnly)
        let combined = TestFixtures.settings(candidateDisplayMode: .combined)

        XCTAssertEqual(RustEngineBridge.appConfig(sideBySide).candidateDisplayMode, .sideBySide)
        XCTAssertEqual(RustEngineBridge.appConfig(romanOnly).candidateDisplayMode, .romanOnly)
        XCTAssertEqual(RustEngineBridge.appConfig(combined).candidateDisplayMode, .combined)
        XCTAssertEqual(RustEngineBridge.continuousAppConfig(romanOnly).candidateDisplayMode, .romanOnly)
        XCTAssertEqual(RustEngineBridge.continuousAppConfig(combined).candidateDisplayMode, .combined)
        XCTAssertNotEqual(RustEngineBridge.appConfig(.defaults).candidateDisplayMode, .unspecified)
    }

    /// The combined display rides the wire beside the swap `SettingsStore`
    /// forces on for it: the engine has no reader for the mode itself, so the
    /// hanji-first behaviour it inherits — `continuous_word_space`, the
    /// nextword gates — comes entirely from this flag being `true`.
    func testContinuousAppConfig_underCombined_carriesTheForcedSwap() {
        let combined = TestFixtures.settings(swapped: true, candidateDisplayMode: .combined)
        let config = RustEngineBridge.continuousAppConfig(combined)

        XCTAssertEqual(config.candidateDisplayMode, .combined)
        XCTAssertTrue(config.isTranslateSwapped)
        XCTAssertFalse(config.outputBothScripts)
    }
}
