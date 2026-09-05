//! How often the user has committed each word, and how recently. Port of
//! `Storage/UserFrequencyStore.swift`; SQL byte-identical.

// 詞頻資料庫 — (漢字, canonical TL) 為鍵,引擎排序,這裡只交出計數。

use crate::capacity::LearningCapacity;
use crate::database::{UserDataDatabase, UserDataDatabaseError};
use rusqlite::{params, params_from_iter, Connection};
use std::path::PathBuf;
use taigi_windows_core::composing::FrequencySource;
use taigi_windows_core::engine::FrequencyRow;

const TABLE_NAME: &str = "user_frequency";
const SCHEMA_VERSION: i64 = 2;
/// The columns every `FrequencyRow` read selects, next to the decoder that
/// reads them: the two agree by position.
const ROW_COLUMNS: &str = "word, tl, count, CAST(strftime('%s', last_used) AS INTEGER) * 1000";
/// One order, most-used first, `(word, tl)` as the final tie-break so equal
/// counts keep a stable order between two reads.
const LIST_ORDER: &str = "count DESC, last_used DESC, word ASC, tl ASC";

/// Records candidate commits and answers what the ranker should boost. The
/// engine does the ranking (`engine/ranking/src/score.rs`); this store only
/// hands it counts. A word is the `(漢字, canonical TL)` PAIR (Core Principle
/// #7): 重/tîng and 重/tāng are different rows.
pub struct UserFrequencyStore {
    database: UserDataDatabase,
    capacity: LearningCapacity,
}

impl UserFrequencyStore {
    /// CROSS-PLATFORM INVARIANT — mirrors
    /// `ios/.../Lexicon/Database/UserFrequencyPruner.swift:26-38`.
    pub fn shipped_capacity() -> LearningCapacity {
        LearningCapacity::new(
            TABLE_NAME,
            20_000,
            2_000,
            LearningCapacity::DEFAULT_RECORDS_BETWEEN_CHECKS,
        )
    }

    /// `capacity` is a parameter so a test can hand in a small cap and watch
    /// a real `record` call prune.
    pub fn new(directory: PathBuf, capacity: LearningCapacity) -> Self {
        Self {
            database: UserDataDatabase::new(
                "user_frequency.db",
                "UserFrequencyStore",
                directory,
                apply_schema,
            ),
            capacity,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.database.is_ready()
    }

    pub fn open(&self) {
        self.database.open();
    }

    pub fn open_blocking(&self) {
        self.database.open_blocking();
    }

    /// Counts one commit of `word` under reading `tl`. `tl` may be empty —
    /// a candidate with no canonical reading lands in its own row.
    pub fn record(&self, word: &str, tl: &str) {
        if word.is_empty() {
            return;
        }
        let should_check_capacity = self.capacity.should_enforce(1);
        let (word, tl) = (word.to_owned(), tl.to_owned());
        // The capacity's SQL needs no state from `self`; the check itself is
        // rebuilt on the writer thread from the same literal numbers.
        let capacity = if should_check_capacity {
            Some(self.capacity_for_writer())
        } else {
            None
        };
        self.database.write(move |connection| {
            connection.execute(
                &format!(
                    "INSERT INTO {TABLE_NAME} (word, tl, count, last_used)\nVALUES (?, ?, 1, CURRENT_TIMESTAMP)\nON CONFLICT(word, tl) DO UPDATE SET\n    count = count + 1,\n    last_used = CURRENT_TIMESTAMP;"
                ),
                params![word, tl],
            )?;
            if let Some(capacity) = capacity {
                capacity.enforce(connection)?;
            }
            Ok(())
        });
    }

    fn capacity_for_writer(&self) -> LearningCapacity {
        self.capacity.detached()
    }

    /// Every learned row for any of `words`, or `None` when the store could
    /// not be read.
    pub fn rows_for_words(&self, words: &[String]) -> Option<Vec<FrequencyRow>> {
        if words.is_empty() {
            return Some(Vec::new());
        }
        let placeholders = vec!["?"; words.len()].join(",");
        self.database.read(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {ROW_COLUMNS} FROM {TABLE_NAME}\nWHERE word IN ({placeholders});"
            ))?;
            let rows = statement.query_map(params_from_iter(words.iter()), decode_row)?;
            rows.collect()
        })
    }

    /// Every learned row, most-used first. Exists for the tests and the
    /// export: the capacity ceiling and `delete_all` are only assertable
    /// against the table's actual contents.
    pub fn all_rows(&self) -> Option<Vec<FrequencyRow>> {
        self.database.wait_for_queued_writes();
        self.database.read(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {ROW_COLUMNS} FROM {TABLE_NAME}\nORDER BY {LIST_ORDER};"
            ))?;
            let rows = statement.query_map([], decode_row)?;
            rows.collect()
        })
    }

    /// Forgets everything, and reports how many rows went.
    pub fn delete_all(&self) -> Result<i64, UserDataDatabaseError> {
        self.database.perform(|connection| {
            let existing: i64 = connection.query_row(
                &format!("SELECT COUNT(*) FROM {TABLE_NAME};"),
                [],
                |row| row.get(0),
            )?;
            connection.execute(&format!("DELETE FROM {TABLE_NAME};"), [])?;
            connection.execute("VACUUM;", []).ok();
            Ok(existing)
        })
    }
}

impl FrequencySource for UserFrequencyStore {
    fn rows_for_words(&self, words: &[String]) -> Option<Vec<FrequencyRow>> {
        UserFrequencyStore::rows_for_words(self, words)
    }

    fn record(&self, word: &str, tl: &str) {
        UserFrequencyStore::record(self, word, tl);
    }
}

fn decode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FrequencyRow> {
    Ok(FrequencyRow {
        word: row.get(0)?,
        tl: row.get(1)?,
        count: row.get(2)?,
        last_used_ms: row.get(3)?,
    })
}

/// The current shape, created directly. iOS carries a migrator from a
/// single-`word` unique key; this platform has never shipped the older
/// shape, so there is nothing to migrate.
fn apply_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {TABLE_NAME} (\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    word TEXT NOT NULL,\n    tl TEXT NOT NULL DEFAULT '',\n    count INTEGER DEFAULT 1,\n    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,\n    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,\n    UNIQUE(word, tl)\n);\nCREATE INDEX IF NOT EXISTS idx_count ON {TABLE_NAME}(count DESC);\nCREATE INDEX IF NOT EXISTS idx_last_used ON {TABLE_NAME}(last_used DESC);"
    ))?;
    // No index on `word` alone: the UNIQUE constraint's automatic index
    // already has it as the leftmost column, so it serves `WHERE word IN`.
    connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}
