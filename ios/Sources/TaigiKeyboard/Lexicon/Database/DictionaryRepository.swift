import Foundation
import OSLog
import SQLite3

/// 詞典資料庫 Repository
/// 負責詞典資料的查詢與存取
final class DictionaryRepository: @unchecked Sendable {

    // MARK: - Column Definition

    private enum Column: String {
        case pojRoman = "poj"
        case tlRoman = "tl"
    }

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
                sitbut: settings.taiwanPlantDictEnabled
            )
        }

        /// 是否全部關閉
        var allDisabled: Bool {
            !kautian && !taigitv && !kungge && !itaigi && !taijit && !taihoa && !sitbut
        }

        /// 是否全部開啟
        var allEnabled: Bool {
            kautian && taigitv && kungge && itaigi && taijit && taihoa && sitbut
        }

        /// 建構 SQL WHERE 條件（使用 OR 邏輯）
        func buildWhereCondition() -> String {
            // 全部開啟時不加過濾條件
            if allEnabled { return "" }

            var conditions: [String] = []
            if kautian { conditions.append("kautian = 1") }
            if taigitv { conditions.append("taigitv = 1") }
            if kungge { conditions.append("kungge = 1") }
            if itaigi { conditions.append("itaigi = 1") }
            if taijit { conditions.append("taijit = 1") }
            if taihoa { conditions.append("taihoa = 1") }
            if sitbut { conditions.append("sitbut = 1") }

            guard !conditions.isEmpty else { return "" }
            return "AND (" + conditions.joined(separator: " OR ") + ")"
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
        // 正規化輸入
        let normalizedInput = InputNormalizer.normalize(input, mode: inputMode)

        logger.debug("[TRIE] input='\(input, privacy: .public)' -> normalized='\(normalizedInput, privacy: .public)'")

        guard !normalizedInput.isEmpty else {
            return []
        }

        // 加上 Trie 前綴（poj: 或 tl:）
        let triePrefix = inputMode == .tl ? "tl:" : "poj:"
        let trieKey = triePrefix + normalizedInput

        logger.debug("[TRIE] trieKey='\(trieKey, privacy: .public)'")

        // 1. 完全匹配（確保短詞不被遺漏）
        let exactRowIds = trieService.lookup(trieKey)

        logger.debug("[TRIE] exactRowIds=\(exactRowIds.count)")

        // 2. 前綴搜尋（取較多結果以供後續排序）
        let trieLimit = limit * 3
        let prefixRowIds = trieService.prefixSearch(trieKey, limit: trieLimit)

        logger.debug("[TRIE] prefixRowIds=\(prefixRowIds.count)")

        // 3. 合併去重
        let allRowIds = Array(Set(exactRowIds + prefixRowIds))

        logger.debug("[TRIE] allRowIds=\(allRowIds.count)")

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

        // 全部關閉時不顯示任何結果
        if enabledDicts.allDisabled { return [] }

        let romanColumn = inputMode == .poj ? Column.pojRoman.rawValue : Column.tlRoman.rawValue
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
                SELECT id, \(romanColumn), hanzi, frequency
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
                let roman = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
                let hanziText = sqlite3_column_text(stmt, 2).map(String.init(cString:))
                let hanzi = hanziText?.isEmpty == false ? hanziText : nil
                let frequency = Int(sqlite3_column_int(stmt, 3))

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

    // MARK: - Connection Status

    func isConnected() -> Bool {
        connectionManager.isConnected()
    }
}
