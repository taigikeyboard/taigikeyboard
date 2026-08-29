//! How a learning table stops growing without bound. Port of
//! `Storage/LearningCapacity.swift`.

// 中文: 學習表的列數上限與檢查節流;刪最沒用的列(次數低、久未用)。

use rusqlite::{params, Connection};
use std::sync::Mutex;

/// The row cap of one learning table, and the throttle that decides when to
/// check it. "Least useful" is low count first, then stale — the order iOS
/// and Android both delete in, so the platforms forget the same rows.
/// CROSS-PLATFORM INVARIANT — mirrors
/// `ios/Sources/TaigiKeyboard/Lexicon/Database/UserFrequencyPruner.swift:66-78`.
pub struct LearningCapacity {
    /// Interpolated into SQL. Every caller passes a literal it owns — no
    /// value from outside the process reaches this (`security-rules.md` § SQL).
    table: &'static str,
    max_rows: i64,
    /// Deleting exactly down to the cap would re-trigger on the next write;
    /// over-deleting leaves room to grow back into.
    delete_batch: i64,
    records_between_checks: usize,
    records_since_check: Mutex<usize>,
}

impl LearningCapacity {
    pub const DEFAULT_RECORDS_BETWEEN_CHECKS: usize = 100;

    pub fn new(
        table: &'static str,
        max_rows: i64,
        delete_batch: i64,
        records_between_checks: usize,
    ) -> Self {
        Self {
            table,
            max_rows,
            delete_batch,
            records_between_checks,
            records_since_check: Mutex::new(0),
        }
    }

    /// Counts `records` towards the next check and answers whether this
    /// write is the one that should run it. Counting a whole compound commit
    /// at once keeps the check on a schedule of rows written, not calls made.
    pub fn should_enforce(&self, records: usize) -> bool {
        let mut since = self
            .records_since_check
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *since += records;
        if *since < self.records_between_checks {
            return false;
        }
        *since = 0;
        true
    }

    /// A copy with the same numbers and a fresh throttle — what a queued
    /// write carries to the writer thread, which only calls `enforce`.
    pub fn detached(&self) -> Self {
        Self::new(
            self.table,
            self.max_rows,
            self.delete_batch,
            self.records_between_checks,
        )
    }

    /// Drops the least-useful rows if the table is over its cap. Runs inside
    /// whatever transaction the caller has open.
    pub fn enforce(&self, connection: &Connection) -> rusqlite::Result<()> {
        let row_count: i64 = connection.query_row(
            &format!("SELECT COUNT(*) FROM {};", self.table),
            [],
            |row| row.get(0),
        )?;
        if row_count <= self.max_rows {
            return Ok(());
        }
        connection.execute(
            &format!(
                "DELETE FROM {table} WHERE id IN (\n    SELECT id FROM {table}\n    ORDER BY count ASC, last_used ASC\n    LIMIT ?\n);",
                table = self.table
            ),
            params![self.delete_batch.min(row_count - self.max_rows + self.delete_batch)],
        )?;
        Ok(())
    }
}
