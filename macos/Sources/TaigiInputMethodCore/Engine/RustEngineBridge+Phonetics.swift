// Phonetics slice of the engine bridge: the search keys a custom-dictionary
// entry is stored under, and the one a query is looked up by.

import Foundation

/// One way a custom-dictionary entry can be found.
///
/// `family` is the romanization system the key belongs to (`tl` / `poj` /
/// `tps`), `form` is how much of the reading it carries (`num` / `notone` /
/// `abbrev`), and `key` is the fused string both sides match on. The engine
/// owns all three vocabularies — the platform stores what it is given and asks
/// for the query key through the same op, which is the only thing that keeps
/// the write side and the read side in step
/// (`engine/protos/proto/phonetics.proto:109-118`).
struct CustomSearchKey: Equatable, Sendable {
    let family: String
    let form: String
    let key: String
}

extension RustEngineBridge {
    /// Every key a stored entry should be findable under, so a word added
    /// while typing TL is still found by someone typing POJ.
    ///
    /// An empty result means the roman produced no searchable key — the entry
    /// can still be stored, but nothing would ever match it, which is why the
    /// store treats it as a failure rather than writing a row the search can
    /// never reach.
    static func deriveCustomSearchKeys(roman: String) -> [CustomSearchKey]? {
        var payload = Taigi_Engine_DeriveCustomSearchKeys()
        payload.roman = roman

        var phonetics = Taigi_Engine_PhoneticsRequest()
        phonetics.method = .deriveCustomSearchKeys(payload)
        return customSearchKeys(phonetics, op: "deriveCustomSearchKeys")
    }

    /// The single key the user's current input should be looked up by.
    ///
    /// The family is decided by the engine, not by the caller's mode alone:
    /// input carrying TPS upgrades to the TPS family regardless of the setting
    /// (`phonetics.proto:121-129`). `nil` means there is nothing to look up —
    /// empty input, or residue that cannot form a key.
    static func deriveCustomQueryKey(input: String, mode: InputMode) -> CustomSearchKey? {
        var payload = Taigi_Engine_DeriveCustomQueryKey()
        payload.input = input
        payload.inputMode = mode.rawValue

        var phonetics = Taigi_Engine_PhoneticsRequest()
        phonetics.method = .deriveCustomQueryKey(payload)
        return customSearchKeys(phonetics, op: "deriveCustomQueryKey")?.first
    }

    /// The TL spelling of a POJ reading, for the identity keys a restore
    /// writes.
    ///
    /// A reading stored under its POJ spelling would be a row no lookup
    /// matches: identity is canonical TL. `nil` means the round-trip failed,
    /// and the caller keeps what it had rather than storing a guess.
    static func pojToTl(_ input: String) -> String? {
        var payload = Taigi_Engine_PojToTl()
        payload.input = input

        var phonetics = Taigi_Engine_PhoneticsRequest()
        phonetics.method = .pojToTl(payload)

        let op = "pojToTl"
        guard let responsePayload = roundtrip(payload: .phonetics(phonetics), op: op) else {
            return nil
        }
        guard case let .phonetics(response) = responsePayload else {
            recordFailure(op: op, message: "expected a phonetics payload, got \(responsePayload)")
            return nil
        }
        guard case let .stringResult(result)? = response.result else {
            recordFailure(op: op, message: "response carried no string result")
            return nil
        }
        return result.output
    }

    /// `nil` for a failed round-trip, an empty array for a successful one that
    /// produced no keys — the two mean different things to the store.
    private static func customSearchKeys(
        _ phonetics: Taigi_Engine_PhoneticsRequest,
        op: String,
    ) -> [CustomSearchKey]? {
        guard let payload = roundtrip(payload: .phonetics(phonetics), op: op) else { return nil }
        guard case let .phonetics(response) = payload else {
            recordFailure(op: op, message: "expected a phonetics payload, got \(payload)")
            return nil
        }
        guard case let .customSearchKeysResult(result)? = response.result else {
            recordFailure(op: op, message: "response carried no custom-search-keys result")
            return nil
        }
        return result.keys.map {
            CustomSearchKey(family: $0.family, form: $0.form, key: $0.key)
        }
    }
}
