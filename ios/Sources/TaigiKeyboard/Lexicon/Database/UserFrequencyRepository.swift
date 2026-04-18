import Foundation
import SQLite3

/// User frequency repository.
///
/// Tracks per-word usage counts so the ranker (`CandidateProcessor.sortByScore`)
/// can bias recently- and often-used words toward the top.
/// Split responsibilities:
/// - `UserFrequencySchema`: DDL (CREATE TABLE / CREATE INDEX / metadata seed)
/// - `UserFrequencyPruner`: capacity (`maxEntries`) + delete-oldest algorithm
final class UserFrequencyRepository: @unchecked Sendable {
    // MARK: - Properties

    static let shared = UserFrequencyRepository()

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
    func count(for word: String) -> Int {
        frequencyData(for: word).count
    }

    /// Full frequency snapshot for a single word (count + last-used millis).
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
