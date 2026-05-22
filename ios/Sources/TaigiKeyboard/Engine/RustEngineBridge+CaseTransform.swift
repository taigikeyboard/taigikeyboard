// 中文: RustEngineBridge 的 case-transform 切片擴充。
// 中文: 每一個 per-char / per-word 大小寫轉換都是單次 FFI 呼叫,
// 中文: 模式從 envelope AppConfig.input_mode 傳遞,不需要 ToneToggles。

import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge Case-Transform surface

/// Case-transform extension for `RustEngineBridge`. Single FFI hop per
/// per-char or per-word case operation. Mode is forwarded via envelope
/// `AppConfig.input_mode`; no `ToneToggles` needed (case-transform is
/// independent of POJ doubletap preprocessing).
///
/// Replaces the algorithm body of:
/// - `Input/CaseTransformer.swift` (transformForInput, capitalizeCandidate)
/// - `Input/ToneUtilities.swift` (uppercase/lowercase tone letter, nasal adjust)
/// - `Autocomplete/Services/SuggestionCaseTransformer.swift` (per-word transform)
///
/// Suggestion skip rules (`additionalInfo["isComposingText"]` /
/// `additionalInfo["isNextWord"]`) stay on the platform side — only
/// transform-eligible items reach `transformSuggestionCase(...)`.
// 中文: case-transform 對外接口。建議跳過規則(isComposingText / isNextWord)
// 中文: 留在平台端判斷,只有真正要轉換的項目才送進這裡。
public extension RustEngineBridge {
    // MARK: - Synthesized enum

    /// Three-state shift / case indicator. Bridge-side mirror of the proto
    /// `LetterCase` enum + the iOS `LetterCase` engine enum (which is
    /// removed in commit 8 along with `Input/CaseTransformer.swift`).
    // 中文: 三態 shift / 大小寫指示。橋接層對應 proto LetterCase enum。
    enum CaseTransformLetterCase: Int32, Equatable {
        // 中文: 全小寫。
        case lowercased = 1
        // 中文: 一次性 shift — 首字母大寫,其餘小寫。
        case uppercased = 2 // one-shot shift — first letter upper, rest lower
        // 中文: Caps Lock 全部大寫。
        case capsLocked = 3
    }

    // MARK: - Per-char helpers (single grapheme cluster)

    /// Uppercase a single char/grapheme using mode-aware tone tables. For
    /// multi-character inputs only the first letter is uppercased.
    /// Replaces `ToneUtilities.uppercaseToneLetter`.
    // 中文: 單一 grapheme 的大寫轉換 — 走模式相關的調符表。多字元輸入只動首字母。
    static func uppercaseToneChar(_ input: String, mode: InputMode) -> String {
        var payload = Taigi_Engine_UppercaseToneChar()
        payload.input = input
        return caseStringDispatch(method: .uppercaseToneChar(payload), op: "uppercaseToneChar", mode: mode, fallback: input)
    }

    /// Uppercase ALL characters in `input` using mode-aware tone tables.
    /// Used by Caps Lock paths. Replaces a separate Android API
    /// (`ToneUtilities.fullUppercaseToneLetter`) and the iOS pattern of
    /// passing a multi-char string into `uppercaseToneLetter`.
    // 中文: 把整串輸入全部轉成大寫,Caps Lock 路徑使用。
    static func fullUppercaseToneString(_ input: String, mode: InputMode) -> String {
        var payload = Taigi_Engine_FullUppercaseToneString()
        payload.input = input
        return caseStringDispatch(method: .fullUppercaseToneString(payload), op: "fullUppercaseToneString", mode: mode, fallback: input)
    }

    /// Lowercase a single char/grapheme using mode-aware tone tables.
    /// Replaces `ToneUtilities.lowercaseToneLetter`.
    // 中文: 單一 grapheme 的小寫轉換,走模式相關的調符表。
    static func lowercaseToneChar(_ input: String, mode: InputMode) -> String {
        var payload = Taigi_Engine_LowercaseToneChar()
        payload.input = input
        return caseStringDispatch(method: .lowercaseToneChar(payload), op: "lowercaseToneChar", mode: mode, fallback: input)
    }

    // MARK: - Per-string compound transforms

    /// Apply `letterCase` to `text`. Replaces
    /// `CaseTransformer.transformForInput`.
    // 中文: 依 letterCase 對整段 text 套用大小寫轉換(輸入區用)。
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
    // 中文: 當 autoCapEnabled 且輸入首字母為大寫時,把候選詞首字母也轉大寫。
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
    // 中文: 對單一候選建議套用大小寫轉換。引擎端會自帶鼻音標記大小寫調整,
    // 中文: 不需要額外 FFI 呼叫。
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

        let bytes: [UInt8]
        do {
            bytes = try Array(request.serializedData())
        } catch {
            recordFailure(op: op, message: "encode failed: \(error)")
            return nil
        }

        let responseBytes = bytes.withUnsafeBufferPointer { buf in
            process_request_bytes(buf).toArray()
        }
        guard let response = try? Taigi_Engine_Response(
            serializedBytes: Data(responseBytes),
        ) else {
            recordFailure(op: op, message: "response decode failed")
            return nil
        }
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
