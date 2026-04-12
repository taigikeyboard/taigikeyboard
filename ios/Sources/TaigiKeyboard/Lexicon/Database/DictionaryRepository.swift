import Foundation

/// 詞庫開關設定（從 SharedSettings 讀取）
struct EnabledDictionaries {
    let kautian: Bool // 教育部臺灣台語常用詞辭典
    let taigitv: Bool // 台語新詞辭庫
    let kungge: Bool // 台語工藝詞庫
    let itaigi: Bool // iTaigi 華台對照典
    let taijit: Bool // 台日大辭典
    let taihoa: Bool // 台華線頂對照典
    let sitbut: Bool // 台灣植物名彙
    let stti: Bool // 學科術語辭典
    let khpoo: Bool // 腔口補充資料
    let variant: Bool // 異用字
    let khiin: Bool // 在來字
    let lkk: Bool // LKK漢羅合用建議用字

    /// 從 SharedSettings 讀取設定
    static func fromSettings() -> EnabledDictionaries {
        let settings = SharedSettings.shared
        return EnabledDictionaries(
            kautian: settings.isMoeDictEnabled,
            taigitv: settings.isNewwordDictEnabled,
            kungge: settings.isKunggeDictEnabled,
            itaigi: settings.isITaigiDictEnabled,
            taijit: settings.isTaiwanJapanDictEnabled,
            taihoa: settings.isTaiHuaDictEnabled,
            sitbut: settings.isTaiwanPlantDictEnabled,
            stti: settings.isSttiDictEnabled,
            khpoo: settings.isKhpooDictEnabled,
            variant: settings.isVariantEnabled,
            khiin: settings.isKhiinEnabled,
            lkk: settings.isLkkDictEnabled,
        )
    }

    /// 是否全部開啟（9 個主要來源 + lkk）
    var allEnabled: Bool {
        kautian && taigitv && kungge && itaigi && taijit && taihoa && sitbut && stti && khpoo && lkk
    }

    /// 轉換為 dictionary bitmask（bits 0-11）
    /// Bit layout 必須與 dictionary.bin 一致
    func sourceBitmask() -> UInt16 {
        var mask: UInt16 = 0
        if kautian { mask |= 1 << 0 }
        if taigitv { mask |= 1 << 1 }
        if itaigi { mask |= 1 << 2 }
        if sitbut { mask |= 1 << 3 }
        if taihoa { mask |= 1 << 4 }
        if taijit { mask |= 1 << 5 }
        if kungge { mask |= 1 << 6 }
        if stti { mask |= 1 << 7 }
        if khpoo { mask |= 1 << 8 }
        // khiin = bit 9 (handled separately in filter)
        // dev = bit 10 (always included)
        if lkk { mask |= 1 << 11 }
        return mask
    }

    /// 轉換為 association bitmask（bits 0-8，對應 association.bin 的 9 個來源）
    func associationBitmask() -> UInt16 {
        sourceBitmask() & 0x1FF
    }

    /// association 的 9 個來源是否全部開啟
    var allAssociationSourcesEnabled: Bool {
        kautian && taigitv && itaigi && sitbut && taihoa && taijit && kungge && stti && khpoo
    }
}

/// 詞典查詢（使用 binary mmap 格式）
///
/// 資料流：
/// 1. Trie (key → rowid) — 搜尋索引
/// 2. DictionaryBinaryReader (rowid → record) — 資料查詢
/// 3. Bitmask filter — 來源過濾
final class DictionaryRepository: @unchecked Sendable {
    // MARK: - Properties

    static let shared = DictionaryRepository()

    private let binaryReader: DictionaryBinaryReader?
    private let trieService: TrieService
    private let hanziTrieService: TrieService?
    private let logger = DebugLogger(category: "DictionaryRepository")

    // MARK: - Initialization

    init(
        binaryReader: DictionaryBinaryReader? = nil,
        trieService: TrieService = .shared,
        hanziTrieService: TrieService? = nil,
    ) {
        self.binaryReader = binaryReader ?? DictionaryBinaryReader()
        self.trieService = trieService
        self.hanziTrieService = hanziTrieService
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
            throw DictionaryError.databaseNotAvailable
        }

        guard trieService.isReady else {
            logger.error("[QUERY] Trie not loaded")
            throw DictionaryError.trieNotLoaded
        }

        // 漢字輸入暫不支援
        guard inputType != .hanzi else {
            logger.warning("[QUERY] Hanzi input not supported")
            return []
        }

        let allRowIds = lookupRowIds(input: input, inputMode: inputMode, limit: limit)

        guard !allRowIds.isEmpty else {
            return []
        }

        // Binary 查詢 + 過濾
        let enabledDicts = EnabledDictionaries.fromSettings()

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
            throw DictionaryError.databaseNotAvailable
        }

        guard trieService.isReady else {
            logger.error("[SEARCH-SOURCES] Trie not loaded")
            throw DictionaryError.trieNotLoaded
        }

        let allRowIds = lookupRowIds(input: input, inputMode: inputMode, limit: limit)
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
            throw DictionaryError.databaseNotAvailable
        }

        // 使用 hanzi trie 做前綴搜尋
        guard let hanziTrie = hanziTrieService, hanziTrie.isReady else {
            logger.warning("[HANZI-SEARCH] Hanzi trie not available")
            return []
        }

        let rowIds = hanziTrie.prefixSearch(query, limit: limit * 6)

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

    /// Look up rowIds from trie (exact match + prefix search, deduplicated)
    private func lookupRowIds(
        input: String,
        inputMode: InputMode,
        limit: Int,
    ) -> [Int] {
        let normalizedInput = InputNormalizer.normalize(input, mode: inputMode)
        guard !normalizedInput.isEmpty else { return [] }

        let trieKey = LexiconConstants.TriePrefix.prefix(for: inputMode) + normalizedInput
        let exactRowIds = trieService.lookup(trieKey)
        let prefixRowIds = trieService.prefixSearch(trieKey, limit: limit * 6)

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
        let enabledDicts = EnabledDictionaries.fromSettings()

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
