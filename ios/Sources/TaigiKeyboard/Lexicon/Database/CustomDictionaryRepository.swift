import Foundation
import OSLog
import SQLite3

/// Repository for user custom dictionary entries
/// Stores data in App Group shared container (accessible by Keyboard Extension)
final class CustomDictionaryRepository: @unchecked Sendable {

    // MARK: - Properties

    static let shared = CustomDictionaryRepository()

    private let connectionManager: SQLiteConnectionManager
    private let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "CustomDictionaryRepository"
    )

    private var isTablesCreated = false

    // MARK: - Initialization

    init(connectionManager: SQLiteConnectionManager? = nil) {
        self.connectionManager = connectionManager ?? SQLiteConnectionManager(
            databasePath: Self.getDatabasePath,
            queueLabel: "com.siansiansu.taigikeyboard.customdictionary",
            loggerCategory: "CustomDictionaryRepository"
        )
    }

    // MARK: - Database Path

    private static func getDatabasePath() throws -> String {
        guard let containerURL = SharedSettings.getSharedContainerURL() else {
            throw DictionaryError.databaseNotFound
        }

        try FileManager.default.createDirectory(
            at: containerURL,
            withIntermediateDirectories: true
        )

        let databaseURL = containerURL.appendingPathComponent("custom_dictionary.db")
        return databaseURL.path
    }

    // MARK: - Initialization

    func ensureInitialized() async throws {
        try await connectionManager.ensureInitialized(
            flags: SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE
        )
        try await createTablesIfNeeded()
    }

    private func createTablesIfNeeded() async throws {
        if isTablesCreated { return }

        try await connectionManager.execute { db in
            let sql = """
                CREATE TABLE IF NOT EXISTS custom_dictionary (
                    id TEXT PRIMARY KEY,
                    roman TEXT NOT NULL,
                    hanzi TEXT NOT NULL,
                    notone TEXT DEFAULT '',
                    abbrev TEXT DEFAULT '',
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );
            """

            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                let errorMsg = String(cString: sqlite3_errmsg(db))
                throw DictionaryError.queryPreparationFailed("Create custom_dictionary table failed: \(errorMsg)")
            }

            guard sqlite3_step(stmt) == SQLITE_DONE else {
                let errorMsg = String(cString: sqlite3_errmsg(db))
                sqlite3_finalize(stmt)
                throw DictionaryError.queryExecutionFailed("Create custom_dictionary table failed: \(errorMsg)")
            }
            sqlite3_finalize(stmt)

            // Create indexes for prefix search
            for indexSQL in [
                "CREATE INDEX IF NOT EXISTS idx_custom_roman ON custom_dictionary(roman);",
                "CREATE INDEX IF NOT EXISTS idx_custom_notone ON custom_dictionary(notone);",
                "CREATE INDEX IF NOT EXISTS idx_custom_abbrev ON custom_dictionary(abbrev);"
            ] {
                var indexStmt: OpaquePointer?
                if sqlite3_prepare_v2(db, indexSQL, -1, &indexStmt, nil) == SQLITE_OK {
                    sqlite3_step(indexStmt)
                    sqlite3_finalize(indexStmt)
                }
            }

            // Migration: add notone/abbrev columns if they don't exist (for existing databases)
            let columnsToAdd = ["notone", "abbrev"]
            for column in columnsToAdd {
                var checkStmt: OpaquePointer?
                let checkSQL = "SELECT COUNT(*) FROM pragma_table_info('custom_dictionary') WHERE name = '\(column)';"
                var columnExists = false
                if sqlite3_prepare_v2(db, checkSQL, -1, &checkStmt, nil) == SQLITE_OK {
                    if sqlite3_step(checkStmt) == SQLITE_ROW {
                        columnExists = sqlite3_column_int(checkStmt, 0) > 0
                    }
                    sqlite3_finalize(checkStmt)
                }
                if !columnExists {
                    var migStmt: OpaquePointer?
                    let migrationSQL = "ALTER TABLE custom_dictionary ADD COLUMN \(column) TEXT DEFAULT '';"
                    if sqlite3_prepare_v2(db, migrationSQL, -1, &migStmt, nil) == SQLITE_OK {
                        sqlite3_step(migStmt)
                        sqlite3_finalize(migStmt)
                    }
                }
            }

            // Regenerate notone/abbrev for all entries (ensures values match current generation logic)
            let backfillSQL = "SELECT id, roman FROM custom_dictionary;"
            var backfillStmt: OpaquePointer?
            if sqlite3_prepare_v2(db, backfillSQL, -1, &backfillStmt, nil) == SQLITE_OK {
                let TRANSIENT = SQLiteConnectionManager.sqliteTransient
                while sqlite3_step(backfillStmt) == SQLITE_ROW {
                    let id = String(cString: sqlite3_column_text(backfillStmt, 0))
                    let roman = String(cString: sqlite3_column_text(backfillStmt, 1))
                    let notone = CustomDictionaryService.generateNotone(roman)
                    let abbrev = CustomDictionaryService.generateAbbrev(roman)
                    let updateSQL = "UPDATE custom_dictionary SET notone = ?, abbrev = ? WHERE id = ?;"
                    var updateStmt: OpaquePointer?
                    if sqlite3_prepare_v2(db, updateSQL, -1, &updateStmt, nil) == SQLITE_OK {
                        sqlite3_bind_text(updateStmt, 1, notone, -1, TRANSIENT)
                        sqlite3_bind_text(updateStmt, 2, abbrev, -1, TRANSIENT)
                        sqlite3_bind_text(updateStmt, 3, id, -1, TRANSIENT)
                        sqlite3_step(updateStmt)
                        sqlite3_finalize(updateStmt)
                    }
                }
                sqlite3_finalize(backfillStmt)
            }
        }

        isTablesCreated = true
    }

    // MARK: - CRUD Operations

    /// Insert or update an entry (upsert by id)
    func upsert(_ entry: CustomDictionaryEntry) async throws {
        try await ensureInitialized()
        try await connectionManager.execute { db in
            let sql = """
                INSERT INTO custom_dictionary (id, roman, hanzi, notone, abbrev, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    roman = excluded.roman,
                    hanzi = excluded.hanzi,
                    notone = excluded.notone,
                    abbrev = excluded.abbrev,
                    updated_at = excluded.updated_at;
            """

            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                let errorMsg = String(cString: sqlite3_errmsg(db))
                throw DictionaryError.queryPreparationFailed("Upsert failed: \(errorMsg)")
            }
            defer { sqlite3_finalize(stmt) }

            let notone = CustomDictionaryService.generateNotone(entry.roman)
            let abbrev = CustomDictionaryService.generateAbbrev(entry.roman)

            let TRANSIENT = SQLiteConnectionManager.sqliteTransient
            sqlite3_bind_text(stmt, 1, entry.id, -1, TRANSIENT)
            sqlite3_bind_text(stmt, 2, entry.roman, -1, TRANSIENT)
            sqlite3_bind_text(stmt, 3, entry.hanzi, -1, TRANSIENT)
            sqlite3_bind_text(stmt, 4, notone, -1, TRANSIENT)
            sqlite3_bind_text(stmt, 5, abbrev, -1, TRANSIENT)

            let createdStr = Self.dateFormatter.string(from: entry.createdAt)
            let updatedStr = Self.dateFormatter.string(from: entry.updatedAt)
            sqlite3_bind_text(stmt, 6, createdStr, -1, TRANSIENT)
            sqlite3_bind_text(stmt, 7, updatedStr, -1, TRANSIENT)

            guard sqlite3_step(stmt) == SQLITE_DONE else {
                let errorMsg = String(cString: sqlite3_errmsg(db))
                throw DictionaryError.queryExecutionFailed("Upsert failed: \(errorMsg)")
            }
        }
    }

    /// Fetch all entries ordered by updated_at descending
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

    /// Search entries by roman/notone/abbrev prefix (for autocomplete integration)
    func search(romanPrefix: String, limit: Int = 50) async throws -> [CustomDictionaryEntry] {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            let sql = """
                SELECT id, roman, hanzi, created_at, updated_at
                FROM custom_dictionary
                WHERE roman LIKE ? || '%'
                   OR notone LIKE ? || '%'
                   OR abbrev LIKE ? || '%'
                ORDER BY roman
                LIMIT ?;
            """

            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                return []
            }
            defer { sqlite3_finalize(stmt) }

            let TRANSIENT = SQLiteConnectionManager.sqliteTransient
            let lowered = romanPrefix.lowercased()
            sqlite3_bind_text(stmt, 1, lowered, -1, TRANSIENT)
            sqlite3_bind_text(stmt, 2, lowered, -1, TRANSIENT)
            sqlite3_bind_text(stmt, 3, lowered, -1, TRANSIENT)
            sqlite3_bind_int(stmt, 4, Int32(limit))

            var results: [CustomDictionaryEntry] = []
            while sqlite3_step(stmt) == SQLITE_ROW {
                if let entry = Self.readEntry(from: stmt) {
                    results.append(entry)
                }
            }
            return results
        }
    }

    /// Search entries synchronously (for keyboard extension hot path)
    /// - Parameters:
    ///   - romanPrefix: Raw input for roman/abbrev prefix matching
    ///   - notonePrefix: Normalized (toneless) input for notone prefix matching. Falls back to romanPrefix if nil.
    func searchSync(romanPrefix: String, notonePrefix: String? = nil, limit: Int = 50) -> [CustomDictionaryEntry] {
        guard connectionManager.isConnected() else { return [] }

        do {
            return try connectionManager.executeSync { db in
                let sql = """
                    SELECT id, roman, hanzi, created_at, updated_at
                    FROM custom_dictionary
                    WHERE roman LIKE ? || '%'
                       OR notone LIKE ? || '%'
                       OR abbrev LIKE ? || '%'
                    ORDER BY roman
                    LIMIT ?;
                """

                var stmt: OpaquePointer?
                guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                    return []
                }
                defer { sqlite3_finalize(stmt) }

                let TRANSIENT = SQLiteConnectionManager.sqliteTransient
                let lowered = romanPrefix.lowercased()
                let notoneKey = (notonePrefix ?? romanPrefix).lowercased()
                sqlite3_bind_text(stmt, 1, lowered, -1, TRANSIENT)
                sqlite3_bind_text(stmt, 2, notoneKey, -1, TRANSIENT)
                sqlite3_bind_text(stmt, 3, lowered, -1, TRANSIENT)
                sqlite3_bind_int(stmt, 4, Int32(limit))

                var results: [CustomDictionaryEntry] = []
                while sqlite3_step(stmt) == SQLITE_ROW {
                    if let entry = Self.readEntry(from: stmt) {
                        results.append(entry)
                    }
                }
                return results
            }
        } catch {
            return []
        }
    }

    /// Delete an entry by id
    func delete(id: String) async throws {
        try await ensureInitialized()
        try await connectionManager.execute { db in
            let sql = "DELETE FROM custom_dictionary WHERE id = ?;"
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                return
            }
            defer { sqlite3_finalize(stmt) }

            let TRANSIENT = SQLiteConnectionManager.sqliteTransient
            sqlite3_bind_text(stmt, 1, id, -1, TRANSIENT)
            sqlite3_step(stmt)
        }
    }

    /// Delete all entries
    func deleteAll() async throws {
        try await ensureInitialized()
        try await connectionManager.execute { db in
            let sql = "DELETE FROM custom_dictionary;"
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                return
            }
            defer { sqlite3_finalize(stmt) }
            sqlite3_step(stmt)
        }
    }

    /// Get total entry count
    func count() async throws -> Int {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            let sql = "SELECT COUNT(*) FROM custom_dictionary;"
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                return 0
            }
            defer { sqlite3_finalize(stmt) }
            return sqlite3_step(stmt) == SQLITE_ROW ? Int(sqlite3_column_int(stmt, 0)) : 0
        }
    }

    /// Batch import entries (used by CSV file import)
    func batchImport(_ entries: [CustomDictionaryEntry]) async throws -> Int {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            guard sqlite3_exec(db, "BEGIN TRANSACTION;", nil, nil, nil) == SQLITE_OK else {
                throw DictionaryError.queryExecutionFailed("Failed to begin transaction")
            }

            // Build set of existing roman|hanzi keys for deduplication
            var existingKeys = Set<String>()
            let querySql = "SELECT roman, hanzi FROM custom_dictionary;"
            var queryStmt: OpaquePointer?
            if sqlite3_prepare_v2(db, querySql, -1, &queryStmt, nil) == SQLITE_OK {
                while sqlite3_step(queryStmt) == SQLITE_ROW {
                    let roman = String(cString: sqlite3_column_text(queryStmt, 0))
                    let hanzi = String(cString: sqlite3_column_text(queryStmt, 1))
                    existingKeys.insert("\(roman)|\(hanzi)")
                }
            }
            sqlite3_finalize(queryStmt)

            let sql = """
                INSERT INTO custom_dictionary (id, roman, hanzi, notone, abbrev, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    roman = excluded.roman,
                    hanzi = excluded.hanzi,
                    notone = excluded.notone,
                    abbrev = excluded.abbrev,
                    updated_at = excluded.updated_at;
            """

            var insertedCount = 0
            let TRANSIENT = SQLiteConnectionManager.sqliteTransient

            for entry in entries {
                let key = "\(entry.roman)|\(entry.hanzi)"
                if existingKeys.contains(key) {
                    continue
                }

                var stmt: OpaquePointer?
                guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                    continue
                }
                defer { sqlite3_finalize(stmt) }

                let notone = CustomDictionaryService.generateNotone(entry.roman)
                let abbrev = CustomDictionaryService.generateAbbrev(entry.roman)

                sqlite3_bind_text(stmt, 1, entry.id, -1, TRANSIENT)
                sqlite3_bind_text(stmt, 2, entry.roman, -1, TRANSIENT)
                sqlite3_bind_text(stmt, 3, entry.hanzi, -1, TRANSIENT)
                sqlite3_bind_text(stmt, 4, notone, -1, TRANSIENT)
                sqlite3_bind_text(stmt, 5, abbrev, -1, TRANSIENT)

                let createdStr = Self.dateFormatter.string(from: entry.createdAt)
                let updatedStr = Self.dateFormatter.string(from: entry.updatedAt)
                sqlite3_bind_text(stmt, 6, createdStr, -1, TRANSIENT)
                sqlite3_bind_text(stmt, 7, updatedStr, -1, TRANSIENT)

                if sqlite3_step(stmt) == SQLITE_DONE {
                    existingKeys.insert(key)
                    insertedCount += 1
                }
            }

            guard sqlite3_exec(db, "COMMIT;", nil, nil, nil) == SQLITE_OK else {
                sqlite3_exec(db, "ROLLBACK;", nil, nil, nil)
                throw DictionaryError.queryExecutionFailed("Failed to commit transaction")
            }

            return insertedCount
        }
    }

    // MARK: - Database Management

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

    // MARK: - Helpers

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
            updatedAt: updatedAt
        )
    }
}
