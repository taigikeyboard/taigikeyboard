// Lexicon slice of the engine bridge: loading the dictionary data.

import Foundation

/// Record counts the engine reports after loading. Their only job is to make a
/// successful install self-evident in the log — an install that silently loads
/// zero records looks exactly like a working one until the user types.
struct LexiconInstallStats: Equatable {
    let dictionaryRecordCount: UInt64
    let prefixIndexEntryCount: UInt64
}

/// What one resolve of the user's dictionary toggles answered.
///
/// Both halves come from the same round-trip on purpose: the mask decides
/// which rows the engine returns and the source set decides which badges those
/// rows are labelled with, and two resolves could straddle a settings change.
struct DictionaryFilters: Equatable, Sendable {
    /// The engine's own answer, verbatim.
    let dictionaryFilterBitmask: UInt32
    /// The sources the user has switched on, for labelling results.
    let enabledSources: Set<DictionarySource>

    /// The value to put on the wire.
    ///
    /// `0` is not "no sources": the composing path reads it as "platform did
    /// not wire this" and searches all of them
    /// (`engine/composing/src/dispatch.rs:247`). A user who switched
    /// everything off means it, so their answer goes out as a mask with no
    /// source bits instead.
    ///
    /// The SEARCH path does not share that normalisation — it builds its
    /// filter from the mask verbatim (`engine/lexicon/src/search.rs:190`), so
    /// there `0` already means "nothing enabled" and the all-on sentinel is
    /// `UInt32.max`. That is why a failed resolve has a different fallback per
    /// path; see `RustEngineBridge.allSourcesEnabledSearchBitmask`.
    var wireMask: UInt32 {
        dictionaryFilterBitmask == 0 ? RustEngineBridge.noSourcesEnabledBitmask
            : dictionaryFilterBitmask
    }
}

/// One row of a dictionary search.
///
/// `lengthScore` is the engine's own ranking term, not a usage count — the
/// name is deliberately not "frequency", which is what the user's own commit
/// counts are called everywhere else in this codebase.
struct LexiconRow: Equatable, Sendable {
    let id: Int64
    let roman: String
    let hanzi: String?
    let lengthScore: Int32?
    let sourceBitmask: UInt32?
}

/// The romanization the engine should search in.
enum LexiconInputMode: Int, Sendable {
    case tl = 1
    case poj = 2

    /// Raw values match `Taigi_Engine_InputMode`; a mismatch would search the
    /// wrong key family rather than fail, which is why the correspondence is
    /// asserted in one place rather than at each search.
    var wire: Taigi_Engine_InputMode {
        Taigi_Engine_InputMode(rawValue: rawValue) ?? .unspecified
    }
}

extension RustEngineBridge {
    /// Points the engine at the dictionary data. Sent once per process: the
    /// files are read-only and outlive every composing session.
    ///
    /// `dictionaryVersion` is the platform's stamp for the data it shipped; the
    /// engine records it so a later slice can tell which build's dictionary the
    /// user's data was learned against.
    ///
    /// `nil` means the engine has no lexicon installed. Nothing here retries or
    /// falls back: a search against an uninstalled engine returns no candidates,
    /// which is the same graceful degradation any other empty result produces.
    static func lexiconInstall(
        artifacts: DictionaryArtifacts,
        dictionaryVersion: UInt32,
    ) -> LexiconInstallStats? {
        var install = Taigi_Engine_InstallRequest()
        install.triePath = artifacts.triePath
        install.dictionaryBinPath = artifacts.dictionaryBinPath
        install.associationBinPath = artifacts.associationBinPath
        install.dictionaryVersion = dictionaryVersion
        install.syllableInventoryPath = artifacts.syllableInventoryPath

        let op = "lexiconInstall"
        guard let response = lexiconResponse(.install(install), op: op) else { return nil }
        guard case let .installResult(result)? = response.result else {
            recordFailure(op: op, message: "response carried no install result")
            return nil
        }
        return LexiconInstallStats(
            dictionaryRecordCount: result.dictionaryRecordCount,
            prefixIndexEntryCount: result.prefixIndexEntryCount,
        )
    }

    /// Resolves the user's dictionary toggles into the bitmask the engine
    /// filters candidates by.
    ///
    /// The bit layout — including the kautian subcollection region in bits
    /// 13-25 — belongs to Rust (`engine/lexicon/src/dictionary_filters.rs`),
    /// and this asks for it rather than reproducing it. iOS keeps a mirrored
    /// bit-math fallback for the window where a rebuilt Swift binary meets an
    /// old xcframework (`RustEngineBridge+Lexicon.swift:424-464`); macOS
    /// deliberately does not, per `planning.md` § No redundant fallback — the
    /// two artefacts here are built by one `make build`, and a second copy of
    /// the layout is a second thing to keep in step.
    ///
    /// `nil` means the round-trip failed. Callers resolve this ONCE per query
    /// and pass the answer down: the mask sent to the engine and the source
    /// set used to label the results have to describe one instant, or a toggle
    /// changed mid-search shows badges the results were not filtered by.
    static func lexiconDictionaryFilters(toggles: DictionarySourceToggles) -> DictionaryFilters? {
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

        // Always sent: an absent subcollection message tells the engine to skip
        // the gate and treat every subcollection as on (`lexicon.proto:452-454`),
        // which would quietly ignore the eleven toggles macOS ships.
        var subcollProto = Taigi_Engine_KautianSubcollToggles()
        let subcollections = toggles.kautianSubcollections
        subcollProto.accentLukang = subcollections.accentLukang
        subcollProto.accentSansia = subcollections.accentSansia
        subcollProto.accentTaipak = subcollections.accentTaipak
        subcollProto.accentGilan = subcollections.accentGilan
        subcollProto.accentTainan = subcollections.accentTainan
        subcollProto.accentKaohsiung = subcollections.accentKaohsiung
        subcollProto.accentKinmen = subcollections.accentKinmen
        subcollProto.accentMakung = subcollections.accentMakung
        subcollProto.accentSintik = subcollections.accentSintik
        subcollProto.accentTaichung = subcollections.accentTaichung
        subcollProto.nameAppendix = subcollections.nameAppendix
        togglesProto.kautianSubcoll = subcollProto

        var filters = Taigi_Engine_DictionaryFiltersRequest()
        filters.toggles = togglesProto

        let op = "lexiconDictionaryFilters"
        guard let response = lexiconResponse(.dictionaryFilters(filters), op: op) else { return nil }
        guard case let .dictionaryFiltersResult(result)? = response.result else {
            recordFailure(op: op, message: "response carried no dictionary-filters result")
            return nil
        }
        return DictionaryFilters(
            dictionaryFilterBitmask: result.dictionaryFilterBitmask,
            enabledSources: Set(result.enabledSourceCodes.compactMap(dictionarySource(from:))),
        )
    }

    /// The engine's own name for a source, mapped to ours.
    ///
    /// An explicit switch rather than a raw-value cast: the wire codes are
    /// stable numbers and `DictionarySource` is string-backed, so any
    /// correspondence between them is a coincidence waiting to break. An
    /// unrecognised code is dropped — a newer engine naming a source this
    /// build has never heard of is not a reason to fail a search.
    private static func dictionarySource(
        from code: Taigi_Engine_DictionarySourceCode,
    ) -> DictionarySource? {
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
        case .dictSourceUnspecified, .UNRECOGNIZED: nil
        }
    }

    /// A mask carrying no source bits, for the user who switched every
    /// dictionary off.
    ///
    /// The wire cannot say that with a `0`: the engine reads `0` as "platform
    /// did not wire this" and turns everything back on
    /// (`composing.proto:176-183`), so sending the engine's own all-off answer
    /// verbatim would hand the user every dictionary the moment they turned
    /// the last one off. Bit 13 is the kautian subcollection gate's "active"
    /// flag (`engine/lexicon/src/dictionary_filters.rs`), which makes the mask
    /// non-zero while leaving the source region — bits 0-12 — empty, so no
    /// record passes the filter.
    ///
    /// NAMED CROSS-PLATFORM DIVERGENCE, classified **deferred**
    /// (`.claude/rules/cross-platform-alignment.md` §3): iOS and Android send
    /// the engine's `0` straight through and therefore still search every
    /// dictionary in this state. Fixing them means touching their own bridges,
    /// which is a round of its own.
    static let noSourcesEnabledBitmask: UInt32 = 1 << 13

    /// What a SEARCH sends when the toggles could not be resolved.
    ///
    /// The two paths read a mask differently. Composing normalises `0` to
    /// all-on; search does not, and takes `UInt32.max` as its "filter
    /// disabled" sentinel (`engine/lexicon/src/dictionary_reader.rs:147`).
    /// Sending `0` here would be fail-CLOSED — an FFI hiccup would empty the
    /// dictionary rather than widen it, which is the opposite of what a
    /// failure should degrade to.
    static let allSourcesEnabledSearchBitmask = UInt32.max

    /// The value to put in `FetchAtPos.enabled_sources_bitmask` for `toggles`.
    ///
    /// Three answers rather than one, and the difference matters:
    /// - a resolved mask goes out as it is;
    /// - a resolved mask of `0` means the user turned everything off, and goes
    ///   out as `noSourcesEnabledBitmask`;
    /// - a FAILED resolve goes out as `0`, so the engine falls back to
    ///   searching everything. A failure is not a preference: degrading to a
    ///   wider candidate list is recoverable, degrading to none looks like a
    ///   broken keyboard.
    static func enabledSourcesBitmask(for toggles: DictionarySourceToggles) -> UInt32 {
        lexiconDictionaryFilters(toggles: toggles)?.wireMask ?? 0
    }

    /// Searches by romanization.
    ///
    /// An empty result and a failed round-trip are the same answer here: a
    /// search that could not run shows nothing, which is what a search with no
    /// matches shows too, and there is nothing the user could do differently
    /// either way.
    static func lexiconSearchWithSources(
        input: String,
        inputMode: LexiconInputMode,
        limit: UInt32,
        enabledSourcesBitmask: UInt32,
    ) -> [LexiconRow] {
        var payload = Taigi_Engine_SearchWithSourcesRequest()
        payload.input = input
        payload.inputMode = inputMode.wire
        payload.limit = limit
        payload.enabledSourcesBitmask = enabledSourcesBitmask

        let op = "lexiconSearchWithSources"
        guard let response = lexiconResponse(.searchWithSources(payload), op: op) else { return [] }
        guard case let .searchWithSourcesResult(result)? = response.result else {
            recordFailure(op: op, message: "response carried no search result")
            return []
        }
        return result.rows.map(row(from:))
    }

    /// Searches by 漢字.
    static func lexiconSearchByHanzi(
        query: String,
        inputMode: LexiconInputMode,
        limit: UInt32,
        enabledSourcesBitmask: UInt32,
    ) -> [LexiconRow] {
        var payload = Taigi_Engine_SearchByHanziRequest()
        payload.query = query
        payload.inputMode = inputMode.wire
        payload.limit = limit
        payload.enabledSourcesBitmask = enabledSourcesBitmask

        let op = "lexiconSearchByHanzi"
        guard let response = lexiconResponse(.searchByHanzi(payload), op: op) else { return [] }
        guard case let .searchByHanziResult(result)? = response.result else {
            recordFailure(op: op, message: "response carried no search result")
            return []
        }
        return result.rows.map(row(from:))
    }

    /// Whether `text` contains 漢字, and so which of the two searches to run.
    ///
    /// Asked of the engine rather than tested here: the ranges are an
    /// invariant the three platforms share
    /// (`INVARIANT_LEX_INPUT_CLASSIFICATION_HANZI_RANGE`), and Android's
    /// hand-written version had an unreachable clause for exactly this reason —
    /// its 16-bit character type could not express the extension planes.
    ///
    /// `false` on a failed round-trip, which routes a 漢字 query down the
    /// romanization path and finds nothing, rather than failing the search.
    static func isHanzi(_ text: String) -> Bool {
        var payload = Taigi_Engine_IsHanziRequest()
        payload.text = text

        let op = "isHanzi"
        guard let response = lexiconResponse(.isHanzi(payload), op: op) else { return false }
        guard case let .isHanziResult(result)? = response.result else {
            recordFailure(op: op, message: "response carried no is-hanzi result")
            return false
        }
        return result.isHanzi
    }

    private static func row(from proto: Taigi_Engine_TaigiWord) -> LexiconRow {
        LexiconRow(
            id: proto.id,
            roman: proto.roman,
            hanzi: proto.hasHanji ? proto.hanji : nil,
            lengthScore: proto.hasLengthScore ? proto.lengthScore : nil,
            sourceBitmask: proto.hasSourceBitmask ? proto.sourceBitmask : nil,
        )
    }

    private static func lexiconResponse(
        _ method: Taigi_Engine_LexiconRequest.OneOf_Method,
        op: String,
    ) -> Taigi_Engine_LexiconResponse? {
        var lexicon = Taigi_Engine_LexiconRequest()
        lexicon.method = method
        guard let payload = roundtrip(payload: .lexicon(lexicon), op: op) else { return nil }
        guard case let .lexicon(response) = payload else {
            recordFailure(op: op, message: "expected a lexicon payload, got \(payload)")
            return nil
        }
        return response
    }
}
