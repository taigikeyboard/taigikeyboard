import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge Phonetics + TPS surface

/// Phonetics / TPS extension for `RustEngineBridge`. Holds
/// the typed phonetics methods (the D9.4 surface) plus the lazy
/// `toneVariations` cache and
/// the three private dispatch helpers (`dispatch` / `stringDispatch` /
/// `boolDispatch`) shared by every method here.
///
/// TPS is folded into this file because the TPS ops have zero
/// independent lifecycle from phonetics — they share the same dispatch
/// helpers, so colocation keeps those helpers `private` to one file
/// instead of widening to `internal`.
public extension RustEngineBridge {
    // MARK: Phonetics core (2 ops)

    static func tlToPoj(_ input: String) -> String {
        var payload = Taigi_Engine_TlToPoj()
        payload.input = input
        return stringDispatch(method: .tlToPoj(payload), input: input, op: "tlToPoj")
    }

    /// Lazy-init cache for `Method::GetToneVariations`. Swift `static let`
    /// initializer is dispatch_once-equivalent — thread-safe by construction.
    static let toneVariations: ToneVariationsCache = {
        let resp = dispatch(method: .getToneVariations(Taigi_Engine_GetToneVariations()),
                            op: "getToneVariations")
        guard case let .toneVariationsResult(r)? = resp?.result else {
            recordFailure(op: "getToneVariations", message: "missing result")
            return ToneVariationsCache(poj: [:], tl: [:])
        }
        return ToneVariationsCache(
            poj: r.pojVariations.mapValues { $0.variations },
            tl: r.tlVariations.mapValues { $0.variations },
        )
    }()

    // MARK: TPS (3 ops)

    static func tlDisplayToTPS(_ text: String, orMapsToER: Bool) -> String {
        var payload = Taigi_Engine_TlDisplayToTps()
        payload.text = text
        payload.orMapsToEr = orMapsToER
        return stringDispatch(
            method: .tlDisplayToTps(payload),
            input: text,
            op: "tlDisplayToTps",
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
        let resp = dispatch(method: .tpsInputAdjust(payload), op: "tpsInputAdjust")
        guard case let .tpsAdjustResult(r)? = resp?.result else {
            recordFailure(op: "tpsInputAdjust", message: "missing result")
            return (incoming, nil)
        }
        let replace = r.hasReplaceLast && r.replaceLast.present ? r.replaceLast.output : nil
        return (r.adjusted, replace)
    }

    // MARK: Platform text helpers

    /// Whether `text` is a single punctuation character that attaches to the
    /// preceding word under auto-space (`guá ` + `?` → `guá? `) — the
    /// engine's set, the one every platform asks. False when the engine
    /// fails: no swap, the space stays where it is.
    static func isAttachingPunctuation(_ text: String) -> Bool {
        var payload = Taigi_Engine_IsAttachingPunctuation()
        payload.text = text
        return boolDispatch(method: .isAttachingPunctuation(payload), op: "isAttachingPunctuation")
    }

    /// Whether `text` is the khinsiann dot, which takes the place of an auto
    /// space (`hōo ` + `·` → `hōo·`) — the engine's answer. False when the
    /// engine fails: the space stays where it is.
    static func isJoinerPunctuation(_ text: String) -> Bool {
        var payload = Taigi_Engine_IsJoinerPunctuation()
        payload.text = text
        return boolDispatch(method: .isJoinerPunctuation(payload), op: "isJoinerPunctuation")
    }

    /// The digit-tone spelling of a TL reading that the web dictionaries
    /// search by (`tāi-tsì` → `tai7-tsi3`) — the engine's fold, the one every
    /// platform asks. The reading as written when the engine fails.
    static func externalLookupDigitForm(_ reading: String) -> String {
        var payload = Taigi_Engine_ExternalLookupDigitForm()
        payload.reading = reading
        return stringDispatch(
            method: .externalLookupDigitForm(payload),
            input: reading,
            op: "externalLookupDigitForm",
        )
    }

    // MARK: Private dispatch (phonetics envelope)

    /// Phonetics envelope dispatch — encode → FFI roundtrip → decode the
    /// `PhoneticsResponse` payload. No phonetics op reads an `AppConfig`
    /// snapshot, so none is sent.
    private static func dispatch(
        method: Taigi_Engine_PhoneticsRequest.OneOf_Method,
        op: String,
    ) -> Taigi_Engine_PhoneticsResponse? {
        var phonetics = Taigi_Engine_PhoneticsRequest()
        phonetics.method = method

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.payload = .phonetics(phonetics)

        guard let response = send(request, op: op) else { return nil }
        guard response.error == .ok else {
            recordFailure(op: op, message: "engine returned \(response.error)")
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
    ) -> String {
        guard let resp = dispatch(method: method, op: op) else { return input }
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
        guard let resp = dispatch(method: method, op: op) else { return false }
        guard case let .boolResult(b)? = resp.result else {
            recordFailure(op: op, message: "expected BoolResult")
            return false
        }
        return b.value
    }
}

// MARK: - ToneVariationsCache

/// Init-bulk-pull cache for the callout tone variation tables. Loaded once
/// at first access via `RustEngineBridge.toneVariations`; both POJ + TL
/// maps live in a single payload to amortize FFI cost.
public struct ToneVariationsCache {
    public let poj: [String: [String]]
    public let tl: [String: [String]]
}
