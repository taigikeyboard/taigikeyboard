import Foundation
import SQLite3

/// Capacity policy for the custom-dictionary table.
///
/// Owns the row-count cap (`maxEntries`), aligned with Android's
/// `MAX_ENTRIES`, and the helpers needed to enforce it inside a
/// serialized `SQLiteConnectionManager.execute { db in ... }` block.
/// Stateless — all methods operate on a caller-provided `OpaquePointer`.
enum CustomDictionaryCapacityPolicy {
    /// Maximum number of rows.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/CustomDictionaryCapacityPolicy.kt (MAX_ENTRIES).
    // Drift causes silent divergence.
    static let maxEntries = 30000

    /// True when a row with the given `id` already exists.
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
    static func currentEntryCount(db: OpaquePointer) -> Int {
        (try? sqliteQueryScalarInt(db: db, "SELECT COUNT(*) FROM \(CustomDictionarySchema.tableName);")) ?? 0
    }

    /// Throw if inserting would exceed `maxEntries`. Updating an existing
    /// row (same `id`) is not an insert and bypasses the check.
    /// Must run inside the same transaction as the write to avoid TOCTOU.
    static func guardInsertCapacity(db: OpaquePointer, id: String) throws {
        if entryExists(db: db, id: id) { return }
        guard currentEntryCount(db: db) < maxEntries else {
            throw LexiconError.queryExecutionFailed(
                "Custom dictionary is full (max \(maxEntries) entries)",
            )
        }
    }

    /// Remaining capacity headroom. Negative values clamp to 0.
    static func remainingCapacity(db: OpaquePointer) -> Int {
        max(0, maxEntries - currentEntryCount(db: db))
    }
}
