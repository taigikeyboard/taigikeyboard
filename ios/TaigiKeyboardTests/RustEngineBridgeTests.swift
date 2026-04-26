import XCTest
@testable import TaigiKeyboard

/// D9.2 platform-side acceptance tests for the Rust shared-core FFI.
///
/// **Requires** the dev xcframework built by
/// `engine/scripts/build-xcframework-dev.sh` (i.e. with the `panic-injector`
/// Cargo feature) so T1 actually panics inside the FFI catch boundary. Run
/// the release script before shipping.
///
/// Test ID map vs `docs/engine/ffi-safety.md` §7:
///   T1   panic at FFI                      → `test_T1_*`
///   T2   Drop / cleanup                    → DEFERRED to D9.3 (no handle in D9.2)
///   T3   Thread safety                     → DEFERRED to D9.3
///   T4   Malformed protobuf                → `test_T4_*`
///   T5   Oversized payload                 → `test_T5_*` + boundary cases
///   T6   Logging round-trip                → `test_T6_*`
///   T7'  Empty bytes (reframed from null handle) → `test_T7prime_*`
///   T8/T9 handle lifecycle                 → DEFERRED to D9.3
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

    // MARK: - Op tests (canonical fixtures + 1 keyboard real-world)

    func test_op_tlToPoj_keyboardRealWorld() {
        XCTAssertEqual(RustEngineBridge.tlToPoj("guá"), "góa")
    }

    func test_op_pojToTl_canonical() {
        XCTAssertEqual(RustEngineBridge.pojToTl("góa"), "guá")
    }

    func test_op_normalizeTone_canonical() {
        XCTAssertEqual(RustEngineBridge.normalizeTone("gua2"), "guá")
    }

    func test_op_stripTone_canonical() {
        XCTAssertEqual(RustEngineBridge.stripTone("guá"), "gua2")
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
        // At cap (== MAX_REQUEST_BYTES). Bytes are not a valid Request, so the
        // FFI accepts the size and the inner decoder returns FAIL_PARSE — the
        // important assertion is "did not crash and did not return
        // FailInvariant for an at-cap payload".
        let atCap = [UInt8](repeating: 0x00, count: 2 * 1024 * 1024)
        let response = RustEngineBridge.sendRawBytes(atCap)
        XCTAssertNotNil(response)
        XCTAssertNotEqual(response?.error, .failInvariant)
    }

    // MARK: - T6: logging round-trip

    func test_T6_loggerRoundTrip_warningReachesPlatformSink() {
        let sink = TestLogSink()
        LoggerFactory.install { _ in sink }
        defer {
            LoggerFactory.install { _ in NullLoggerBackend() }
        }
        // Drive a warning through the Rust core. Malformed bytes cause
        // `phonetics::api::run_request` to log a warn-level decode-failure.
        _ = RustEngineBridge.sendRawBytes([0xFF, 0xFE, 0xFD])
        XCTAssertGreaterThanOrEqual(sink.recorded.count, 1, "expected at least one log line")
    }

    // MARK: - T7': empty bytes (reframed from null handle since D9.2 has no handle)

    func test_T7prime_emptyBytes_returnsFailParseOrInvariant() {
        let response = RustEngineBridge.sendRawBytes([])
        XCTAssertNotNil(response)
        // Empty is wire-valid (zero-byte Request decodes to defaults), but
        // missing payload triggers FAIL_INVARIANT in `run_request`.
        XCTAssertTrue(
            response?.error == .failInvariant || response?.error == .failParse,
            "expected FAIL_INVARIANT or FAIL_PARSE, got \(String(describing: response?.error))"
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
