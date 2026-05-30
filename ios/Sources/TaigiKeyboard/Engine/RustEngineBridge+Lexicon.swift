// 中文: RustEngineBridge 的 Lexicon 讀取路徑切片擴充。
// 中文: 含 install / search / assoc lookup / classify-input / dictionary-filters / isHanzi。
// 中文: 與 NextWord 切片同一套 "proto roundtrip + synthesized value type" 編排。

import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge Lexicon surface

/// Lexicon read-path extension for `RustEngineBridge`. Follows the same
/// "proto roundtrip helpers + Swift-friendly synthesized value types
/// co-located in one file" pattern used by the NextWord, composing, and
/// phonetics extensions.
// 中文: Lexicon 讀取路徑的 bridge 擴充入口 — 所有對 lexicon engine 的呼叫都從這個 extension 進。
public extension RustEngineBridge {
    // MARK: - Synthesized value types

    /// Bridge-synthesized companion to the proto `TaigiWord` returned by
    /// lexicon search responses. Optional fields surface as Swift
    /// `Optional` per proto3 `optional` semantics; consumer in
    /// `LexiconService` converts to the platform-side `TaigiWord`.
    // 中文: lexicon 搜尋回傳的 TaigiWord 對應 Swift struct,Optional 欄位走 proto3 optional 語意。
    struct LexiconRow: Equatable {
        public let id: Int64
        public let roman: String
        public let hanzi: String?
        public let lengthScore: Int32?
        public let sourceBitmask: UInt32?
    }

    /// Bridge-synthesized companion to proto `LexiconAssocEntry`. Consumed
    /// by iOS `NextWordService` for bundled bigram lookups.
    // 中文: 內建詞組(bigram)查詢結果的對應 struct,給 iOS NextWordService 使用。
    struct LexiconAssocEntry: Equatable {
        public let previousWord: String
        public let candidateWord: String
        public let candidateTl: String
        public let count: UInt32
    }

    /// Engine install diagnostic counts, surfaced for dogfood-time
    /// inspection through `RustEngineBridge.diagnostics()` callers.
    // 中文: lexicon engine 安裝後的診斷計數 — dogfood 時透過 diagnostics() 觀察。
    struct LexiconInstallStats: Equatable {
        public let dictionaryRecordCount: UInt64
        public let prefixIndexEntryCount: UInt64
    }

    /// Lexicon engine `inputType` (mirrors proto `InputType`).
    // 中文: lexicon engine 接收的 inputType enum,對應 proto InputType。
    enum LexiconInputType: Int32, Equatable {
        case unspecified = 0
        case romanNoTone = 1
        case romanWithTone = 2
        case hanzi = 3
    }

    /// Lexicon engine `inputMode` (mirrors proto `InputMode`).
    // 中文: lexicon engine 接收的 inputMode enum(TL/POJ/TPS),對應 proto InputMode。
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
    // 中文: 使用者的 12 顆字典開關 snapshot,欄位順序與 proto DictionaryToggles 對齊。
    // 中文: 一律透過 init(from: EngineSettings) 建立,搜尋呼叫端不可自行拼裝以免 snapshot 被切開。
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
        // 中文: kautian subcollection 啟用狀態 (10 腔調 + 姓名附錄)。iOS 一律帶值,故 proto 子訊息恆存在,引擎恆執行 subcollection gate。
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

    /// Output of `lexiconDictionaryFilters` — ready-to-send bitmasks plus
    /// the decoded enabled-source set for Tab3 retag. Replaces verbatim
    /// platform `EnabledDictionaries` bit math (deleted in v3.5.8 slice).
    ///
    /// `assocLookupBitmask` carries the `UInt32.max` sentinel when all 9
    /// association sources are on — preserves the documented
    /// `lexicon.proto:166-173` shortcut. Caller forwards directly to
    /// `lexiconAssocLookup(enabledSourcesBitmask:)`.
    ///
    /// `internal` (not `public`) because `enabledSources` references the
    /// internal `DictionarySource` enum; matches `ClassificationResult`'s
    /// pattern below.
    // 中文: lexiconDictionaryFilters 的輸出 — 含可直接送出的 bitmask 與 Tab3 retag 用的啟用 source 集合。
    // 中文: 9 顆 assoc source 全開時 assocLookupBitmask 走 UInt32.max sentinel,保留 proto 的捷徑語意。
    internal struct DictionaryFilters: Equatable {
        let dictionaryFilterBitmask: UInt32
        let assocLookupBitmask: UInt32
        let enabledSources: Set<DictionarySource>
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
    // 中文: 安裝或原子重灌 lexicon engine 狀態。鍵盤啟動時呼叫一次,冪等。
    // 中文: syllableInventoryPath 必填 — 空字串 = 不載入 syllable inventory,FetchAtPos
    // 中文: 會 graceful-degrade 為空候選;傳絕對路徑才能啟用連續輸入候選查詢。
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
    // 中文: IME 自動完成主入口。Hanzi inputType 一律回 [] — 由 D-8 硬性保證(INVARIANT_LEX_HANZI_GUARD)。
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
    // 中文: Tab3 多字典來源查詢入口。
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
    // 中文: Tab3 用漢字前綴查字典。
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

    /// Bundled-bigram lookup. Called by `NextWordService` after the
    /// commit 10 rewire (replaces direct `AssociationBinaryReader.lookup`).
    // 中文: 內建詞組查詢 — NextWordService 從這裡取詞組關聯。
    static func lexiconAssocLookup(
        previousWord: String,
        limit: UInt32,
        enabledSourcesBitmask: UInt32,
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
                count: entry.count,
            )
        }
    }

    // MARK: - Classification (v3.5.7)

    /// Classifier output — pairs the resolved `InputType` with the
    /// engine-built `searchKey`. C-1 (v3.5.9 D) retired the TPS→TL
    /// pre-conversion; `searchKey` is now an identity passthrough of the
    /// raw input. The `tps:` FST family is queried directly via
    /// `SearchRequest{input_mode=.tps}`.
    // 中文: classifyInput 的輸出 — InputType + searchKey;C-1 之後 searchKey = raw 原樣,TPS 改由 SearchRequest{input_mode=.tps} 直接命中 tps: 族群。
    internal struct ClassificationResult: Equatable {
        let inputType: InputType
        let searchKey: String
    }

    /// Classify `rawInput` into `(InputType, searchKey)`. Single FFI hop —
    /// `lexicon::classify_input` keeps tone / TPS detection inside Rust,
    /// replacing the platform-side per-keystroke ladder that previously
    /// chained multiple phonetics ops per keypress. See
    /// `INVARIANT_LEX_INPUT_CLASSIFICATION_PRECEDENCE`.
    // 中文: 判定 rawInput 屬於哪一種 InputType,並回傳要拿去查詢的 searchKey。
    // 中文: 單一 FFI 呼叫,tone / TPS 偵測都留在 Rust 側,取代舊有平台端的多重 phonetics 串呼叫。
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
            searchKey: r.searchKey,
        )
    }

    /// Resolve user's 12-toggle dictionary preferences into ready-to-send
    /// filter bitmasks + enabled-source set. Single FFI hop replaces the
    /// pre-v3.5.8 verbatim-mirrored `EnabledDictionaries` bit math.
    ///
    /// Call ONCE per query and pass the result down the search pipeline;
    /// re-resolving inside `fetchSystemResults` would split the snapshot.
    // 中文: 把使用者的 12 顆字典開關轉成可直接送出的 filter bitmask + 啟用 source 集合。
    // 中文: 每次查詢只能呼叫一次,結果向下游傳遞;在 fetchSystemResults 內重算會切開 snapshot。
    internal static func lexiconDictionaryFilters(toggles: DictionaryToggles) -> DictionaryFilters {
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
            enabledSources: Set(r.enabledSourceCodes.compactMap(platformDictionarySource(from:))),
        )
    }

    /// Tab3 short-circuit predicate. True iff `text` contains any CJK
    /// codepoint (Unified + Extensions A-E). See
    /// `INVARIANT_LEX_INPUT_CLASSIFICATION_HANZI_RANGE`.
    // 中文: Tab3 漢字判斷捷徑 — text 含 CJK Unified + Ext A-E 任一字元就回 true。
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

    /// Map `Taigi_Engine_InputType` to the platform `InputType` enum.
    /// Unspecified / unrecognised values fall back to `.romanWithoutTone`
    /// (matches the safe-fallback contract of `lexiconDispatch` errors).
    /// Mirrors Android `LexiconBridge.platformInputType` —
    /// must drift together.
    // 中文: 把 proto 端的 InputType 映射到平台端的 InputType,未知值走 .romanWithoutTone fallback。
    // 中文: 必須與 Android LexiconBridge.platformInputType 同步漂移。
    private static func platformInputType(from proto: Taigi_Engine_InputType) -> InputType {
        switch proto {
        case .hanzi: .hanzi
        case .romanWithTone: .romanWithTone
        case .romanNoTone: .romanWithoutTone
        case .unspecified, .UNRECOGNIZED:
            .romanWithoutTone
        }
    }

    /// Fallback only for platform/Rust binary skew where method 18 is absent.
    /// Rust `engine/lexicon/src/dictionary_filters.rs::compute_filters` is
    /// authoritative; keep this bit layout in sync with
    /// `engine/protos/proto/lexicon.proto`. Bit positions pinned by the
    /// 6 inline Rust golden tests.
    // 中文: 平台與 Rust binary skew 時的後備 — 當 method 18 缺席才會走到這。
    // 中文: 真值來源是 Rust compute_filters,bit 配置與 lexicon.proto 須保持同步。
    private static func platformFallbackFilters(toggles: DictionaryToggles) -> DictionaryFilters {
        var dictMask: UInt32 = 0
        if toggles.kautian { dictMask |= 1 << 0 }
        if toggles.taigitv { dictMask |= 1 << 1 }
        if toggles.itaigi { dictMask |= 1 << 2 }
        if toggles.sitbut { dictMask |= 1 << 3 }
        if toggles.taihoa { dictMask |= 1 << 4 }
        if toggles.taijit { dictMask |= 1 << 5 }
        if toggles.kungge { dictMask |= 1 << 6 }
        if toggles.stti { dictMask |= 1 << 7 }
        if toggles.khpoo { dictMask |= 1 << 8 }
        if toggles.khiin { dictMask |= 1 << 9 }
        if toggles.dev { dictMask |= 1 << 10 }
        if toggles.lkk { dictMask |= 1 << 11 }
        if toggles.variant { dictMask |= 1 << 12 }
        dictMask |= encodeKautianSubcollWire(toggles)

        let allAssocOn = toggles.kautian && toggles.taigitv && toggles.itaigi
            && toggles.sitbut && toggles.taihoa && toggles.taijit
            && toggles.kungge && toggles.stti && toggles.khpoo
        let assocMask: UInt32 = allAssocOn ? UInt32.max : (dictMask & 0x1FF)

        var enabled: Set<DictionarySource> = [.custom]
        if toggles.dev { enabled.insert(.dev) }
        if toggles.kautian { enabled.insert(.kautian) }
        if toggles.taigitv { enabled.insert(.taigitv) }
        if toggles.itaigi { enabled.insert(.itaigi) }
        if toggles.sitbut { enabled.insert(.sitbut) }
        if toggles.taihoa { enabled.insert(.taihoa) }
        if toggles.taijit { enabled.insert(.taijit) }
        if toggles.kungge { enabled.insert(.kungge) }
        if toggles.stti { enabled.insert(.stti) }
        if toggles.khpoo { enabled.insert(.khpoo) }
        if toggles.khiin { enabled.insert(.khiin) }
        if toggles.lkk { enabled.insert(.lkk) }
        return DictionaryFilters(
            dictionaryFilterBitmask: dictMask,
            assocLookupBitmask: assocMask,
            enabledSources: enabled,
        )
    }

    /// kautian subcollection wire ENCODE — fallback-only mirror of Rust
    /// `engine/lexicon/src/dictionary_filters.rs::encode_kautian_subcoll_wire`.
    /// Returns the wire high region (bit 13 active + bits 14..=25 enable mask)
    /// when the kautian master is on; `0` otherwise (kautian rows drop via the
    /// source-OR anyway). Keeps fallback behaviour identical to the engine so a
    /// binary-skew session does not silently revert subcollection toggles.
    ///
    /// CROSS-PLATFORM INVARIANT — bit positions mirror
    /// `engine/lexicon/src/dictionary_reader.rs` (`WIRE_KAUTIAN_SUBCOLL_*` /
    /// `KAUTIAN_SUBTAG_*`). Drift causes silent subcollection-filter divergence.
    // 中文: kautian subcollection wire ENCODE — 僅供 binary-skew fallback 的鏡像,對齊 Rust encode_kautian_subcoll_wire。
    private static func encodeKautianSubcollWire(_ toggles: DictionaryToggles) -> UInt32 {
        guard toggles.kautian else { return 0 }
        let activeBit: UInt32 = 1 << 13
        let shift: UInt32 = 14
        let mainBit: UInt16 = 0
        let accentShift: UInt16 = 1
        let nameBit: UInt16 = 11
        let sub = toggles.kautianSubcoll
        var subtag: UInt16 = 1 << mainBit // main always on when master on
        let accents = [
            sub.lukang, sub.sansia, sub.taipak, sub.gilan, sub.tainan,
            sub.kaohsiung, sub.kinmen, sub.makung, sub.sintik, sub.taichung,
        ]
        for (index, isOn) in accents.enumerated() where isOn {
            subtag |= 1 << (accentShift + UInt16(index))
        }
        if sub.nameAppendix { subtag |= 1 << nameBit }
        return activeBit | (UInt32(subtag) << shift)
    }

    /// Map proto `DictionarySourceCode` to the platform `DictionarySource`
    /// enum. Explicit switch (no `rawValue` / `ordinal` reliance — Swift
    /// `DictionarySource` is `String`-backed; codes are wire-stable per
    /// `lexicon.proto::DictionarySourceCode`).
    /// Unspecified / unrecognised codes return `nil` and the caller drops them.
    // 中文: 把 proto 的 DictionarySourceCode 顯式 switch 到平台 DictionarySource enum。
    // 中文: 不依賴 rawValue / ordinal,因為 Swift DictionarySource 是 String-backed。未知碼回 nil,呼叫端丟棄。
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

    // MARK: - Ranking surface (1 op + test seam)

    /// Per-candidate score breakdown returned alongside `ranked` when the
    /// caller opts in. Six fields sum to the engine's sort key.
    // 中文: 單一候選詞的分數細項。六個欄位加總即引擎的 sort key。
    struct ScoreBreakdown: Equatable {
        public let userFreqScore: Int
        public let recencyBonus: Int
        public let exactBonus: Int
        public let completionPenalty: Int
        public let closenessBonus: Int
        public let baseFreqScore: Int

        public var total: Int {
            userFreqScore + recencyBonus + exactBonus + completionPenalty + closenessBonus + baseFreqScore
        }
    }

    /// Composite return for the lexicon ranking pipeline. Production
    /// callers typically just read `ranked`; tests inspect `breakdowns`
    /// to pin scoring math on the bridge boundary.
    // 中文: 排序管線的複合回傳值。Production 通常只用 ranked,測試用 breakdowns 鎖住分數運算。
    struct CandidateRanking: Equatable {
        public let ranked: [TaigiWord]
        public let breakdowns: [ScoreBreakdown]
    }

    /// `Method::ProcessCandidates` — runs the lexicon ranking pipeline
    /// (dedup → score → sort → optional TPS display-dedup) atomically in
    /// the Rust core. Mirrors `engine/ranking/src/process.rs`.
    ///
    /// Cold-start callers that lack a connected user-frequency DB pass
    /// `mergeOrderOnly: true` so engine dedup runs without scoring +
    /// sorting — the score-sort is deterministic but reorders candidates
    /// against the "merged-order on cold-start" behavior. (The keyboard
    /// candidate path no longer drives this — the platform lexicon
    /// fallback that set `mergeOrderOnly: true` on cold-start was retired
    /// in v3.5.8 Item 13; the engine now owns Continuous ranking.)
    ///
    /// `tpsDedupEnabled` is platform-decided (audit § 3) — pass
    /// `inputMode == .tps` from the call site. The engine never derives
    /// it from `AppConfig.inputMode`.
    ///
    /// `nowMs` is caller-supplied for deterministic recency-window math
    /// in tests; production passes `Int64(Date().timeIntervalSince1970 * 1000)`.
    ///
    /// In `#if DEBUG`, requests + logs the per-candidate `ScoreBreakdown`
    /// alongside the ranked list so dogfood traces include per-candidate
    /// score detail. Release builds skip the breakdown (zero
    /// serialization overhead).
    // 中文: 候選詞排序管線 — dedup → score → sort → 可選 TPS display-dedup,全在 Rust 端 atomic 執行。
    // 中文: tpsDedupEnabled 由平台決定(看是否為 TPS layout),不從 AppConfig 推導。
    // 中文: nowMs 由 caller 提供,讓 recency 視窗運算在測試中可重現。
    // 中文: DEBUG 模式會額外要求 ScoreBreakdown 並寫入 log,Release 跳過該欄位節省序列化成本。
    static func processCandidates(
        raw: [TaigiWord],
        normalizedInput: String,
        tpsDedupEnabled: Bool,
        frequencyData: [String: FrequencyData],
        nowMs: Int64,
        mergeOrderOnly: Bool = false,
    ) -> [TaigiWord] {
        #if DEBUG
            let detailed = processCandidatesDetailed(
                raw: raw,
                normalizedInput: normalizedInput,
                tpsDedupEnabled: tpsDedupEnabled,
                frequencyData: frequencyData,
                nowMs: nowMs,
                includeBreakdown: true,
                mergeOrderOnly: mergeOrderOnly,
            )
            if detailed.breakdowns.count == detailed.ranked.count {
                let logger = LoggerFactory.make(category: "RustEngineBridge")
                for (index, word) in detailed.ranked.enumerated() {
                    let b = detailed.breakdowns[index]
                    logger.debug("[SCORE] input='\(normalizedInput)' | \(word.roman) \(word.hanzi ?? ""): user=\(b.userFreqScore) recency=\(b.recencyBonus) exact=\(b.exactBonus) close=\(b.closenessBonus) base=\(b.baseFreqScore) completion=\(b.completionPenalty) total=\(b.total)")
                }
            }
            return detailed.ranked
        #else
            return processCandidatesDetailed(
                raw: raw,
                normalizedInput: normalizedInput,
                tpsDedupEnabled: tpsDedupEnabled,
                frequencyData: frequencyData,
                nowMs: nowMs,
                includeBreakdown: false,
                mergeOrderOnly: mergeOrderOnly,
            ).ranked
        #endif
    }

    /// Test seam — same FFI call as `processCandidates`, plus access to
    /// the per-candidate `ScoreBreakdown` payload. The Swift-side parity
    /// tests in `RustEngineBridgeRankingTests` use this to assert the
    /// engine's score arithmetic; production code stays on the public
    /// `processCandidates` method which discards the breakdown after
    /// debug logging.
    // 中文: 測試專用接口 — 與 processCandidates 同一條 FFI 呼叫,但會回傳分數細項。
    // 中文: 給 RustEngineBridgeRankingTests 鎖住引擎側的分數算法,Production 用上面那個版本。
    static func processCandidatesDetailed(
        raw: [TaigiWord],
        normalizedInput: String,
        tpsDedupEnabled: Bool,
        frequencyData: [String: FrequencyData],
        nowMs: Int64,
        includeBreakdown: Bool,
        mergeOrderOnly: Bool = false,
    ) -> CandidateRanking {
        var payload = Taigi_Engine_ProcessCandidatesRequest()
        payload.raw = raw.map(taigiWordToProto)
        payload.normalizedInput = normalizedInput
        payload.tpsDedupEnabled = tpsDedupEnabled
        payload.freq = frequencyData.map { key, value in
            var entry = Taigi_Engine_FrequencyEntry()
            entry.displayTextKey = key
            entry.count = UInt32(max(0, value.count))
            entry.lastUsedMs = value.lastUsedMillis
            return entry
        }
        payload.nowMs = nowMs
        payload.includeBreakdown = includeBreakdown
        payload.mergeOrderOnly = mergeOrderOnly

        let resp = lexiconDispatch(method: .processCandidates(payload), op: "processCandidates")
        guard case let .processCandidatesResult(result)? = resp?.result else {
            recordFailure(op: "processCandidates", message: "missing process_candidates_result")
            return CandidateRanking(ranked: raw, breakdowns: [])
        }
        let ranked = result.ranked.map(taigiWordFromProto)
        let breakdowns = result.breakdown.map(scoreBreakdownFromProto)
        return CandidateRanking(ranked: ranked, breakdowns: breakdowns)
    }

    // MARK: - Ranking private helpers

    private static func scoreBreakdownFromProto(_ proto: Taigi_Engine_ScoreBreakdown) -> ScoreBreakdown {
        ScoreBreakdown(
            userFreqScore: Int(proto.userFreqScore),
            recencyBonus: Int(proto.recencyBonus),
            exactBonus: Int(proto.exactBonus),
            completionPenalty: Int(proto.completionPenalty),
            closenessBonus: Int(proto.closenessBonus),
            baseFreqScore: Int(proto.baseFreqScore),
        )
    }

    private static func taigiWordToProto(_ word: TaigiWord) -> Taigi_Engine_TaigiWord {
        var proto = Taigi_Engine_TaigiWord()
        proto.id = Int64(word.id)
        proto.roman = word.roman
        if let hanzi = word.hanzi { proto.hanji = hanzi }
        if let length = word.lengthScore { proto.lengthScore = Int32(length) }
        if let mask = word.sourceBitmask { proto.sourceBitmask = UInt32(mask) }
        return proto
    }

    private static func taigiWordFromProto(_ proto: Taigi_Engine_TaigiWord) -> TaigiWord {
        TaigiWord(
            id: Int(proto.id),
            roman: proto.roman,
            hanzi: proto.hasHanji ? proto.hanji : nil,
            lengthScore: proto.hasLengthScore ? Int(proto.lengthScore) : nil,
            sourceBitmask: proto.hasSourceBitmask ? UInt16(truncatingIfNeeded: proto.sourceBitmask) : nil,
        )
    }

    // MARK: - Lexicon envelope dispatch

    /// Lexicon envelope dispatch — encode → FFI roundtrip → decode the
    /// `LexiconResponse` payload. Used by every lexicon method in this
    /// file (search / assoc / classify-input / dictionary-filters / isHanzi
    /// / processCandidates). No `AppConfig` snapshot needed — lexicon ops
    /// read no live config.
    // 中文: lexicon envelope 的 dispatch — encode → FFI roundtrip → decode。
    // 中文: 不需要 AppConfig 因為 lexicon op 不讀 live config。
    private static func lexiconDispatch(
        method: Taigi_Engine_LexiconRequest.OneOf_Method,
        op: String,
    ) -> Taigi_Engine_LexiconResponse? {
        var lexicon = Taigi_Engine_LexiconRequest()
        lexicon.method = method

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.payload = .lexicon(lexicon)

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
        guard case let .lexicon(payload) = response.payload else {
            recordFailure(op: op, message: "missing lexicon payload")
            return nil
        }
        return payload
    }
}

/// Build `DictionaryToggles` from an `EngineSettings` snapshot. Centralises
/// the boolean assembly so search call sites can't accidentally diverge.
// 中文: 從 EngineSettings snapshot 組出 DictionaryToggles 的集中點,避免搜尋呼叫端各自拼裝而漂移。
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
