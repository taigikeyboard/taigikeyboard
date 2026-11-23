import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge Case-Transform surface

/// Case-transform extension for `RustEngineBridge`. Single FFI hop per
/// typed key. Casing is the same in every input mode; the envelope
/// `AppConfig` carries only Nasal mark in POJ capitals (§53), the one field
/// the engine's case handler reads.
public extension RustEngineBridge {
    // MARK: - Synthesized enum

    /// Three-state shift / case indicator. Bridge-side mirror of the proto
    /// `LetterCase` enum + the pre-Rust iOS `LetterCase` enum (removed).
    enum CaseTransformLetterCase: Int32, Equatable {
        case lowercased = 1
        case uppercased = 2 // one-shot shift — first letter upper, rest lower
        case capsLocked = 3
    }

    // MARK: - Per-string compound transforms

    /// Apply `letterCase` to `text`. Replaces
    /// `CaseTransformer.transformForInput`.
    static func transformInputCase(
        _ text: String,
        letterCase: CaseTransformLetterCase,
        isNasalMarkerUppercaseEnabled: Bool,
    ) -> String {
        var payload = Taigi_Engine_TransformInputCase()
        payload.text = text
        payload.letterCase = Taigi_Engine_LetterCase(rawValue: Int(letterCase.rawValue)) ?? .unspecified
        return caseStringDispatch(method: .transformInputCase(payload), op: "transformInputCase", isNasalMarkerUppercaseEnabled: isNasalMarkerUppercaseEnabled, fallback: text)
    }

    // MARK: - Private dispatch helper

    /// Common dispatch path — every case op returns a single string result.
    /// Returns `fallback` on FFI failure (matches the safe-fallback contract
    /// of other bridge methods); the bridge logs the failure separately.
    private static func caseStringDispatch(
        method: Taigi_Engine_CaseRequest.OneOf_Method,
        op: String,
        isNasalMarkerUppercaseEnabled: Bool,
        fallback: String,
    ) -> String {
        guard let resp = caseDispatch(
            method: method,
            op: op,
            isNasalMarkerUppercaseEnabled: isNasalMarkerUppercaseEnabled,
        ) else {
            return fallback
        }
        guard case let .stringResult(r)? = resp.result else {
            recordFailure(op: op, message: "missing string_result")
            return fallback
        }
        return r.output
    }

    /// Case-transform dispatch — single FFI hop per word/char.
    private static func caseDispatch(
        method: Taigi_Engine_CaseRequest.OneOf_Method,
        op: String,
        isNasalMarkerUppercaseEnabled: Bool,
    ) -> Taigi_Engine_CaseResponse? {
        var caseReq = Taigi_Engine_CaseRequest()
        caseReq.method = method

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.payload = .caseTransform(caseReq)
        request.configSnapshot = caseAppConfig(isNasalMarkerUppercaseEnabled: isNasalMarkerUppercaseEnabled)

        guard let response = send(request, op: op) else { return nil }
        guard response.error == .ok else {
            recordFailure(op: op, message: "engine returned \(response.error)")
            return nil
        }
        guard case let .caseTransform(payload) = response.payload else {
            recordFailure(op: op, message: "missing case payload")
            return nil
        }
        return payload
    }

    /// Only the field `phonetics::case_requests::handle` reads — the input
    /// mode does not change casing. Inverted on the wire, as in `appConfig`
    /// (proto default = the marker follows the case, §53).
    internal static func caseAppConfig(isNasalMarkerUppercaseEnabled: Bool) -> Taigi_Engine_AppConfig {
        var config = Taigi_Engine_AppConfig()
        config.forceLowercaseNasalMarker = !isNasalMarkerUppercaseEnabled
        return config
    }
}
