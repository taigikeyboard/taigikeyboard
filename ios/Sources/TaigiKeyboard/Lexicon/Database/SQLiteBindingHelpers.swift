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
