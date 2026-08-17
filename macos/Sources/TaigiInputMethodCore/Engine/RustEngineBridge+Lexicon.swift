// Lexicon slice of the engine bridge: loading the dictionary data.

import Foundation

/// Record counts the engine reports after loading. Their only job is to make a
/// successful install self-evident in the log — an install that silently loads
/// zero records looks exactly like a working one until the user types.
struct LexiconInstallStats: Equatable {
    let dictionaryRecordCount: UInt64
    let prefixIndexEntryCount: UInt64
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
    /// `nil` means the round-trip failed; `enabledSourcesBitmask(for:)` is what
    /// turns this answer into something safe to put on the wire.
    static func lexiconDictionaryFilters(toggles: DictionarySourceToggles) -> UInt32? {
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
        return result.dictionaryFilterBitmask
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
        guard let resolved = lexiconDictionaryFilters(toggles: toggles) else { return 0 }
        return resolved == 0 ? noSourcesEnabledBitmask : resolved
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
