// How a learning table stops growing without bound.

import Foundation

/// The row cap of one learning table, and the throttle that decides when to
/// check it.
///
/// Shared by both stores because the policy is the same one twice over: count
/// the rows now and then, and when there are too many, drop the least useful.
/// Only the table and the numbers differ.
///
/// "Least useful" is low count first, then stale — the order iOS and Android
/// both delete in, so the three platforms forget the same rows.
/// CROSS-PLATFORM INVARIANT — mirrors
/// ios/Sources/TaigiKeyboard/Lexicon/Database/UserFrequencyPruner.swift:66-78.
///
/// `@unchecked Sendable` covers the throttle counter, which has its own lock;
/// the rest is immutable.
final class LearningCapacity: @unchecked Sendable {
    /// The table name, which is interpolated into SQL. Every caller passes a
    /// literal it owns — no value that came from outside the process reaches
    /// this, which is what keeps the interpolation safe
    /// (`.claude/rules/security-rules.md` § SQL: identifiers need a hardcoded
    /// whitelist, values need binding).
    private let table: String
    private let maxRows: Int
    /// Deleting exactly down to the cap would re-trigger on the next write.
    /// Over-deleting leaves room to grow back into.
    private let deleteBatch: Int
    private let recordsBetweenChecks: Int

    private let counterLock = NSLock()
    private var recordsSinceCheck = 0

    init(table: String, maxRows: Int, deleteBatch: Int, recordsBetweenChecks: Int = 100) {
        self.table = table
        self.maxRows = maxRows
        self.deleteBatch = deleteBatch
        self.recordsBetweenChecks = recordsBetweenChecks
    }

    /// Counts `records` towards the next check and answers whether this write
    /// is the one that should run it. Counting a whole compound commit at once
    /// keeps the check on a schedule of rows written rather than of calls made.
    func shouldEnforce(after records: Int) -> Bool {
        counterLock.withLock {
            recordsSinceCheck += records
            guard recordsSinceCheck >= recordsBetweenChecks else { return false }
            recordsSinceCheck = 0
            return true
        }
    }

    /// Drops the least-useful rows if the table is over its cap. Runs on the
    /// store's queue, inside whatever transaction the caller has open.
    func enforce(_ connection: SQLiteConnection) throws {
        guard let rowCount = try connection.scalar("SELECT COUNT(*) FROM \(table);"),
              rowCount > maxRows
        else { return }

        try connection.run(
            """
            DELETE FROM \(table) WHERE id IN (
                SELECT id FROM \(table)
                ORDER BY count ASC, last_used ASC
                LIMIT ?
            );
            """,
            [.integer(min(deleteBatch, rowCount - maxRows + deleteBatch))],
        )
    }
}
