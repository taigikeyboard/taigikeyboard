// Phonetics slice of the engine bridge: the one check the macOS controller
// makes itself before a key reaches desktop-core — whether the key's
// character could swap with the armed auto space (`ComposingBackend.panel`).

import Foundation

extension RustEngineBridge {
    /// Whether `text` is, whole, a punctuation mark that attaches to the
    /// preceding word under auto-space (`guá ` + `?` → `guá? `) — the
    /// engine's set, the one every platform asks. False when the engine
    /// fails: no swap, the space stays where it is.
    static func isAttachingPunctuation(_ text: String) -> Bool {
        let op = "isAttachingPunctuation"
        var check = Taigi_Engine_IsAttachingPunctuation()
        check.text = text
        var request = Taigi_Engine_PhoneticsRequest()
        request.method = .isAttachingPunctuation(check)
        guard let payload = roundtrip(payload: .phonetics(request), op: op) else { return false }
        guard case let .phonetics(response) = payload, case let .boolResult(result)? = response.result else {
            recordFailure(op: op, message: "expected a phonetics bool result, got \(payload)")
            return false
        }
        return result.value
    }
}
