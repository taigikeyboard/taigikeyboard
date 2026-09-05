// SQLite C API 共用包裝 — bindText 與 sqliteExecSimple,給 lexicon 內各 repository 使用。

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
// 一次性執行無綁定的 DDL / PRAGMA。錯誤會被靜默吞掉,需要錯誤處理的呼叫方
// 必須直接用 sqlite3_prepare_v2。
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
// 與 sqliteExecSimple 相對的「會丟錯」版本;migration 必須用這個,
// 否則失敗被吞掉後版本號照蓋,DB 狀態與版本號說法不符。
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
// 會丟錯的單值查詢。migration 的判斷要用這個:查詢失敗回 0/false 會讓分支走錯,
// 且錯誤會被吞掉看不出來。
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
// 讀 PRAGMA user_version(schema 版本閘門);全新 DB 或失敗回 0。
func sqliteReadUserVersion(db: OpaquePointer) -> Int {
    var stmt: OpaquePointer?
    defer { sqlite3_finalize(stmt) }
    guard sqlite3_prepare_v2(db, "PRAGMA user_version", -1, &stmt, nil) == SQLITE_OK else { return 0 }
    return sqlite3_step(stmt) == SQLITE_ROW ? Int(sqlite3_column_int(stmt, 0)) : 0
}

/// Write `PRAGMA user_version = N`. PRAGMA cannot bind values; `version` is a
/// caller-supplied schema constant, never user input.
// 寫 PRAGMA user_version = N;PRAGMA 不能綁定,version 為呼叫端 schema 常數。
func sqliteSetUserVersion(db: OpaquePointer, version: Int) {
    sqliteExecSimple(db: db, "PRAGMA user_version = \(version)")
}

/// `true` when a table named `table` exists.
// 表是否存在。
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
// 表是否有某欄。table 為硬編 schema 常數(非使用者輸入)。
func sqliteColumnExists(db: OpaquePointer, table: String, column: String) -> Bool {
    let sql = "SELECT COUNT(*) FROM pragma_table_info('\(table)') WHERE name = ?;"
    var stmt: OpaquePointer?
    defer { sqlite3_finalize(stmt) }
    guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return false }
    stmt.bindText(1, column)
    return sqlite3_step(stmt) == SQLITE_ROW && sqlite3_column_int(stmt, 0) > 0
}
