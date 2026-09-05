import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge Phonetics + Derivation + TPS surface

/// Phonetics / Derivation / TPS extension for `RustEngineBridge`. Holds
/// the 15 typed phonetics methods (Codex v2 §7 review per
/// `~/.claude/rules/round-workflow.md` § Codex review sandwich — D9.4
/// surface) plus the lazy `toneVariations` cache and
/// the three private dispatch helpers (`dispatch` / `stringDispatch` /
/// `boolDispatch`) shared by every method here.
///
/// TPS is folded into this file because the 5 TPS ops have zero
/// independent lifecycle from phonetics — they share the same dispatch
/// helpers, so colocation keeps those helpers `private` to one file
/// instead of widening to `internal`.
public extension RustEngineBridge {
    // MARK: Phonetics core (8 ops)

    /// `Method::NormalizeTone` — input + AppConfig.input_mode + ToneToggles →
    /// tone-marked string. Caller MUST supply `ToneToggles`; engine reads
    /// them per request (live-read invariant).
    static func normalizeTone(
        _ input: String,
        mode: InputMode,
        toggles: ToneToggles,
    ) -> String {
        var payload = Taigi_Engine_NormalizeTone()
        payload.input = input
        return stringDispatch(
            method: .normalizeTone(payload),
            input: input,
            op: "normalizeTone",
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    static func stripTone(_ input: String) -> (bare: String, tone: String) {
        var payload = Taigi_Engine_StripTone()
        payload.input = input
        let resp = dispatch(method: .stripTone(payload), op: "stripTone", config: nil)
        guard case let .stripToneResult(r)? = resp?.result else {
            recordFailure(op: "stripTone", message: "missing result")
            return (input, "")
        }
        return (r.bare, r.tone)
    }

    static func pojToTl(_ input: String) -> String {
        var payload = Taigi_Engine_PojToTl()
        payload.input = input
        return stringDispatch(method: .pojToTl(payload), input: input, op: "pojToTl", config: nil)
    }

    static func tlToPoj(_ input: String) -> String {
        var payload = Taigi_Engine_TlToPoj()
        payload.input = input
        return stringDispatch(method: .tlToPoj(payload), input: input, op: "tlToPoj", config: nil)
    }

    static func normalizeToTl(_ input: String) -> String {
        var payload = Taigi_Engine_NormalizeToTl()
        payload.input = input
        return stringDispatch(
            method: .normalizeToTl(payload),
            input: input,
            op: "normalizeToTl",
            config: nil,
        )
    }

    static func normalizeInput(_ input: String) -> String {
        var payload = Taigi_Engine_NormalizeInput()
        payload.input = input
        return stringDispatch(
            method: .normalizeInput(payload),
            input: input,
            op: "normalizeInput",
            config: nil,
        )
    }

    /// Replaces platform `TaigiUnicode.nfdPreprocessed(_:)`. Lookup-side
    /// NFD prep used by `ExternalLookupURLBuilder` before tone stripping.
    /// Distinct semantics from `normalizeInput` — this preserves tone
    /// diacritics; only nasal markers (ⁿ / ᴺ → "nn") and standalone
    /// `\u{0358}` → `o` are rewritten.
    static func nfdPreprocessForLookup(_ input: String) -> String {
        var payload = Taigi_Engine_NfdPreprocessForLookup()
        payload.input = input
        return stringDispatch(
            method: .nfdPreprocessForLookup(payload),
            input: input,
            op: "nfdPreprocessForLookup",
            config: nil,
        )
    }

    static func restoreTone(_ text: String) -> String? {
        var payload = Taigi_Engine_RestoreTone()
        payload.text = text
        let resp = dispatch(method: .restoreTone(payload), op: "restoreTone", config: nil)
        guard case let .optionalStringResult(r)? = resp?.result else {
            recordFailure(op: "restoreTone", message: "missing result")
            return nil
        }
        return r.present ? r.output : nil
    }

    /// Lazy-init cache for `Method::GetToneVariations`. Swift `static let`
    /// initializer is dispatch_once-equivalent — thread-safe by construction.
    static let toneVariations: ToneVariationsCache = {
        let resp = dispatch(method: .getToneVariations(Taigi_Engine_GetToneVariations()),
                            op: "getToneVariations",
                            config: nil)
        guard case let .toneVariationsResult(r)? = resp?.result else {
            recordFailure(op: "getToneVariations", message: "missing result")
            return ToneVariationsCache(poj: [:], tl: [:])
        }
        return ToneVariationsCache(
            poj: r.pojVariations.mapValues { $0.variations },
            tl: r.tlVariations.mapValues { $0.variations },
        )
    }()

    // MARK: Derivation (2 ops)

    static func deriveNotone(_ roman: String) -> String {
        var payload = Taigi_Engine_DeriveNotone()
        payload.roman = roman
        return stringDispatch(method: .deriveNotone(payload), input: roman, op: "deriveNotone", config: nil)
    }

    static func deriveAbbrev(_ roman: String) -> String {
        var payload = Taigi_Engine_DeriveAbbrev()
        payload.roman = roman
        return stringDispatch(method: .deriveAbbrev(payload), input: roman, op: "deriveAbbrev", config: nil)
    }

    /// `Method::DeriveCustomSearchKeys` — WRITE side (v3.6.1 R3). Full
    /// {tl, poj, tps} × {num, notone, abbrev} (+ TPS variant) bundle for a
    /// stored custom-dict roman. The platform materializes these into the
    /// `custom_search_key` side table so a query in any input mode finds the
    /// entry. Empty bundle on FFI failure / residue-only input.
    static func deriveCustomSearchKeys(_ roman: String) -> [CustomSearchKey] {
        var payload = Taigi_Engine_DeriveCustomSearchKeys()
        payload.roman = roman
        return customSearchKeys(method: .deriveCustomSearchKeys(payload), op: "deriveCustomSearchKeys")
    }

    /// `Method::DeriveCustomQueryKey` — READ side (v3.6.1 R3). Single
    /// family-native key for the current `input` + `mode`. Effective family is
    /// upgraded to TPS by the engine when the raw input carries Bopomofo, so
    /// the caller passes its settings mode verbatim. `nil` for residue-only /
    /// empty input or FFI failure.
    static func deriveCustomQueryKey(_ input: String, mode: InputMode) -> CustomSearchKey? {
        var payload = Taigi_Engine_DeriveCustomQueryKey()
        payload.input = input
        payload.inputMode = customSearchInputMode(mode)
        return customSearchKeys(method: .deriveCustomQueryKey(payload), op: "deriveCustomQueryKey").first
    }

    // MARK: TPS (5 ops)

    static func containsTPS(_ text: String) -> Bool {
        var payload = Taigi_Engine_ContainsTps()
        payload.text = text
        return boolDispatch(method: .containsTps(payload), op: "containsTps")
    }

    static func tlNumericToTPS(_ text: String, orMapsToER: Bool) -> String {
        var payload = Taigi_Engine_TlNumericToTps()
        payload.text = text
        payload.orMapsToEr = orMapsToER
        return stringDispatch(
            method: .tlNumericToTps(payload),
            input: text,
            op: "tlNumericToTps",
            config: nil,
        )
    }

    static func tlDisplayToTPS(_ text: String, orMapsToER: Bool) -> String {
        var payload = Taigi_Engine_TlDisplayToTps()
        payload.text = text
        payload.orMapsToEr = orMapsToER
        return stringDispatch(
            method: .tlDisplayToTps(payload),
            input: text,
            op: "tlDisplayToTps",
            config: nil,
        )
    }

    static func isTPSToneMark(_ char: Character) -> Bool {
        var payload = Taigi_Engine_IsTpsToneMark()
        payload.char = String(char)
        return boolDispatch(method: .isTpsToneMark(payload), op: "isTpsToneMark")
    }

    static func tpsInputAdjust(
        incoming: String,
        rawInput: String,
    ) -> (adjusted: String, replaceLast: String?) {
        var payload = Taigi_Engine_TpsInputAdjust()
        payload.incoming = incoming
        payload.rawInput = rawInput
        let resp = dispatch(method: .tpsInputAdjust(payload), op: "tpsInputAdjust", config: nil)
        guard case let .tpsAdjustResult(r)? = resp?.result else {
            recordFailure(op: "tpsInputAdjust", message: "missing result")
            return (incoming, nil)
        }
        let replace = r.hasReplaceLast && r.replaceLast.present ? r.replaceLast.output : nil
        return (r.adjusted, replace)
    }

    // MARK: Private dispatch (phonetics envelope)

    /// Phonetics envelope dispatch — encode → FFI roundtrip → decode the
    /// `PhoneticsResponse` payload. `config` is optional because most
    /// phonetics ops don't need an `AppConfig` snapshot; `NormalizeTone`
    /// is the live-read holdout that supplies one.
    private static func dispatch(
        method: Taigi_Engine_PhoneticsRequest.OneOf_Method,
        op: String,
        config: Taigi_Engine_AppConfig?,
    ) -> Taigi_Engine_PhoneticsResponse? {
        var phonetics = Taigi_Engine_PhoneticsRequest()
        phonetics.method = method

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.payload = .phonetics(phonetics)
        if let config { request.configSnapshot = config }

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
        guard case let .phonetics(payload) = response.payload else {
            recordFailure(op: op, message: "missing phonetics payload")
            return nil
        }
        return payload
    }

    private static func stringDispatch(
        method: Taigi_Engine_PhoneticsRequest.OneOf_Method,
        input: String,
        op: String,
        config: Taigi_Engine_AppConfig?,
    ) -> String {
        guard let resp = dispatch(method: method, op: op, config: config) else { return input }
        guard case let .stringResult(s)? = resp.result else {
            recordFailure(op: op, message: "expected StringResult")
            return input
        }
        return s.output
    }

    private static func boolDispatch(
        method: Taigi_Engine_PhoneticsRequest.OneOf_Method,
        op: String,
    ) -> Bool {
        guard let resp = dispatch(method: method, op: op, config: nil) else { return false }
        guard case let .boolResult(b)? = resp.result else {
            recordFailure(op: op, message: "expected BoolResult")
            return false
        }
        return b.value
    }

    /// Shared decode for the two custom-dict search-key ops — both return a
    /// `CustomSearchKeysResult` (the write op a full bundle, the query op 0/1).
    private static func customSearchKeys(
        method: Taigi_Engine_PhoneticsRequest.OneOf_Method,
        op: String,
    ) -> [CustomSearchKey] {
        guard let resp = dispatch(method: method, op: op, config: nil) else { return [] }
        guard case let .customSearchKeysResult(r)? = resp.result else {
            recordFailure(op: op, message: "expected CustomSearchKeysResult")
            return []
        }
        return r.keys.map { CustomSearchKey(family: $0.family, form: $0.form, key: $0.key) }
    }

    /// Map the platform `InputMode` to the engine `input_mode` string. TPS maps
    /// to "tl" because TPS is a layout, not an engine mode — the engine upgrades
    /// to the TPS family via `contains_tps` on the raw input (mirrors
    /// `RustEngineBridge.appConfig`).
    private static func customSearchInputMode(_ mode: InputMode) -> String {
        switch mode {
        case .poj: "poj"
        case .english: "english"
        case .tl, .tps: "tl"
        }
    }
}

// MARK: - CustomSearchKey

/// One custom-dictionary cross-mode search key (v3.6.1 R3). Mirrors the proto
/// `CustomSearchKey`: `family` ∈ {tl, poj, tps}, `form` ∈ {num, notone,
/// abbrev}, `key` the fused family-native search string. Written to the
/// `custom_search_key` side table; the query op returns one to match against
/// it. Pure value type (no KeyboardKit / UIKit).
public struct CustomSearchKey: Equatable, Sendable {
    public let family: String
    public let form: String
    public let key: String
}

// MARK: - ToneVariationsCache

/// Init-bulk-pull cache for the callout tone variation tables. Loaded once
/// at first access via `RustEngineBridge.toneVariations`; both POJ + TL
/// maps live in a single payload to amortize FFI cost.
public struct ToneVariationsCache {
    public let poj: [String: [String]]
    public let tl: [String: [String]]
}
