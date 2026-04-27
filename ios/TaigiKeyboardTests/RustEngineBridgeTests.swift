import XCTest
@testable import TaigiKeyboard

/// D9.2/D9.4 platform-side acceptance tests for the Rust shared-core FFI.
///
/// **Requires** the dev xcframework built by
/// `engine/scripts/build-xcframework-dev.sh` (i.e. with the `panic-injector`
/// Cargo feature) so T1 actually panics inside the FFI catch boundary.
///
/// D9.4 expanded the bridge surface from 4 to 17 ops. This file keeps the
/// D9.2 lifecycle / FFI-safety tests (T1/T4/T5/T6/T7') intact and adds
/// smoke coverage for every new op. Branch-level fixture coverage lives in
/// `engine/phonetics/tests/d9_4_ops.rs`; call-site parity coverage lands
/// alongside the platform call-site swaps in commits 7–8.
final class RustEngineBridgeTests: XCTestCase {
    override class func setUp() {
        super.setUp()
        RustEngineBridge.install()
    }

    // MARK: - Idempotence

    func test_install_isIdempotent() {
        RustEngineBridge.install()
        RustEngineBridge.install()
    }

    // MARK: - Phonetics core (9 ops)

    func test_op_normalizeTone_TL() {
        let toggles = ToneToggles(isDoubleTapOOEnabled: false, isDoubleTapNNEnabled: false)
        XCTAssertEqual(
            RustEngineBridge.normalizeTone("gua2", mode: .tl, toggles: toggles),
            "guá"
        )
    }

    func test_op_stripTone_returnsBareAndToneTuple() {
        let result = RustEngineBridge.stripTone("guá")
        XCTAssertEqual(result.bare, "gua")
        XCTAssertEqual(result.tone, "2")
    }

    func test_op_pojToTl_canonical() {
        XCTAssertEqual(RustEngineBridge.pojToTl("góa"), "guá")
    }

    func test_op_tlToPoj_canonical() {
        XCTAssertEqual(RustEngineBridge.tlToPoj("guá"), "góa")
    }

    func test_op_normalizeToTL_passthrough() {
        XCTAssertEqual(RustEngineBridge.normalizeToTl("hoo"), "hoo")
    }

    func test_op_normalizeInput_extractsToneFromDiacritic() {
        XCTAssertEqual(RustEngineBridge.normalizeInput("hó"), "ho2")
    }

    func test_op_restoreTone_returnsBareForToneMarked() {
        XCTAssertEqual(RustEngineBridge.restoreTone("hó"), "ho")
    }

    func test_op_restoreTone_returnsNilForPlain() {
        XCTAssertNil(RustEngineBridge.restoreTone("ho"))
    }

    func test_op_hasToneMarks_trueForDiacritic() {
        XCTAssertTrue(RustEngineBridge.hasToneMarks("hó"))
    }

    func test_op_hasToneMarks_falseForPlain() {
        XCTAssertFalse(RustEngineBridge.hasToneMarks("ho"))
    }

    func test_op_toneVariations_lazyCache_returnsBothModes() {
        let cache = RustEngineBridge.toneVariations
        XCTAssertFalse(cache.poj.isEmpty, "POJ map should populate")
        XCTAssertFalse(cache.tl.isEmpty, "TL map should populate")
        XCTAssertNotNil(cache.tl["a"])
        XCTAssertNotNil(cache.poj["a"])
        XCTAssertNotNil(cache.tl["oo"])
        XCTAssertNotNil(cache.poj["o\u{0358}"])
    }

    // MARK: - Derivation (2 ops)

    func test_op_deriveNotone_stripsDiacriticsDigitsHyphensSpaces() {
        XCTAssertEqual(RustEngineBridge.deriveNotone("Gâu-tsá 2"), "gautsa")
    }

    func test_op_deriveAbbrev_returnsFirstCharPerSyllable() {
        XCTAssertEqual(RustEngineBridge.deriveAbbrev("gâu-tsá"), "gt")
    }

    // MARK: - TPS (6 ops)

    func test_op_containsTPS_trueForZhuyin() {
        XCTAssertTrue(RustEngineBridge.containsTPS("ㄉㄧㄠ"))
    }

    func test_op_containsTPS_falseForLatin() {
        XCTAssertFalse(RustEngineBridge.containsTPS("tiau"))
    }

    func test_op_tpsToTL_basic() {
        let out = RustEngineBridge.tpsToTL("ㄉㄧㄠˊ")
        XCTAssertTrue(out.contains("tiau"), "got: \(out)")
    }

    func test_op_tlNumericToTPS_basic() {
        let out = RustEngineBridge.tlNumericToTPS("tiau5", orMapsToER: false)
        XCTAssertFalse(out.isEmpty, "TL numeric → TPS should produce zhuyin")
    }

    func test_op_tlDisplayToTPS_basic() {
        let out = RustEngineBridge.tlDisplayToTPS("tiâu", orMapsToER: false)
        XCTAssertFalse(out.isEmpty, "TL display → TPS should produce zhuyin")
    }

    func test_op_isTPSToneMark_acuteIsToneMark() {
        XCTAssertTrue(RustEngineBridge.isTPSToneMark("\u{02ca}"))
    }

    func test_op_isTPSToneMark_letterIsNotToneMark() {
        XCTAssertFalse(RustEngineBridge.isTPSToneMark("a"))
    }

    func test_op_tpsInputAdjust_dualForm() {
        let result = RustEngineBridge.tpsInputAdjust(incoming: "ㄇ", rawInput: "ㄚ")
        XCTAssertEqual(result.adjusted, "ㆬ")
        XCTAssertNil(result.replaceLast)
    }

    func test_op_tpsInputAdjust_palatalization() {
        let result = RustEngineBridge.tpsInputAdjust(incoming: "ㄧ", rawInput: "ㄗ")
        XCTAssertEqual(result.adjusted, "ㄧ")
        XCTAssertEqual(result.replaceLast, "ㄐ")
    }

    func test_op_tpsInputAdjust_syllabicNasal() {
        let result = RustEngineBridge.tpsInputAdjust(incoming: "\u{02ca}", rawInput: "ㄇ")
        XCTAssertEqual(result.adjusted, "\u{02ca}")
        XCTAssertEqual(result.replaceLast, "ㆬ")
    }

    // MARK: - Diagnostics (Codex v2 §8 / v3 §7)

    func test_diagnostics_initialState_isEmpty() {
        RustEngineBridge.resetDiagnosticsForTesting()
        let snapshot = RustEngineBridge.diagnostics()
        XCTAssertEqual(snapshot.failureCount, 0)
        XCTAssertTrue(snapshot.recentErrors.isEmpty)
    }

    // MARK: - T1: panic at FFI

    func test_T1_panicForTest_returnsFailInternal_processSurvives() {
        let response = RustEngineBridge.panicForTestRaw()
        XCTAssertNotNil(response)
        XCTAssertEqual(response?.error, .failInternal)
    }

    // MARK: - T4: malformed protobuf

    func test_T4_malformedBytes_returnsFailParse() {
        let garbage: [UInt8] = [0xFF, 0xFE, 0xFD, 0x01, 0x02, 0x03]
        let response = RustEngineBridge.sendRawBytes(garbage)
        XCTAssertNotNil(response)
        XCTAssertEqual(response?.error, .failParse)
    }

    // MARK: - T5: oversized payload + boundary

    func test_T5_overCap_returnsFailInvariant() {
        let oversized = [UInt8](repeating: 0x00, count: 2 * 1024 * 1024 + 1)
        let response = RustEngineBridge.sendRawBytes(oversized)
        XCTAssertNotNil(response)
        XCTAssertEqual(response?.error, .failInvariant)
    }

    func test_T5_atCap_returnsParseOrInvariant() {
        let atCap = [UInt8](repeating: 0x00, count: 2 * 1024 * 1024)
        let response = RustEngineBridge.sendRawBytes(atCap)
        XCTAssertNotNil(response)
        XCTAssertNotEqual(response?.error, .failInvariant)
    }

    // MARK: - T6: logging round-trip

    func test_T6_loggerRoundTrip_warningReachesPlatformSink() {
        let sink = TestLogSink()
        LoggerFactory.install { _ in sink }
        defer { LoggerFactory.install { _ in NullLoggerBackend() } }
        _ = RustEngineBridge.sendRawBytes([0xFF, 0xFE, 0xFD])
        XCTAssertGreaterThanOrEqual(sink.recorded.count, 1)
    }

    // MARK: - T7': empty bytes

    func test_T7prime_emptyBytes_returnsFailParseOrInvariant() {
        let response = RustEngineBridge.sendRawBytes([])
        XCTAssertNotNil(response)
        XCTAssertTrue(
            response?.error == .failInvariant || response?.error == .failParse,
            "got \(String(describing: response?.error))"
        )
    }
}

// MARK: - Test logger sink

private final class TestLogSink: LoggerBackend, @unchecked Sendable {
    private let lock = NSLock()
    private var lines: [String] = []

    var recorded: [String] {
        lock.lock(); defer { lock.unlock() }
        return lines
    }

    func debug(_ message: @autoclosure () -> String) { record(message()) }
    func info(_ message: @autoclosure () -> String) { record(message()) }
    func warning(_ message: @autoclosure () -> String) { record(message()) }
    func error(_ message: @autoclosure () -> String) { record(message()) }

    private func record(_ message: String) {
        lock.lock(); defer { lock.unlock() }
        lines.append(message)
    }
}
