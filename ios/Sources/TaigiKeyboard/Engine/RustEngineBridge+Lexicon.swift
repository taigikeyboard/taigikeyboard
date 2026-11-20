import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge Lexicon surface

/// Lexicon read-path extension for `RustEngineBridge`. Follows the same
/// "proto roundtrip helpers + Swift-friendly synthesized value types
/// co-located in one file" pattern used by the NextWord, composing, and
/// phonetics extensions.
public extension RustEngineBridge {
    // MARK: - Synthesized value types

    /// Engine install diagnostic counts.
    struct LexiconInstallStats: Equatable {
        public let dictionaryRecordCount: UInt64
        public let prefixIndexEntryCount: UInt64
    }

    /// 12-toggle snapshot the user's dictionary preference state.
    /// Field order mirrors `engine/protos/proto/lexicon.proto::DictionarySourceToggles`.
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
        /// `kautian_subcollections` proto message is always present and the engine
        /// always runs the subcollection gate. Field order mirrors
        /// config.yaml `dialect_columns` / `KautianSubcollectionToggles` proto.
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
            public let altReading: Bool
        }
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

    /// The Manage Dictionaries search in one call (`DictionarySearch`): the
    /// engine picks the Hanji or romanization lookup, filters by `toggles`,
    /// renders the romanization and puts the custom entries first. `inputMode`
    /// is the settings spelling (`InputMode.rawValue`). `nil` = the engine
    /// failed (recorded). Blocks while the user data is still opening — call
    /// it off the main thread.
    static func dictionarySearch(
        query: String,
        inputMode: String,
        toggles: DictionaryToggles,
        customDictionaryEnabled: Bool,
        limit: Int,
    ) -> [Taigi_Engine_DictionarySearchRow]? {
        var payload = Taigi_Engine_DictionarySearchRequest()
        payload.query = query
        payload.inputMode = inputMode
        payload.toggles = dictionaryTogglesProto(toggles)
        payload.customDictionaryEnabled = customDictionaryEnabled
        payload.limit = UInt32(clamping: limit)
        guard let resp = lexiconDispatch(method: .dictionarySearch(payload), op: "dictionarySearch") else {
            return nil
        }
        guard case let .dictionarySearchResult(r)? = resp.result else {
            recordFailure(op: "dictionarySearch", message: "missing dictionary_search result")
            return nil
        }
        return r.rows
    }

    /// Proto form of the user's dictionary toggles, shared by
    /// `dictionarySearch`, `continuousAppConfig` and `nextwordPredictNext`.
    static func dictionaryTogglesProto(_ toggles: DictionaryToggles) -> Taigi_Engine_DictionarySourceToggles {
        var togglesProto = Taigi_Engine_DictionarySourceToggles()
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
        var subcollProto = Taigi_Engine_KautianSubcollectionToggles()
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
        subcollProto.altReading = toggles.kautianSubcoll.altReading
        togglesProto.kautianSubcollections = subcollProto
        return togglesProto
    }

    // MARK: - Helpers

    /// Map proto `DictionarySourceCode` to the platform `DictionarySource`
    /// enum. Explicit switch (no `rawValue` / `ordinal` reliance — Swift
    /// `DictionarySource` is `String`-backed; codes are wire-stable per
    /// `lexicon.proto::DictionarySourceCode`).
    /// Unspecified / unrecognised codes return `nil` and the caller drops them.
    internal static func platformDictionarySource(from code: Taigi_Engine_DictionarySourceCode) -> DictionarySource? {
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
    /// file (install / dictionarySearch). No `AppConfig` snapshot needed —
    /// lexicon ops read no live config.
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
            recordFailure(op: op, message: "engine returned \(response.error)")
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
                altReading: settings.isKautianAltReadingEnabled,
            ),
        )
    }
}
