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

    /// 12-toggle snapshot the user's dictionary preference state.
    /// Field order mirrors `engine/protos/proto/lexicon.proto::DictionaryToggles`.
    /// Build via `init(from settings: EngineSettings)`; never construct
    /// piecemeal at search call sites — that splits the snapshot.
    struct DictionaryToggles: Equatable, Sendable {
        public let kautian: Bool
        public let taigitv: Bool
        public let itaigi: Bool
        public let sitbut: Bool
        public let taihoa: Bool
        public let taijit: Bool
        public let kungge: Bool
        public let stti: Bool
        public let khpoo: Bool
        public let variant: Bool
        public let khiin: Bool
        public let lkk: Bool
    }

    /// Output of `lexiconDictionaryFilters` — ready-to-send bitmasks plus
    /// the decoded enabled-source set for Tab3 retag. Replaces verbatim
    /// platform `EnabledDictionaries` bit math (deleted in v3.5.8 slice).
    ///
    /// `assocLookupBitmask` carries the `UInt32.max` sentinel when all 9
    /// association sources are on — preserves the documented
    /// `lexicon.proto:166-173` shortcut. Caller forwards directly to
    /// `lexiconAssocLookup(enabledSourcesBitmask:)`.
    struct DictionaryFilters: Equatable, Sendable {
        public let dictionaryFilterBitmask: UInt32
        public let assocLookupBitmask: UInt32
        public let enabledSources: Set<DictionarySource>
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

    /// Classify `rawInput` into `(InputType, searchKey)`. Single FFI hop —
    /// `lexicon::classify_input` keeps tone / TPS detection inside Rust,
    /// replacing the platform-side per-keystroke ladder that previously
    /// chained multiple phonetics ops per keypress. See
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

    /// Resolve user's 12-toggle dictionary preferences into ready-to-send
    /// filter bitmasks + enabled-source set. Single FFI hop replaces the
    /// pre-v3.5.8 verbatim-mirrored `EnabledDictionaries` bit math.
    ///
    /// Call ONCE per query and pass the result down the search pipeline;
    /// re-resolving inside `fetchSystemResults` would split the snapshot.
    static func lexiconDictionaryFilters(toggles: DictionaryToggles) -> DictionaryFilters {
        var togglesProto = Taigi_Engine_DictionaryToggles()
        togglesProto.kautian = toggles.kautian
        togglesProto.taigitv = toggles.taigitv
        togglesProto.itaigi = toggles.itaigi
        togglesProto.sitbut = toggles.sitbut
        togglesProto.taihoa = toggles.taihoa
        togglesProto.taijit = toggles.taijit
        togglesProto.kungge = toggles.kungge
        togglesProto.stti = toggles.stti
        togglesProto.khpoo = toggles.khpoo
        togglesProto.variant = toggles.variant
        togglesProto.khiin = toggles.khiin
        togglesProto.lkk = toggles.lkk
        var payload = Taigi_Engine_DictionaryFiltersRequest()
        payload.toggles = togglesProto
        // Binary skew fallback: when method 18 dispatch fails (e.g. Swift
        // updated but xcframework not rebuilt) but methods 12-17 still work,
        // the dev-only fallback would silently strip user-enabled dictionaries.
        // Mirror Rust `engine/lexicon/src/dictionary_filters.rs::compute_filters`
        // here so search/assoc call paths continue to honor user toggles.
        // Codex PR #210 r3182714295.
        guard let resp = lexiconDispatch(method: .dictionaryFilters(payload), op: "lexiconDictionaryFilters") else {
            return platformFallbackFilters(toggles: toggles)
        }
        guard case let .dictionaryFiltersResult(r)? = resp.result else {
            recordFailure(op: "lexiconDictionaryFilters", message: "missing dictionary_filters result")
            return platformFallbackFilters(toggles: toggles)
        }
        return DictionaryFilters(
            dictionaryFilterBitmask: r.dictionaryFilterBitmask,
            assocLookupBitmask: r.assocLookupBitmask,
            enabledSources: Set(r.enabledSourceCodes.compactMap(platformDictionarySource(from:)))
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

    /// Fallback only for platform/Rust binary skew where method 18 is absent.
    /// Rust `engine/lexicon/src/dictionary_filters.rs::compute_filters` is
    /// authoritative; keep this bit layout in sync with
    /// `engine/protos/proto/lexicon.proto`. Bit positions pinned by the
    /// 6 inline Rust golden tests.
    private static func platformFallbackFilters(toggles: DictionaryToggles) -> DictionaryFilters {
        var dictMask: UInt32 = 0
        if toggles.kautian { dictMask |= 1 << 0 }
        if toggles.taigitv { dictMask |= 1 << 1 }
        if toggles.itaigi  { dictMask |= 1 << 2 }
        if toggles.sitbut  { dictMask |= 1 << 3 }
        if toggles.taihoa  { dictMask |= 1 << 4 }
        if toggles.taijit  { dictMask |= 1 << 5 }
        if toggles.kungge  { dictMask |= 1 << 6 }
        if toggles.stti    { dictMask |= 1 << 7 }
        if toggles.khpoo   { dictMask |= 1 << 8 }
        if toggles.khiin   { dictMask |= 1 << 9 }
        dictMask |= 1 << 10 // dev always
        if toggles.lkk     { dictMask |= 1 << 11 }
        if toggles.variant { dictMask |= 1 << 12 }

        let allAssocOn = toggles.kautian && toggles.taigitv && toggles.itaigi
            && toggles.sitbut && toggles.taihoa && toggles.taijit
            && toggles.kungge && toggles.stti && toggles.khpoo
        let assocMask: UInt32 = allAssocOn ? UInt32.max : (dictMask & 0x1FF)

        var enabled: Set<DictionarySource> = [.dev, .custom]
        if toggles.kautian { enabled.insert(.kautian) }
        if toggles.taigitv { enabled.insert(.taigitv) }
        if toggles.itaigi  { enabled.insert(.itaigi) }
        if toggles.sitbut  { enabled.insert(.sitbut) }
        if toggles.taihoa  { enabled.insert(.taihoa) }
        if toggles.taijit  { enabled.insert(.taijit) }
        if toggles.kungge  { enabled.insert(.kungge) }
        if toggles.stti    { enabled.insert(.stti) }
        if toggles.khpoo   { enabled.insert(.khpoo) }
        if toggles.khiin   { enabled.insert(.khiin) }
        if toggles.lkk     { enabled.insert(.lkk) }
        return DictionaryFilters(
            dictionaryFilterBitmask: dictMask,
            assocLookupBitmask: assocMask,
            enabledSources: enabled
        )
    }

    /// Map proto `DictionarySourceCode` to the platform `DictionarySource`
    /// enum. Explicit switch (no `rawValue` / `ordinal` reliance — Swift
    /// `DictionarySource` is `String`-backed; codes are wire-stable per
    /// `lexicon.proto::DictionarySourceCode`).
    /// Unspecified / unrecognised codes return `nil` and the caller drops them.
    private static func platformDictionarySource(from code: Taigi_Engine_DictionarySourceCode) -> DictionarySource? {
        switch code {
        case .dictSourceKautian: return .kautian
        case .dictSourceTaigitv: return .taigitv
        case .dictSourceItaigi:  return .itaigi
        case .dictSourceSitbut:  return .sitbut
        case .dictSourceTaihoa:  return .taihoa
        case .dictSourceTaijit:  return .taijit
        case .dictSourceKungge:  return .kungge
        case .dictSourceStti:    return .stti
        case .dictSourceKhpoo:   return .khpoo
        case .dictSourceKhiin:   return .khiin
        case .dictSourceLkk:     return .lkk
        case .dictSourceDev:     return .dev
        case .dictSourceCustom:  return .custom
        case .dictSourceUnspecified, .UNRECOGNIZED:
            return nil
        }
    }
}

/// Build `DictionaryToggles` from an `EngineSettings` snapshot. Centralises
/// the boolean assembly so search call sites can't accidentally diverge.
extension RustEngineBridge.DictionaryToggles {
    init(from settings: EngineSettings) {
        self.init(
            kautian: settings.isMoeDictEnabled,
            taigitv: settings.isNewwordDictEnabled,
            itaigi: settings.isITaigiDictEnabled,
            sitbut: settings.isTaiwanPlantDictEnabled,
            taihoa: settings.isTaiHuaDictEnabled,
            taijit: settings.isTaiwanJapanDictEnabled,
            kungge: settings.isKunggeDictEnabled,
            stti: settings.isSttiDictEnabled,
            khpoo: settings.isKhpooDictEnabled,
            variant: settings.isVariantEnabled,
            khiin: settings.isKhiinEnabled,
            lkk: settings.isLkkDictEnabled
        )
    }
}
