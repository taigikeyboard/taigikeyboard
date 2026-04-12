import Foundation
import SQLite3

/// NextWord 下一詞預測服務
///
/// 使用「相鄰字 Bigram」模型預測下一個字：
/// - 選擇「早安」→ 用「安」查詢 → 預測下一個字
///
/// 資料來源：
/// - association.bin (binary mmap): 字典關聯（冷啟動）
/// - user_association.db: 使用者學習（個人化）
final class NextWordService: @unchecked Sendable {
    // MARK: - Constants

    private enum Constants {
        static let defaultLimit = 30

        // 權重設定
        static let userWeight: Double = 50.0
        static let dictWeight: Double = 1.0

        /// 時間衰減：半衰期 168 小時（一週）
        static let decayHalfLifeHours: Double = 168.0

        // Memory strength: ensures user entries rank above dict entries
        static let learningBonus: Double = 300.0
        static let highUsageDecayFloor: Double = 0.95 // count >= 3: near-permanent
        static let lowUsageDecayFloor: Double = 0.3 // count < 3: prevents full decay
        static let highUsageThreshold: Int = 3

        // 使用者關聯上限
        static let maxUserAssociations = 50000
        static let pruneCheckInterval = 100
        static let pruneBatchSize = 5000

        /// SQLite SQLITE_TRANSIENT destructor type for bind calls
        static let sqliteTransient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
    }

    // MARK: - Prediction Result

    /// NextWord 預測結果
    struct Prediction {
        let hanzi: String // 預測的下一個字/詞
        let tl: String // TL 羅馬字
        let score: Double // 排序分數
    }

    // MARK: - Properties

    static let shared = NextWordService()

    private let associationReader: AssociationBinaryReader?
    private let userConnectionManager: SQLiteConnectionManager
    private let logger = DebugLogger(category: "NextWordService")

    private var recordCounter = 0

    // MARK: - Initialization

    init(associationReader: AssociationBinaryReader? = nil) {
        self.associationReader = associationReader ?? AssociationBinaryReader()

        userConnectionManager = SQLiteConnectionManager(
            databasePath: Self.getUserDatabasePath,
            queueLabel: "com.siansiansu.taigikeyboard.nextword.user",
            loggerCategory: "NextWordService.User",
        )
    }

    // MARK: - Database Paths

    private static func getUserDatabasePath() throws -> String {
        guard let containerURL = SharedSettings.sharedContainerURL else {
            throw DictionaryError.databaseNotFound
        }

        try FileManager.default.createDirectory(
            at: containerURL,
            withIntermediateDirectories: true,
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
    func predict(word: String, roman: String = "", limit: Int = Constants.defaultLimit) async -> [Prediction] {
        guard !word.isEmpty else { return [] }

        // Bigram 模型：使用最後一字作為字典查詢 key
        let lastChar = String(word.last!)

        logger.debug("[PREDICT][ENTRY] word='\(word)' lastChar='\(lastChar)'")

        var results: [String: Prediction] = [:]

        // 1. 查詢字典關聯（用最後一字）
        await queryDictAssociations(lastChar: lastChar, limit: limit, results: &results)
        let dictCount = results.count
        logger.debug("[PREDICT][DICT] dictResults.count=\(dictCount) for lastChar='\(lastChar)'")

        // 2. 查詢使用者關聯（用完整詞 + 羅馬字 context）
        await queryUserAssociations(word: word, roman: roman, limit: limit, results: &results)
        let totalCount = results.count
        logger.debug("[PREDICT][USER] after user merge: totalResults.count=\(totalCount) (user added \(totalCount - dictCount) new entries) for word='\(word)'")

        // 3. 按分數排序，返回結果
        let sortedResults = results.values
            .sorted { $0.score > $1.score }
            .prefix(limit)
            .map(\.self)

        logger.debug("[PREDICT] '\(word)' -> \(sortedResults.count) results")

        return Array(sortedResults)
    }

    /// 記錄使用者選詞關聯
    ///
    /// - Parameters:
    ///   - prev: 前一個選中的詞（漢字）
    ///   - prevTl: 前一個選中的詞（TL）
    ///   - nextHanzi: 當前選中的詞（漢字）
    ///   - nextTl: 當前選中的詞（TL）
    func recordAssociation(
        prev: String,
        prevTl: String = "",
        nextHanzi: String,
        nextTl: String = "",
    ) async {
        guard !prev.isEmpty, !nextHanzi.isEmpty else { return }

        do {
            try await ensureUserTablesCreated()

            try await userConnectionManager.execute { [weak self] db in
                guard let self else { return }
                try insertOrUpdateAssociation(
                    db: db,
                    prev: prev,
                    prevTl: prevTl,
                    nextHanzi: nextHanzi,
                    nextTl: nextTl,
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

    /// Delete a single user association entry
    func deleteAssociation(_ entry: AssociationEntry) async {
        do {
            try await ensureUserTablesCreated()
            try await userConnectionManager.execute { db in
                let sql = "DELETE FROM user_association WHERE prev_word = ? AND prev_tl = ? AND next_word = ? AND next_tl = ?"
                var stmt: OpaquePointer?
                defer { sqlite3_finalize(stmt) }
                guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return }
                sqlite3_bind_text(stmt, 1, entry.prevWord, -1, Constants.sqliteTransient)
                sqlite3_bind_text(stmt, 2, entry.prevTl, -1, Constants.sqliteTransient)
                sqlite3_bind_text(stmt, 3, entry.nextWord, -1, Constants.sqliteTransient)
                sqlite3_bind_text(stmt, 4, entry.nextTl, -1, Constants.sqliteTransient)
                sqlite3_step(stmt)
            }
        } catch {
            logger.error("[DELETE] Failed to delete association: \(error.localizedDescription)")
        }
    }

    /// Import association entries with merge strategy: keep higher count
    func batchImportAssociations(
        entries: [(prevWord: String, prevTl: String, nextWord: String, nextTl: String, count: Int)],
    ) async throws -> Int {
        try await ensureUserTablesCreated()
        return try await userConnectionManager.execute { db in
            let sql = """
                INSERT INTO user_association (prev_word, prev_tl, next_word, next_tl, count, last_used)
                VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                ON CONFLICT(prev_word, next_word, next_tl) DO UPDATE SET
                    prev_tl = excluded.prev_tl,
                    count = MAX(count, excluded.count),
                    last_used = CURRENT_TIMESTAMP;
            """

            if sqlite3_exec(db, "BEGIN TRANSACTION;", nil, nil, nil) != SQLITE_OK {
                return 0
            }

            // Prepare statement once and reuse for all entries
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                sqlite3_exec(db, "ROLLBACK;", nil, nil, nil)
                return 0
            }
            defer { sqlite3_finalize(stmt) }

            var imported = 0
            for entry in entries {
                sqlite3_reset(stmt)
                sqlite3_clear_bindings(stmt)

                sqlite3_bind_text(stmt, 1, entry.prevWord, -1, Constants.sqliteTransient)
                sqlite3_bind_text(stmt, 2, entry.prevTl, -1, Constants.sqliteTransient)
                sqlite3_bind_text(stmt, 3, entry.nextWord, -1, Constants.sqliteTransient)
                sqlite3_bind_text(stmt, 4, entry.nextTl, -1, Constants.sqliteTransient)
                sqlite3_bind_int(stmt, 5, Int32(entry.count))

                if sqlite3_step(stmt) == SQLITE_DONE {
                    imported += 1
                }
            }

            sqlite3_exec(db, "COMMIT;", nil, nil, nil)
            return imported
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
    func associationCount() async -> Int {
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

    /// 使用者關聯資料
    struct AssociationEntry {
        let prevWord: String
        let prevTl: String
        let nextWord: String
        let nextTl: String
        let count: Int
    }

    /// 取得所有使用者關聯
    func allAssociations() async -> [AssociationEntry] {
        do {
            try await ensureUserTablesCreated()
            return try await userConnectionManager.execute { db in
                let sql = """
                    SELECT prev_word, prev_tl, next_word, next_tl, count
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
                    let prevTl = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
                    let nextWord = sqlite3_column_text(stmt, 2).map(String.init(cString:)) ?? ""
                    let nextTl = sqlite3_column_text(stmt, 3).map(String.init(cString:)) ?? ""
                    let count = Int(sqlite3_column_int(stmt, 4))

                    results.append(AssociationEntry(
                        prevWord: prevWord,
                        prevTl: prevTl,
                        nextWord: nextWord,
                        nextTl: nextTl,
                        count: count,
                    ))
                }
                return results
            }
        } catch {
            logger.error("[USER] allAssociations failed: \(error.localizedDescription)")
            return []
        }
    }

    // MARK: - Private Methods

    /// 查詢字典關聯（從 association.bin binary reader）
    private func queryDictAssociations(
        lastChar: String,
        limit: Int,
        results: inout [String: Prediction],
    ) async {
        guard let reader = associationReader else {
            logger.warning("[DICT] Association binary reader not available")
            return
        }

        // Over-fetch 2x to account for deduplication when merging dict + user results
        let entries = reader.lookup(prevWord: lastChar, limit: limit * 2)

        // Apply dictionary source filter
        let enabledDicts = EnabledDictionaries.fromSettings()
        let enabledMask = enabledDicts.associationBitmask()
        let allEnabled = enabledDicts.allAssociationSourcesEnabled

        for entry in entries {
            guard AssociationBinaryReader.passesFilter(
                entryBitmask: entry.bitmask,
                enabledMask: enabledMask,
                allEnabled: allEnabled,
            ) else { continue }

            let prediction = Prediction(
                hanzi: entry.nextWord,
                tl: entry.nextTl,
                score: Double(entry.count) * Constants.dictWeight,
            )
            let key = "\(prediction.hanzi)\t\(prediction.tl)"
            results[key] = prediction
        }
    }

    /// Raw user association row from DB (hanzi, tl, count, lastUsedMs)
    private typealias UserAssociationRow = (hanzi: String, tl: String, count: Int, lastUsedMs: Int64)

    /// 查詢使用者關聯
    private func queryUserAssociations(
        word: String,
        roman: String = "",
        limit: Int,
        results: inout [String: Prediction],
    ) async {
        do {
            try await ensureUserTablesCreated()

            let userResults = try await userConnectionManager.execute { [weak self] db -> [UserAssociationRow] in
                guard let self else { return [] }
                return try queryUserAssociationsFromDB(db: db, word: word, roman: roman, limit: limit)
            }

            for row in userResults {
                let userScore = calculateUserScore(count: row.count, lastUsedMs: row.lastUsedMs)
                let key = "\(row.hanzi)\t\(row.tl)"

                if let existing = results[key] {
                    results[key] = Prediction(
                        hanzi: row.hanzi,
                        tl: row.tl.isEmpty ? existing.tl : row.tl,
                        score: existing.score + userScore,
                    )
                } else {
                    results[key] = Prediction(
                        hanzi: row.hanzi,
                        tl: row.tl,
                        score: userScore,
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
        roman: String = "",
        limit: Int,
    ) throws -> [UserAssociationRow] {
        let sql = """
            SELECT next_word, next_tl, count,
                   strftime('%s', last_used) * 1000 AS last_used_ms
            FROM user_association
            WHERE prev_word = ? AND (prev_tl = ? OR prev_tl = '')
            ORDER BY count DESC
            LIMIT ?
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            return []
        }
        defer { sqlite3_finalize(stmt) }

        sqlite3_bind_text(stmt, 1, word, -1, Constants.sqliteTransient)
        sqlite3_bind_text(stmt, 2, roman, -1, Constants.sqliteTransient)
        // Over-fetch 2x to account for deduplication when merging dict + user results
        sqlite3_bind_int(stmt, 3, Int32(limit * 2))

        var results: [UserAssociationRow] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            let nextWord = sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? ""
            let nextTl = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
            let count = Int(sqlite3_column_int(stmt, 2))
            let lastUsedMs = sqlite3_column_int64(stmt, 3)

            results.append((hanzi: nextWord, tl: nextTl, count: count, lastUsedMs: lastUsedMs))
        }

        return results
    }

    // MARK: - User Database Management

    private var isUserTablesCreated = false

    private func ensureUserTablesCreated() async throws {
        if isUserTablesCreated { return }

        try await userConnectionManager.ensureInitialized(
            flags: SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
        )

        try await userConnectionManager.execute { [weak self] db in
            guard let self else { return }
            try createUserTables(db: db)
        }

        isUserTablesCreated = true
    }

    private func createUserTables(db: OpaquePointer) throws {
        // Migrate: if old table exists with UNIQUE(prev_word, next_word), recreate it
        try migrateUserTables(db: db)

        let createTable = """
            CREATE TABLE IF NOT EXISTS user_association (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prev_word TEXT NOT NULL,
                prev_tl TEXT DEFAULT '',
                next_word TEXT NOT NULL,
                next_tl TEXT DEFAULT '',
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(prev_word, next_word, next_tl)
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
        for indexSQL in [
            "CREATE INDEX IF NOT EXISTS idx_user_prev_word ON user_association(prev_word);",
            "CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl ON user_association(prev_word, prev_tl);",
        ] {
            var indexStmt: OpaquePointer?
            if sqlite3_prepare_v2(db, indexSQL, -1, &indexStmt, nil) == SQLITE_OK {
                sqlite3_step(indexStmt)
                sqlite3_finalize(indexStmt)
            }
        }
    }

    /// Schema version for user_association.db (mirrors Android's DATABASE_VERSION)
    private static let userSchemaVersion = 4

    /// Migrate user_association.db to current schema version using PRAGMA user_version.
    /// v3: UNIQUE(prev_word, next_word) → UNIQUE(prev_word, next_word, next_tl)
    /// v4: Add prev_tl column (ALTER TABLE, preserves data)
    private func migrateUserTables(db: OpaquePointer) throws {
        // Read current schema version
        var versionStmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, "PRAGMA user_version", -1, &versionStmt, nil) == SQLITE_OK else { return }
        let currentVersion = sqlite3_step(versionStmt) == SQLITE_ROW
            ? Int(sqlite3_column_int(versionStmt, 0))
            : 0
        sqlite3_finalize(versionStmt)

        guard currentVersion < Self.userSchemaVersion else { return }

        logger.info("[MIGRATE] user_association.db v\(currentVersion) -> v\(Self.userSchemaVersion)")

        if currentVersion < 3 {
            // v0/v1/v2 → v3: DROP + CREATE (old schema incompatible)
            var dropStmt: OpaquePointer?
            if sqlite3_prepare_v2(db, "DROP TABLE IF EXISTS user_association", -1, &dropStmt, nil) == SQLITE_OK {
                sqlite3_step(dropStmt)
                sqlite3_finalize(dropStmt)
            }

            var dropIdxStmt: OpaquePointer?
            if sqlite3_prepare_v2(db, "DROP INDEX IF EXISTS idx_user_prev_word", -1, &dropIdxStmt, nil) == SQLITE_OK {
                sqlite3_step(dropIdxStmt)
                sqlite3_finalize(dropIdxStmt)
            }
        }

        if currentVersion >= 3, currentVersion < 4 {
            // v3 → v4: Add prev_tl column (preserves existing data)
            var alterStmt: OpaquePointer?
            if sqlite3_prepare_v2(db, "ALTER TABLE user_association ADD COLUMN prev_tl TEXT DEFAULT ''", -1, &alterStmt, nil) == SQLITE_OK {
                sqlite3_step(alterStmt)
                sqlite3_finalize(alterStmt)
            }

            var indexStmt: OpaquePointer?
            if sqlite3_prepare_v2(db, "CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl ON user_association(prev_word, prev_tl)", -1, &indexStmt, nil) == SQLITE_OK {
                sqlite3_step(indexStmt)
                sqlite3_finalize(indexStmt)
            }
        }

        // Stamp new version
        var stampStmt: OpaquePointer?
        if sqlite3_prepare_v2(db, "PRAGMA user_version = \(Self.userSchemaVersion)", -1, &stampStmt, nil) == SQLITE_OK {
            sqlite3_step(stampStmt)
            sqlite3_finalize(stampStmt)
        }
    }

    private func insertOrUpdateAssociation(
        db: OpaquePointer,
        prev: String,
        prevTl: String,
        nextHanzi: String,
        nextTl: String,
    ) throws {
        let sql = """
            INSERT INTO user_association (prev_word, prev_tl, next_word, next_tl, count, last_used)
            VALUES (?, ?, ?, ?, 1, CURRENT_TIMESTAMP)
            ON CONFLICT(prev_word, next_word, next_tl) DO UPDATE SET
                prev_tl = excluded.prev_tl,
                count = count + 1,
                last_used = CURRENT_TIMESTAMP
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw DictionaryError.queryPreparationFailed(errorMsg)
        }
        defer { sqlite3_finalize(stmt) }

        sqlite3_bind_text(stmt, 1, prev, -1, Constants.sqliteTransient)
        sqlite3_bind_text(stmt, 2, prevTl, -1, Constants.sqliteTransient)
        sqlite3_bind_text(stmt, 3, nextHanzi, -1, Constants.sqliteTransient)
        sqlite3_bind_text(stmt, 4, nextTl, -1, Constants.sqliteTransient)

        if sqlite3_step(stmt) != SQLITE_DONE {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw DictionaryError.queryExecutionFailed(errorMsg)
        }
    }

    // MARK: - Scoring

    /// Calculate user-layer score with decay floor + learning bonus
    ///
    /// Ensures user entries always rank above dict entries (max ~300).
    /// High-usage entries (count >= 3) get near-permanent retention.
    private func calculateUserScore(count: Int, lastUsedMs: Int64) -> Double {
        let decay = calculateDecay(lastUsedMs: lastUsedMs)
        let rawScore = Double(count) * Constants.userWeight
        let decayFloor = count >= Constants.highUsageThreshold
            ? Constants.highUsageDecayFloor
            : Constants.lowUsageDecayFloor
        let effectiveDecay = max(decayFloor, decay)
        return rawScore * effectiveDecay + Constants.learningBonus
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
            let currentCount = await associationCount()

            guard currentCount > Constants.maxUserAssociations else {
                logger.debug("[PRUNE] No pruning needed: \(currentCount) <= \(Constants.maxUserAssociations)")
                return
            }

            let deleteCount = min(
                Constants.pruneBatchSize,
                currentCount - Constants.maxUserAssociations + Constants.pruneBatchSize,
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
}
