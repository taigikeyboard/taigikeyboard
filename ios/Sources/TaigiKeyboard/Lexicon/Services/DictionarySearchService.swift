import Foundation

/// Search service backing dictionary search on the Manage Dictionaries page.
///
/// Orchestrates dictionary browsing (distinct from keyboard-candidate generation
/// in `LexiconService`): CJK vs roman path selection, custom-dict prefix search,
/// kautian-first ordering, and source-tag filtering for badge display.
///
/// System-dict queries flow through `LexiconClient` (`EngineLexiconClient` in
/// production wraps `RustEngineBridge.lexiconSearchByHanji` / `lexiconSearchWithSources`). The Rust engine's bitmask filter drops rows
/// where ZERO enabled bits match — but multi-source rows that overlap at
/// least one enabled source still arrive with their full source bitmask.
/// `retagSources` then trims each row's `sources` array to enabled-only so
/// the badge UI reflects the user's current toggles.
final class DictionarySearchService: @unchecked Sendable {
    // MARK: - Dependencies

    private let lexicon: any LexiconClient
    private let userData: any UserDataClient
    private let settingsProvider: EngineSettingsProvider
    private let logger = DebugLogger(category: "DictionarySearchService")

    // MARK: - Init

    init(
        lexicon: any LexiconClient = EngineLexiconClient(),
        userData: any UserDataClient = CompositionRoot.userData,
        settingsProvider: EngineSettingsProvider = SharedSettings.shared,
    ) {
        self.lexicon = lexicon
        self.userData = userData
        self.settingsProvider = settingsProvider
    }

    /// The FST family an input mode selects. `.tps` hits the `tps:` family so
    /// a Zhuyin query finds its rows; `.english` never reaches the lexicon from
    /// the keyboard, so it reads the TL family.
    /// CROSS-PLATFORM INVARIANT — mirrors android/…/ime/dictionary/DictionarySearchService.kt
    /// `lexiconMode`. Drift changes which index a dictionary-search query searches.
    static func lexiconMode(_ inputMode: InputMode) -> RustEngineBridge.LexiconInputMode {
        switch inputMode {
        case .tl, .english: .tl
        case .poj: .poj
        case .tps: .tps
        }
    }

    // MARK: - Public API

    /// Search the user's enabled dictionaries for `query`.
    ///
    /// Hanji queries use the CJK path; roman queries also consult the user's
    /// custom dictionary. System rows keep the engine's order (corpus order,
    /// MOE rows first — `lexicon::search`); custom-dict hits lead the list.
    /// Awaits Trie readiness so searches arriving during the bootstrap window
    /// don't return empty.
    func search(
        query: String,
        limit: Int = 20,
    ) async throws -> [DictionarySearchResult] {
        guard !query.isEmpty else { return [] }

        let inputMode = settingsProvider.current.inputMode
        let isCJK = lexicon.isHanji(query)
        logger.debug("[SEARCH] query='\(query)' isCJK=\(isCJK) inputMode=\(String(describing: inputMode))")

        // Resolve filter bitmask + enabled-source set ONCE per query and
        // hand both down the pipeline. Splitting the snapshot (resolving
        // again in retag) would let toggle changes mid-search produce a
        // mask/badge mismatch (Codex pre-impl BLOCK 5).
        let toggles = RustEngineBridge.DictionaryToggles(from: settingsProvider.current)
        let filters = lexicon.dictionaryFilters(toggles: toggles)

        let systemResults = fetchSystemResults(
            query: query,
            inputMode: inputMode,
            isCJK: isCJK,
            limit: limit,
            filterBitmask: filters.dictionaryFilterBitmask,
        )
        let customResults = isCJK ? [] : await lookupCustomDictionary(query: query, limit: limit)

        let prepared = systemResults.map { retagSources($0, enabled: filters.enabledSources) }

        return customResults + prepared
    }

    // MARK: - Private

    private func fetchSystemResults(
        query: String,
        inputMode: InputMode,
        isCJK: Bool,
        limit: Int,
        filterBitmask: UInt32,
    ) -> [DictionarySearchResult] {
        let bridgeMode = Self.lexiconMode(inputMode)
        let rows = isCJK
            ? lexicon.searchByHanji(
                query: query,
                inputMode: bridgeMode,
                limit: UInt32(limit),
                enabledSourcesBitmask: filterBitmask,
            )
            : lexicon.searchWithSources(
                input: query,
                inputMode: bridgeMode,
                limit: UInt32(limit),
                enabledSourcesBitmask: filterBitmask,
            )
        return rows.map { row in
            // Engine returns raw `tl`; render to POJ when in POJ mode.
            let roman = inputMode == .poj ? RustEngineBridge.tlToPoj(row.roman) : row.roman
            return DictionarySearchResult(
                id: Int(row.id),
                roman: roman,
                tl: row.roman,
                hanji: row.hanji,
                sources: row.sources,
            )
        }
    }

    /// The user's own words for `query`, found the way the keyboard finds
    /// them — by the key the query derives, prefix-matched
    /// (`SearchCustomEntries`, roadmap P7b).
    private func lookupCustomDictionary(query: String, limit: Int) async -> [DictionarySearchResult] {
        let settings = settingsProvider.current
        guard settings.isCustomDictEnabled else { return [] }
        let entries = await userData.search(query: query, mode: settings.inputMode, limit: limit)
        return entries.map { entry in
            DictionarySearchResult(
                id: DictionarySearchResult.customDictMarkerId,
                roman: entry.roman,
                tl: entry.roman,
                hanji: entry.hanji,
                sources: [.custom],
            )
        }
    }

    /// Drop source tags the user has disabled so badges reflect current toggles.
    /// DB-layer filtering has already excluded results whose sources are all disabled.
    private func retagSources(
        _ result: DictionarySearchResult,
        enabled: Set<DictionarySource>,
    ) -> DictionarySearchResult {
        DictionarySearchResult(
            id: result.id,
            roman: result.roman,
            tl: result.tl,
            hanji: result.hanji,
            sources: result.sources.filter { enabled.contains($0) },
        )
    }
}
