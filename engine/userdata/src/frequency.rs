//! How often the user has committed each word, and how recently. Port of
//! `Storage/UserFrequencyStore.swift`; SQL byte-identical.

use crate::capacity::LearningCapacity;
use crate::database::{
    has_column, immediate_transaction, table_exists, JournalMode, StoreSchema, UserDataDatabase,
    UserDataDatabaseError,
};
use crate::stores::FrequencySource;
use crate::types::FrequencyRow;
use rusqlite::{params, params_from_iter, Connection};
use std::path::PathBuf;

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
/// hands it counts. A word is the `(Hanji, canonical TL)` PAIR (Core Principle
/// #7): 重/tîng and 重/tāng are different rows.
pub struct UserFrequencyStore {
    database: UserDataDatabase,
    capacity: LearningCapacity,
}

impl UserFrequencyStore {
    /// Ported from
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
    pub fn new(path: PathBuf, journal: JournalMode, capacity: LearningCapacity) -> Self {
        Self {
            database: UserDataDatabase::new(
                "UserFrequencyStore",
                path,
                journal,
                StoreSchema {
                    apply: apply_schema,
                    max_known_version: SCHEMA_VERSION,
                    marks_takeover_on_open: true,
                },
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

    /// Merges a backup's counts: a `(word, tl)` already stored keeps the
    /// larger count, and every imported row counts as used now — the phones'
    /// `.taigi` merge (iOS `UserFrequencyRepository.batchImportMerge`). One
    /// transaction, then the cap, so no reader sees the table over it.
    /// Answers how many rows were merged.
    pub fn import_merge(
        &self,
        rows: Vec<(String, String, i64)>,
    ) -> Result<usize, UserDataDatabaseError> {
        let capacity = self.capacity_for_writer();
        self.database.perform(move |connection| {
            immediate_transaction::<_, UserDataDatabaseError>(connection, |connection| {
                let mut statement = connection.prepare(&format!(
                    "INSERT INTO {TABLE_NAME} (word, tl, count, last_used)\nVALUES (?, ?, ?, CURRENT_TIMESTAMP)\nON CONFLICT(word, tl) DO UPDATE SET\n    count = MAX(count, excluded.count),\n    last_used = CURRENT_TIMESTAMP;"
                ))?;
                let mut merged = 0;
                for (word, tl, count) in rows.iter().filter(|(word, ..)| !word.is_empty()) {
                    statement.execute(params![word, tl, count])?;
                    merged += 1;
                }
                capacity.enforce(connection)?;
                Ok(merged)
            })
        })
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

/// The current shape: a pre-pair-key table rebuilt first, then created
/// directly when absent. Stamps 2, the number iOS and Android already use.
fn apply_schema(connection: &Connection) -> rusqlite::Result<()> {
    migrate_to_pair_key_if_needed(connection)?;
    connection.execute_batch(&table_ddl(TABLE_NAME))?;
    // No index on `word` alone: the UNIQUE constraint's automatic index
    // already has it as the leftmost column, so it serves `WHERE word IN`.
    connection.execute_batch(&format!(
        "CREATE INDEX IF NOT EXISTS idx_count ON {TABLE_NAME}(count DESC);\nCREATE INDEX IF NOT EXISTS idx_last_used ON {TABLE_NAME}(last_used DESC);"
    ))?;
    connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

fn table_ddl(name: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {name} (\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    word TEXT NOT NULL,\n    tl TEXT NOT NULL DEFAULT '',\n    count INTEGER DEFAULT 1,\n    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,\n    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,\n    UNIQUE(word, tl)\n);"
    )
}

/// Rebuilds a pre-pair-key table (inline `word UNIQUE`, no `tl` column —
/// iOS / Android before R5) into the `(word, tl)` shape: `tl = ''` for every
/// old row, `id` / `count` / `last_used` / `created_at` kept. SQLite cannot
/// drop an inline UNIQUE, so create-new / copy / drop / rename in one
/// immediate transaction (the old `idx_word` goes with the old table).
/// Gated on the SHAPE, not `user_version`: iOS can leave a new-shape file at
/// 0. Ported from iOS `UserFrequencySchema.swift` `migrateToPairKeyIfNeeded` and
/// Android `UserFrequencyService.kt` `migrateToPairKey`.
fn migrate_to_pair_key_if_needed(connection: &Connection) -> rusqlite::Result<()> {
    let is_pre_pair_key = |connection: &Connection| -> rusqlite::Result<bool> {
        Ok(table_exists(connection, TABLE_NAME)? && !has_column(connection, TABLE_NAME, "tl")?)
    };
    if !is_pre_pair_key(connection)? {
        return Ok(());
    }
    immediate_transaction::<_, rusqlite::Error>(connection, |connection| {
        // Another process may have rebuilt it between the check and the lock.
        if !is_pre_pair_key(connection)? {
            return Ok(());
        }
        let staging = format!("{TABLE_NAME}_pairkey_migrate");
        connection.execute_batch(&table_ddl(&staging))?;
        connection.execute_batch(&format!(
            "INSERT INTO {staging} (id, word, tl, count, last_used, created_at)\n    SELECT id, word, '', count, last_used, created_at FROM {TABLE_NAME};\nDROP TABLE {TABLE_NAME};\nALTER TABLE {staging} RENAME TO {TABLE_NAME};"
        ))
    })
}
