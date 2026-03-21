import Foundation
import OSLog
import SQLite3

/// 詞典資料庫 Repository
/// 負責詞典資料的查詢與存取
final class DictionaryRepository: @unchecked Sendable {

    // MARK: - 詞庫開關設定

    /// 詞庫開關設定結構
    private struct EnabledDictionaries {
        let kautian: Bool   // 教育部臺灣台語常用詞辭典
        let taigitv: Bool   // 台語新詞辭庫
        let kungge: Bool    // 台語工藝詞庫
        let itaigi: Bool    // iTaigi 華台對照典
        let taijit: Bool    // 台日大辭典
        let taihoa: Bool    // 台華線頂對照典
        let sitbut: Bool    // 台灣植物名彙
        let stti: Bool      // 學科術語辭典
        let khpoo: Bool     // 腔口補充資料
        let variant: Bool   // 異用字
        let khiin: Bool     // 在來字
        let lkk: Bool       // LKK漢羅合用建議用字

        /// 從 SharedSettings 讀取設定
        static func fromSettings() -> EnabledDictionaries {
            let settings = SharedSettings.shared
            return EnabledDictionaries(
                kautian: settings.moeDictEnabled,
                taigitv: settings.newwordDictEnabled,
                kungge: settings.kunggeDictEnabled,
                itaigi: settings.iTaigiDictEnabled,
                taijit: settings.taiwanJapanDictEnabled,
                taihoa: settings.taiHuaDictEnabled,
                sitbut: settings.taiwanPlantDictEnabled,
                stti: settings.sttiDictEnabled,
                khpoo: settings.khpooDictEnabled,
                variant: settings.variantEnabled,
                khiin: settings.khiin,
                lkk: settings.lkkDictEnabled
            )
        }

        /// 是否全部關閉
        var allDisabled: Bool {
            !kautian && !taigitv && !kungge && !itaigi && !taijit && !taihoa && !sitbut && !stti && !khpoo && !lkk
        }

        /// 是否全部開啟
        var allEnabled: Bool {
            kautian && taigitv && kungge && itaigi && taijit && taihoa && sitbut && stti && khpoo && lkk
        }

        /// 建構 SQL WHERE 條件（使用 OR 邏輯）
        func buildWhereCondition() -> String {
            var result = ""

            // 異用字過濾：關閉時只顯示非異用字
            if !variant {
                result += "AND is_variant = 0 "
            }

            // 在來字過濾：關閉時排除在來字
            if !khiin {
                result += "AND khiin = 0 "
            }

            // 全部開啟時不加詞庫過濾條件
            if allEnabled { return result }

            var conditions: [String] = []
            if kautian { conditions.append("kautian = 1") }
            if taigitv { conditions.append("taigitv = 1") }
            if kungge { conditions.append("kungge = 1") }
            if itaigi { conditions.append("itaigi = 1") }
            if taijit { conditions.append("taijit = 1") }
            if taihoa { conditions.append("taihoa = 1") }
            if sitbut { conditions.append("sitbut = 1") }
            if stti { conditions.append("stti = 1") }
            if khpoo { conditions.append("khpoo = 1") }
            if lkk { conditions.append("lkk = 1") }

            // Always include dev supplement entries
            conditions.append("dev = 1")

            if !conditions.isEmpty {
                result += "AND (" + conditions.joined(separator: " OR ") + ")"
            }

            return result
        }
    }

    // MARK: - Properties

    static let shared = DictionaryRepository()

    private let connectionManager: SQLiteConnectionManager
    private let trieService: TrieService
    private let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "DictionaryRepository"
    )

    // MARK: - Initialization

    init(
        connectionManager: SQLiteConnectionManager? = nil,
        trieService: TrieService = .shared
    ) {
        self.connectionManager = connectionManager ?? SQLiteConnectionManager(
            databasePath: Self.getDatabasePath,
            queueLabel: "com.taigikeyboard.dictionary",
            loggerCategory: "DictionaryRepository"
        )
        self.trieService = trieService
    }

    // MARK: - Database Path

    private static func getDatabasePath() throws -> String {
        let bundle = Bundle(for: DictionaryRepository.self)
        guard let path = bundle.path(
            forResource: LexiconConstants.Database.fileName,
            ofType: LexiconConstants.Database.fileExtension
        ) else {
            throw DictionaryError.databaseNotFound
        }
        return path
    }

    // MARK: - Query Methods

    /// 查詢詞典（使用 Trie 搜尋）
    ///
    /// - Throws: DictionaryError.trieNotLoaded 如果 Trie 未載入
    func query(
        for input: String,
        inputType: InputType,
        inputMode: InputMode,
        limit: Int = LexiconConstants.Search.defaultLimit
    ) async throws -> [TaigiWord] {
        guard !input.isEmpty else {
            return []
        }

        try await connectionManager.ensureInitialized()

        // 確認 Trie 已載入
        guard trieService.isReady else {
            logger.error("[QUERY] Trie not loaded")
            throw DictionaryError.trieNotLoaded
        }

        // 漢字輸入暫不支援
        guard inputType != .hanzi else {
            logger.warning("[QUERY] Hanzi input not supported")
            return []
        }

        return try await queryWithTrie(
            input: input,
            inputMode: inputMode,
            limit: limit
        )
    }

    // MARK: - Trie Query

    /// 使用 Trie 查詢詞典
    ///
    /// 流程：
    /// 1. 正規化輸入（調符→數字、小寫、去連字符）
    /// 2. Trie 完全匹配 + 前綴搜尋取得 rowid
    /// 3. SQLite 批次查詢完整資料
    private func queryWithTrie(
        input: String,
        inputMode: InputMode,
        limit: Int
    ) async throws -> [TaigiWord] {
        // 正規化輸入（包含調符或 POJ 特殊字符時需要轉換）
        let normalizedInput = InputNormalizer.normalize(input, mode: inputMode)

        logger.debug("[TRIE] input='\(input, privacy: .public)' -> normalized='\(normalizedInput, privacy: .public)'")

        guard !normalizedInput.isEmpty else {
            return []
        }

        let trieKey = LexiconConstants.TriePrefix.prefix(for: inputMode) + normalizedInput

        // 1. 完全匹配（確保短詞不被遺漏）
        let exactRowIds = trieService.lookup(trieKey)

        // 2. 前綴搜尋（取較多結果以供後續排序）
        let trieLimit = limit * 6
        let prefixRowIds = trieService.prefixSearch(trieKey, limit: trieLimit)

        // 3. 合併去重
        let allRowIds = Array(Set(exactRowIds + prefixRowIds))

        logger.debug("[TRIE] exact=\(exactRowIds.count) prefix=\(prefixRowIds.count) merged=\(allRowIds.count)")

        guard !allRowIds.isEmpty else {
            return []
        }

        // SQLite 批次查詢
        return try await connectionManager.execute { db in
            try self.queryByIds(
                db: db,
                ids: allRowIds,
                inputMode: inputMode,
                limit: limit
            )
        }
    }

    /// 依 rowid 批次查詢 SQLite
    private func queryByIds(
        db: OpaquePointer,
        ids: [Int],
        inputMode: InputMode,
        limit: Int
    ) throws -> [TaigiWord] {
        guard !ids.isEmpty else { return [] }

        // 讀取詞庫開關設定
        let enabledDicts = EnabledDictionaries.fromSettings()

        let dictCondition = enabledDicts.buildWhereCondition()

        // 分批查詢（避免 SQL 太長）
        let batchSize = 500
        var allResults: [TaigiWord] = []

        // 手動分批處理
        var startIndex = 0
        while startIndex < ids.count {
            let endIndex = min(startIndex + batchSize, ids.count)
            let batch = Array(ids[startIndex..<endIndex])
            startIndex = endIndex
            let placeholders = batch.map { _ in "?" }.joined(separator: ",")

            let sql = """
                SELECT id, tl, hanzi, frequency
                FROM dictionary
                WHERE id IN (\(placeholders))
                \(dictCondition)
                ORDER BY frequency DESC
                LIMIT ?
            """

            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                let errorMsg = String(cString: sqlite3_errmsg(db))
                throw DictionaryError.queryPreparationFailed("Query prep failed: \(errorMsg)")
            }
            defer { sqlite3_finalize(stmt) }

            // 綁定參數
            for (index, id) in batch.enumerated() {
                sqlite3_bind_int(stmt, Int32(index + 1), Int32(id))
            }
            sqlite3_bind_int(stmt, Int32(batch.count + 1), Int32(limit))

            // 提取結果
            while sqlite3_step(stmt) == SQLITE_ROW {
                let id = Int(sqlite3_column_int(stmt, 0))
                let tlRoman = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
                let hanziText = sqlite3_column_text(stmt, 2).map(String.init(cString:))
                let hanzi = hanziText?.isEmpty == false ? hanziText : nil
                let frequency = Int(sqlite3_column_int(stmt, 3))

                // Convert TL -> POJ for display in POJ mode
                let roman = inputMode == .poj ? RomanizationConverter.tlToPOJ(tlRoman) : tlRoman

                allResults.append(TaigiWord(
                    id: id,
                    roman: roman,
                    hanzi: hanzi,
                    lengthScore: frequency
                ))
            }
        }

        // 按 frequency 排序並限制結果數
        return allResults
            .sorted { ($0.lengthScore ?? 0) > ($1.lengthScore ?? 0) }
            .prefix(limit)
            .map { $0 }
    }

    // MARK: - Search With Sources (for dictionary exploration)

    /// Search dictionary and return results with source information.
    /// Searches all sources (no dictionary filter) for exploration purposes.
    func searchWithSources(
        input: String,
        inputMode: InputMode,
        limit: Int = 50
    ) async throws -> [DictionarySearchResult] {
        guard !input.isEmpty else { return [] }

        try await connectionManager.ensureInitialized()

        guard trieService.isReady else {
            logger.error("[SEARCH-SOURCES] Trie not loaded")
            throw DictionaryError.trieNotLoaded
        }

        let normalizedInput = InputNormalizer.normalize(input, mode: inputMode)
        guard !normalizedInput.isEmpty else { return [] }

        let trieKey = LexiconConstants.TriePrefix.prefix(for: inputMode) + normalizedInput

        let exactRowIds = trieService.lookup(trieKey)
        let prefixRowIds = trieService.prefixSearch(trieKey, limit: limit * 6)
        let allRowIds = Array(Set(exactRowIds + prefixRowIds))

        guard !allRowIds.isEmpty else { return [] }

        return try await connectionManager.execute { db in
            try self.queryByIdsWithSources(
                db: db,
                ids: allRowIds,
                inputMode: inputMode,
                limit: limit
            )
        }
    }

    /// Search dictionary by hanzi (漢字) and return results with source information.
    /// Uses direct SQL LIKE query since hanzi is not indexed in the trie.
    func searchByHanzi(
        query: String,
        inputMode: InputMode,
        limit: Int = 50
    ) async throws -> [DictionarySearchResult] {
        guard !query.isEmpty else { return [] }

        logger.debug("[HANZI-SEARCH] query='\(query, privacy: .public)' limit=\(limit)")

        try await connectionManager.ensureInitialized()

        let results = try await connectionManager.execute { db in
            try self.queryByHanziLike(
                db: db,
                query: query,
                inputMode: inputMode,
                limit: limit
            )
        }

        logger.debug("[HANZI-SEARCH] returned \(results.count) results")
        if let first = results.first {
            logger.debug("[HANZI-SEARCH] first: \(first.roman, privacy: .public) / \(first.hanzi ?? "", privacy: .public)")
        }

        return results
    }

    /// Query SQLite with LIKE on hanzi column
    private func queryByHanziLike(
        db: OpaquePointer,
        query: String,
        inputMode: InputMode,
        limit: Int
    ) throws -> [DictionarySearchResult] {
        let enabledDicts = EnabledDictionaries.fromSettings()
        let dictCondition = enabledDicts.buildWhereCondition()

        let sql = """
            SELECT id, tl, hanzi, frequency,
                   kautian, taigitv, itaigi, sitbut, taihoa, taijit,
                   kungge, stti, khpoo, khiin, lkk, dev
            FROM dictionary
            WHERE hanzi LIKE ?
            \(dictCondition)
            ORDER BY frequency DESC
            LIMIT ?
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw DictionaryError.queryPreparationFailed("Hanzi query prep failed: \(errorMsg)")
        }
        defer { sqlite3_finalize(stmt) }

        let likePattern = "%\(query)%"
        logger.debug("[HANZI-SQL] LIKE pattern='\(likePattern, privacy: .public)'")
        sqlite3_bind_text(stmt, 1, (likePattern as NSString).utf8String, -1, nil)
        sqlite3_bind_int(stmt, 2, Int32(limit))

        var results: [DictionarySearchResult] = []

        while sqlite3_step(stmt) == SQLITE_ROW {
            let id = Int(sqlite3_column_int(stmt, 0))
            let tlRoman = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
            let hanziText = sqlite3_column_text(stmt, 2).map(String.init(cString:))
            let hanzi = hanziText?.isEmpty == false ? hanziText : nil
            let frequency = Int(sqlite3_column_int(stmt, 3))

            // Map source boolean columns (columns 4-15)
            var sources: [DictionarySource] = []
            let sourceColumns: [DictionarySource] = [
                .kautian, .taigitv, .itaigi, .sitbut, .taihoa, .taijit,
                .kungge, .stti, .khpoo, .khiin, .lkk, .dev
            ]
            for (offset, source) in sourceColumns.enumerated() {
                if sqlite3_column_int(stmt, Int32(4 + offset)) == 1 {
                    sources.append(source)
                }
            }

            let roman = inputMode == .poj ? RomanizationConverter.tlToPOJ(tlRoman) : tlRoman

            results.append(DictionarySearchResult(
                id: id,
                roman: roman,
                tl: tlRoman,
                hanzi: hanzi,
                frequency: frequency,
                sources: sources
            ))
        }

        return results
    }

    /// Query SQLite with source columns included
    private func queryByIdsWithSources(
        db: OpaquePointer,
        ids: [Int],
        inputMode: InputMode,
        limit: Int
    ) throws -> [DictionarySearchResult] {
        guard !ids.isEmpty else { return [] }

        let batchSize = 500
        var allResults: [DictionarySearchResult] = []

        var startIndex = 0
        while startIndex < ids.count {
            let endIndex = min(startIndex + batchSize, ids.count)
            let batch = Array(ids[startIndex..<endIndex])
            startIndex = endIndex
            let placeholders = batch.map { _ in "?" }.joined(separator: ",")

            let enabledDicts = EnabledDictionaries.fromSettings()
            let dictCondition = enabledDicts.buildWhereCondition()

            let sql = """
                SELECT id, tl, hanzi, frequency,
                       kautian, taigitv, itaigi, sitbut, taihoa, taijit,
                       kungge, stti, khpoo, khiin, lkk, dev
                FROM dictionary
                WHERE id IN (\(placeholders))
                \(dictCondition)
                ORDER BY frequency DESC
                LIMIT ?
            """

            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                let errorMsg = String(cString: sqlite3_errmsg(db))
                throw DictionaryError.queryPreparationFailed("Query prep failed: \(errorMsg)")
            }
            defer { sqlite3_finalize(stmt) }

            for (index, id) in batch.enumerated() {
                sqlite3_bind_int(stmt, Int32(index + 1), Int32(id))
            }
            sqlite3_bind_int(stmt, Int32(batch.count + 1), Int32(limit))

            while sqlite3_step(stmt) == SQLITE_ROW {
                let id = Int(sqlite3_column_int(stmt, 0))
                let tlRoman = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
                let hanziText = sqlite3_column_text(stmt, 2).map(String.init(cString:))
                let hanzi = hanziText?.isEmpty == false ? hanziText : nil
                let frequency = Int(sqlite3_column_int(stmt, 3))

                // Map source boolean columns (columns 4-15)
                var sources: [DictionarySource] = []
                let sourceColumns: [DictionarySource] = [
                    .kautian, .taigitv, .itaigi, .sitbut, .taihoa, .taijit,
                    .kungge, .stti, .khpoo, .khiin, .lkk, .dev
                ]
                for (offset, source) in sourceColumns.enumerated() {
                    if sqlite3_column_int(stmt, Int32(4 + offset)) == 1 {
                        sources.append(source)
                    }
                }

                let roman = inputMode == .poj ? RomanizationConverter.tlToPOJ(tlRoman) : tlRoman

                allResults.append(DictionarySearchResult(
                    id: id,
                    roman: roman,
                    tl: tlRoman,
                    hanzi: hanzi,
                    frequency: frequency,
                    sources: sources
                ))
            }
        }

        return allResults
            .sorted { $0.frequency > $1.frequency }
            .prefix(limit)
            .map { $0 }
    }

    // MARK: - Connection Status

    func isConnected() -> Bool {
        connectionManager.isConnected()
    }
}
