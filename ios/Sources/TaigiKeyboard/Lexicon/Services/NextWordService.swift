import Foundation
import SQLite3

/// File-local helper to reduce `sqlite3_bind_text(_, _, _, -1, TRANSIENT)` boilerplate.
private extension OpaquePointer? {
    func bindText(_ index: Int32, _ value: String) {
        sqlite3_bind_text(self, index, value, -1, SQLiteConnectionManager.sqliteTransient)
    }
}

/// NextWord 下一詞預測服務
///
/// 使用「相鄰字 Bigram」模型預測下一個字：
/// - 選擇「早安」→ 用「安」查詢 → 預測下一個字
///
/// 資料來源：
/// - `association.bin` (binary mmap): 字典關聯（冷啟動）
/// - `user_association.db`: 使用者學習（個人化）
///
/// 檔案內部分成四個段落：公開 API、預測流程（字典端 / 使用者端）、
/// user DB schema + 遷移、評分 + 衰減 + 清理。各段以 `// MARK:` 區隔。
final class NextWordService: @unchecked Sendable {
    // MARK: - Constants

    ///
    /// CROSS-PLATFORM INVARIANT — the scoring constants below
    /// (userWeight, dictWeight, decayHalfLifeHours, learningBonus,
    /// *DecayFloor, *Threshold) MUST mirror Android NextWordService.kt.
    /// Drift causes silent ranking divergence between platforms.
    ///
    private enum Constants {
        static let defaultLimit = 30

        // Source weights
        static let userWeight: Double = 50.0
        static let dictWeight: Double = 1.0

        /// Time decay: half-life 168 hours (1 week)
        static let decayHalfLifeHours: Double = 168.0

        // Memory strength: ensures user entries rank above dict entries
        static let learningBonus: Double = 300.0
        static let highUsageDecayFloor: Double = 0.95 // count >= 3: near-permanent
        static let lowUsageDecayFloor: Double = 0.3 // count < 3: prevents full decay
        static let highUsageThreshold: Int = 3

        // User-association capacity
        static let maxUserAssociations = 50000
        static let pruneCheckInterval = 100
        static let pruneBatchSize = 5000
    }

    /// Schema version for `user_association.db` (mirrors Android's DATABASE_VERSION).
    private static let userSchemaVersion = 4

    // MARK: - Public Types

    /// NextWord 預測結果
    struct Prediction {
        let hanzi: String // 預測的下一個字/詞
        let tl: String // TL 羅馬字
        let score: Double // 排序分數
    }

    /// 使用者關聯資料。UI (DictionaryTab AssociationDataView) 依賴此公開型別。
    struct AssociationEntry {
        let prevWord: String
        let prevTl: String
        let nextWord: String
        let nextTl: String
        let count: Int
    }

    // MARK: - Properties

    static let shared = NextWordService()

    private let associationReader: AssociationBinaryReader?
    private let userConnectionManager: SQLiteConnectionManager
    private let logger = DebugLogger(category: "NextWordService")

    /// Lock protecting mutable state (`_recordCounter`, `_isUserTablesCreated`).
    private let stateLock = NSLock()
    private var _recordCounter = 0
    private var _isUserTablesCreated = false

    // MARK: - Initialization

    init(associationReader: AssociationBinaryReader? = nil) {
        self.associationReader = associationReader ?? AssociationBinaryReader()

        userConnectionManager = SQLiteConnectionManager(
            databasePath: Self.getUserDatabasePath,
            queueLabel: "com.siansiansu.taigikeyboard.nextword.user",
            loggerCategory: "NextWordService.User",
        )
    }

    // MARK: - Public API: Prediction

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
        guard let last = word.last else { return [] }
        let lastChar = String(last)

        logger.debug("[PREDICT][ENTRY] word='\(word)' lastChar='\(lastChar)'")

        var results: [String: Prediction] = [:]

        await queryDictAssociations(lastChar: lastChar, limit: limit, results: &results)
        let dictCount = results.count
        logger.debug("[PREDICT][DICT] dictResults.count=\(dictCount) for lastChar='\(lastChar)'")

        await queryUserAssociations(word: word, roman: roman, limit: limit, results: &results)
        let totalCount = results.count
        logger.debug("[PREDICT][USER] after user merge: totalResults.count=\(totalCount) (user added \(totalCount - dictCount) new entries) for word='\(word)'")

        let sortedResults = Array(results.values
            .sorted { $0.score > $1.score }
            .prefix(limit))
        logger.debug("[PREDICT] '\(word)' -> \(sortedResults.count) results")
        return sortedResults
    }

    // MARK: - Public API: Recording

    /// 記錄使用者選詞關聯
    func recordAssociation(
        prev: String,
        prevTl: String = "",
        nextHanzi: String,
        nextTl: String = "",
    ) async {
        guard !prev.isEmpty, !nextHanzi.isEmpty else { return }

        do {
            try await ensureUserTablesCreated()
            try await userConnectionManager.execute { db in
                try Self.insertOrUpdateAssociation(
                    db: db, prev: prev, prevTl: prevTl,
                    nextHanzi: nextHanzi, nextTl: nextTl,
                )
            }
            logger.debug("[RECORD] '\(prev)' -> '\(nextHanzi)'")

            // 定期檢查是否需要清理
            let shouldPrune: Bool = stateLock.withLock {
                _recordCounter += 1
                if _recordCounter >= Constants.pruneCheckInterval {
                    _recordCounter = 0
                    return true
                }
                return false
            }
            if shouldPrune {
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

    /// Delete a single user association entry.
    func deleteAssociation(_ entry: AssociationEntry) async {
        do {
            try await ensureUserTablesCreated()
            try await userConnectionManager.execute { db in
                let sql = "DELETE FROM user_association WHERE prev_word = ? AND prev_tl = ? AND next_word = ? AND next_tl = ?"
                var stmt: OpaquePointer?
                defer { sqlite3_finalize(stmt) }
                guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return }
                stmt.bindText(1, entry.prevWord)
                stmt.bindText(2, entry.prevTl)
                stmt.bindText(3, entry.nextWord)
                stmt.bindText(4, entry.nextTl)
                sqlite3_step(stmt)
            }
        } catch {
            logger.error("[DELETE] Failed to delete association: \(error.localizedDescription)")
        }
    }

    /// Import association entries with merge-by-max strategy: keep the higher count.
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

            if sqlite3_exec(db, "BEGIN TRANSACTION;", nil, nil, nil) != SQLITE_OK { return 0 }

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

                stmt.bindText(1, entry.prevWord)
                stmt.bindText(2, entry.prevTl)
                stmt.bindText(3, entry.nextWord)
                stmt.bindText(4, entry.nextTl)
                sqlite3_bind_int(stmt, 5, Int32(entry.count))

                if sqlite3_step(stmt) == SQLITE_DONE {
                    imported += 1
                }
            }

            sqlite3_exec(db, "COMMIT;", nil, nil, nil)
            return imported
        }
    }

    /// 取得使用者關聯數量
    func associationCount() async -> Int {
        do {
            try await ensureUserTablesCreated()
            return try await userConnectionManager.execute { db in
                Self.countRows(db: db)
            }
        } catch {
            return 0
        }
    }

    /// 取得所有使用者關聯（for backup / UI listing）
    func allAssociations() async -> [AssociationEntry] {
        do {
            try await ensureUserTablesCreated()
            return try await userConnectionManager.execute { db in
                Self.fetchAllAssociations(db: db)
            }
        } catch {
            logger.error("[USER] allAssociations failed: \(error.localizedDescription)")
            return []
        }
    }

    // MARK: - Lifecycle

    /// 刪除使用者關聯資料庫
    static func deleteUserDatabase() throws {
        shared.userConnectionManager.close()
        shared.stateLock.withLock { shared._isUserTablesCreated = false }

        let path = try getUserDatabasePath()
        if FileManager.default.fileExists(atPath: path) {
            try FileManager.default.removeItem(atPath: path)
        }
    }

    // MARK: - Prediction Pipeline

    /// Dict-side prediction — reads from the shared `association.bin` mmap,
    /// applies the user's enabled-dictionary bitmask filter.
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
        let enabledDicts = EnabledDictionaries.fromSettings()

        for entry in entries {
            guard AssociationBinaryReader.passesFilter(
                entryBitmask: entry.bitmask,
                enabledDicts: enabledDicts,
            ) else { continue }

            let prediction = Prediction(
                hanzi: entry.nextWord,
                tl: entry.nextTl,
                score: Double(entry.count) * Constants.dictWeight,
            )
            results["\(prediction.hanzi)\t\(prediction.tl)"] = prediction
        }
    }

    /// Raw user-association row from DB (hanzi, tl, count, lastUsedMs).
    private typealias UserAssociationRow = (hanzi: String, tl: String, count: Int, lastUsedMs: Int64)

    /// User-side prediction — reads from `user_association.db` and merges
    /// (adding scores) with any dict-side predictions already in `results`.
    private func queryUserAssociations(
        word: String,
        roman: String = "",
        limit: Int,
        results: inout [String: Prediction],
    ) async {
        do {
            try await ensureUserTablesCreated()

            let userResults = try await userConnectionManager.execute { db -> [UserAssociationRow] in
                Self.fetchUserAssociationRows(db: db, word: word, roman: roman, limit: limit)
            }

            let nowMs = Int64(Date().timeIntervalSince1970 * 1000)
            for row in userResults {
                let userScore = Self.calculateUserScore(count: row.count, lastUsedMs: row.lastUsedMs, nowMs: nowMs)
                let key = "\(row.hanzi)\t\(row.tl)"

                if let existing = results[key] {
                    results[key] = Prediction(
                        hanzi: row.hanzi,
                        tl: row.tl.isEmpty ? existing.tl : row.tl,
                        score: existing.score + userScore,
                    )
                } else {
                    results[key] = Prediction(hanzi: row.hanzi, tl: row.tl, score: userScore)
                }
            }
        } catch {
            logger.error("[USER] Query failed: \(error.localizedDescription)")
        }
    }

    // MARK: - User DB Schema & Queries

    private func ensureUserTablesCreated() async throws {
        if stateLock.withLock({ _isUserTablesCreated }) { return }

        try await userConnectionManager.ensureInitialized(
            flags: SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
        )

        try await userConnectionManager.execute { [logger] db in
            try Self.createUserTables(db: db, logger: logger)
        }

        stateLock.withLock { _isUserTablesCreated = true }
    }

    private static func createUserTables(db: OpaquePointer, logger: DebugLogger) throws {
        // Migrate first — drops incompatible v<3 schemas before the CREATE below.
        try migrateUserTables(db: db, logger: logger)

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
        defer { sqlite3_finalize(stmt) }

        guard sqlite3_step(stmt) == SQLITE_DONE else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw DictionaryError.queryExecutionFailed(errorMsg)
        }

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

    /// Migrate `user_association.db` forward to the current schema version using
    /// `PRAGMA user_version`. Transitions:
    /// - v<3 → v3: DROP + CREATE (old schema incompatible).
    /// - v3 → v4: ALTER TABLE adds `prev_tl` (data preserved).
    private static func migrateUserTables(db: OpaquePointer, logger: DebugLogger) throws {
        var versionStmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, "PRAGMA user_version", -1, &versionStmt, nil) == SQLITE_OK else { return }
        let currentVersion = sqlite3_step(versionStmt) == SQLITE_ROW
            ? Int(sqlite3_column_int(versionStmt, 0))
            : 0
        sqlite3_finalize(versionStmt)

        guard currentVersion < userSchemaVersion else { return }

        logger.info("[MIGRATE] user_association.db v\(currentVersion) -> v\(userSchemaVersion)")

        if currentVersion < 3 {
            for sql in [
                "DROP TABLE IF EXISTS user_association",
                "DROP INDEX IF EXISTS idx_user_prev_word",
            ] {
                var stmt: OpaquePointer?
                if sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK {
                    sqlite3_step(stmt)
                    sqlite3_finalize(stmt)
                }
            }
        }

        if currentVersion >= 3, currentVersion < 4 {
            for sql in [
                "ALTER TABLE user_association ADD COLUMN prev_tl TEXT DEFAULT ''",
                "CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl ON user_association(prev_word, prev_tl)",
            ] {
                var stmt: OpaquePointer?
                if sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK {
                    sqlite3_step(stmt)
                    sqlite3_finalize(stmt)
                }
            }
        }

        // Stamp new version
        var stampStmt: OpaquePointer?
        if sqlite3_prepare_v2(db, "PRAGMA user_version = \(userSchemaVersion)", -1, &stampStmt, nil) == SQLITE_OK {
            sqlite3_step(stampStmt)
            sqlite3_finalize(stampStmt)
        }
    }

    private static func insertOrUpdateAssociation(
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

        stmt.bindText(1, prev)
        stmt.bindText(2, prevTl)
        stmt.bindText(3, nextHanzi)
        stmt.bindText(4, nextTl)

        if sqlite3_step(stmt) != SQLITE_DONE {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw DictionaryError.queryExecutionFailed(errorMsg)
        }
    }

    private static func fetchUserAssociationRows(
        db: OpaquePointer,
        word: String,
        roman: String,
        limit: Int,
    ) -> [UserAssociationRow] {
        let sql = """
            SELECT next_word, next_tl, count,
                   strftime('%s', last_used) * 1000 AS last_used_ms
            FROM user_association
            WHERE prev_word = ? AND (prev_tl = ? OR prev_tl = '')
            ORDER BY count DESC
            LIMIT ?
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_finalize(stmt) }

        stmt.bindText(1, word)
        stmt.bindText(2, roman)
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

    private static func fetchAllAssociations(db: OpaquePointer) -> [AssociationEntry] {
        let sql = """
            SELECT prev_word, prev_tl, next_word, next_tl, count
            FROM user_association
            ORDER BY count DESC, last_used DESC
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_finalize(stmt) }

        var results: [AssociationEntry] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            let prevWord = sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? ""
            let prevTl = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
            let nextWord = sqlite3_column_text(stmt, 2).map(String.init(cString:)) ?? ""
            let nextTl = sqlite3_column_text(stmt, 3).map(String.init(cString:)) ?? ""
            let count = Int(sqlite3_column_int(stmt, 4))
            results.append(AssociationEntry(
                prevWord: prevWord, prevTl: prevTl,
                nextWord: nextWord, nextTl: nextTl,
                count: count,
            ))
        }
        return results
    }

    private static func countRows(db: OpaquePointer) -> Int {
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(db, "SELECT COUNT(*) FROM user_association", -1, &stmt, nil) == SQLITE_OK else {
            return 0
        }
        return sqlite3_step(stmt) == SQLITE_ROW ? Int(sqlite3_column_int(stmt, 0)) : 0
    }

    // MARK: - Scoring

    /// Calculate user-layer score with decay floor + learning bonus.
    ///
    /// Ensures user entries always rank above dict entries (max ~300).
    /// High-usage entries (count >= 3) get near-permanent retention.
    ///
    /// `nowMs` is injected at the call site so tests can pin the decay window
    /// without reaching for a global clock (per Codex v1.1 guidance — clock
    /// injection only at the decay-sensitive site).
    private static func calculateUserScore(count: Int, lastUsedMs: Int64, nowMs: Int64) -> Double {
        let decay = calculateDecay(lastUsedMs: lastUsedMs, nowMs: nowMs)
        let rawScore = Double(count) * Constants.userWeight
        let decayFloor = count >= Constants.highUsageThreshold
            ? Constants.highUsageDecayFloor
            : Constants.lowUsageDecayFloor
        let effectiveDecay = max(decayFloor, decay)
        return rawScore * effectiveDecay + Constants.learningBonus
    }

    /// 計算時間衰減因子
    ///
    /// 指數衰減公式（參考 RIME）:
    ///   decay = exp(-ageHours / halfLifeHours * ln(2))
    private static func calculateDecay(lastUsedMs: Int64, nowMs: Int64) -> Double {
        let ageHours = Double(nowMs - lastUsedMs) / 3_600_000.0
        // ln(2) ≈ 0.693
        return exp(-ageHours / Constants.decayHalfLifeHours * 0.693)
    }

    // MARK: - Pruning

    /// 清理舊的使用者關聯（當超過上限時）
    /// Over-deletes by `pruneBatchSize` so the table sits below the cap between
    /// prune runs instead of oscillating around it.
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

    // MARK: - Database Path

    private static func getUserDatabasePath() throws -> String {
        guard let containerURL = SharedSettings.sharedContainerURL else {
            throw DictionaryError.databaseNotFound
        }
        try FileManager.default.createDirectory(
            at: containerURL,
            withIntermediateDirectories: true,
        )
        return containerURL.appendingPathComponent("user_association.db").path
    }
}
