@testable import TaigiKeyboard
import XCTest

// The wire config every request carries: what `RustEngineBridge.appConfig` and the composing
// projection `continuousAppConfig` put on each field. Mirrors Android `EngineAppConfigTest`.
final class RustEngineBridgeAppConfigTests: XCTestCase {
    /// The TPS layout goes out as `"tps"` with the swap and Syllable Separator as stored — the engine
    /// applies the TPS fold itself (`AppConfig::renders_hanji_first` / `rendered_syllable_joiner`).
    func testContinuousAppConfig_tpsLayout_sendsTpsWithTheStoredFlags() {
        let settings = StubEngineSettings(
            inputMode: .tps,
            isHanjiFirst: false,
            syllableSeparator: .noSeparator,
            isTpsOrMappedToER: true,
        )

        let config = RustEngineBridge.continuousAppConfig(settings)

        XCTAssertEqual(config.inputMode, "tps")
        XCTAssertFalse(config.isHanjiFirst, "the swap goes out unfolded; the engine reads tps as Hanji-first")
        XCTAssertEqual(config.syllableSeparator, .none, "the separator goes out as stored; the engine exempts tps")
        XCTAssertTrue(config.tpsOrMapsToEr, "the or→er dialect switch rides the config")
    }

    func testContinuousAppConfig_romanizationModes_forwardEverySetting() {
        let cases: [(InputMode, String)] = [(.tl, "tl"), (.poj, "poj"), (.english, "english")]
        for (mode, wire) in cases {
            let settings = StubEngineSettings(
                inputMode: mode,
                isHanjiFirst: true,
                isOutputBothScripts: true,
                candidateDisplayMode: .combined,
                syllableSeparator: .space,
                pojMarkerOptions: PojMarkerOptions(
                    isDoubleTapOOEnabled: true,
                    isDoubleTapNNEnabled: true,
                    isNasalMarkerUppercaseEnabled: false,
                ),
            )

            let config = RustEngineBridge.continuousAppConfig(settings)

            XCTAssertEqual(config.inputMode, wire, "\(mode)")
            XCTAssertTrue(config.isHanjiFirst, "\(mode)")
            XCTAssertTrue(config.outputBothScripts, "\(mode)")
            XCTAssertEqual(config.candidateDisplayMode, .combined, "\(mode)")
            XCTAssertEqual(config.syllableSeparator, .space, "\(mode)")
            XCTAssertTrue(config.ooDoubletapEnabled, "\(mode)")
            XCTAssertTrue(config.nnDoubletapEnabled, "\(mode)")
            XCTAssertTrue(config.forceLowercaseNasalMarker, "\(mode): ⁿ becomes ᴺ OFF is inverted on the wire")
            XCTAssertFalse(config.tpsOrMapsToEr, "\(mode)")
        }
    }

    /// Nextword and case transform pass only what they read; the rest keeps the proto defaults.
    func testAppConfig_unpassedFields_keepTheProtoDefaults() {
        let config = RustEngineBridge.appConfig(mode: .tps, isHanjiFirst: true)

        XCTAssertEqual(config.inputMode, "tps")
        XCTAssertTrue(config.isHanjiFirst)
        XCTAssertFalse(config.ooDoubletapEnabled)
        XCTAssertFalse(config.nnDoubletapEnabled)
        XCTAssertFalse(config.forceLowercaseNasalMarker)
        XCTAssertFalse(config.outputBothScripts)
        XCTAssertEqual(config.candidateDisplayMode, .sideBySide)
        XCTAssertEqual(config.syllableSeparator, .hyphen)
        XCTAssertFalse(config.tpsOrMapsToEr)
    }
}
