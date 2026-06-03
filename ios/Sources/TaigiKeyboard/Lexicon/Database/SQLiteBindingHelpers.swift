// 中文: SQLite C API 共用包裝 — bindText 與 sqliteExecSimple,給 lexicon 內各 repository 使用。

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
// 中文: 一次性執行無綁定的 DDL / PRAGMA。錯誤會被靜默吞掉,需要錯誤處理的呼叫方
// 中文: 必須直接用 sqlite3_prepare_v2。
func sqliteExecSimple(db: OpaquePointer, _ sql: String) {
    var stmt: OpaquePointer?
    if sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK {
        sqlite3_step(stmt)
        sqlite3_finalize(stmt)
    }
}

// MARK: - Schema introspection (shared across schema / migrator files)

/// Read `PRAGMA user_version` (the integer schema-version gate). Returns `0`
/// for a fresh DB or on any prepare/step failure.
// 中文: 讀 PRAGMA user_version(schema 版本閘門);全新 DB 或失敗回 0。
func sqliteReadUserVersion(db: OpaquePointer) -> Int {
    var stmt: OpaquePointer?
    defer { sqlite3_finalize(stmt) }
    guard sqlite3_prepare_v2(db, "PRAGMA user_version", -1, &stmt, nil) == SQLITE_OK else { return 0 }
    return sqlite3_step(stmt) == SQLITE_ROW ? Int(sqlite3_column_int(stmt, 0)) : 0
}

/// Write `PRAGMA user_version = N`. PRAGMA cannot bind values; `version` is a
/// caller-supplied schema constant, never user input.
// 中文: 寫 PRAGMA user_version = N;PRAGMA 不能綁定,version 為呼叫端 schema 常數。
func sqliteSetUserVersion(db: OpaquePointer, version: Int) {
    sqliteExecSimple(db: db, "PRAGMA user_version = \(version)")
}

/// `true` when a table named `table` exists.
// 中文: 表是否存在。
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
// 中文: 表是否有某欄。table 為硬編 schema 常數(非使用者輸入)。
func sqliteColumnExists(db: OpaquePointer, table: String, column: String) -> Bool {
    let sql = "SELECT COUNT(*) FROM pragma_table_info('\(table)') WHERE name = ?;"
    var stmt: OpaquePointer?
    defer { sqlite3_finalize(stmt) }
    guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return false }
    stmt.bindText(1, column)
    return sqlite3_step(stmt) == SQLITE_ROW && sqlite3_column_int(stmt, 0) > 0
}
