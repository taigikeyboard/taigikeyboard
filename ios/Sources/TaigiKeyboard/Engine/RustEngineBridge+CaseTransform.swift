import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge Case-Transform surface

/// Case-transform extension for `RustEngineBridge`. Single FFI hop per
/// per-char or per-word case operation. Mode and ⁿ becomes ᴺ in capitals (§53) are forwarded
/// via the envelope `AppConfig`; the double-tap folds are not (case-transform
/// is independent of POJ preprocessing).
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
    static func uppercaseToneChar(_ input: String, mode: InputMode, isNasalMarkerUppercaseEnabled: Bool) -> String {
        var payload = Taigi_Engine_UppercaseToneChar()
        payload.input = input
        return caseStringDispatch(method: .uppercaseToneChar(payload), op: "uppercaseToneChar", mode: mode, isNasalMarkerUppercaseEnabled: isNasalMarkerUppercaseEnabled, fallback: input)
    }

    /// Uppercase ALL characters in `input` using mode-aware tone tables.
    /// Used by Caps Lock paths. Replaces a separate Android API
    /// (`ToneUtilities.fullUppercaseToneLetter`) and the iOS pattern of
    /// passing a multi-char string into `uppercaseToneLetter`.
    static func fullUppercaseToneString(_ input: String, mode: InputMode, isNasalMarkerUppercaseEnabled: Bool) -> String {
        var payload = Taigi_Engine_FullUppercaseToneString()
        payload.input = input
        return caseStringDispatch(method: .fullUppercaseToneString(payload), op: "fullUppercaseToneString", mode: mode, isNasalMarkerUppercaseEnabled: isNasalMarkerUppercaseEnabled, fallback: input)
    }

    /// Lowercase a single char/grapheme using mode-aware tone tables.
    /// Replaces `ToneUtilities.lowercaseToneLetter`.
    static func lowercaseToneChar(_ input: String, mode: InputMode, isNasalMarkerUppercaseEnabled: Bool) -> String {
        var payload = Taigi_Engine_LowercaseToneChar()
        payload.input = input
        return caseStringDispatch(method: .lowercaseToneChar(payload), op: "lowercaseToneChar", mode: mode, isNasalMarkerUppercaseEnabled: isNasalMarkerUppercaseEnabled, fallback: input)
    }

    // MARK: - Per-string compound transforms

    /// Apply `letterCase` to `text`. Replaces
    /// `CaseTransformer.transformForInput`.
    static func transformInputCase(
        _ text: String,
        letterCase: CaseTransformLetterCase,
        mode: InputMode,
        isNasalMarkerUppercaseEnabled: Bool,
    ) -> String {
        var payload = Taigi_Engine_TransformInputCase()
        payload.text = text
        payload.letterCase = Taigi_Engine_LetterCase(rawValue: Int(letterCase.rawValue)) ?? .unspecified
        return caseStringDispatch(method: .transformInputCase(payload), op: "transformInputCase", mode: mode, isNasalMarkerUppercaseEnabled: isNasalMarkerUppercaseEnabled, fallback: text)
    }

    /// Per-suggestion case transformation. Output is post-processed via
    /// engine-side `adjustNasalMarkerCase` (no separate FFI hop needed).
    /// Replaces the body of `SuggestionCaseTransformer.transform` per word.
    static func transformSuggestionCase(
        original: String,
        composing: String,
        letterCase: CaseTransformLetterCase,
        mode: InputMode,
        isNasalMarkerUppercaseEnabled: Bool,
    ) -> String {
        var payload = Taigi_Engine_TransformSuggestion()
        payload.originalText = original
        payload.composingText = composing
        payload.letterCase = Taigi_Engine_LetterCase(rawValue: Int(letterCase.rawValue)) ?? .unspecified
        return caseStringDispatch(
            method: .transformSuggestion(payload),
            op: "transformSuggestionCase",
            mode: mode,
            isNasalMarkerUppercaseEnabled: isNasalMarkerUppercaseEnabled,
            fallback: original,
        )
    }

    // MARK: - Private dispatch helper

    /// Common dispatch path — every case op returns a single string result.
    /// Returns `fallback` on FFI failure (matches the safe-fallback contract
    /// of other bridge methods); the bridge logs the failure separately.
    private static func caseStringDispatch(
        method: Taigi_Engine_CaseRequest.OneOf_Method,
        op: String,
        mode: InputMode,
        isNasalMarkerUppercaseEnabled: Bool,
        fallback: String,
    ) -> String {
        guard let resp = caseDispatch(
            method: method,
            op: op,
            mode: mode,
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

    /// Case-transform dispatch — single FFI hop per word/char. Mode and
    /// ⁿ becomes ᴺ in capitals are carried via the envelope `AppConfig`; the double-tap
    /// folds are not, case-transform is independent of POJ preprocessing.
    private static func caseDispatch(
        method: Taigi_Engine_CaseRequest.OneOf_Method,
        op: String,
        mode: InputMode,
        isNasalMarkerUppercaseEnabled: Bool,
    ) -> Taigi_Engine_CaseResponse? {
        var caseReq = Taigi_Engine_CaseRequest()
        caseReq.method = method

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.payload = .caseTransform(caseReq)
        // Case-transform is independent of POJ doubletap preprocessing —
        // pass an explicit folds-off snapshot so the engine `AppConfig`
        // doesn't accidentally pick up unrelated state.
        request.configSnapshot = appConfig(
            mode: mode,
            toggles: PojMarkerOptions(
                isDoubleTapOOEnabled: false,
                isDoubleTapNNEnabled: false,
                isNasalMarkerUppercaseEnabled: isNasalMarkerUppercaseEnabled,
            ),
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
