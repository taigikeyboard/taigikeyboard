import Foundation
import SQLite3

/// Pruning policy + algorithm for `user_frequency`.
///
/// Stateless: the repository owns the throttle counter and the DB connection;
/// this type only answers "are we over capacity?" and "delete the least-useful
/// rows" against a caller-provided `OpaquePointer`. Selection order matches
/// Android: `count ASC, last_used ASC` — low-usage + stale rows go first.
enum UserFrequencyPruner {
    /// Soft cap on total rows.
    static let maxEntries = 20000
    /// Number of `recordWord` calls between prune checks.
    static let recordCheckInterval = 100
    /// Over-delete by this many rows so the table sits below the cap
    /// between prune runs instead of oscillating around it.
    static let pruneBatchSize = 2000

    /// Run pruning if the current row count exceeds capacity. Performs
    /// count + delete in the same `execute` block the caller passes in,
    /// so both reads are consistent and no second transaction is needed.
    /// Returns the number of rows actually deleted (0 when no pruning needed).
    @discardableResult
    static func pruneIfNeeded(db: OpaquePointer, logger: DebugLogger) -> Int {
        let currentCount = rowCount(db: db)
        guard currentCount > maxEntries else { return 0 }

        let deleteCount = min(
            pruneBatchSize,
            currentCount - maxEntries + pruneBatchSize,
        )
        deleteOldest(db: db, limit: deleteCount)
        logger.info("[PRUNE] Deleted \(deleteCount) frequency entries (was \(currentCount))")
        return deleteCount
    }

    // MARK: - SQL primitives

    /// Total row count, or -1 on prepare failure (matches the repository's
    /// existing `totalCount()` contract for a closed/broken connection).
    static func rowCount(db: OpaquePointer) -> Int {
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(
            db,
            "SELECT COUNT(*) FROM \(UserFrequencySchema.tableName);",
            -1, &stmt, nil,
        ) == SQLITE_OK else {
            return -1
        }
        return sqlite3_step(stmt) == SQLITE_ROW ? Int(sqlite3_column_int(stmt, 0)) : -1
    }

    private static func deleteOldest(db: OpaquePointer, limit: Int) {
        let sql = """
            DELETE FROM \(UserFrequencySchema.tableName)
            WHERE id IN (
                SELECT id FROM \(UserFrequencySchema.tableName)
                ORDER BY count ASC, last_used ASC
                LIMIT ?
            )
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return }
        defer { sqlite3_finalize(stmt) }
        sqlite3_bind_int(stmt, 1, Int32(limit))
        sqlite3_step(stmt)
    }
}
