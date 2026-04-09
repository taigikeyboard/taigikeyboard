import Foundation

/// ViewModel for dictionary search in Tab 3
@MainActor
final class DictionarySearchViewModel: ObservableObject {
    @Published var searchText = ""
    @Published var results: [DictionarySearchResult] = []
    @Published var isSearching = false

    private var searchTask: Task<Void, Never>?
    private let repository = DictionaryRepository.shared
    private let logger = DebugLogger(category: "DictionarySearchVM")

    init() {
        // Trigger LexiconService init to ensure Trie loads
        _ = LexiconService.shared
    }

    /// Build set of enabled dictionary sources from current settings
    private static func enabledSources() -> Set<DictionarySource> {
        let s = SharedSettings.shared
        var sources: Set<DictionarySource> = [.dev, .custom]
        if s.moeDictEnabled { sources.insert(.kautian) }
        if s.newwordDictEnabled { sources.insert(.taigitv) }
        if s.kunggeDictEnabled { sources.insert(.kungge) }
        if s.iTaigiDictEnabled { sources.insert(.itaigi) }
        if s.taiwanJapanDictEnabled { sources.insert(.taijit) }
        if s.taiHuaDictEnabled { sources.insert(.taihoa) }
        if s.taiwanPlantDictEnabled { sources.insert(.sitbut) }
        if s.sttiDictEnabled { sources.insert(.stti) }
        if s.khpooDictEnabled { sources.insert(.khpoo) }
        if s.khiin { sources.insert(.khiin) }
        if s.lkkDictEnabled { sources.insert(.lkk) }
        return sources
    }

    /// Called when search text changes; debounces 300ms then searches
    func onSearchTextChanged() {
        searchTask?.cancel()

        let query = searchText.trimmingCharacters(in: .whitespaces)

        guard !query.isEmpty else {
            results = []
            isSearching = false
            return
        }

        isSearching = true

        searchTask = Task {
            // 300ms debounce
            try? await Task.sleep(nanoseconds: 300_000_000)
            guard !Task.isCancelled else { return }

            do {
                let inputMode = SharedSettings.shared.inputMode

                // Detect CJK input and use hanzi search path
                let isCJK = query.unicodeScalars.contains {
                    (0x4E00 ... 0x9FFF).contains($0.value) ||
                        (0x3400 ... 0x4DBF).contains($0.value) ||
                        (0x20000 ... 0x2A6DF).contains($0.value)
                }

                logger.debug("[SEARCH] query='\(query)' isCJK=\(isCJK) inputMode=\(String(describing: inputMode))")

                let searchResults: [DictionarySearchResult]
                if isCJK {
                    searchResults = try await repository.searchByHanzi(
                        query: query,
                        inputMode: inputMode,
                        limit: 20,
                    )
                    logger.debug("[SEARCH] hanzi path returned \(searchResults.count) results")
                } else {
                    searchResults = try await repository.searchWithSources(
                        input: query,
                        inputMode: inputMode,
                        limit: 20,
                    )
                    logger.debug("[SEARCH] roman path returned \(searchResults.count) results")
                }

                // Also search custom dictionary (only for romanization input)
                let customResults: [DictionarySearchResult]
                if isCJK {
                    customResults = []
                } else {
                    let isToneAware = query.contains { $0.isNumber }
                    let searchPrefix = isToneAware
                        ? query.lowercased()
                        .replacingOccurrences(of: "-", with: "")
                        .replacingOccurrences(of: " ", with: "")
                        : CustomDictionaryService.generateNotone(query)
                    let customEntries = CustomDictionaryRepository.shared.searchSync(
                        prefix: searchPrefix,
                        isToneAware: isToneAware,
                        limit: 20,
                    )
                    customResults = customEntries.map { entry in
                        DictionarySearchResult(
                            id: -2,
                            roman: entry.roman,
                            tl: entry.roman,
                            hanzi: entry.hanzi,
                            frequency: Int.max,
                            sources: [.custom],
                        )
                    }
                }

                guard !Task.isCancelled else { return }
                // Sort: kautian (教育部) first, then by frequency
                let sorted = searchResults.sorted { a, b in
                    let aIsMoe = a.sources.contains(.kautian)
                    let bIsMoe = b.sources.contains(.kautian)
                    if aIsMoe != bIsMoe { return aIsMoe }
                    return a.frequency > b.frequency
                }
                // Filter source tags to only show enabled dictionaries
                let enabledSources = Self.enabledSources()
                let filtered = sorted.map { result in
                    DictionarySearchResult(
                        id: result.id,
                        roman: result.roman,
                        tl: result.tl,
                        hanzi: result.hanzi,
                        frequency: result.frequency,
                        sources: result.sources.filter { enabledSources.contains($0) },
                    )
                }
                results = customResults + filtered
                isSearching = false
            } catch {
                guard !Task.isCancelled else { return }
                logger.error("[SEARCH] Failed: \(error.localizedDescription)")
                results = []
                isSearching = false
            }
        }
    }
}
