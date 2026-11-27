// Proves the desktop shell seam links and round-trips from Swift.

import Foundation
import RustTaigiSwift
import SwiftProtobuf
@testable import TaigiInputMethodCore
import XCTest

/// Smoke tests for `macos/crates/taigi-macos-ffi` over the `RustTaigi` C module:
/// `desktop_request_bytes`, `install_logger_sink`, and `set_log_level`.
final class DesktopCoreFfiSmokeTests: XCTestCase {
    func testVersionRequest_answersTheLibraryVersion() throws {
        var request = Taigi_DesktopShell_DesktopRequest()
        request.version = Taigi_DesktopShell_VersionRequest()

        let responseBytes = try DesktopCoreBridge.send([UInt8](request.serializedData()))
        let response = try Taigi_DesktopShell_DesktopResponse(serializedBytes: Data(responseBytes))

        XCTAssertEqual(response.error, .ok)
        // The crate version (`macos/Cargo.toml`); its exact value is pinned by
        // the crate's own Rust test. Here: a Rust-built reply, not a default.
        XCTAssertNotNil(
            response.version.version.wholeMatch(of: /\d+\.\d+\.\d+/),
            "expected a semantic version, got \"\(response.version.version)\"",
        )
    }

    /// Negative control (rust-ffi-safety.md §6 T4): only the Rust side turns
    /// undecodable bytes into `FAIL_PARSE`, so this tells "Rust answered" from
    /// "Swift decoded an empty default response".
    func testMalformedBytes_returnFailParse() throws {
        let responseBytes = DesktopCoreBridge.send([0xFF, 0xFF, 0xFF, 0xFF])
        let response = try Taigi_DesktopShell_DesktopResponse(serializedBytes: Data(responseBytes))

        XCTAssertEqual(response.error, .failParse)
    }

    func testInstallLoggerSink_isIdempotent() {
        DesktopCoreBridge.installLoggerSink()
        DesktopCoreBridge.installLoggerSink()
    }

    /// Negative/round-trip control (rust-ffi-safety.md §6 T6): Rust `log::info!`
    /// emitted by `set_log_level(3)` crosses the swift-bridge `SwiftLoggerSink`.
    func testLoggerSink_receivesRustLogRecords() {
        final class LogCapture: @unchecked Sendable {
            private let lock = NSLock()
            private var entries: [(UInt8, String, String)] = []

            func append(_ entry: (UInt8, String, String)) {
                lock.lock()
                entries.append(entry)
                lock.unlock()
            }

            func snapshot() -> [(UInt8, String, String)] {
                lock.lock()
                defer { lock.unlock() }
                return entries
            }
        }

        let capture = LogCapture()
        install_logger_sink(SwiftLoggerSink { level, category, message in
            capture.append((level, category, message))
        })
        defer {
            set_log_level(0)
            install_logger_sink(SwiftLoggerSink { _, _, _ in })
        }

        set_log_level(3)
        let records = capture.snapshot()
        XCTAssertTrue(
            records.contains { $0.0 == 2 && $0.2.contains("Info") },
            "expected Rust info log record from set_log_level(3), got \(records)",
        )
    }
}
