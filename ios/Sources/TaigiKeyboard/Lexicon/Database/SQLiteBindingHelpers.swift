import Foundation
import SQLite3

/// Shared helpers for `sqlite3_*` boilerplate used by SQLite repositories
/// in this module.
extension OpaquePointer? {
    /// Bind a UTF-8 text parameter at `index`, copying the bytes (TRANSIENT).
    func bindText(_ index: Int32, _ value: String) {
        sqlite3_bind_text(self, index, value, -1, SQLiteConnectionManager.sqliteTransient)
    }
}

/// One-shot prepare/step/finalize for DDL or PRAGMA statements that take no
/// bindings and whose return code is not consulted (CREATE INDEX IF NOT EXISTS,
/// ALTER TABLE, PRAGMA user_version = N, etc.).
///
/// Errors are silently ignored — callers requiring success/failure semantics
/// must use `sqlite3_prepare_v2` directly.
func sqliteExecSimple(db: OpaquePointer, _ sql: String) {
    var stmt: OpaquePointer?
    if sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK {
        sqlite3_step(stmt)
        sqlite3_finalize(stmt)
    }
}

/// Prepare/step/finalize for a statement that takes no bindings, throwing on
/// failure. The counterpart to `sqliteExecSimple` for callers — migrations
/// above all — where a silently-ignored error would leave the database in a
/// state the version stamp then lies about.
func sqliteExecChecked(db: OpaquePointer, _ sql: String) throws {
    var stmt: OpaquePointer?
    defer { sqlite3_finalize(stmt) }
    guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
        throw LexiconError.queryPreparationFailed(String(cString: sqlite3_errmsg(db)))
    }
    guard sqlite3_step(stmt) == SQLITE_DONE else {
        throw LexiconError.queryExecutionFailed(String(cString: sqlite3_errmsg(db)))
    }
}

/// Run a single-value query, throwing on failure. The counterpart to the
/// introspection helpers below for callers — migrations again — where a
/// question answered `0` / `false` because the query itself failed would
/// silently drive the wrong branch.
func sqliteQueryScalarInt(db: OpaquePointer, _ sql: String) throws -> Int {
    var stmt: OpaquePointer?
    defer { sqlite3_finalize(stmt) }
    guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
        throw LexiconError.queryPreparationFailed(String(cString: sqlite3_errmsg(db)))
    }
    guard sqlite3_step(stmt) == SQLITE_ROW else {
        throw LexiconError.queryExecutionFailed(String(cString: sqlite3_errmsg(db)))
    }
    return Int(sqlite3_column_int(stmt, 0))
}

// MARK: - Schema introspection (shared across schema / migrator files)

/// Read `PRAGMA user_version` (the integer schema-version gate). Returns `0`
/// for a fresh DB or on any prepare/step failure.
func sqliteReadUserVersion(db: OpaquePointer) -> Int {
    var stmt: OpaquePointer?
    defer { sqlite3_finalize(stmt) }
    guard sqlite3_prepare_v2(db, "PRAGMA user_version", -1, &stmt, nil) == SQLITE_OK else { return 0 }
    return sqlite3_step(stmt) == SQLITE_ROW ? Int(sqlite3_column_int(stmt, 0)) : 0
}

/// Write `PRAGMA user_version = N`. PRAGMA cannot bind values; `version` is a
/// caller-supplied schema constant, never user input.
func sqliteSetUserVersion(db: OpaquePointer, version: Int) {
    sqliteExecSimple(db: db, "PRAGMA user_version = \(version)")
}

/// `true` when a table named `table` exists.
func sqliteTableExists(db: OpaquePointer, table: String) -> Bool {
    var stmt: OpaquePointer?
    defer { sqlite3_finalize(stmt) }
    let sql = "SELECT 1 FROM sqlite_master WHERE type='table' AND name=? LIMIT 1;"
    guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return false }
    stmt.bindText(1, table)
    return sqlite3_step(stmt) == SQLITE_ROW
}

/// `true` when `table` has a column named `column`. `table` is a hardcoded
/// schema constant (not user input) — `pragma_table_info` cannot bind an
/// identifier.
func sqliteColumnExists(db: OpaquePointer, table: String, column: String) -> Bool {
    let sql = "SELECT COUNT(*) FROM pragma_table_info('\(table)') WHERE name = ?;"
    var stmt: OpaquePointer?
    defer { sqlite3_finalize(stmt) }
    guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return false }
    stmt.bindText(1, column)
    return sqlite3_step(stmt) == SQLITE_ROW && sqlite3_column_int(stmt, 0) > 0
}
