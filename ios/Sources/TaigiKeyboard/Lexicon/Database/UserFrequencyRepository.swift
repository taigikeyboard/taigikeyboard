import Foundation
import SQLite3

/// User frequency repository.
///
/// Tracks per-word usage counts so the ranker (`CandidateProcessor.sortByScore`)
/// can bias recently- and often-used words toward the top. Runs capped at
/// `Constants.maxEntries`; least-used rows are pruned in the background.
final class UserFrequencyRepository: @unchecked Sendable {
    // MARK: - Constants

    private enum Constants {
        static let maxEntries = 20000
        static let pruneCheckInterval = 100
        static let pruneBatchSize = 2000
    }

    // MARK: - Types

    /// Per-word frequency snapshot (count + last-used timestamp).
    struct FrequencyData {
        let count: Int
        let lastUsedMillis: Int64 // Unix timestamp in milliseconds

        static let empty = FrequencyData(count: 0, lastUsedMillis: 0)
    }

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

    /// Record a word usage. Periodically triggers background pruning once every
    /// `pruneCheckInterval` records so the table stays under capacity without
    /// adding latency to every single write.
    func recordWord(_ word: String) async {
        do {
            try await ensureInitialized()
            try await connectionManager.execute { db in
                try Self.insertOrUpdateWord(db: db, word: word, logger: self.logger)
            }

            let shouldPrune: Bool = stateLock.withLock {
                _recordCounter += 1
                if _recordCounter >= Constants.pruneCheckInterval {
                    _recordCounter = 0
                    return true
                }
                return false
            }
            if shouldPrune {
                await pruneOldEntries()
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
                INSERT INTO user_frequency (word, count, last_used)
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
            guard sqlite3_prepare_v2(db, "DELETE FROM user_frequency WHERE word = ?", -1, &stmt, nil) == SQLITE_OK else { return }
            defer { sqlite3_finalize(stmt) }
            stmt.bindText(1, word)
            sqlite3_step(stmt)
        }
    }

    // MARK: - Lifecycle

    /// Close the connection and remove the on-disk file.
    func deleteDatabase() throws {
        connectionManager.close()
        stateLock.withLock {
            _tableCreationTask = nil
            _tableCreationGeneration &+= 1
            _recordCounter = 0
        }

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
                Self.countRows(db: db)
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
                    try Self.createFrequencyTable(db: db)
                    Self.createFrequencyIndexes(db: db)
                    Self.createMetadataTable(db: db)
                    Self.seedMetadata(db: db)
                }
            }
            _tableCreationTask = new
            return (new, gen)
        }
        do {
            try await task.value
        } catch {
            stateLock.withLock {
                // Only clear if we still own the cached task generation —
                // avoids wiping a newer task that a later caller installed.
                if _tableCreationGeneration == generation {
                    _tableCreationTask = nil
                }
            }
            throw error
        }
    }

    private static func createFrequencyTable(db: OpaquePointer) throws {
        let sql = """
            CREATE TABLE IF NOT EXISTS user_frequency (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word TEXT NOT NULL UNIQUE,
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw LexiconError.queryPreparationFailed("Create user_frequency table failed: \(errorMsg)")
        }
        defer { sqlite3_finalize(stmt) }

        guard sqlite3_step(stmt) == SQLITE_DONE else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw LexiconError.queryExecutionFailed("Create user_frequency table failed: \(errorMsg)")
        }
    }

    private static func createFrequencyIndexes(db: OpaquePointer) {
        for sql in [
            "CREATE INDEX IF NOT EXISTS idx_word ON user_frequency(word);",
            "CREATE INDEX IF NOT EXISTS idx_count ON user_frequency(count DESC);",
            "CREATE INDEX IF NOT EXISTS idx_last_used ON user_frequency(last_used DESC);",
        ] {
            sqliteExecSimple(db: db, sql)
        }
    }

    private static func createMetadataTable(db: OpaquePointer) {
        sqliteExecSimple(db: db, """
            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT
            );
        """)
    }

    private static func seedMetadata(db: OpaquePointer) {
        let sql = """
            INSERT OR IGNORE INTO metadata (key, value) VALUES
            ('app_version', ?),
            ('schema_version', '1.0'),
            ('created_date', datetime('now')),
            ('last_modified', datetime('now'));
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return }
        defer { sqlite3_finalize(stmt) }

        stmt.bindText(1, appVersion())
        sqlite3_step(stmt)
    }

    private static func appVersion() -> String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "1.0.0"
    }

    // MARK: - Query Helpers

    private static func insertOrUpdateWord(db: OpaquePointer, word: String, logger: DebugLogger) throws {
        let sql = """
            INSERT INTO user_frequency (word, count, last_used)
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
        let sql = "SELECT count, strftime('%s', last_used) * 1000 FROM user_frequency WHERE word = ?;"
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
            FROM user_frequency
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
            SELECT word, count FROM user_frequency
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

    private static func countRows(db: OpaquePointer) -> Int {
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(db, "SELECT COUNT(*) FROM user_frequency;", -1, &stmt, nil) == SQLITE_OK else {
            return -1
        }
        return sqlite3_step(stmt) == SQLITE_ROW ? Int(sqlite3_column_int(stmt, 0)) : -1
    }

    // MARK: - Pruning

    /// Delete the least-used rows when the table exceeds `maxEntries`.
    /// Over-deletes by `pruneBatchSize` so the table sits well below the
    /// cap between prune runs instead of oscillating around it.
    private func pruneOldEntries() async {
        do {
            let currentCount = try await connectionManager.execute { db in
                Self.countRows(db: db)
            }
            guard currentCount > Constants.maxEntries else { return }

            let deleteCount = min(
                Constants.pruneBatchSize,
                currentCount - Constants.maxEntries + Constants.pruneBatchSize,
            )

            try await connectionManager.execute { db in
                let sql = """
                    DELETE FROM user_frequency
                    WHERE id IN (
                        SELECT id FROM user_frequency
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

            logger.info("[PRUNE] Deleted \(deleteCount) frequency entries (was \(currentCount))")
        } catch {
            logger.error("[PRUNE] Failed: \(error.localizedDescription)")
        }
    }
}
