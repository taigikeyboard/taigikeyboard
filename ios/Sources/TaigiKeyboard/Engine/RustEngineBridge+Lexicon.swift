import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge Lexicon surface

/// Lexicon read-path extension for `RustEngineBridge`. Follows the same
/// "proto roundtrip helpers + Swift-friendly synthesized value types
/// co-located in one file" pattern used by the NextWord, composing, and
/// phonetics extensions.
public extension RustEngineBridge {
    // MARK: - Synthesized value types

    /// Bridge-synthesized companion to the proto `TaigiWord` returned by
    /// lexicon search responses. Optional fields surface as Swift
    /// `Optional` per proto3 `optional` semantics; consumer in
    /// `LexiconService` converts to the platform-side `TaigiWord`.
    struct LexiconRow: Equatable {
        public let id: Int64
        public let roman: String
        public let hanzi: String?
        public let lengthScore: Int32?
        public let sourceBitmask: UInt32?
    }

    /// Bridge-synthesized companion to proto `LexiconAssocEntry`. Consumed
    /// by iOS `NextWordService` for bundled bigram lookups.
    struct LexiconAssocEntry: Equatable {
        public let previousWord: String
        public let candidateWord: String
        public let candidateTl: String
        public let count: UInt32
    }

    /// Engine install diagnostic counts, surfaced for dogfood-time
    /// inspection through `RustEngineBridge.diagnostics()` callers.
    struct LexiconInstallStats: Equatable {
        public let dictionaryRecordCount: UInt64
        public let prefixIndexEntryCount: UInt64
    }

    /// Lexicon engine `inputType` (mirrors proto `InputType`).
    enum LexiconInputType: Int32, Equatable {
        case unspecified = 0
        case romanNoTone = 1
        case romanWithTone = 2
        case hanzi = 3
    }

    /// Lexicon engine `inputMode` (mirrors proto `InputMode`).
    enum LexiconInputMode: Int32, Equatable {
        case unspecified = 0
        case tl = 1
        case poj = 2
        case tps = 3
    }

    // MARK: - Methods

    /// Install (or atomically reinstall) the lexicon engine state. Called
    /// once at keyboard extension launch with absolute Bundle paths;
    /// idempotent — calling again with the same paths is a no-op
    /// observation-wise (engine swaps state atomically on success).
    @discardableResult
    static func lexiconInstall(
        triePath: String,
        dictionaryBinPath: String,
        associationBinPath: String,
        dictionaryVersion: UInt32
    ) -> LexiconInstallStats? {
        var payload = Taigi_Engine_InstallRequest()
        payload.triePath = triePath
        payload.dictionaryBinPath = dictionaryBinPath
        payload.associationBinPath = associationBinPath
        payload.dictionaryVersion = dictionaryVersion
        guard let resp = lexiconDispatch(method: .install(payload), op: "lexiconInstall") else {
            return nil
        }
        guard case let .installResult(r)? = resp.result else {
            recordFailure(op: "lexiconInstall", message: "missing install result")
            return nil
        }
        return LexiconInstallStats(
            dictionaryRecordCount: r.dictionaryRecordCount,
            prefixIndexEntryCount: r.prefixIndexEntryCount
        )
    }

    /// IME autocomplete entry. Hanzi `inputType` returns `[]` per D-8
    /// hard guard pinned by `INVARIANT_LEX_HANZI_GUARD` (commit 12 adds
    /// the platform parity test).
    static func lexiconSearch(
        input: String,
        inputType: LexiconInputType,
        inputMode: LexiconInputMode,
        limit: UInt32,
        tpsOrMappedToER: Bool,
        enabledSourcesBitmask: UInt32
    ) -> [LexiconRow] {
        var payload = Taigi_Engine_SearchRequest()
        payload.input = input
        payload.inputType = Taigi_Engine_InputType(rawValue: Int(inputType.rawValue)) ?? .unspecified
        payload.inputMode = Taigi_Engine_InputMode(rawValue: Int(inputMode.rawValue)) ?? .unspecified
        payload.limit = limit
        payload.tpsOrMappedToEr = tpsOrMappedToER
        payload.enabledSourcesBitmask = enabledSourcesBitmask
        guard let resp = lexiconDispatch(method: .search(payload), op: "lexiconSearch") else {
            return []
        }
        guard case let .searchResult(r)? = resp.result else {
            recordFailure(op: "lexiconSearch", message: "missing search result")
            return []
        }
        return r.rows.map(taigiWordToRow)
    }

    /// Tab3 multi-source dictionary lookup.
    static func lexiconSearchWithSources(
        input: String,
        inputMode: LexiconInputMode,
        limit: UInt32,
        enabledSourcesBitmask: UInt32
    ) -> [LexiconRow] {
        var payload = Taigi_Engine_SearchWithSourcesRequest()
        payload.input = input
        payload.inputMode = Taigi_Engine_InputMode(rawValue: Int(inputMode.rawValue)) ?? .unspecified
        payload.limit = limit
        payload.enabledSourcesBitmask = enabledSourcesBitmask
        guard let resp = lexiconDispatch(method: .searchWithSources(payload), op: "lexiconSearchWithSources") else {
            return []
        }
        guard case let .searchWithSourcesResult(r)? = resp.result else {
            recordFailure(op: "lexiconSearchWithSources", message: "missing result")
            return []
        }
        return r.rows.map(taigiWordToRow)
    }

    /// Tab3 hanzi-prefix dictionary lookup.
    static func lexiconSearchByHanzi(
        query: String,
        inputMode: LexiconInputMode,
        limit: UInt32,
        enabledSourcesBitmask: UInt32
    ) -> [LexiconRow] {
        var payload = Taigi_Engine_SearchByHanziRequest()
        payload.query = query
        payload.inputMode = Taigi_Engine_InputMode(rawValue: Int(inputMode.rawValue)) ?? .unspecified
        payload.limit = limit
        payload.enabledSourcesBitmask = enabledSourcesBitmask
        guard let resp = lexiconDispatch(method: .searchByHanzi(payload), op: "lexiconSearchByHanzi") else {
            return []
        }
        guard case let .searchByHanziResult(r)? = resp.result else {
            recordFailure(op: "lexiconSearchByHanzi", message: "missing result")
            return []
        }
        return r.rows.map(taigiWordToRow)
    }

    /// Bundled-bigram lookup. Called by `NextWordService` after the
    /// commit 10 rewire (replaces direct `AssociationBinaryReader.lookup`).
    static func lexiconAssocLookup(
        previousWord: String,
        limit: UInt32,
        enabledSourcesBitmask: UInt32
    ) -> [LexiconAssocEntry] {
        var payload = Taigi_Engine_AssocLookupRequest()
        payload.previousWord = previousWord
        payload.limit = limit
        payload.enabledSourcesBitmask = enabledSourcesBitmask
        guard let resp = lexiconDispatch(method: .assocLookup(payload), op: "lexiconAssocLookup") else {
            return []
        }
        guard case let .assocLookupResult(r)? = resp.result else {
            recordFailure(op: "lexiconAssocLookup", message: "missing result")
            return []
        }
        return r.entries.map { entry in
            LexiconAssocEntry(
                previousWord: entry.previousWord,
                candidateWord: entry.candidateWord,
                candidateTl: entry.candidateTl,
                count: entry.count
            )
        }
    }

    // MARK: - Classification (v3.5.7)

    /// Classifier output — pairs the resolved `InputType` with the
    /// engine-built `searchKey` (TPS-converted on the engine side).
    internal struct ClassificationResult: Equatable {
        let inputType: InputType
        let searchKey: String
    }

    /// Classify `rawInput` into `(InputType, searchKey)`. Single FFI hop
    /// replaces the per-keystroke ladder of `hasToneMarks` /
    /// `containsTPS` / `tpsToTL` calls. See
    /// `INVARIANT_LEX_INPUT_CLASSIFICATION_PRECEDENCE`.
    internal static func classifyInput(_ raw: String) -> ClassificationResult {
        var payload = Taigi_Engine_ClassifyInputRequest()
        payload.raw = raw
        guard let resp = lexiconDispatch(method: .classifyInput(payload), op: "classifyInput") else {
            return ClassificationResult(inputType: .romanWithoutTone, searchKey: raw)
        }
        guard case let .classifyInputResult(r)? = resp.result else {
            recordFailure(op: "classifyInput", message: "missing classify_input result")
            return ClassificationResult(inputType: .romanWithoutTone, searchKey: raw)
        }
        return ClassificationResult(
            inputType: platformInputType(from: r.inputType),
            searchKey: r.searchKey
        )
    }

    /// Tab3 short-circuit predicate. True iff `text` contains any CJK
    /// codepoint (Unified + Extensions A-E). See
    /// `INVARIANT_LEX_INPUT_CLASSIFICATION_HANZI_RANGE`.
    static func isHanzi(_ text: String) -> Bool {
        var payload = Taigi_Engine_IsHanziRequest()
        payload.text = text
        guard let resp = lexiconDispatch(method: .isHanzi(payload), op: "isHanzi") else {
            return false
        }
        guard case let .isHanziResult(r)? = resp.result else {
            recordFailure(op: "isHanzi", message: "missing is_hanzi result")
            return false
        }
        return r.isHanzi
    }

    // MARK: - Private helpers

    private static func taigiWordToRow(_ proto: Taigi_Engine_TaigiWord) -> LexiconRow {
        LexiconRow(
            id: proto.id,
            roman: proto.roman,
            hanzi: proto.hasHanji ? proto.hanji : nil,
            lengthScore: proto.hasLengthScore ? proto.lengthScore : nil,
            sourceBitmask: proto.hasSourceBitmask ? proto.sourceBitmask : nil
        )
    }

    /// Map `Taigi_Engine_InputType` to the platform `InputType` enum.
    /// Unspecified / unrecognised values fall back to `.romanWithoutTone`
    /// (matches the safe-fallback contract of `lexiconDispatch` errors).
    /// Mirrors Android `LexiconBridge.platformInputType` —
    /// must drift together.
    private static func platformInputType(from proto: Taigi_Engine_InputType) -> InputType {
        switch proto {
        case .hanzi:           return .hanzi
        case .romanWithTone:   return .romanWithTone
        case .romanNoTone:     return .romanWithoutTone
        case .unspecified, .UNRECOGNIZED:
            return .romanWithoutTone
        }
    }
}
