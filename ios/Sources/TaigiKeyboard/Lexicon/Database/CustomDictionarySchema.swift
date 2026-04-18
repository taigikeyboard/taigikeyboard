import Foundation
import SQLite3

/// Custom-dictionary DB schema (DDL only).
///
/// Owns the table + indexes for `custom_dictionary.db`. Data migrations
/// (adding missing columns on pre-existing databases, backfilling derived
/// search keys) live in `CustomDictionaryMigrator` so this type stays purely
/// DDL — no domain logic, no derivation, no capacity policy.
/// Callers must serialize access (typically via `SQLiteConnectionManager.execute`).
enum CustomDictionarySchema {
    static let tableName = "custom_dictionary"

    /// Create the primary table + all indexes. Idempotent via `IF NOT EXISTS`.
    static func ensureTables(db: OpaquePointer) throws {
        try createMainTable(db: db)
        createIndexes(db: db)
    }

    /// Check whether a column exists on `custom_dictionary`.
    /// Public so `CustomDictionaryMigrator` can gate `ALTER TABLE` calls.
    static func columnExists(db: OpaquePointer, column: String) -> Bool {
        let sql = "SELECT COUNT(*) FROM pragma_table_info('\(tableName)') WHERE name = ?;"
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return false }
        defer { sqlite3_finalize(stmt) }
        stmt.bindText(1, column)
        return sqlite3_step(stmt) == SQLITE_ROW && sqlite3_column_int(stmt, 0) > 0
    }

    // MARK: - Private

    private static func createMainTable(db: OpaquePointer) throws {
        let sql = """
            CREATE TABLE IF NOT EXISTS \(tableName) (
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
            throw LexiconError.queryPreparationFailed("Create \(tableName) failed: \(String(cString: sqlite3_errmsg(db)))")
        }
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_step(stmt) == SQLITE_DONE else {
            throw LexiconError.queryExecutionFailed("Create \(tableName) failed: \(String(cString: sqlite3_errmsg(db)))")
        }
    }

    private static func createIndexes(db: OpaquePointer) {
        for sql in [
            "CREATE INDEX IF NOT EXISTS idx_custom_roman ON \(tableName)(roman);",
            "CREATE INDEX IF NOT EXISTS idx_custom_notone ON \(tableName)(notone);",
            "CREATE INDEX IF NOT EXISTS idx_custom_abbrev ON \(tableName)(abbrev);",
            "CREATE INDEX IF NOT EXISTS idx_custom_roman_num ON \(tableName)(roman_num);",
        ] {
            sqliteExecSimple(db: db, sql)
        }
    }
}
