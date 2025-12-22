import Foundation
import OSLog
import SQLite3

/// NextWord 下一詞預測服務
///
/// 使用「相鄰字 Bigram」模型預測下一個字：
/// - 選擇「早安」→ 用「安」查詢 → 預測下一個字
///
/// 資料來源：
/// - dictionary.db (word_association 表): 字典關聯（冷啟動）
/// - user_association.db: 使用者學習（個人化）
final class NextWordService: @unchecked Sendable {

    // MARK: - Constants

    private enum Constants {
        static let defaultLimit = 30

        // 權重設定
        static let userWeight: Double = 50.0
        static let dictWeight: Double = 1.0

        // 時間衰減：半衰期 168 小時（一週）
        static let decayHalfLifeHours: Double = 168.0

        // 使用者關聯上限
        static let maxUserAssociations = 50_000
        static let pruneCheckInterval = 100
        static let pruneBatchSize = 5_000

        // 超時設定
        static let associationTimeoutMs: Int64 = 10_000  // 連續選詞間隔 10 秒
        static let contextTimeoutMs: Int64 = 30_000      // 上下文超時 30 秒
    }

    // MARK: - Prediction Result

    /// NextWord 預測結果
    struct Prediction {
        let hanzi: String       // 預測的下一個字/詞
        let tl: String          // TL 羅馬字
        let poj: String         // POJ 羅馬字
        let score: Double       // 排序分數
    }

    // MARK: - Properties

    static let shared = NextWordService()

    private let dictConnectionManager: SQLiteConnectionManager
    private let userConnectionManager: SQLiteConnectionManager
    private let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "NextWordService"
    )

    private var recordCounter = 0

    // MARK: - Initialization

    init() {
        self.dictConnectionManager = SQLiteConnectionManager(
            databasePath: Self.getDictDatabasePath,
            queueLabel: "com.siansiansu.taigikeyboard.nextword.dict",
            loggerCategory: "NextWordService.Dict"
        )

        self.userConnectionManager = SQLiteConnectionManager(
            databasePath: Self.getUserDatabasePath,
            queueLabel: "com.siansiansu.taigikeyboard.nextword.user",
            loggerCategory: "NextWordService.User"
        )
    }

    // MARK: - Database Paths

    private static func getDictDatabasePath() throws -> String {
        let bundle = Bundle(for: NextWordService.self)
        guard let path = bundle.path(
            forResource: LexiconConstants.Database.fileName,
            ofType: LexiconConstants.Database.fileExtension
        ) else {
            throw DictionaryError.databaseNotFound
        }
        return path
    }

    private static func getUserDatabasePath() throws -> String {
        guard let containerURL = SharedSettings.getSharedContainerURL() else {
            throw DictionaryError.databaseNotFound
        }

        try FileManager.default.createDirectory(
            at: containerURL,
            withIntermediateDirectories: true
        )

        let databaseURL = containerURL.appendingPathComponent("user_association.db")
        return databaseURL.path
    }

    // MARK: - Public API

    /// 預測下一個詞
    ///
    /// 使用混合式 Bigram 模型：
    /// - 字典層級：用最後一字查詢 → 預測單字
    /// - 使用者層級：用完整詞查詢 → 預測完整詞
    ///
    /// - Parameters:
    ///   - word: 當前選中的詞
    ///   - limit: 最大結果數
    /// - Returns: 預測結果列表
    func predict(word: String, limit: Int = Constants.defaultLimit) async -> [Prediction] {
        guard !word.isEmpty else { return [] }

        // Bigram 模型：使用最後一字作為字典查詢 key
        let lastChar = String(word.last!)

        var results: [String: Prediction] = [:]

        // 1. 查詢字典關聯（用最後一字）
        await queryDictAssociations(lastChar: lastChar, limit: limit, results: &results)

        // 2. 查詢使用者關聯（用完整詞）
        await queryUserAssociations(word: word, limit: limit, results: &results)

        // 3. 按分數排序，返回結果
        let sortedResults = results.values
            .sorted { $0.score > $1.score }
            .prefix(limit)
            .map { $0 }

        logger.debug("[PREDICT] '\(word)' -> \(sortedResults.count) results")

        return Array(sortedResults)
    }

    /// 記錄使用者選詞關聯
    ///
    /// - Parameters:
    ///   - prev: 前一個選中的詞
    ///   - nextHanzi: 當前選中的詞（漢字）
    ///   - nextTl: 當前選中的詞（TL）
    ///   - nextPoj: 當前選中的詞（POJ）
    func recordAssociation(
        prev: String,
        nextHanzi: String,
        nextTl: String = "",
        nextPoj: String = ""
    ) async {
        guard !prev.isEmpty, !nextHanzi.isEmpty else { return }

        do {
            try await ensureUserTablesCreated()

            try await userConnectionManager.execute { [weak self] db in
                guard let self else { return }
                try self.insertOrUpdateAssociation(
                    db: db,
                    prev: prev,
                    nextHanzi: nextHanzi,
                    nextTl: nextTl,
                    nextPoj: nextPoj
                )
            }

            logger.debug("[RECORD] '\(prev)' -> '\(nextHanzi)'")

            // 定期檢查是否需要清理
            recordCounter += 1
            if recordCounter >= Constants.pruneCheckInterval {
                recordCounter = 0
                await pruneOldAssociations()
            }
        } catch {
            logger.error("[RECORD] Failed: \(error.localizedDescription)")
        }
    }

    /// 清除所有使用者關聯資料
    func clearAllAssociations() async {
        do {
            try await ensureUserTablesCreated()
            try await userConnectionManager.execute { db in
                var stmt: OpaquePointer?
                if sqlite3_prepare_v2(db, "DELETE FROM user_association", -1, &stmt, nil) == SQLITE_OK {
                    sqlite3_step(stmt)
                    sqlite3_finalize(stmt)
                }
            }
            logger.info("[CLEAR] All user associations cleared")
        } catch {
            logger.error("[CLEAR] Failed: \(error.localizedDescription)")
        }
    }

    /// 刪除使用者關聯資料庫
    static func deleteUserDatabase() throws {
        // 關閉連接
        shared.userConnectionManager.close()
        shared.isUserTablesCreated = false

        // 刪除檔案
        let path = try getUserDatabasePath()
        if FileManager.default.fileExists(atPath: path) {
            try FileManager.default.removeItem(atPath: path)
        }
    }

    /// 取得使用者關聯數量
    func getAssociationCount() async -> Int {
        do {
            try await ensureUserTablesCreated()
            return try await userConnectionManager.execute { db in
                var stmt: OpaquePointer?
                defer { sqlite3_finalize(stmt) }

                guard sqlite3_prepare_v2(db, "SELECT COUNT(*) FROM user_association", -1, &stmt, nil) == SQLITE_OK else {
                    return 0
                }

                if sqlite3_step(stmt) == SQLITE_ROW {
                    return Int(sqlite3_column_int(stmt, 0))
                }
                return 0
            }
        } catch {
            return 0
        }
    }

    /// 使用者關聯資料（用於 Debug Zone 顯示）
    struct AssociationEntry {
        let prevWord: String
        let nextWord: String
        let nextTl: String
        let nextPoj: String
        let count: Int
    }

    /// 取得所有使用者關聯（用於 Debug Zone）
    func getAllAssociations() async -> [AssociationEntry] {
        do {
            try await ensureUserTablesCreated()
            return try await userConnectionManager.execute { db in
                let sql = """
                    SELECT prev_word, next_word, next_tl, next_poj, count
                    FROM user_association
                    ORDER BY count DESC, last_used DESC
                """

                var stmt: OpaquePointer?
                guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                    return []
                }
                defer { sqlite3_finalize(stmt) }

                var results: [AssociationEntry] = []
                while sqlite3_step(stmt) == SQLITE_ROW {
                    let prevWord = sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? ""
                    let nextWord = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
                    let nextTl = sqlite3_column_text(stmt, 2).map(String.init(cString:)) ?? ""
                    let nextPoj = sqlite3_column_text(stmt, 3).map(String.init(cString:)) ?? ""
                    let count = Int(sqlite3_column_int(stmt, 4))

                    results.append(AssociationEntry(
                        prevWord: prevWord,
                        nextWord: nextWord,
                        nextTl: nextTl,
                        nextPoj: nextPoj,
                        count: count
                    ))
                }
                return results
            }
        } catch {
            logger.error("[DEBUG] getAllAssociations failed: \(error.localizedDescription)")
            return []
        }
    }

    // MARK: - Private Methods

    /// 查詢字典關聯
    private func queryDictAssociations(
        lastChar: String,
        limit: Int,
        results: inout [String: Prediction]
    ) async {
        do {
            try await dictConnectionManager.ensureInitialized(flags: SQLITE_OPEN_READONLY)

            let dictResults = try await dictConnectionManager.execute { [weak self] db -> [Prediction] in
                guard let self else { return [] }
                return try self.queryDictAssociationsFromDB(db: db, lastChar: lastChar, limit: limit)
            }

            for prediction in dictResults {
                results[prediction.hanzi] = prediction
            }
        } catch {
            logger.error("[DICT] Query failed: \(error.localizedDescription)")
        }
    }

    private func queryDictAssociationsFromDB(
        db: OpaquePointer,
        lastChar: String,
        limit: Int
    ) throws -> [Prediction] {
        // 建立詞庫過濾條件
        let dictCondition = buildDictWhereCondition()

        let sql = """
            SELECT next_word, next_tl, next_poj, count
            FROM word_association
            WHERE prev_word = ?
            \(dictCondition)
            ORDER BY count DESC
            LIMIT ?
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw DictionaryError.queryPreparationFailed(errorMsg)
        }
        defer { sqlite3_finalize(stmt) }

        let TRANSIENT = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
        sqlite3_bind_text(stmt, 1, lastChar, -1, TRANSIENT)
        sqlite3_bind_int(stmt, 2, Int32(limit * 2))

        var predictions: [Prediction] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            let nextWord = sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? ""
            let nextTl = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
            let nextPoj = sqlite3_column_text(stmt, 2).map(String.init(cString:)) ?? ""
            let count = sqlite3_column_int(stmt, 3)

            predictions.append(Prediction(
                hanzi: nextWord,
                tl: nextTl,
                poj: nextPoj,
                score: Double(count) * Constants.dictWeight
            ))
        }

        return predictions
    }

    /// 查詢使用者關聯
    private func queryUserAssociations(
        word: String,
        limit: Int,
        results: inout [String: Prediction]
    ) async {
        do {
            try await ensureUserTablesCreated()

            let userResults = try await userConnectionManager.execute { [weak self] db -> [(Prediction, Int64)] in
                guard let self else { return [] }
                return try self.queryUserAssociationsFromDB(db: db, word: word, limit: limit)
            }

            for (prediction, lastUsedMs) in userResults {
                let decay = calculateDecay(lastUsedMs: lastUsedMs)
                let userScore = prediction.score * decay

                if let existing = results[prediction.hanzi] {
                    // 合併分數
                    results[prediction.hanzi] = Prediction(
                        hanzi: prediction.hanzi,
                        tl: prediction.tl.isEmpty ? existing.tl : prediction.tl,
                        poj: prediction.poj.isEmpty ? existing.poj : prediction.poj,
                        score: existing.score + userScore
                    )
                } else {
                    results[prediction.hanzi] = Prediction(
                        hanzi: prediction.hanzi,
                        tl: prediction.tl,
                        poj: prediction.poj,
                        score: userScore
                    )
                }
            }
        } catch {
            logger.error("[USER] Query failed: \(error.localizedDescription)")
        }
    }

    private func queryUserAssociationsFromDB(
        db: OpaquePointer,
        word: String,
        limit: Int
    ) throws -> [(Prediction, Int64)] {
        let sql = """
            SELECT next_word, next_tl, next_poj, count,
                   strftime('%s', last_used) * 1000 AS last_used_ms
            FROM user_association
            WHERE prev_word = ?
            ORDER BY count DESC
            LIMIT ?
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            return []
        }
        defer { sqlite3_finalize(stmt) }

        let TRANSIENT = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
        sqlite3_bind_text(stmt, 1, word, -1, TRANSIENT)
        sqlite3_bind_int(stmt, 2, Int32(limit * 2))

        var results: [(Prediction, Int64)] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            let nextWord = sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? ""
            let nextTl = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
            let nextPoj = sqlite3_column_text(stmt, 2).map(String.init(cString:)) ?? ""
            let count = sqlite3_column_int(stmt, 3)
            let lastUsedMs = sqlite3_column_int64(stmt, 4)

            let prediction = Prediction(
                hanzi: nextWord,
                tl: nextTl,
                poj: nextPoj,
                score: Double(count) * Constants.userWeight
            )
            results.append((prediction, lastUsedMs))
        }

        return results
    }

    // MARK: - User Database Management

    private var isUserTablesCreated = false

    private func ensureUserTablesCreated() async throws {
        if isUserTablesCreated { return }

        try await userConnectionManager.ensureInitialized(
            flags: SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE
        )

        try await userConnectionManager.execute { [weak self] db in
            guard let self else { return }
            try self.createUserTables(db: db)
        }

        isUserTablesCreated = true
    }

    private func createUserTables(db: OpaquePointer) throws {
        let createTable = """
            CREATE TABLE IF NOT EXISTS user_association (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prev_word TEXT NOT NULL,
                next_word TEXT NOT NULL,
                next_tl TEXT,
                next_poj TEXT,
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(prev_word, next_word)
            );
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, createTable, -1, &stmt, nil) == SQLITE_OK else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw DictionaryError.queryPreparationFailed(errorMsg)
        }

        guard sqlite3_step(stmt) == SQLITE_DONE else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            sqlite3_finalize(stmt)
            throw DictionaryError.queryExecutionFailed(errorMsg)
        }
        sqlite3_finalize(stmt)

        // 建立索引
        let indexSQL = "CREATE INDEX IF NOT EXISTS idx_user_prev_word ON user_association(prev_word);"
        var indexStmt: OpaquePointer?
        if sqlite3_prepare_v2(db, indexSQL, -1, &indexStmt, nil) == SQLITE_OK {
            sqlite3_step(indexStmt)
            sqlite3_finalize(indexStmt)
        }
    }

    private func insertOrUpdateAssociation(
        db: OpaquePointer,
        prev: String,
        nextHanzi: String,
        nextTl: String,
        nextPoj: String
    ) throws {
        let sql = """
            INSERT INTO user_association (prev_word, next_word, next_tl, next_poj, count, last_used)
            VALUES (?, ?, ?, ?, 1, CURRENT_TIMESTAMP)
            ON CONFLICT(prev_word, next_word) DO UPDATE SET
                count = count + 1,
                next_tl = CASE WHEN ? != '' THEN ? ELSE next_tl END,
                next_poj = CASE WHEN ? != '' THEN ? ELSE next_poj END,
                last_used = CURRENT_TIMESTAMP
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw DictionaryError.queryPreparationFailed(errorMsg)
        }
        defer { sqlite3_finalize(stmt) }

        let TRANSIENT = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
        sqlite3_bind_text(stmt, 1, prev, -1, TRANSIENT)
        sqlite3_bind_text(stmt, 2, nextHanzi, -1, TRANSIENT)
        sqlite3_bind_text(stmt, 3, nextTl, -1, TRANSIENT)
        sqlite3_bind_text(stmt, 4, nextPoj, -1, TRANSIENT)
        sqlite3_bind_text(stmt, 5, nextTl, -1, TRANSIENT)
        sqlite3_bind_text(stmt, 6, nextTl, -1, TRANSIENT)
        sqlite3_bind_text(stmt, 7, nextPoj, -1, TRANSIENT)
        sqlite3_bind_text(stmt, 8, nextPoj, -1, TRANSIENT)

        if sqlite3_step(stmt) != SQLITE_DONE {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw DictionaryError.queryExecutionFailed(errorMsg)
        }
    }

    // MARK: - Time Decay

    /// 計算時間衰減因子
    ///
    /// 使用指數衰減公式（參考 RIME）：
    /// decay = exp(-ageHours / halfLifeHours * ln(2))
    private func calculateDecay(lastUsedMs: Int64) -> Double {
        let now = Int64(Date().timeIntervalSince1970 * 1000)
        let ageHours = Double(now - lastUsedMs) / 3_600_000.0
        // ln(2) ≈ 0.693
        return exp(-ageHours / Constants.decayHalfLifeHours * 0.693)
    }

    // MARK: - Pruning

    /// 清理舊的使用者關聯（當超過上限時）
    private func pruneOldAssociations() async {
        do {
            let currentCount = await getAssociationCount()

            guard currentCount > Constants.maxUserAssociations else {
                logger.debug("[PRUNE] No pruning needed: \(currentCount) <= \(Constants.maxUserAssociations)")
                return
            }

            let deleteCount = min(
                Constants.pruneBatchSize,
                currentCount - Constants.maxUserAssociations + Constants.pruneBatchSize
            )

            try await userConnectionManager.execute { db in
                let sql = """
                    DELETE FROM user_association
                    WHERE id IN (
                        SELECT id FROM user_association
                        ORDER BY count ASC, last_used ASC
                        LIMIT ?
                    )
                """

                var stmt: OpaquePointer?
                guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return }
                defer { sqlite3_finalize(stmt) }

                sqlite3_bind_int(stmt, 1, Int32(deleteCount))
                sqlite3_step(stmt)
            }

            logger.info("[PRUNE] Deleted \(deleteCount) associations (was \(currentCount))")
        } catch {
            logger.error("[PRUNE] Failed: \(error.localizedDescription)")
        }
    }

    // MARK: - Dictionary Filter

    /// 建立詞庫過濾 WHERE 條件
    private func buildDictWhereCondition() -> String {
        let settings = SharedSettings.shared

        var conditions: [String] = []
        if settings.moeDictEnabled { conditions.append("kautian = 1") }
        if settings.newwordDictEnabled { conditions.append("taigitv = 1") }
        if settings.iTaigiDictEnabled { conditions.append("itaigi = 1") }
        if settings.taiwanPlantDictEnabled { conditions.append("sitbut = 1") }
        if settings.taiHuaDictEnabled { conditions.append("taihoa = 1") }
        if settings.taiwanJapanDictEnabled { conditions.append("taijit = 1") }
        if settings.kunggeDictEnabled { conditions.append("kungge = 1") }

        // 全部開啟時不加過濾條件
        let allEnabled = settings.moeDictEnabled && settings.newwordDictEnabled &&
            settings.iTaigiDictEnabled && settings.taiwanPlantDictEnabled &&
            settings.taiHuaDictEnabled && settings.taiwanJapanDictEnabled &&
            settings.kunggeDictEnabled

        if allEnabled {
            return ""
        }

        // 全部關閉時返回不可能的條件
        if conditions.isEmpty {
            return "AND 0"
        }

        return "AND (\(conditions.joined(separator: " OR ")))"
    }
}
