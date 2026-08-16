// Swift-side entry point to the shared Rust engine (`engine/swift-ffi`).

import Foundation
import RustTaigiSwift
import SwiftProtobuf

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
    private nonisolated(unsafe) static var isLoggerSinkInstalled = false

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

    // MARK: - Envelope round-trip

    private static let requestIDLock = NSLock()
    private nonisolated(unsafe) static var lastRequestID: UInt32 = 0

    private static func nextRequestID() -> UInt32 {
        requestIDLock.lock()
        defer { requestIDLock.unlock() }
        lastRequestID &+= 1
        return lastRequestID
    }

    /// Encodes one request, sends it, and returns the response payload when the
    /// engine reported success. `nil` means the round-trip failed rather than
    /// "the engine had nothing to say" — callers must keep the two apart,
    /// because a failed round-trip leaves the engine's state untouched and any
    /// snapshot synthesized here would contradict it.
    ///
    /// Shared by every slice (composing, lexicon, …): the envelope, the id
    /// sequence, the error checks, and the failure log are identical for all of
    /// them, and only the payload case differs.
    static func roundtrip(
        payload: Taigi_Engine_Request.OneOf_Payload,
        op: String,
        generation: UInt64 = 0,
        config: Taigi_Engine_AppConfig? = nil,
    ) -> Taigi_Engine_Response.OneOf_Payload? {
        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.generation = generation
        request.payload = payload
        if let config {
            request.configSnapshot = config
        }

        let requestBytes: [UInt8]
        do {
            requestBytes = try Array(request.serializedData())
        } catch {
            recordFailure(op: op, message: "encode failed: \(error)")
            return nil
        }

        logger.debug("[FFI->] op=\(op) id=\(request.id) generation=\(generation)")
        guard let response = try? Taigi_Engine_Response(
            serializedBytes: Data(processRequest(requestBytes)),
        ) else {
            recordFailure(op: op, message: "response decode failed")
            return nil
        }
        // The seam is synchronous and single-threaded per call, so a mismatched
        // id means the response belongs to some other request — reading its
        // payload would apply another operation's state to this one.
        guard response.id == request.id else {
            recordFailure(op: op, message: "response id \(response.id) does not match request \(request.id)")
            return nil
        }
        guard response.error == .ok else {
            recordFailure(op: op, message: "engine returned \(response.error)")
            return nil
        }
        guard let responsePayload = response.payload else {
            recordFailure(op: op, message: "response carried no payload")
            return nil
        }
        return responsePayload
    }

    private static let logger = DebugLogger(category: "RustEngineBridge")

    /// One place for every bridge failure, so a degraded engine is visible in
    /// `make log` instead of surfacing only as candidates that never appear.
    /// Deliberately no `assertionFailure`: the malformed-input tests drive these
    /// paths on purpose, and a debug-only trap would fail the suite that proves
    /// the failure handling works.
    static func recordFailure(op: String, message: String) {
        logger.error("[\(op)] \(message)")
    }

    // MARK: - AppConfig

    /// The engine holds no settings of its own; every request carries the
    /// snapshot it should be rendered under.
    /// Set on every request, not only the ones that read it. The composing
    /// engine ignores it; the next-word engine rejects the unset value outright
    /// (`engine/nextword/src/decide.rs:61`), and a field that is populated only
    /// on the paths that currently need it is one a later slice forgets to set.
    static func appConfig(_ settings: EngineSettings) -> Taigi_Engine_AppConfig {
        var config = Taigi_Engine_AppConfig()
        config.inputMode = settings.inputMode.rawValue
        config.ooDoubletapEnabled = settings.isDoubleTapOOEnabled
        config.nnDoubletapEnabled = settings.isDoubleTapNNEnabled
        config.platformID = .macos
        return config
    }

    /// `appConfig` plus the two word-boundary-spacing flags the engine consults
    /// while rendering a continuous composition's nailed prefix
    /// (`docs/engine/continuous-input-ranking.md` §10.2). Applied only at the
    /// entry points that render that prefix, matching iOS, so a mis-set flag
    /// cannot leak spacing changes into the ordinary composing path.
    static func continuousAppConfig(_ settings: EngineSettings) -> Taigi_Engine_AppConfig {
        var config = appConfig(settings)
        config.isTranslateSwapped = settings.isTranslateSwapped
        config.outputBothScripts = settings.isOutputBothScripts
        return config
    }
}
