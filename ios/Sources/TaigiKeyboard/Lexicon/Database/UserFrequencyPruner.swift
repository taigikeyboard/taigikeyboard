// 中文: user_frequency 表的 pruning 政策 — 軟上限、判斷頻率、批次刪除大小,
// 中文: 篩選順序與 Android 一致(count ASC, last_used ASC)。

import Foundation
import SQLite3

/// Pruning policy + algorithm for `user_frequency`.
///
/// Stateless: the repository owns the throttle counter and the DB connection;
/// this type only answers "are we over capacity?" and "delete the least-useful
/// rows" against a caller-provided `OpaquePointer`. Selection order matches
/// Android: `count ASC, last_used ASC` — low-usage + stale rows go first.
// 中文: stateless pruning 政策 — repository 持有節流計數器與 DB 連線,本類別只負責計算與 DELETE。
enum UserFrequencyPruner {
    /// Soft cap on total rows.
    // 中文: row 軟上限。
    static let maxEntries = 20000
    /// Number of `recordWord` calls between prune checks.
    // 中文: 每幾次 recordWord 檢查一次 prune,避免每次寫入都加成本。
    static let recordCheckInterval = 100
    /// Over-delete by this many rows so the table sits below the cap
    /// between prune runs instead of oscillating around it.
    // 中文: 多砍一些 row,讓表在下一次 prune 前能維持在上限以下,
    // 中文: 不會在 cap 附近震盪。
    static let pruneBatchSize = 2000

    /// Run pruning if the current row count exceeds capacity. Performs
    /// count + delete in the same `execute` block the caller passes in,
    /// so both reads are consistent and no second transaction is needed.
    /// Returns the number of rows actually deleted (0 when no pruning needed).
    // 中文: 超量才執行 prune。count + delete 在同一個 execute block 內,
    // 中文: 保證讀寫一致,不需要第二個 transaction。
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
    // 中文: 總 row 數,prepare 失敗回 -1 — 對齊 repository.totalCount() 對連線壞掉的契約。
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
