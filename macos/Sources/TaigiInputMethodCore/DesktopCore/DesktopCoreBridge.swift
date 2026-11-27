// Swift-side entry point to the desktop shell seam (`macos/crates/taigi-macos-ffi`).

import Foundation
import RustTaigiSwift

/// Thin wrapper around the macOS Rust archive (`macos/crates/taigi-macos-ffi`):
/// the desktop-shell protobuf seam (`desktop_request_bytes`,
/// `macos/crates/taigi-macos-ffi/proto/desktop_shell.proto`; requests in
/// docs/architecture/macos-desktop-core-roadmap.md D3) and the Rust logger sink.
enum DesktopCoreBridge {
    /// Severity of a record arriving FROM Rust. Mirrors the iOS constants in
    /// `ios/Sources/TaigiKeyboard/Engine/SwiftLoggerSink.swift`.
    private enum EngineLogRecordLevel {
        static let error: UInt8 = 0
        static let warning: UInt8 = 1
        static let info: UInt8 = 2
    }

    /// Threshold sent TO Rust via `set_log_level`. A DIFFERENT scale from
    /// `EngineLogRecordLevel` — it is shifted by one and reserves 0 for Off,
    /// which no record ever carries (`taigi-macos-ffi` `set_log_level`).
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

    /// Sends an encoded `taigi.desktop_shell.DesktopRequest` across the seam
    /// and returns the encoded `DesktopResponse`. The Rust side catches its
    /// own panics and always answers with a decodable response.
    static func send(_ requestBytes: [UInt8]) -> [UInt8] {
        requestBytes.withUnsafeBufferPointer { buffer in
            desktop_request_bytes(buffer).toArray()
        }
    }

    /// Encodes one request — with `settings`, the snapshot it runs under,
    /// when there is one — sends it, and returns the reply `expected` picks
    /// out of an OK response. `nil` — logged here — when the round-trip
    /// failed, the core refused, or it answered OK with another kind of
    /// reply or none.
    static func roundtrip<Reply>(
        _ request: Taigi_DesktopShell_DesktopRequest.OneOf_Request,
        settings: Taigi_DesktopShell_SettingsSnapshot? = nil,
        op: String,
        transport: ([UInt8]) -> [UInt8] = send,
        expected: (Taigi_DesktopShell_DesktopResponse.OneOf_Reply) -> Reply?,
    ) -> Reply? {
        guard case let .success(response) = response(to: request, settings: settings, op: op, transport: transport) else {
            return nil
        }
        guard response.error == .ok else {
            recordFailure(op: op, message: "core returned \(response.error)")
            return nil
        }
        guard let reply = response.reply.flatMap(expected) else {
            recordFailure(op: op, message: "expected reply missing, got \(String(describing: response.reply))")
            return nil
        }
        return reply
    }

    /// Why a request has no decoded response — logged where it happened.
    enum SeamFailure: Error {
        /// The request could not be encoded, so nothing was sent.
        case notSent
        /// The request was sent and may have run; its answer did not decode.
        case undecodable
    }

    /// Encodes one request, sends it and decodes the response, whatever its
    /// error — for a caller that acts on which error it is, or on whether a
    /// failed request was sent at all (the core back end, roadmap D4).
    static func response(
        to request: Taigi_DesktopShell_DesktopRequest.OneOf_Request,
        settings: Taigi_DesktopShell_SettingsSnapshot?,
        op: String,
        transport: ([UInt8]) -> [UInt8] = send,
    ) -> Result<Taigi_DesktopShell_DesktopResponse, SeamFailure> {
        let requestBytes: [UInt8]
        do {
            requestBytes = try envelope(request, settings: settings).serializedBytes()
        } catch {
            recordFailure(op: op, message: "encode failed: \(error)")
            return .failure(.notSent)
        }
        guard let response = try? Taigi_DesktopShell_DesktopResponse(serializedBytes: transport(requestBytes)) else {
            recordFailure(op: op, message: "response decode failed")
            return .failure(.undecodable)
        }
        return .success(response)
    }

    /// Logs a seam failure.
    private static func recordFailure(op: String, message: String) {
        logger.error("[\(op)] \(message)")
    }

    /// The request and its settings snapshot in one envelope: they cross
    /// the seam together and the core applies both or neither.
    static func envelope(
        _ request: Taigi_DesktopShell_DesktopRequest.OneOf_Request,
        settings: Taigi_DesktopShell_SettingsSnapshot?,
    ) -> Taigi_DesktopShell_DesktopRequest {
        var envelope = Taigi_DesktopShell_DesktopRequest()
        envelope.request = request
        if let settings {
            envelope.settings = settings
        }
        return envelope
    }

    private static let logger = DebugLogger(category: "DesktopCoreBridge")
}

// MARK: - Wire forms the requests share

extension Taigi_DesktopShell_KeyEvent {
    /// The `NSEvent` fields the core translates (`key_translation.rs`).
    init(_ key: KeyEventSnapshot) {
        assert(
            key.specialKeyRawValue != nil || !key.isNamedSpecialKey,
            "a named key with no raw value would reach the core as text",
        )
        self.init()
        if let keyCode = key.keyCode {
            self.keyCode = UInt32(keyCode)
        }
        if let characters = key.characters {
            self.characters = characters
        }
        if let characters = key.charactersIgnoringModifiers {
            charactersIgnoringModifiers = characters
        }
        modifierFlags = UInt64(key.modifiers.rawValue)
        if let specialKey = key.specialKeyRawValue {
            self.specialKey = specialKey
        }
    }
}

extension CandidateNavigation {
    /// The core's direction, or nil for one this side does not know.
    init?(_ direction: Taigi_DesktopShell_CandidateNavigation) {
        switch direction {
        case .left: self = .left
        case .right: self = .right
        case .up: self = .up
        case .down: self = .down
        case .pageUp: self = .pageUp
        case .pageDown: self = .pageDown
        case .nextCandidate: self = .nextCandidate
        case .previousCandidate: self = .previousCandidate
        case .unspecified, .UNRECOGNIZED: return nil
        }
    }
}
