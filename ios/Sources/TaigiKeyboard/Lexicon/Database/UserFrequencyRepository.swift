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
    func recordWord(_ word: String, tl: String) async {
        do {
            try await ensureInitialized()
            try await connectionManager.execute { db in
                try Self.insertOrUpdateWord(db: db, word: word, tl: tl)
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
            // Fire-and-forget: a failed frequency write must never block
            // typing. Single boundary — the helper throws the real sqlite
            // error so this logs once (no double-swallow). DebugLogger no-ops
            // in release.
            logger.error("frequency.record.failed word=\(word) tl=\(tl) error=\(error.localizedDescription)")
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

    /// Batch lookup — one SQL round-trip for many words. R5: returns one
    /// ROW per `(word, tl)` reading (a word may yield several: each learned
    /// reading + the legacy `tl == ""` bucket), so the engine can build its
    /// `(display_text, canonical_tl)` pair-keyed `FrequencyMap`. Keyed on
    /// `word` only (`WHERE word IN`), so the caller still dedupes the query
    /// keys by display text.
    func frequencyDataBatch(for words: [String]) -> [FrequencyRow] {
        guard connectionManager.isConnected(), !words.isEmpty else { return [] }
        do {
            return try connectionManager.executeSync { db in
                Self.queryFrequencyDataBatch(db: db, words: words)
            }
        } catch {
            return []
        }
    }

    // MARK: - Mutations

    /// Import with merge-by-max strategy so restoring an older backup never
    /// stomps the user's current (higher) counts. R5: keyed on the
    /// `(word, tl)` pair so each reading merges independently. A backup
    /// written before R5 (no `tl`) imports with `tl == ""` — the legacy
    /// fallback bucket — which is exactly the tolerant behaviour the engine
    /// expects.
    func batchImportMerge(entries: [(word: String, tl: String, count: Int)]) async throws -> Int {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            let sql = """
                INSERT INTO \(UserFrequencySchema.tableName) (word, tl, count, last_used)
                VALUES (?, ?, ?, CURRENT_TIMESTAMP)
                ON CONFLICT(word, tl) DO UPDATE SET
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
                stmt.bindText(2, entry.tl)
                sqlite3_bind_int(stmt, 3, Int32(entry.count))

                if sqlite3_step(stmt) == SQLITE_DONE {
                    imported += 1
                }
            }

            sqlite3_exec(db, "COMMIT;", nil, nil, nil)
            return imported
        }
    }

    /// All `(word, tl, count)` rows, one per learned reading + any legacy
    /// `tl == ""` row. Preserves the R5 per-reading identity. Shared by all
    /// three consumers — the `.taigi` backup export, the 詞頻 management viewer
    /// (which lists + deletes per `(word, tl)`), AND the hand-editable CSV
    /// export — all per-reading. Do NOT add viewer-only SQL (limit / filter)
    /// here — it would leak into backup; split a wrapper if their needs diverge.
    func allFrequencyRowsAsync() async -> [(word: String, tl: String, count: Int)] {
        do {
            try await ensureInitialized()
            return try await connectionManager.execute { db in
                Self.queryAllFrequencyRows(db: db)
            }
        } catch {
            return []
        }
    }

    /// Delete a single `(word, tl)` reading from the frequency table. R5
    /// (#7): identity is the pair, so 一字多音 (重/tāng vs 重/tîng) delete
    /// independently. Deleting the legacy `tl == ""` row removes only the
    /// fallback bucket; re-learned exact-reading rows survive.
    func deleteWord(_ word: String, tl: String) async throws {
        try await ensureInitialized()
        try await connectionManager.execute { db in
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(
                db,
                "DELETE FROM \(UserFrequencySchema.tableName) WHERE word = ? AND tl = ?",
                -1, &stmt, nil,
            ) == SQLITE_OK else { return }
            defer { sqlite3_finalize(stmt) }
            stmt.bindText(1, word)
            stmt.bindText(2, tl)
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

    private static func insertOrUpdateWord(
        db: OpaquePointer,
        word: String,
        tl: String,
    ) throws {
        // R5 pair-key (#7): identity is `(word, tl)` — `tl` is the
        // candidate's canonical-TL reading so 重/tîng and 重/tāng increment
        // separate buckets. A normal commit always carries a canonical TL;
        // `tl == ""` only when the candidate genuinely has none (wire skew /
        // TPS-OOV) and writes the legacy fallback bucket.
        let sql = """
            INSERT INTO \(UserFrequencySchema.tableName) (word, tl, count, last_used)
            VALUES (?, ?, 1, CURRENT_TIMESTAMP)
            ON CONFLICT(word, tl) DO UPDATE SET
                count = count + 1,
                last_used = CURRENT_TIMESTAMP;
        """

        // R6: throw the real sqlite error rather than log-and-return so the
        // single `recordWord` boundary records the failure exactly once.
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            throw LexiconError.queryPreparationFailed(String(cString: sqlite3_errmsg(db)))
        }
        defer { sqlite3_finalize(stmt) }

        stmt.bindText(1, word)
        stmt.bindText(2, tl)

        guard sqlite3_step(stmt) == SQLITE_DONE else {
            throw LexiconError.queryExecutionFailed(String(cString: sqlite3_errmsg(db)))
        }
    }

    private static func queryFrequencyData(db: OpaquePointer, word: String) -> FrequencyData {
        // R5: a word may now span several `(word, tl)` rows. This
        // single-word accessor (UI count display / compat) aggregates them:
        // total count + most-recent last_used. The pair-keyed ranking path
        // does NOT use this — it consults the per-reading bucket via
        // `frequencyDataBatch`.
        let sql = """
            SELECT SUM(count), MAX(strftime('%s', last_used) * 1000)
            FROM \(UserFrequencySchema.tableName)
            WHERE word = ?;
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return .empty }
        defer { sqlite3_finalize(stmt) }

        stmt.bindText(1, word)

        if sqlite3_step(stmt) == SQLITE_ROW {
            // SUM/MAX over zero rows yields SQL NULL → column_int = 0.
            let count = Int(sqlite3_column_int(stmt, 0))
            let lastUsedMillis = sqlite3_column_int64(stmt, 1)
            return FrequencyData(count: count, lastUsedMillis: lastUsedMillis)
        }
        return .empty
    }

    private static func queryFrequencyDataBatch(db: OpaquePointer, words: [String]) -> [FrequencyRow] {
        let placeholders = words.map { _ in "?" }.joined(separator: ",")
        let sql = """
            SELECT word, tl, count, strftime('%s', last_used) * 1000
            FROM \(UserFrequencySchema.tableName)
            WHERE word IN (\(placeholders));
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_finalize(stmt) }

        for (index, word) in words.enumerated() {
            stmt.bindText(Int32(index + 1), word)
        }

        var rows: [FrequencyRow] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            let word = sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? ""
            let tl = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
            let count = Int(sqlite3_column_int(stmt, 2))
            let lastUsedMillis = sqlite3_column_int64(stmt, 3)
            rows.append(FrequencyRow(word: word, tl: tl, data: FrequencyData(count: count, lastUsedMillis: lastUsedMillis)))
        }
        return rows
    }

    private static func queryAllFrequencyRows(db: OpaquePointer) -> [(word: String, tl: String, count: Int)] {
        // Deterministic tie-break (word, tl) so equal count/time rows keep a
        // stable order across the viewer list + backup export.
        let sql = """
            SELECT word, tl, count FROM \(UserFrequencySchema.tableName)
            ORDER BY count DESC, last_used DESC, word ASC, tl ASC;
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_finalize(stmt) }

        var results: [(word: String, tl: String, count: Int)] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            let word = sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? ""
            let tl = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
            let count = Int(sqlite3_column_int(stmt, 2))
            results.append((word: word, tl: tl, count: count))
        }
        return results
    }
}
