// Phonetics slice of the engine bridge: tone stripping and the romanization
// conversions the settings pages render with.

import Foundation

extension RustEngineBridge {
    /// The syllable without its tone, and the tone digit that was on it.
    ///
    /// `nil` when the round-trip fails — the caller keeps its input rather than
    /// acting on a half-answer.
    static func stripTone(_ input: String) -> (bare: String, tone: String)? {
        var payload = Taigi_Engine_StripTone()
        payload.input = input

        let op = "stripTone"
        guard let response = phoneticsResponse(.stripTone(payload), op: op) else { return nil }
        guard case let .stripToneResult(result)? = response.result else {
            recordFailure(op: op, message: "response carried no strip-tone result")
            return nil
        }
        return (bare: result.bare, tone: result.tone)
    }

    /// Taigi-specific Unicode preprocessing before a lookup: the nasal marker
    /// and `o͘` are folded to the ASCII spellings the external dictionaries
    /// index by.
    static func nfdPreprocessForLookup(_ input: String) -> String? {
        var payload = Taigi_Engine_NfdPreprocessForLookup()
        payload.input = input
        return stringResult(.nfdPreprocessForLookup(payload), op: "nfdPreprocessForLookup")
    }

    /// The POJ spelling of a TL reading, for rendering results while the user
    /// is typing POJ.
    static func tlToPoj(_ input: String) -> String? {
        var payload = Taigi_Engine_TlToPoj()
        payload.input = input
        return stringResult(.tlToPoj(payload), op: "tlToPoj")
    }

    private static func stringResult(
        _ method: Taigi_Engine_PhoneticsRequest.OneOf_Method,
        op: String,
    ) -> String? {
        guard let response = phoneticsResponse(method, op: op) else { return nil }
        guard case let .stringResult(result)? = response.result else {
            recordFailure(op: op, message: "response carried no string result")
            return nil
        }
        return result.output
    }

    private static func phoneticsResponse(
        _ method: Taigi_Engine_PhoneticsRequest.OneOf_Method,
        op: String,
    ) -> Taigi_Engine_PhoneticsResponse? {
        var phonetics = Taigi_Engine_PhoneticsRequest()
        phonetics.method = method
        guard let payload = roundtrip(payload: .phonetics(phonetics), op: op) else { return nil }
        guard case let .phonetics(response) = payload else {
            recordFailure(op: op, message: "expected a phonetics payload, got \(payload)")
            return nil
        }
        return response
    }
}
