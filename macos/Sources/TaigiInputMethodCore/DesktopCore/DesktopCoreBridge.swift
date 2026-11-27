// Swift-side entry point to the desktop shell seam (`macos/crates/taigi-macos-ffi`).

import Foundation
import RustTaigiSwift

/// Thin wrapper around `desktop_request_bytes`, the second bytes-in / bytes-out
/// function in the Rust archive, beside the engine's (`RustEngineBridge`).
/// It carries `taigi.desktop_shell` envelopes
/// (`macos/crates/taigi-macos-ffi/proto/desktop_shell.proto`; requests in
/// docs/architecture/macos-desktop-core-roadmap.md D3).
enum DesktopCoreBridge {
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
