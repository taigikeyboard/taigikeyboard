import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge Case-Transform surface

/// Case-transform extension for `RustEngineBridge`. Single FFI hop per
/// per-char or per-word case operation. Mode is forwarded via envelope
/// `AppConfig.input_mode`; no `ToneToggles` needed (case-transform is
/// independent of POJ doubletap preprocessing).
///
/// Suggestion skip rules (`additionalInfo["isComposingText"]` /
/// `additionalInfo["isNextWord"]`) stay on the platform side — only
/// transform-eligible items reach `transformSuggestionCase(...)`.
public extension RustEngineBridge {
    // MARK: - Synthesized enum

    /// Three-state shift / case indicator. Bridge-side mirror of the proto
    /// `LetterCase` enum + the pre-Rust iOS `LetterCase` enum (removed).
    enum CaseTransformLetterCase: Int32, Equatable {
        case lowercased = 1
        case uppercased = 2 // one-shot shift — first letter upper, rest lower
        case capsLocked = 3
    }

    // MARK: - Per-char helpers (single grapheme cluster)

    /// Uppercase a single char/grapheme using mode-aware tone tables. For
    /// multi-character inputs only the first letter is uppercased.
    /// Replaces `ToneUtilities.uppercaseToneLetter`.
    static func uppercaseToneChar(_ input: String, mode: InputMode) -> String {
        var payload = Taigi_Engine_UppercaseToneChar()
        payload.input = input
        return caseStringDispatch(method: .uppercaseToneChar(payload), op: "uppercaseToneChar", mode: mode, fallback: input)
    }

    /// Uppercase ALL characters in `input` using mode-aware tone tables.
    /// Used by Caps Lock paths. Replaces a separate Android API
    /// (`ToneUtilities.fullUppercaseToneLetter`) and the iOS pattern of
    /// passing a multi-char string into `uppercaseToneLetter`.
    static func fullUppercaseToneString(_ input: String, mode: InputMode) -> String {
        var payload = Taigi_Engine_FullUppercaseToneString()
        payload.input = input
        return caseStringDispatch(method: .fullUppercaseToneString(payload), op: "fullUppercaseToneString", mode: mode, fallback: input)
    }

    /// Lowercase a single char/grapheme using mode-aware tone tables.
    /// Replaces `ToneUtilities.lowercaseToneLetter`.
    static func lowercaseToneChar(_ input: String, mode: InputMode) -> String {
        var payload = Taigi_Engine_LowercaseToneChar()
        payload.input = input
        return caseStringDispatch(method: .lowercaseToneChar(payload), op: "lowercaseToneChar", mode: mode, fallback: input)
    }

    // MARK: - Per-string compound transforms

    /// Apply `letterCase` to `text`. Replaces
    /// `CaseTransformer.transformForInput`.
    static func transformInputCase(
        _ text: String,
        letterCase: CaseTransformLetterCase,
        mode: InputMode,
    ) -> String {
        var payload = Taigi_Engine_TransformInputCase()
        payload.text = text
        payload.letterCase = Taigi_Engine_LetterCase(rawValue: Int(letterCase.rawValue)) ?? .unspecified
        return caseStringDispatch(method: .transformInputCase(payload), op: "transformInputCase", mode: mode, fallback: text)
    }

    /// Capitalize candidate first letter when `autoCapEnabled` and `input`
    /// starts with an uppercase letter. Replaces
    /// `CaseTransformer.capitalizeCandidate`.
    static func capitalizeCandidate(
        _ text: String,
        basedOn input: String,
        autoCapEnabled: Bool,
        mode: InputMode,
    ) -> String {
        var payload = Taigi_Engine_CapitalizeCandidate()
        payload.text = text
        payload.input = input
        payload.autoCapEnabled = autoCapEnabled
        return caseStringDispatch(method: .capitalizeCandidate(payload), op: "capitalizeCandidate", mode: mode, fallback: text)
    }

    /// Per-suggestion case transformation. Output is post-processed via
    /// engine-side `adjustNasalMarkerCase` (no separate FFI hop needed).
    /// Replaces the body of `SuggestionCaseTransformer.transform` per word.
    static func transformSuggestionCase(
        original: String,
        composing: String,
        letterCase: CaseTransformLetterCase,
        mode: InputMode,
    ) -> String {
        var payload = Taigi_Engine_TransformSuggestion()
        payload.originalText = original
        payload.composingText = composing
        payload.letterCase = Taigi_Engine_LetterCase(rawValue: Int(letterCase.rawValue)) ?? .unspecified
        return caseStringDispatch(method: .transformSuggestion(payload), op: "transformSuggestionCase", mode: mode, fallback: original)
    }

    // MARK: - Private dispatch helper

    /// Common dispatch path — every case op returns a single string result.
    /// Returns `fallback` on FFI failure (matches the safe-fallback contract
    /// of other bridge methods); the bridge logs the failure separately.
    private static func caseStringDispatch(
        method: Taigi_Engine_CaseRequest.OneOf_Method,
        op: String,
        mode: InputMode,
        fallback: String,
    ) -> String {
        guard let resp = caseDispatch(method: method, op: op, mode: mode) else {
            return fallback
        }
        guard case let .stringResult(r)? = resp.result else {
            recordFailure(op: op, message: "missing string_result")
            return fallback
        }
        return r.output
    }

    /// Case-transform dispatch — single FFI hop per word/char. Mode is
    /// carried via envelope `AppConfig.input_mode` (engine reads it for
    /// tone-table lookup). No `ToneToggles` needed: case-transform is
    /// independent of POJ doubletap preprocessing.
    private static func caseDispatch(
        method: Taigi_Engine_CaseRequest.OneOf_Method,
        op: String,
        mode: InputMode,
    ) -> Taigi_Engine_CaseResponse? {
        var caseReq = Taigi_Engine_CaseRequest()
        caseReq.method = method

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.payload = .caseTransform(caseReq)
        // Case-transform is independent of POJ doubletap preprocessing —
        // pass an explicit "all-off" snapshot so the engine `AppConfig`
        // doesn't accidentally pick up unrelated state.
        request.configSnapshot = appConfig(
            mode: mode,
            toggles: ToneToggles(isDoubleTapOOEnabled: false, isDoubleTapNNEnabled: false),
        )

        guard let response = send(request, op: op) else { return nil }
        guard response.error == .ok else {
            recordFailure(op: op, message: "engine returned \(response.error)", code: Int32(response.error.rawValue))
            return nil
        }
        guard case let .caseTransform(payload) = response.payload else {
            recordFailure(op: op, message: "missing case payload")
            return nil
        }
        return payload
    }
}
