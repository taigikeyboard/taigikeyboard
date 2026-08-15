// Swift-side entry point to the shared Rust engine (`engine/swift-ffi`).

import Foundation
import RustTaigiSwift

/// Thin wrapper around the single bytes-in / bytes-out FFI function the Rust
/// shared core exports. Named after the iOS `RustEngineBridge` because it plays
/// the same role; the typed per-slice surfaces (composing, lexicon, …) arrive
/// with the slices that need them.
enum RustEngineBridge {
    /// Severity of a record arriving FROM Rust. Mirrors the iOS constants in
    /// `ios/Sources/TaigiKeyboard/Engine/SwiftLoggerSink.swift`.
    private enum EngineLogRecordLevel {
        static let error: UInt8 = 0
        static let warning: UInt8 = 1
        static let info: UInt8 = 2
    }

    /// Threshold sent TO Rust via `set_log_level`. A DIFFERENT scale from
    /// `EngineLogRecordLevel` — it is shifted by one and reserves 0 for Off,
    /// which no record ever carries (`engine/swift-ffi/src/lib.rs` `set_log_level`).
    /// Naming both is what keeps a bare `4` from meaning Debug in one call and
    /// Trace in the other.
    private enum EngineLogFilter {
        static let off: UInt8 = 0
        static let debug: UInt8 = 4
    }

    private static let installLock = NSLock()
    private static nonisolated(unsafe) var isLoggerSinkInstalled = false

    /// Idempotent. Routes Rust `log::*` records into `DebugLogger`, so they
    /// obey the same release-builds-log-nothing rule as native call sites.
    /// Called once at bootstrap; the underlying Rust `log::set_logger` is
    /// one-shot.
    static func installLoggerSink() {
        installLock.lock()
        defer { installLock.unlock() }
        guard !isLoggerSinkInstalled else { return }
        install_logger_sink(SwiftLoggerSink(handleRecord: forwardEngineRecord))
        // Release silences Rust at the source. Leaving the Rust default of Warn
        // in place would still marshal every warning across the FFI seam and
        // build two Swift strings, only for `DebugLogger` to discard them.
        set_log_level(isDebugBuild ? EngineLogFilter.debug : EngineLogFilter.off)
        isLoggerSinkInstalled = true
    }

    private static var isDebugBuild: Bool {
        #if DEBUG
            true
        #else
            false
        #endif
    }

    private static func forwardEngineRecord(level: UInt8, category: String, message: String) {
        let logger = DebugLogger(category: category)
        switch level {
        case EngineLogRecordLevel.error: logger.error(message)
        case EngineLogRecordLevel.warning: logger.warning(message)
        case EngineLogRecordLevel.info: logger.info(message)
        default: logger.debug(message)
        }
    }

    /// Sends an encoded `taigi.engine.Request` across the FFI seam and returns
    /// the encoded `taigi.engine.Response`. The Rust side catches its own
    /// panics and always answers with a decodable `Response`.
    static func processRequest(_ requestBytes: [UInt8]) -> [UInt8] {
        requestBytes.withUnsafeBufferPointer { buffer in
            process_request_bytes(buffer).toArray()
        }
    }
}
