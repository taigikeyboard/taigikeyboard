import Foundation

/// Search service backing the Dictionary tab.
///
/// Orchestrates dictionary browsing (distinct from keyboard-candidate generation
/// in `LexiconService`): CJK vs roman path selection, custom-dict prefix search,
/// kautian-first ordering, and source-tag filtering for badge display.
///
/// `DictionaryRepository` performs DB-layer filtering using the current
/// settings snapshot, so disabled dictionaries never show up in results.
/// This service only filters each result's `sources` array so the badge UI
/// reflects the user's current toggles.
final class DictionarySearchService: @unchecked Sendable {
    // MARK: - Dependencies

    private let repository: DictionaryRepository
    private let customDictionaryRepository: CustomDictionaryRepository
    private let settingsProvider: EngineSettingsProvider
    private let logger = DebugLogger(category: "DictionarySearchService")

    // MARK: - Init

    init(
        repository: DictionaryRepository? = nil,
        customDictionaryRepository: CustomDictionaryRepository = .shared,
        settingsProvider: EngineSettingsProvider = SharedSettings.shared,
    ) {
        self.customDictionaryRepository = customDictionaryRepository
        self.settingsProvider = settingsProvider
        // Build a repository that shares the injected settings provider so
        // DB-layer filtering agrees with the service's enabled-source view.
        self.repository = repository ?? DictionaryRepository(
            settingsProvider: settingsProvider,
        )
        // Bootstrap Trie + custom dictionary via the shared lexicon service so
        // a fresh launch straight into Dictionary has the indexes ready.
        _ = LexiconService.shared
    }

    // MARK: - Public API

    /// Search the user's enabled dictionaries for `query`.
    ///
    /// Hanzi queries use the CJK path; roman queries also consult the user's
    /// custom dictionary. Results are sorted with kautian (教育部) first, then
    /// by frequency; custom-dict hits lead the list.
    func search(
        query: String,
        inputMode: InputMode,
        limit: Int = 20,
    ) async throws -> [DictionarySearchResult] {
        guard !query.isEmpty else { return [] }

        let isCJK = CandidateProcessor.isHanzi(query)
        logger.debug("[SEARCH] query='\(query)' isCJK=\(isCJK) inputMode=\(String(describing: inputMode))")

        let systemResults = try await fetchSystemResults(
            query: query,
            inputMode: inputMode,
            isCJK: isCJK,
            limit: limit,
        )
        let customResults = isCJK ? [] : lookupCustomDictionary(query: query)

        let enabled = EnabledDictionaries(from: settingsProvider.current).enabledSources
        let prepared = sortByMoeThenFrequency(systemResults)
            .map { retagSources($0, enabled: enabled) }

        return customResults + prepared
    }

    // MARK: - Private

    private func fetchSystemResults(
        query: String,
        inputMode: InputMode,
        isCJK: Bool,
        limit: Int,
    ) async throws -> [DictionarySearchResult] {
        if isCJK {
            return try await repository.searchByHanzi(query: query, inputMode: inputMode, limit: limit)
        }
        return try await repository.searchWithSources(input: query, inputMode: inputMode, limit: limit)
    }

    private func lookupCustomDictionary(query: String) -> [DictionarySearchResult] {
        guard settingsProvider.current.isCustomDictEnabled else { return [] }
        let (prefix, isToneAware) = CustomDictionaryDerivation.searchPrefix(for: query)
        let entries = customDictionaryRepository.searchSync(
            prefix: prefix,
            isToneAware: isToneAware,
            limit: 20,
        )
        return entries.map { entry in
            DictionarySearchResult(
                id: DictionarySearchResult.customDictMarkerId,
                roman: entry.roman,
                tl: entry.roman,
                hanzi: entry.hanzi,
                frequency: Int.max,
                sources: [.custom],
            )
        }
    }

    /// Kautian (教育部) results first, then descending frequency.
    private func sortByMoeThenFrequency(_ results: [DictionarySearchResult]) -> [DictionarySearchResult] {
        results.sorted { a, b in
            let aMoe = a.sources.contains(.kautian)
            let bMoe = b.sources.contains(.kautian)
            if aMoe != bMoe { return aMoe }
            return a.frequency > b.frequency
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
            hanzi: result.hanzi,
            frequency: result.frequency,
            sources: result.sources.filter { enabled.contains($0) },
        )
    }
}
