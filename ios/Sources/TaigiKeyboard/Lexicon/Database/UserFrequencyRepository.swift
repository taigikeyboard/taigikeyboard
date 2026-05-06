// 中文: 使用者詞頻 repository — 紀錄每個詞使用次數,給排序管線當分數依據。
// 中文: schema、pruning 已拆到鄰近檔案,本檔只剩對外 API 與 single-flight schema gate。

import Foundation
import SQLite3

/// User frequency repository.
///
/// Tracks per-word usage counts so the ranker
/// (`RustEngineBridge.processCandidates` → `engine/ranking/`) can bias
/// recently- and often-used words toward the top.
/// Split responsibilities:
/// - `UserFrequencySchema`: DDL (CREATE TABLE / CREATE INDEX / metadata seed)
/// - `UserFrequencyPruner`: capacity (`maxEntries`) + delete-oldest algorithm
// 中文: 使用者詞頻主類 — 對外 API + 節流計數器 + single-flight schema gate。
final class UserFrequencyRepository: @unchecked Sendable {
    // MARK: - Properties

    private let connectionManager: SQLiteConnectionManager
    private let logger = DebugLogger(category: "UserFrequencyRepository")

    /// Lock protecting mutable state (`_tableCreationTask`,
    /// `_tableCreationGeneration`, `_recordCounter`).
    private let stateLock = NSLock()
    /// Async-once gate for schema creation/migration — concurrent callers
    /// await the same `Task`; nil cache on failure allows retry.
    private var _tableCreationTask: Task<Void, Error>?
    /// Bumped every time `_tableCreationTask` is replaced. Used instead of
    /// identity comparison (Task is a struct, `===` unavailable) to ensure
    /// error handlers only clear the cache they created.
    private var _tableCreationGeneration: UInt64 = 0
    private var _recordCounter = 0

    // MARK: - Initialization

    init(connectionManager: SQLiteConnectionManager? = nil) {
        self.connectionManager = connectionManager ?? SQLiteConnectionManager(
            databasePath: { try SharedDatabasePath.resolve(filename: "user_frequency.db") },
            queueLabel: "com.siansiansu.taigikeyboard.userfrequency",
            loggerCategory: "UserFrequencyRepository",
        )
    }

    /// Ensure the DB connection is open and schema has been applied.
    // 中文: 確保 DB 連線打開且 schema 已套用 — 寫入前必呼叫。
    func ensureInitialized() async throws {
        try await connectionManager.ensureInitialized(
            flags: SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
        )
        try await createTablesIfNeeded()
    }

    // MARK: - Recording

    /// Record a word usage. Periodically triggers background pruning once
    /// every `UserFrequencyPruner.recordCheckInterval` records so the table
    /// stays under capacity without adding latency to every single write.
    // 中文: 紀錄一次詞使用。每 recordCheckInterval 次觸發一次背景 prune,
    // 中文: 表會控制在上限內,但不會讓每次寫入都付 prune 成本。
    func recordWord(_ word: String) async {
        do {
            try await ensureInitialized()
            try await connectionManager.execute { db in
                try Self.insertOrUpdateWord(db: db, word: word, logger: self.logger)
            }

            let shouldPrune: Bool = stateLock.withLock {
                _recordCounter += 1
                if _recordCounter >= UserFrequencyPruner.recordCheckInterval {
                    _recordCounter = 0
                    return true
                }
                return false
            }
            if shouldPrune {
                _ = try? await connectionManager.execute { db in
                    UserFrequencyPruner.pruneIfNeeded(db: db, logger: self.logger)
                }
            }
        } catch {
            logger.error("[RECORD] Failed to record usage for: \(word)")
        }
    }

    // MARK: - Queries

    /// Usage count for a single word.
    // 中文: 取得單一詞的使用次數。
    func count(for word: String) -> Int {
        frequencyData(for: word).count
    }

    /// Full frequency snapshot for a single word (count + last-used millis).
    // 中文: 取得單一詞的完整頻率快照(count + 上次使用毫秒)。
    func frequencyData(for word: String) -> FrequencyData {
        guard connectionManager.isConnected() else { return .empty }
        do {
            return try connectionManager.executeSync { db in
                Self.queryFrequencyData(db: db, word: word)
            }
        } catch {
            return .empty
        }
    }

    /// Batch lookup — one SQL round-trip for many words.
    // 中文: 批次查詢 — 一個 SQL round-trip 拿多筆,排序管線用這個。
    func frequencyDataBatch(for words: [String]) -> [String: FrequencyData] {
        guard connectionManager.isConnected(), !words.isEmpty else { return [:] }
        do {
            return try connectionManager.executeSync { db in
                Self.queryFrequencyDataBatch(db: db, words: words)
            }
        } catch {
            return [:]
        }
    }

    /// Top N words by count then recency. Sync variant used by the UI layer
    /// that already awaited `ensureInitialized()` upstream.
    // 中文: 取使用次數最多的前 N 個詞(同步版),呼叫端要先 ensureInitialized。
    func topWords(limit: Int = 100) -> [(word: String, count: Int)] {
        guard connectionManager.isConnected() else { return [] }
        do {
            return try connectionManager.executeSync { db in
                Self.queryTopWords(db: db, limit: limit)
            }
        } catch {
            return []
        }
    }

    /// Top N words with explicit init — safer from backup / management flows.
    // 中文: 取使用次數最多的前 N 個詞(async 版),含明確初始化,適合備份 / 管理流程。
    func topWordsAsync(limit: Int = 100) async -> [(word: String, count: Int)] {
        do {
            try await ensureInitialized()
            return try await connectionManager.execute { db in
                Self.queryTopWords(db: db, limit: limit)
            }
        } catch {
            return []
        }
    }

    // MARK: - Mutations

    /// Import with merge-by-max strategy so restoring an older backup never
    /// stomps the user's current (higher) counts.
    // 中文: 批次匯入 — merge-by-max 策略,還原舊備份不會把當前較高的 count 蓋掉。
    func batchImportMerge(entries: [(word: String, count: Int)]) async throws -> Int {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            let sql = """
                INSERT INTO \(UserFrequencySchema.tableName) (word, count, last_used)
                VALUES (?, ?, CURRENT_TIMESTAMP)
                ON CONFLICT(word) DO UPDATE SET
                    count = MAX(count, excluded.count),
                    last_used = CURRENT_TIMESTAMP;
            """

            guard sqlite3_exec(db, "BEGIN TRANSACTION;", nil, nil, nil) == SQLITE_OK else { return 0 }

            var imported = 0
            for entry in entries {
                var stmt: OpaquePointer?
                guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { continue }
                defer { sqlite3_finalize(stmt) }

                stmt.bindText(1, entry.word)
                sqlite3_bind_int(stmt, 2, Int32(entry.count))

                if sqlite3_step(stmt) == SQLITE_DONE {
                    imported += 1
                }
            }

            sqlite3_exec(db, "COMMIT;", nil, nil, nil)
            return imported
        }
    }

    /// Delete a single word from the frequency table.
    // 中文: 從頻率表刪除單一詞。
    func deleteWord(_ word: String) async throws {
        try await ensureInitialized()
        try await connectionManager.execute { db in
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(
                db,
                "DELETE FROM \(UserFrequencySchema.tableName) WHERE word = ?",
                -1, &stmt, nil,
            ) == SQLITE_OK else { return }
            defer { sqlite3_finalize(stmt) }
            stmt.bindText(1, word)
            sqlite3_step(stmt)
        }
    }

    // MARK: - Lifecycle

    /// Close the connection and remove the on-disk file.
    // 中文: 關閉連線並移除檔案。在關閉前先取消正在進行中的初始化 Task,
    // 中文: 避免它與下一個 caller 安裝的新 Task 賽跑。
    func deleteDatabase() throws {
        // Cancel the in-flight init Task (if any) BEFORE closing the
        // connection so it bails out rather than racing against a fresh
        // Task installed by the next caller.
        let priorTask = stateLock.withLock { () -> Task<Void, Error>? in
            let task = _tableCreationTask
            _tableCreationTask = nil
            _tableCreationGeneration &+= 1
            _recordCounter = 0
            return task
        }
        priorTask?.cancel()
        connectionManager.close()

        let path = try SharedDatabasePath.resolve(filename: "user_frequency.db")
        if FileManager.default.fileExists(atPath: path) {
            try FileManager.default.removeItem(atPath: path)
        }
    }

    func isConnected() -> Bool {
        connectionManager.isConnected()
    }

    /// Total entry count for the frequency table, or `-1` if the DB is closed.
    // 中文: 頻率表總 row 數,連線壞掉回 -1。
    func totalCount() -> Int {
        guard connectionManager.isConnected() else { return -1 }
        do {
            return try connectionManager.executeSync { db in
                UserFrequencyPruner.rowCount(db: db)
            }
        } catch {
            return -1
        }
    }

    // MARK: - Schema (async-once gate)

    /// Single-flight schema initialization. Concurrent callers await the
    /// same `Task`; once it succeeds subsequent calls await a completed
    /// task (near-free). Failures clear the cache so the next caller retries.
    // 中文: schema 初始化 single-flight gate — 並行呼叫共用一個 Task,失敗清掉 cache。
    private func createTablesIfNeeded() async throws {
        let (task, generation) = stateLock.withLock { () -> (Task<Void, Error>, UInt64) in
            if let existing = _tableCreationTask {
                return (existing, _tableCreationGeneration)
            }
            _tableCreationGeneration &+= 1
            let gen = _tableCreationGeneration
            let connection = self.connectionManager
            let new = Task {
                try await connection.execute { db in
                    try UserFrequencySchema.ensureTables(db: db)
                }
            }
            _tableCreationTask = new
            return (new, gen)
        }
        do {
            try await task.value
        } catch {
            stateLock.withLock {
                if _tableCreationGeneration == generation {
                    _tableCreationTask = nil
                }
            }
            throw error
        }
    }

    // MARK: - Query Helpers

    private static func insertOrUpdateWord(db: OpaquePointer, word: String, logger: DebugLogger) throws {
        let sql = """
            INSERT INTO \(UserFrequencySchema.tableName) (word, count, last_used)
            VALUES (?, 1, CURRENT_TIMESTAMP)
            ON CONFLICT(word) DO UPDATE SET
                count = count + 1,
                last_used = CURRENT_TIMESTAMP;
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            logger.error("[RECORD] Failed to prepare statement")
            return
        }
        defer { sqlite3_finalize(stmt) }

        stmt.bindText(1, word)

        if sqlite3_step(stmt) != SQLITE_DONE {
            logger.error("[RECORD] Failed to record usage for: \(word)")
        }
    }

    private static func queryFrequencyData(db: OpaquePointer, word: String) -> FrequencyData {
        let sql = """
            SELECT count, strftime('%s', last_used) * 1000
            FROM \(UserFrequencySchema.tableName)
            WHERE word = ?;
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return .empty }
        defer { sqlite3_finalize(stmt) }

        stmt.bindText(1, word)

        if sqlite3_step(stmt) == SQLITE_ROW {
            let count = Int(sqlite3_column_int(stmt, 0))
            let lastUsedMillis = sqlite3_column_int64(stmt, 1)
            return FrequencyData(count: count, lastUsedMillis: lastUsedMillis)
        }
        return .empty
    }

    private static func queryFrequencyDataBatch(db: OpaquePointer, words: [String]) -> [String: FrequencyData] {
        let placeholders = words.map { _ in "?" }.joined(separator: ",")
        let sql = """
            SELECT word, count, strftime('%s', last_used) * 1000
            FROM \(UserFrequencySchema.tableName)
            WHERE word IN (\(placeholders));
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return [:] }
        defer { sqlite3_finalize(stmt) }

        for (index, word) in words.enumerated() {
            stmt.bindText(Int32(index + 1), word)
        }

        var result: [String: FrequencyData] = [:]
        while sqlite3_step(stmt) == SQLITE_ROW {
            let word = sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? ""
            let count = Int(sqlite3_column_int(stmt, 1))
            let lastUsedMillis = sqlite3_column_int64(stmt, 2)
            result[word] = FrequencyData(count: count, lastUsedMillis: lastUsedMillis)
        }
        return result
    }

    private static func queryTopWords(db: OpaquePointer, limit: Int) -> [(word: String, count: Int)] {
        let sql = """
            SELECT word, count FROM \(UserFrequencySchema.tableName)
            ORDER BY count DESC, last_used DESC
            LIMIT ?;
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_finalize(stmt) }

        sqlite3_bind_int64(stmt, 1, Int64(limit))

        var results: [(word: String, count: Int)] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            let word = sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? ""
            let count = Int(sqlite3_column_int(stmt, 1))
            results.append((word: word, count: count))
        }
        return results
    }
}
