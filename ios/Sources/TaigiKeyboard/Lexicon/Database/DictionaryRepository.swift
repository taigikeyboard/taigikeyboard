import Foundation

/// 詞典查詢（使用 binary mmap 格式）
///
/// 資料流：
/// 1. Trie (key → rowid) — 搜尋索引
/// 2. DictionaryBinaryReader (rowid → record) — 資料查詢
/// 3. Bitmask filter — 來源過濾
final class DictionaryRepository: @unchecked Sendable {
    // MARK: - Properties

    private let binaryReader: DictionaryBinaryReader?
    private let trieService: TrieService
    private let settingsProvider: EngineSettingsProvider
    private let logger = DebugLogger(category: "DictionaryRepository")

    // MARK: - Initialization

    init(
        binaryReader: DictionaryBinaryReader? = nil,
        trieService: TrieService = CompositionRoot.trieService,
        settingsProvider: EngineSettingsProvider = SharedSettings.shared,
    ) {
        self.binaryReader = binaryReader ?? DictionaryBinaryReader()
        self.trieService = trieService
        self.settingsProvider = settingsProvider
    }

    // MARK: - Query Methods

    /// 查詢詞典（使用 Trie 搜尋 + binary 資料查詢）
    func query(
        for input: String,
        inputType: InputType,
        inputMode: InputMode,
        limit: Int = LexiconConstants.Search.defaultLimit,
    ) async throws -> [TaigiWord] {
        guard !input.isEmpty else {
            return []
        }

        guard let reader = binaryReader else {
            throw LexiconError.databaseNotAvailable
        }

        guard trieService.isReady else {
            logger.error("[QUERY] Trie not loaded")
            throw LexiconError.trieNotLoaded
        }

        // 漢字輸入暫不支援
        guard inputType != .hanzi else {
            logger.warning("[QUERY] Hanzi input not supported")
            return []
        }

        let allRowIds = lookupRowIds(input: input, inputMode: inputMode)

        guard !allRowIds.isEmpty else {
            return []
        }

        // Binary 查詢 + 過濾
        let enabledDicts = EnabledDictionaries(from: settingsProvider.current)

        var results: [TaigiWord] = []
        for rowId in allRowIds {
            guard let record = reader.record(at: rowId) else { continue }
            guard DictionaryBinaryReader.passesFilter(
                recordBitmask: record.bitmask,
                enabledDicts: enabledDicts,
            ) else { continue }

            let roman = inputMode == .poj
                ? RomanizationConverter.tlToPOJ(record.tl)
                : record.tl

            results.append(TaigiWord(
                id: rowId,
                roman: roman,
                hanzi: record.hanzi,
                lengthScore: Int(record.frequency),
                sourceBitmask: record.bitmask,
            ))
        }

        return Array(results
            .sorted { ($0.lengthScore ?? 0) > ($1.lengthScore ?? 0) }
            .prefix(limit))
    }

    // MARK: - Search With Sources (for dictionary exploration)

    /// Search dictionary and return results with source information
    func searchWithSources(
        input: String,
        inputMode: InputMode,
        limit: Int = 50,
    ) async throws -> [DictionarySearchResult] {
        guard !input.isEmpty else { return [] }

        guard let reader = binaryReader else {
            throw LexiconError.databaseNotAvailable
        }

        guard trieService.isReady else {
            logger.error("[SEARCH-SOURCES] Trie not loaded")
            throw LexiconError.trieNotLoaded
        }

        let allRowIds = lookupRowIds(input: input, inputMode: inputMode)
        guard !allRowIds.isEmpty else { return [] }

        return buildSearchResults(
            reader: reader,
            rowIds: allRowIds,
            inputMode: inputMode,
            limit: limit,
        )
    }

    /// Search dictionary by hanzi prefix (漢字前綴搜尋)
    func searchByHanzi(
        query: String,
        inputMode: InputMode,
        limit: Int = 50,
    ) async throws -> [DictionarySearchResult] {
        guard !query.isEmpty else { return [] }

        logger.debug("[HANZI-SEARCH] query='\(query)' limit=\(limit)")

        guard let reader = binaryReader else {
            throw LexiconError.databaseNotAvailable
        }

        guard trieService.isReady else {
            logger.error("[HANZI-SEARCH] Trie not loaded")
            throw LexiconError.trieNotLoaded
        }

        // 使用 hanzi: prefix 在主 trie 做前綴搜尋
        let trieKey = LexiconConstants.TriePrefix.hanzi + query
        let rowIds = trieService.prefixSearch(trieKey)

        logger.debug("[HANZI-SEARCH] trie returned \(rowIds.count) rowids")

        guard !rowIds.isEmpty else { return [] }

        let results = buildSearchResults(
            reader: reader,
            rowIds: rowIds,
            inputMode: inputMode,
            limit: limit,
        )

        logger.debug("[HANZI-SEARCH] returned \(results.count) results")

        return results
    }

    // MARK: - Connection Status

    func isConnected() -> Bool {
        binaryReader != nil
    }

    // MARK: - Private Methods

    /// Look up rowIds from trie (exact match + prefix search, deduplicated, no artificial limit)
    private func lookupRowIds(
        input: String,
        inputMode: InputMode,
    ) -> [Int] {
        let normalizedInput = InputNormalizer.normalize(input, mode: inputMode)
        guard !normalizedInput.isEmpty else { return [] }

        let trieKey = LexiconConstants.TriePrefix.prefix(for: inputMode) + normalizedInput
        let exactRowIds = trieService.lookup(trieKey)
        let prefixRowIds = trieService.prefixSearch(trieKey)

        logger.debug("[TRIE] input='\(input)' exact=\(exactRowIds.count) prefix=\(prefixRowIds.count)")

        return Array(Set(exactRowIds + prefixRowIds))
    }

    /// Build DictionarySearchResult array from rowIds with filtering and sorting
    private func buildSearchResults(
        reader: DictionaryBinaryReader,
        rowIds: [Int],
        inputMode: InputMode,
        limit: Int,
    ) -> [DictionarySearchResult] {
        let enabledDicts = EnabledDictionaries(from: settingsProvider.current)

        var results: [DictionarySearchResult] = []
        for rowId in rowIds {
            guard let record = reader.record(at: rowId) else { continue }
            guard DictionaryBinaryReader.passesFilter(
                recordBitmask: record.bitmask,
                enabledDicts: enabledDicts,
            ) else { continue }

            let roman = inputMode == .poj
                ? RomanizationConverter.tlToPOJ(record.tl)
                : record.tl

            let sources = DictionaryBinaryReader.sourcesFromBitmask(record.bitmask)

            results.append(DictionarySearchResult(
                id: rowId,
                roman: roman,
                tl: record.tl,
                hanzi: record.hanzi,
                frequency: Int(record.frequency),
                sources: sources,
            ))
        }

        return Array(results
            .sorted { $0.frequency > $1.frequency }
            .prefix(limit))
    }
}
