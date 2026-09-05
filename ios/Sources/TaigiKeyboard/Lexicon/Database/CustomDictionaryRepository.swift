import Foundation
import SQLite3

/// Repository for user custom dictionary entries.
///
/// Stores data in the App Group shared container so the keyboard extension
/// can read it. Public API covers CRUD, search (async + sync hot path), and
/// batched CSV import. Split responsibilities:
/// - `CustomDictionarySchema`: DDL (CREATE TABLE / CREATE INDEX)
/// - `CustomDictionaryMigrator`: forward data migrations (ALTER + backfill)
/// - `CustomDictionaryCapacityPolicy`: row-count cap + TOCTOU-safe guard
/// - `CustomDictionaryDerivation`: pure derivation of search-key variants
final class CustomDictionaryRepository: @unchecked Sendable {
    // MARK: - Properties

    private let connectionManager: SQLiteConnectionManager
    private let logger = DebugLogger(category: "CustomDictionaryRepository")

    /// Lock protecting mutable state (`_tableCreationTask`, `_tableCreationGeneration`).
    private let stateLock = NSLock()
    /// Async-once gate for schema + migration — concurrent callers await
    /// the same `Task`; nil cache on failure allows retry.
    private var _tableCreationTask: Task<Void, Error>?
    /// Bumped every time `_tableCreationTask` is replaced so error handlers
    /// only clear the cache they created (Task is a struct, no `===`).
    private var _tableCreationGeneration: UInt64 = 0

    // MARK: - Initialization

    init(connectionManager: SQLiteConnectionManager? = nil) {
        self.connectionManager = connectionManager ?? SQLiteConnectionManager(
            databasePath: { try SharedDatabasePath.resolve(filename: "custom_dictionary.db") },
            queueLabel: "com.siansiansu.taigikeyboard.customdictionary",
            loggerCategory: "CustomDictionaryRepository",
        )
    }

    func ensureInitialized() async throws {
        try await connectionManager.ensureInitialized(
            flags: SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
        )
        try await createTablesIfNeeded()
    }

    // MARK: - CRUD

    /// Insert or update an entry (upsert by id).
    func upsert(_ entry: CustomDictionaryEntry) async throws {
        try await ensureInitialized()
        try await connectionManager.execute { db in
            // Capacity guard runs in the same `execute` block as the write
            // to avoid a TOCTOU race with concurrent writers.
            try CustomDictionaryCapacityPolicy.guardInsertCapacity(db: db, id: entry.id)

            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, Self.upsertEntrySQL, -1, &stmt, nil) == SQLITE_OK else {
                throw LexiconError.queryPreparationFailed(
                    "Upsert failed: \(String(cString: sqlite3_errmsg(db)))",
                )
            }
            defer { sqlite3_finalize(stmt) }

            Self.bindEntry(stmt, entry)

            guard sqlite3_step(stmt) == SQLITE_DONE else {
                throw LexiconError.queryExecutionFailed(
                    "Upsert failed: \(String(cString: sqlite3_errmsg(db)))",
                )
            }

            Self.writeSearchKeys(db: db, entryId: entry.id, roman: entry.roman)
        }
    }

    /// Fetch all entries ordered by updated_at descending.
    func fetchAll() async throws -> [CustomDictionaryEntry] {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            let sql = """
                SELECT id, roman, hanzi, created_at, updated_at
                FROM \(CustomDictionarySchema.tableName)
                ORDER BY updated_at DESC;
            """
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                return []
            }
            defer { sqlite3_finalize(stmt) }

            var results: [CustomDictionaryEntry] = []
            while sqlite3_step(stmt) == SQLITE_ROW {
                if let entry = Self.readEntry(from: stmt) {
                    results.append(entry)
                }
            }
            return results
        }
    }

    /// Cross-mode prefix search (v3.6.1 R3). Query keys come from
    /// `CustomDictionaryDerivation.queryKey(for:mode:)`, which is family-native
    /// to the current input mode, so an entry stored in any mode is found.
    /// - Parameters:
    ///   - family: Search-key family (`tl` / `poj` / `tps`).
    ///   - form: Search-key form (`num` / `notone` / `abbrev`).
    ///   - key: Fused search string (already normalized + lowercased by the
    ///     engine — bind verbatim, do NOT re-lowercase).
    func search(family: String, form: String, key: String, limit: Int = 50) async throws -> [CustomDictionaryEntry] {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            Self.runPrefixSearch(db: db, family: family, form: form, key: key, limit: limit)
        }
    }

    /// Search entries synchronously (for the keyboard extension hot path).
    /// Returns `[]` when the DB is not yet connected — callers must accept
    /// empty results on the very first keystroke rather than blocking.
    func searchSync(family: String, form: String, key: String, limit: Int = 50) -> [CustomDictionaryEntry] {
        guard connectionManager.isConnected() else { return [] }
        do {
            return try connectionManager.executeSync { db in
                Self.runPrefixSearch(db: db, family: family, form: form, key: key, limit: limit)
            }
        } catch {
            return []
        }
    }

    /// Delete an entry by id (and its `custom_search_key` side rows).
    func delete(id: String) async throws {
        try await ensureInitialized()
        try await connectionManager.execute { db in
            let sql = "DELETE FROM \(CustomDictionarySchema.tableName) WHERE id = ?;"
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return }
            defer { sqlite3_finalize(stmt) }

            stmt.bindText(1, id)
            sqlite3_step(stmt)

            Self.deleteSearchKeys(db: db, entryId: id)
        }
    }

    /// Delete all entries (and all `custom_search_key` side rows).
    func deleteAll() async throws {
        try await ensureInitialized()
        try await connectionManager.execute { db in
            sqliteExecSimple(db: db, "DELETE FROM \(CustomDictionarySchema.tableName);")
            sqliteExecSimple(db: db, "DELETE FROM \(CustomDictionarySchema.searchKeyTableName);")
            // R6: reclaim freed pages after a full clear. Best-effort —
            // VACUUM needs exclusive access + ~2x temp; a failure leaves the
            // file larger but intact (sqliteExecSimple is silent). Runs after
            // the DELETEs committed, outside any transaction.
            sqliteExecSimple(db: db, "VACUUM")
        }
    }

    /// Total entry count.
    func count() async throws -> Int {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            CustomDictionaryCapacityPolicy.currentEntryCount(db: db)
        }
    }

    // MARK: - Batch Import

    private static let importBatchSize = 500

    /// Import a CSV-sourced batch. Commits every `importBatchSize` entries
    /// so a single transaction can never hold locks for long, and stops
    /// early when `maxEntries` is reached. Duplicates (same `roman|hanzi`
    /// key) are skipped.
    func batchImport(_ entries: [CustomDictionaryEntry]) async throws -> Int {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            let remainingCapacity = CustomDictionaryCapacityPolicy.remainingCapacity(db: db)
            guard remainingCapacity > 0 else {
                self.logger.debug("[IMPORT] Custom dictionary is full (\(CustomDictionaryCapacityPolicy.maxEntries) entries)")
                return 0
            }

            var existingKeys = Self.existingRomanHanziKeys(db: db)
            var insertedCount = 0

            for batchStart in stride(from: 0, to: entries.count, by: Self.importBatchSize) {
                let batchEnd = min(batchStart + Self.importBatchSize, entries.count)

                guard sqlite3_exec(db, "BEGIN TRANSACTION;", nil, nil, nil) == SQLITE_OK else {
                    throw LexiconError.queryExecutionFailed("Failed to begin transaction")
                }

                for i in batchStart ..< batchEnd {
                    if insertedCount >= remainingCapacity { break }

                    let entry = entries[i]
                    let key = "\(entry.roman)|\(entry.hanzi)"
                    if existingKeys.contains(key) { continue }

                    var stmt: OpaquePointer?
                    guard sqlite3_prepare_v2(db, Self.upsertEntrySQL, -1, &stmt, nil) == SQLITE_OK else { continue }
                    defer { sqlite3_finalize(stmt) }

                    Self.bindEntry(stmt, entry)

                    if sqlite3_step(stmt) == SQLITE_DONE {
                        Self.writeSearchKeys(db: db, entryId: entry.id, roman: entry.roman)
                        existingKeys.insert(key)
                        insertedCount += 1
                    }
                }

                guard sqlite3_exec(db, "COMMIT;", nil, nil, nil) == SQLITE_OK else {
                    sqlite3_exec(db, "ROLLBACK;", nil, nil, nil)
                    throw LexiconError.queryExecutionFailed("Failed to commit batch transaction")
                }

                if insertedCount >= remainingCapacity { break }
            }

            return insertedCount
        }
    }

    // MARK: - Lifecycle

    func deleteDatabase() throws {
        // Cancel the in-flight init Task (if any) BEFORE closing the
        // connection so it bails out rather than racing against a fresh
        // Task installed by the next caller.
        let priorTask = stateLock.withLock { () -> Task<Void, Error>? in
            let task = _tableCreationTask
            _tableCreationTask = nil
            _tableCreationGeneration &+= 1
            return task
        }
        priorTask?.cancel()
        connectionManager.close()

        let path = try SharedDatabasePath.resolve(filename: "custom_dictionary.db")
        if FileManager.default.fileExists(atPath: path) {
            try FileManager.default.removeItem(atPath: path)
        }
    }

    func isConnected() -> Bool {
        connectionManager.isConnected()
    }

    // MARK: - Schema (async-once gate)

    /// Single-flight schema + migration initialization. Concurrent callers
    /// await the same `Task`; once it succeeds subsequent calls await a
    /// completed task (near-free). Failures clear the cache for retry.
    private func createTablesIfNeeded() async throws {
        let (task, generation) = stateLock.withLock { () -> (Task<Void, Error>, UInt64) in
            if let existing = _tableCreationTask {
                return (existing, _tableCreationGeneration)
            }
            _tableCreationGeneration &+= 1
            let gen = _tableCreationGeneration
            let connection = self.connectionManager
            let logger = self.logger
            let new = Task {
                try await connection.execute { db in
                    try CustomDictionarySchema.ensureTables(db: db)
                    CustomDictionaryMigrator.runIfNeeded(db: db, logger: logger)
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

    private static let upsertEntrySQL = """
        INSERT INTO \(CustomDictionarySchema.tableName) (id, roman, hanzi, notone, abbrev, roman_num, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            roman = excluded.roman,
            hanzi = excluded.hanzi,
            notone = excluded.notone,
            abbrev = excluded.abbrev,
            roman_num = excluded.roman_num,
            updated_at = excluded.updated_at;
    """

    private static func bindEntry(_ stmt: OpaquePointer?, _ entry: CustomDictionaryEntry) {
        stmt.bindText(1, entry.id)
        stmt.bindText(2, entry.roman)
        stmt.bindText(3, entry.hanzi)
        stmt.bindText(4, CustomDictionaryDerivation.generateNotone(entry.roman))
        stmt.bindText(5, CustomDictionaryDerivation.generateAbbrev(entry.roman))
        stmt.bindText(6, CustomDictionaryDerivation.generateRomanNum(entry.roman))
        stmt.bindText(7, dateFormatter.string(from: entry.createdAt))
        stmt.bindText(8, dateFormatter.string(from: entry.updatedAt))
    }

    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/CustomDictionaryService.kt (custom_search_key query). Drift causes silent divergence.
    //
    // `form IN (?, 'abbrev')` lets an abbrev side row satisfy any query in the
    // same family — preserving the old `notone LIKE ? OR abbrev LIKE ?`
    // behavior. `DISTINCT` because one entry can match several side rows.
    // `key` is bound verbatim (engine already normalized + lowercased it).
    private static func runPrefixSearch(
        db: OpaquePointer,
        family: String,
        form: String,
        key: String,
        limit: Int,
    ) -> [CustomDictionaryEntry] {
        let sql = """
            SELECT DISTINCT c.id, c.roman, c.hanzi, c.created_at, c.updated_at
            FROM \(CustomDictionarySchema.tableName) c
            JOIN \(CustomDictionarySchema.searchKeyTableName) k
                ON k.\(CustomDictionarySchema.searchKeyEntryIdColumn) = c.id
            WHERE k.\(CustomDictionarySchema.searchKeyFamilyColumn) = ?
              AND k.\(CustomDictionarySchema.searchKeyFormColumn) IN (?, 'abbrev')
              AND k.\(CustomDictionarySchema.searchKeyKeyColumn) LIKE ? || '%'
            ORDER BY c.roman
            LIMIT ?;
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_finalize(stmt) }

        stmt.bindText(1, family)
        stmt.bindText(2, form)
        stmt.bindText(3, key)
        sqlite3_bind_int(stmt, 4, Int32(limit))

        var results: [CustomDictionaryEntry] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            if let entry = readEntry(from: stmt) {
                results.append(entry)
            }
        }
        return results
    }

    // MARK: - Search-Key Side Table

    private static let insertSearchKeySQL = """
        INSERT INTO \(CustomDictionarySchema.searchKeyTableName)
            (\(CustomDictionarySchema.searchKeyEntryIdColumn), \(CustomDictionarySchema.searchKeyFamilyColumn), \(CustomDictionarySchema.searchKeyFormColumn), \(CustomDictionarySchema.searchKeyKeyColumn))
        VALUES (?, ?, ?, ?);
    """

    /// Replace an entry's `custom_search_key` rows: delete by entry id, then
    /// insert the full cross-mode bundle from
    /// `CustomDictionaryDerivation.searchKeys(for:)`. Called after every upsert
    /// and batch insert so the side table never drifts from the entry's roman.
    private static func writeSearchKeys(db: OpaquePointer, entryId: String, roman: String) {
        deleteSearchKeys(db: db, entryId: entryId)

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, insertSearchKeySQL, -1, &stmt, nil) == SQLITE_OK else { return }
        defer { sqlite3_finalize(stmt) }

        for searchKey in CustomDictionaryDerivation.searchKeys(for: roman) {
            sqlite3_reset(stmt)
            sqlite3_clear_bindings(stmt)
            stmt.bindText(1, entryId)
            stmt.bindText(2, searchKey.family)
            stmt.bindText(3, searchKey.form)
            stmt.bindText(4, searchKey.key)
            sqlite3_step(stmt)
        }
    }

    private static func deleteSearchKeys(db: OpaquePointer, entryId: String) {
        let sql = "DELETE FROM \(CustomDictionarySchema.searchKeyTableName) WHERE \(CustomDictionarySchema.searchKeyEntryIdColumn) = ?;"
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return }
        defer { sqlite3_finalize(stmt) }
        stmt.bindText(1, entryId)
        sqlite3_step(stmt)
    }

    private static func existingRomanHanziKeys(db: OpaquePointer) -> Set<String> {
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(
            db,
            "SELECT roman, hanzi FROM \(CustomDictionarySchema.tableName);",
            -1, &stmt, nil,
        ) == SQLITE_OK else {
            return []
        }
        var keys = Set<String>()
        while sqlite3_step(stmt) == SQLITE_ROW {
            let roman = String(cString: sqlite3_column_text(stmt, 0))
            let hanzi = String(cString: sqlite3_column_text(stmt, 1))
            keys.insert("\(roman)|\(hanzi)")
        }
        return keys
    }

    // MARK: - Row Reader

    private static let dateFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd HH:mm:ss"
        formatter.timeZone = TimeZone(identifier: "UTC")
        return formatter
    }()

    private static func readEntry(from stmt: OpaquePointer?) -> CustomDictionaryEntry? {
        guard let stmt else { return nil }

        let id = sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? ""
        let roman = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
        let hanzi = sqlite3_column_text(stmt, 2).map(String.init(cString:)) ?? ""
        let createdStr = sqlite3_column_text(stmt, 3).map(String.init(cString:)) ?? ""
        let updatedStr = sqlite3_column_text(stmt, 4).map(String.init(cString:)) ?? ""

        let createdAt = dateFormatter.date(from: createdStr) ?? Date()
        let updatedAt = dateFormatter.date(from: updatedStr) ?? Date()

        return CustomDictionaryEntry(
            id: id,
            roman: roman,
            hanzi: hanzi,
            createdAt: createdAt,
            updatedAt: updatedAt,
        )
    }
}
