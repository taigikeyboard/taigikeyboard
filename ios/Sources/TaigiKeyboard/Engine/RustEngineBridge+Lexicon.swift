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
        public let dev: Bool
        /// kautian subcollection enable state (10 accents + name appendix).
        /// iOS always populates this (the app ships the toggles), so the
        /// `kautian_subcoll` proto message is always present and the engine
        /// always runs the subcollection gate. Field order mirrors
        /// config.yaml `dialect_columns` / `KautianSubcollToggles` proto.
        public let kautianSubcoll: KautianSubcoll

        public struct KautianSubcoll: Equatable, Sendable {
            public let lukang: Bool
            public let sansia: Bool
            public let taipak: Bool
            public let gilan: Bool
            public let tainan: Bool
            public let kaohsiung: Bool
            public let kinmen: Bool
            public let makung: Bool
            public let sintik: Bool
            public let taichung: Bool
            public let nameAppendix: Bool
        }
    }

    /// Output of `lexiconDictionaryFilters` — ready-to-send bitmask plus
    /// the decoded enabled-source set for Tab3 retag. Replaces verbatim
    /// platform `EnabledDictionaries` bit math (deleted in v3.5.8 slice).
    ///
    /// `internal` (not `public`) because `enabledSources` references the
    /// internal `DictionarySource` enum.
    internal struct DictionaryFilters: Equatable {
        let dictionaryFilterBitmask: UInt32
        let enabledSources: Set<DictionarySource>

        /// What a failed resolve degrades to: every source on. `UInt32.max` is
        /// the engine's "filter disabled" sentinel on both the search path
        /// (`dictionary_reader.rs::Filter::from_enabled_bitmask`) and the
        /// composing path. Fail-open on purpose — a wider candidate list is
        /// recoverable, an empty one looks like a broken keyboard. Mirrors
        /// Android `RustEngineBridge.DictionaryFilters.ALL_SOURCES_ENABLED`.
        static let allSourcesEnabled = DictionaryFilters(
            dictionaryFilterBitmask: UInt32.max,
            enabledSources: Set(DictionarySource.allCases),
        )
    }

    // MARK: - Methods

    /// Install (or atomically reinstall) the lexicon engine state. Called
    /// once at keyboard extension launch with absolute Bundle paths;
    /// idempotent — calling again with the same paths is a no-op
    /// observation-wise (engine swaps state atomically on success).
    ///
    /// v3.5.8 Phase 6 added `syllableInventoryPath` (TL syllable inventory
    /// FST). Required parameter — empty string means "skip inventory" and
    /// `FetchAtPos` then graceful-degrades to empty candidates. Pass an
    /// absolute Bundle path to enable continuous-input candidate fetch.
    @discardableResult
    static func lexiconInstall(
        triePath: String,
        dictionaryBinPath: String,
        associationBinPath: String,
        dictionaryVersion: UInt32,
        syllableInventoryPath: String,
    ) -> LexiconInstallStats? {
        var payload = Taigi_Engine_InstallRequest()
        payload.triePath = triePath
        payload.dictionaryBinPath = dictionaryBinPath
        payload.associationBinPath = associationBinPath
        payload.dictionaryVersion = dictionaryVersion
        payload.syllableInventoryPath = syllableInventoryPath
        guard let resp = lexiconDispatch(method: .install(payload), op: "lexiconInstall") else {
            return nil
        }
        guard case let .installResult(r)? = resp.result else {
            recordFailure(op: "lexiconInstall", message: "missing install result")
            return nil
        }
        return LexiconInstallStats(
            dictionaryRecordCount: r.dictionaryRecordCount,
            prefixIndexEntryCount: r.prefixIndexEntryCount,
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
        enabledSourcesBitmask: UInt32,
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
        enabledSourcesBitmask: UInt32,
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
        enabledSourcesBitmask: UInt32,
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

    /// Resolve user's 12-toggle dictionary preferences into ready-to-send
    /// filter bitmasks + enabled-source set. Single FFI hop replaces the
    /// pre-v3.5.8 verbatim-mirrored `EnabledDictionaries` bit math.
    ///
    /// Call ONCE per query and pass the result down the search pipeline;
    /// re-resolving inside `fetchSystemResults` would split the snapshot.
    internal static func lexiconDictionaryFilters(toggles: DictionaryToggles) -> DictionaryFilters {
        var payload = Taigi_Engine_DictionaryFiltersRequest()
        payload.toggles = dictionaryTogglesProto(toggles)
        // The bit layout belongs to Rust (`compute_filters`); no platform
        // mirror. `lexiconDispatch` records its own failures.
        guard let resp = lexiconDispatch(method: .dictionaryFilters(payload), op: "lexiconDictionaryFilters") else {
            return .allSourcesEnabled
        }
        guard case let .dictionaryFiltersResult(r)? = resp.result else {
            recordFailure(op: "lexiconDictionaryFilters", message: "missing dictionary_filters result")
            return .allSourcesEnabled
        }
        return DictionaryFilters(
            dictionaryFilterBitmask: r.dictionaryFilterBitmask,
            enabledSources: Set(r.enabledSourceCodes.compactMap(platformDictionarySource(from:))),
        )
    }

    /// Proto form of the user's dictionary toggles, shared by
    /// `lexiconDictionaryFilters` and `nextwordPredictNext`.
    static func dictionaryTogglesProto(_ toggles: DictionaryToggles) -> Taigi_Engine_DictionaryToggles {
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
        togglesProto.dev = toggles.dev
        // Always set the subcollection message (iOS ships the toggles) so the
        // engine runs the gate; absence would signal legacy all-on (DD5).
        var subcollProto = Taigi_Engine_KautianSubcollToggles()
        subcollProto.accentLukang = toggles.kautianSubcoll.lukang
        subcollProto.accentSansia = toggles.kautianSubcoll.sansia
        subcollProto.accentTaipak = toggles.kautianSubcoll.taipak
        subcollProto.accentGilan = toggles.kautianSubcoll.gilan
        subcollProto.accentTainan = toggles.kautianSubcoll.tainan
        subcollProto.accentKaohsiung = toggles.kautianSubcoll.kaohsiung
        subcollProto.accentKinmen = toggles.kautianSubcoll.kinmen
        subcollProto.accentMakung = toggles.kautianSubcoll.makung
        subcollProto.accentSintik = toggles.kautianSubcoll.sintik
        subcollProto.accentTaichung = toggles.kautianSubcoll.taichung
        subcollProto.nameAppendix = toggles.kautianSubcoll.nameAppendix
        togglesProto.kautianSubcoll = subcollProto
        return togglesProto
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
            sourceBitmask: proto.hasSourceBitmask ? proto.sourceBitmask : nil,
        )
    }

    /// Map proto `DictionarySourceCode` to the platform `DictionarySource`
    /// enum. Explicit switch (no `rawValue` / `ordinal` reliance — Swift
    /// `DictionarySource` is `String`-backed; codes are wire-stable per
    /// `lexicon.proto::DictionarySourceCode`).
    /// Unspecified / unrecognised codes return `nil` and the caller drops them.
    private static func platformDictionarySource(from code: Taigi_Engine_DictionarySourceCode) -> DictionarySource? {
        switch code {
        case .dictSourceKautian: .kautian
        case .dictSourceTaigitv: .taigitv
        case .dictSourceItaigi: .itaigi
        case .dictSourceSitbut: .sitbut
        case .dictSourceTaihoa: .taihoa
        case .dictSourceTaijit: .taijit
        case .dictSourceKungge: .kungge
        case .dictSourceStti: .stti
        case .dictSourceKhpoo: .khpoo
        case .dictSourceKhiin: .khiin
        case .dictSourceLkk: .lkk
        case .dictSourceDev: .dev
        case .dictSourceCustom: .custom
        case .dictSourceUnspecified, .UNRECOGNIZED:
            nil
        }
    }

    // MARK: - Lexicon envelope dispatch

    /// Lexicon envelope dispatch — encode → FFI roundtrip → decode the
    /// `LexiconResponse` payload. Used by every lexicon method in this
    /// file (search / assoc / dictionary-filters / isHanzi). No `AppConfig` snapshot needed — lexicon ops
    /// read no live config.
    private static func lexiconDispatch(
        method: Taigi_Engine_LexiconRequest.OneOf_Method,
        op: String,
    ) -> Taigi_Engine_LexiconResponse? {
        var lexicon = Taigi_Engine_LexiconRequest()
        lexicon.method = method

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.payload = .lexicon(lexicon)

        guard let response = send(request, op: op) else { return nil }
        guard response.error == .ok else {
            recordFailure(op: op, message: "engine returned \(response.error)", code: Int32(response.error.rawValue))
            return nil
        }
        guard case let .lexicon(payload) = response.payload else {
            recordFailure(op: op, message: "missing lexicon payload")
            return nil
        }
        return payload
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
            lkk: settings.isLkkDictEnabled,
            dev: settings.isDevDictEnabled,
            kautianSubcoll: KautianSubcoll(
                lukang: settings.isKautianAccentLukangEnabled,
                sansia: settings.isKautianAccentSansiaEnabled,
                taipak: settings.isKautianAccentTaipakEnabled,
                gilan: settings.isKautianAccentGilanEnabled,
                tainan: settings.isKautianAccentTainanEnabled,
                kaohsiung: settings.isKautianAccentKaohsiungEnabled,
                kinmen: settings.isKautianAccentKinmenEnabled,
                makung: settings.isKautianAccentMakungEnabled,
                sintik: settings.isKautianAccentSintikEnabled,
                taichung: settings.isKautianAccentTaichungEnabled,
                nameAppendix: settings.isKautianNameAppendixEnabled,
            ),
        )
    }
}
