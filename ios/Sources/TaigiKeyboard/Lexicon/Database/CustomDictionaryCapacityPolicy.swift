// 自訂詞庫容量上限策略 — 在 SQLiteConnectionManager 序列化區段內檢查 row count
// 與 row 是否已存在,提供 TOCTOU 安全的 insert guard。

import Foundation
import SQLite3

/// Capacity policy for the custom-dictionary table.
///
/// Owns the row-count cap (`maxEntries`), aligned with Android's
/// `MAX_ENTRIES`, and the helpers needed to enforce it inside a
/// serialized `SQLiteConnectionManager.execute { db in ... }` block.
/// Stateless — all methods operate on a caller-provided `OpaquePointer`.
// 自訂詞庫的容量限制策略 — stateless,所有方法吃外部傳入的 OpaquePointer。
enum CustomDictionaryCapacityPolicy {
    /// Maximum number of rows.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/CustomDictionaryCapacityPolicy.kt (MAX_ENTRIES).
    // Drift causes silent divergence.
    static let maxEntries = 30000

    /// True when a row with the given `id` already exists.
    // 指定 id 的 row 是否已存在。
    static func entryExists(db: OpaquePointer, id: String) -> Bool {
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(
            db,
            "SELECT 1 FROM \(CustomDictionarySchema.tableName) WHERE id = ? LIMIT 1;",
            -1, &stmt, nil,
        ) == SQLITE_OK else {
            return false
        }
        stmt.bindText(1, id)
        return sqlite3_step(stmt) == SQLITE_ROW
    }

    /// Current row count. Returns 0 on prepare failure so the caller can
    /// treat connection problems as "not full" — subsequent writes will
    /// surface the underlying error.
    // 目前 row 數。prepare 失敗回 0,讓 caller 把連線問題當成 "not full",
    // 真正的錯誤交給後續寫入呼叫去暴露。
    static func currentEntryCount(db: OpaquePointer) -> Int {
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(
            db,
            "SELECT COUNT(*) FROM \(CustomDictionarySchema.tableName);",
            -1, &stmt, nil,
        ) == SQLITE_OK else {
            return 0
        }
        return sqlite3_step(stmt) == SQLITE_ROW ? Int(sqlite3_column_int(stmt, 0)) : 0
    }

    /// Throw if inserting would exceed `maxEntries`. Updating an existing
    /// row (same `id`) is not an insert and bypasses the check.
    /// Must run inside the same transaction as the write to avoid TOCTOU.
    // 插入前的容量 guard — 必須與寫入在同一個 transaction,避免 TOCTOU。
    // 更新既存 row(id 已存在)不算 insert,直接跳過檢查。
    static func guardInsertCapacity(db: OpaquePointer, id: String) throws {
        if entryExists(db: db, id: id) { return }
        guard currentEntryCount(db: db) < maxEntries else {
            throw LexiconError.queryExecutionFailed(
                "Custom dictionary is full (max \(maxEntries) entries)",
            )
        }
    }

    /// Remaining capacity headroom. Negative values clamp to 0.
    // 剩餘容量,負值 clamp 為 0。
    static func remainingCapacity(db: OpaquePointer) -> Int {
        max(0, maxEntries - currentEntryCount(db: db))
    }
}
