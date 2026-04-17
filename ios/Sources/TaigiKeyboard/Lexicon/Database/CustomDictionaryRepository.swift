import Foundation
import SQLite3

/// File-local helper to reduce `sqlite3_bind_text(_, _, _, -1, TRANSIENT)` boilerplate.
private extension OpaquePointer? {
    func bindText(_ index: Int32, _ value: String) {
        sqlite3_bind_text(self, index, value, -1, SQLiteConnectionManager.sqliteTransient)
    }
}

/// Repository for user custom dictionary entries.
///
/// Stores data in the App Group shared container so the keyboard extension
/// can read it. Public API covers CRUD, search (async + sync hot path), and
/// batched CSV import. Schema management is encapsulated in the `Schema`
/// section below and runs on first use.
final class CustomDictionaryRepository: @unchecked Sendable {
    // MARK: - Properties

    static let shared = CustomDictionaryRepository()

    /// Maximum number of custom dictionary entries (aligned with Android MAX_ENTRY_COUNT).
    static let maxEntries = 30000

    private let connectionManager: SQLiteConnectionManager
    private let logger = DebugLogger(category: "CustomDictionaryRepository")

    private var isTablesCreated = false

    // MARK: - Initialization

    init(connectionManager: SQLiteConnectionManager? = nil) {
        self.connectionManager = connectionManager ?? SQLiteConnectionManager(
            databasePath: Self.getDatabasePath,
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
            // Capacity guard: check inside the same execute block to avoid
            // a TOCTOU race with concurrent writers. Updating an existing
            // entry (same id) does not count as a new insert.
            if !Self.entryExists(db: db, id: entry.id) {
                let currentCount = Self.currentEntryCount(db: db)
                guard currentCount < Self.maxEntries else {
                    throw DictionaryError.queryExecutionFailed("Custom dictionary is full (max \(Self.maxEntries) entries)")
                }
            }

            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, Self.upsertEntrySQL, -1, &stmt, nil) == SQLITE_OK else {
                let errorMsg = String(cString: sqlite3_errmsg(db))
                throw DictionaryError.queryPreparationFailed("Upsert failed: \(errorMsg)")
            }
            defer { sqlite3_finalize(stmt) }

            Self.bindEntry(stmt, entry)

            guard sqlite3_step(stmt) == SQLITE_DONE else {
                let errorMsg = String(cString: sqlite3_errmsg(db))
                throw DictionaryError.queryExecutionFailed("Upsert failed: \(errorMsg)")
            }
        }
    }

    /// Fetch all entries ordered by updated_at descending.
    func fetchAll() async throws -> [CustomDictionaryEntry] {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            let sql = "SELECT id, roman, hanzi, created_at, updated_at FROM custom_dictionary ORDER BY updated_at DESC;"
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

    /// Search entries by prefix (for autocomplete integration).
    /// - Parameters:
    ///   - prefix: Preprocessed search prefix (roman_num key for toned, notone key for toneless).
    ///   - isToneAware: When true, matches `roman_num`; otherwise matches `notone`.
    func search(prefix: String, isToneAware: Bool, limit: Int = 50) async throws -> [CustomDictionaryEntry] {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            Self.runPrefixSearch(db: db, prefix: prefix, isToneAware: isToneAware, limit: limit)
        }
    }

    /// Search entries synchronously (for the keyboard extension hot path).
    /// Returns `[]` when the DB is not yet connected — callers must accept empty
    /// results on the very first keystroke rather than blocking on init.
    func searchSync(prefix: String, isToneAware: Bool, limit: Int = 50) -> [CustomDictionaryEntry] {
        guard connectionManager.isConnected() else { return [] }
        do {
            return try connectionManager.executeSync { db in
                Self.runPrefixSearch(db: db, prefix: prefix, isToneAware: isToneAware, limit: limit)
            }
        } catch {
            return []
        }
    }

    /// Delete an entry by id.
    func delete(id: String) async throws {
        try await ensureInitialized()
        try await connectionManager.execute { db in
            let sql = "DELETE FROM custom_dictionary WHERE id = ?;"
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return }
            defer { sqlite3_finalize(stmt) }

            stmt.bindText(1, id)
            sqlite3_step(stmt)
        }
    }

    /// Delete all entries.
    func deleteAll() async throws {
        try await ensureInitialized()
        try await connectionManager.execute { db in
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, "DELETE FROM custom_dictionary;", -1, &stmt, nil) == SQLITE_OK else { return }
            defer { sqlite3_finalize(stmt) }
            sqlite3_step(stmt)
        }
    }

    /// Total entry count.
    func count() async throws -> Int {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            Self.currentEntryCount(db: db)
        }
    }

    // MARK: - Batch Import

    private static let importBatchSize = 500

    /// Import a CSV-sourced batch. Commits every `importBatchSize` entries so a
    /// single transaction can never hold locks for long, and stops early when
    /// `maxEntries` is reached. Duplicates (same `roman|hanzi` key) are skipped.
    func batchImport(_ entries: [CustomDictionaryEntry]) async throws -> Int {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            let remainingCapacity = Self.maxEntries - Self.currentEntryCount(db: db)
            guard remainingCapacity > 0 else {
                self.logger.debug("[IMPORT] Custom dictionary is full (\(Self.maxEntries) entries)")
                return 0
            }

            var existingKeys = Self.existingRomanHanziKeys(db: db)
            var insertedCount = 0

            for batchStart in stride(from: 0, to: entries.count, by: Self.importBatchSize) {
                let batchEnd = min(batchStart + Self.importBatchSize, entries.count)

                guard sqlite3_exec(db, "BEGIN TRANSACTION;", nil, nil, nil) == SQLITE_OK else {
                    throw DictionaryError.queryExecutionFailed("Failed to begin transaction")
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
                        existingKeys.insert(key)
                        insertedCount += 1
                    }
                }

                guard sqlite3_exec(db, "COMMIT;", nil, nil, nil) == SQLITE_OK else {
                    sqlite3_exec(db, "ROLLBACK;", nil, nil, nil)
                    throw DictionaryError.queryExecutionFailed("Failed to commit batch transaction")
                }

                if insertedCount >= remainingCapacity { break }
            }

            return insertedCount
        }
    }

    // MARK: - Lifecycle

    func deleteDatabase() throws {
        connectionManager.close()
        isTablesCreated = false

        let path = try Self.getDatabasePath()
        if FileManager.default.fileExists(atPath: path) {
            try FileManager.default.removeItem(atPath: path)
        }
    }

    func isConnected() -> Bool {
        connectionManager.isConnected()
    }

    // MARK: - Database Path

    private static func getDatabasePath() throws -> String {
        guard let containerURL = SharedSettings.sharedContainerURL else {
            throw DictionaryError.databaseNotFound
        }
        try FileManager.default.createDirectory(
            at: containerURL,
            withIntermediateDirectories: true,
        )
        return containerURL.appendingPathComponent("custom_dictionary.db").path
    }

    // MARK: - Schema

    private func createTablesIfNeeded() async throws {
        if isTablesCreated { return }

        try await connectionManager.execute { db in
            try Self.createMainTable(db: db)
            Self.createIndexes(db: db)
            Self.migrateAddMissingColumns(db: db)
            Self.backfillDerivedColumns(db: db)
        }

        isTablesCreated = true
    }

    private static func createMainTable(db: OpaquePointer) throws {
        let sql = """
            CREATE TABLE IF NOT EXISTS custom_dictionary (
                id TEXT PRIMARY KEY,
                roman TEXT NOT NULL,
                hanzi TEXT NOT NULL,
                notone TEXT DEFAULT '',
                abbrev TEXT DEFAULT '',
                roman_num TEXT DEFAULT '',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw DictionaryError.queryPreparationFailed("Create custom_dictionary table failed: \(errorMsg)")
        }
        defer { sqlite3_finalize(stmt) }

        guard sqlite3_step(stmt) == SQLITE_DONE else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw DictionaryError.queryExecutionFailed("Create custom_dictionary table failed: \(errorMsg)")
        }
    }

    private static func createIndexes(db: OpaquePointer) {
        let indexSQLs = [
            "CREATE INDEX IF NOT EXISTS idx_custom_roman ON custom_dictionary(roman);",
            "CREATE INDEX IF NOT EXISTS idx_custom_notone ON custom_dictionary(notone);",
            "CREATE INDEX IF NOT EXISTS idx_custom_abbrev ON custom_dictionary(abbrev);",
            "CREATE INDEX IF NOT EXISTS idx_custom_roman_num ON custom_dictionary(roman_num);",
        ]
        for sql in indexSQLs {
            var stmt: OpaquePointer?
            if sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK {
                sqlite3_step(stmt)
                sqlite3_finalize(stmt)
            }
        }
    }

    /// Add derived columns that may be missing on databases created before they
    /// existed. The whitelist of column names is required because SQLite DDL
    /// cannot parameterize column identifiers.
    private static func migrateAddMissingColumns(db: OpaquePointer) {
        let allowedColumns: Set = ["notone", "abbrev", "roman_num"]
        for column in allowedColumns {
            if columnExists(db: db, column: column) { continue }
            var stmt: OpaquePointer?
            let sql = "ALTER TABLE custom_dictionary ADD COLUMN \(column) TEXT DEFAULT '';"
            if sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK {
                sqlite3_step(stmt)
                sqlite3_finalize(stmt)
            }
        }
    }

    /// Re-derive notone/abbrev/roman_num for every row so values always match
    /// the current generation logic. Prepares the UPDATE statement once and
    /// reuses it across rows (reset + clear bindings per iteration).
    private static func backfillDerivedColumns(db: OpaquePointer) {
        var selectStmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, "SELECT id, roman FROM custom_dictionary;", -1, &selectStmt, nil) == SQLITE_OK else { return }
        defer { sqlite3_finalize(selectStmt) }

        var updateStmt: OpaquePointer?
        let updateSQL = "UPDATE custom_dictionary SET notone = ?, abbrev = ?, roman_num = ? WHERE id = ?;"
        guard sqlite3_prepare_v2(db, updateSQL, -1, &updateStmt, nil) == SQLITE_OK else { return }
        defer { sqlite3_finalize(updateStmt) }

        while sqlite3_step(selectStmt) == SQLITE_ROW {
            let id = String(cString: sqlite3_column_text(selectStmt, 0))
            let roman = String(cString: sqlite3_column_text(selectStmt, 1))

            sqlite3_reset(updateStmt)
            sqlite3_clear_bindings(updateStmt)

            updateStmt.bindText(1, CustomDictionaryService.generateNotone(roman))
            updateStmt.bindText(2, CustomDictionaryService.generateAbbrev(roman))
            updateStmt.bindText(3, CustomDictionaryService.generateRomanNum(roman))
            updateStmt.bindText(4, id)
            sqlite3_step(updateStmt)
        }
    }

    private static func columnExists(db: OpaquePointer, column: String) -> Bool {
        let sql = "SELECT COUNT(*) FROM pragma_table_info('custom_dictionary') WHERE name = ?;"
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return false }
        defer { sqlite3_finalize(stmt) }
        stmt.bindText(1, column)
        return sqlite3_step(stmt) == SQLITE_ROW && sqlite3_column_int(stmt, 0) > 0
    }

    // MARK: - Query Helpers

    private static let upsertEntrySQL = """
        INSERT INTO custom_dictionary (id, roman, hanzi, notone, abbrev, roman_num, created_at, updated_at)
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
        let notone = CustomDictionaryService.generateNotone(entry.roman)
        let abbrev = CustomDictionaryService.generateAbbrev(entry.roman)
        let romanNum = CustomDictionaryService.generateRomanNum(entry.roman)

        stmt.bindText(1, entry.id)
        stmt.bindText(2, entry.roman)
        stmt.bindText(3, entry.hanzi)
        stmt.bindText(4, notone)
        stmt.bindText(5, abbrev)
        stmt.bindText(6, romanNum)
        stmt.bindText(7, dateFormatter.string(from: entry.createdAt))
        stmt.bindText(8, dateFormatter.string(from: entry.updatedAt))
    }

    private static func runPrefixSearch(
        db: OpaquePointer,
        prefix: String,
        isToneAware: Bool,
        limit: Int,
    ) -> [CustomDictionaryEntry] {
        let column = isToneAware ? "roman_num" : "notone"
        let sql = """
            SELECT id, roman, hanzi, created_at, updated_at
            FROM custom_dictionary
            WHERE \(column) LIKE ? || '%'
               OR abbrev LIKE ? || '%'
            ORDER BY roman
            LIMIT ?;
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_finalize(stmt) }

        let lowered = prefix.lowercased()
        stmt.bindText(1, lowered)
        stmt.bindText(2, lowered)
        sqlite3_bind_int(stmt, 3, Int32(limit))

        var results: [CustomDictionaryEntry] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            if let entry = readEntry(from: stmt) {
                results.append(entry)
            }
        }
        return results
    }

    private static func entryExists(db: OpaquePointer, id: String) -> Bool {
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(db, "SELECT 1 FROM custom_dictionary WHERE id = ? LIMIT 1;", -1, &stmt, nil) == SQLITE_OK else {
            return false
        }
        stmt.bindText(1, id)
        return sqlite3_step(stmt) == SQLITE_ROW
    }

    private static func currentEntryCount(db: OpaquePointer) -> Int {
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(db, "SELECT COUNT(*) FROM custom_dictionary;", -1, &stmt, nil) == SQLITE_OK else {
            return 0
        }
        return sqlite3_step(stmt) == SQLITE_ROW ? Int(sqlite3_column_int(stmt, 0)) : 0
    }

    private static func existingRomanHanziKeys(db: OpaquePointer) -> Set<String> {
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(db, "SELECT roman, hanzi FROM custom_dictionary;", -1, &stmt, nil) == SQLITE_OK else {
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
