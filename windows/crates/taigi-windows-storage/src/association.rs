//! Which word the user tends to type after which. Port of
//! `Storage/UserAssociationStore.swift`; SQL byte-identical.

// 詞關聯(bigram)資料庫,兩端都帶 canonical TL;schema v6 與 iOS/Android 一致。

use crate::capacity::LearningCapacity;
use crate::database::{immediate_transaction, UserDataDatabase, UserDataDatabaseError};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use taigi_windows_core::composing::AssociationSink;
use taigi_windows_core::engine::AssociationPair;

const TABLE_NAME: &str = "user_association";
/// CROSS-PLATFORM INVARIANT — mirrors iOS `NextWordSchema.schemaVersion` and
/// Android `NextWordService.DATABASE_VERSION`.
const SCHEMA_VERSION: i64 = 6;
const ROW_COLUMNS: &str = "prev_word, prev_tl, next_word, next_tl, count";
const LIST_ORDER: &str = "count DESC, last_used DESC, prev_word ASC, next_word ASC";

/// A stored bigram and how many times it has been seen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssociationRow {
    pub pair: AssociationPair,
    pub count: i64,
}

/// Records the bigrams the engine decides are worth learning. Write-only on
/// the desktop today (no next-word surface); what it buys is that when one
/// lands it starts with the user's real history.
pub struct UserAssociationStore {
    database: UserDataDatabase,
    capacity: LearningCapacity,
}

impl UserAssociationStore {
    /// CROSS-PLATFORM INVARIANT — mirrors
    /// `ios/.../NextWord/Services/NextWordService.swift:31-33`.
    pub fn shipped_capacity() -> LearningCapacity {
        LearningCapacity::new(
            TABLE_NAME,
            50_000,
            5_000,
            LearningCapacity::DEFAULT_RECORDS_BETWEEN_CHECKS,
        )
    }

    pub fn new(directory: PathBuf, capacity: LearningCapacity) -> Self {
        Self {
            database: UserDataDatabase::new(
                "user_association.db",
                "UserAssociationStore",
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

    /// Records `pairs` in order, in one transaction: a compound word's
    /// bigrams collide on the UNIQUE key when a word repeats, and writing
    /// them sequentially is what makes the counts land in a defined state.
    pub fn record(&self, pairs: &[AssociationPair]) {
        let recordable: Vec<AssociationPair> = pairs
            .iter()
            .filter(|pair| !pair.previous.is_empty() && !pair.next.is_empty())
            .cloned()
            .collect();
        if recordable.is_empty() {
            return;
        }
        let capacity = if self.capacity.should_enforce(recordable.len()) {
            Some(self.capacity.detached())
        } else {
            None
        };
        self.database.write(move |connection| {
            immediate_transaction(connection, |connection| {
                for pair in &recordable {
                    connection.execute(
                        &format!(
                            "INSERT INTO {TABLE_NAME}\n    (prev_word, prev_tl, next_word, next_tl, count, last_used)\nVALUES (?, ?, ?, ?, 1, CURRENT_TIMESTAMP)\nON CONFLICT(prev_word, prev_tl, next_word, next_tl) DO UPDATE SET\n    count = count + 1,\n    last_used = CURRENT_TIMESTAMP;"
                        ),
                        params![pair.previous, pair.previous_tl, pair.next, pair.next_tl],
                    )?;
                }
                if let Some(capacity) = &capacity {
                    capacity.enforce(connection)?;
                }
                Ok(())
            })
        });
    }

    /// Every learned row with its count, most-used first, or `None` when
    /// the store could not be read. For the tests: the capacity ceiling and
    /// `delete_all` are only assertable against the table's contents.
    pub fn all_rows(&self) -> Option<Vec<AssociationRow>> {
        self.database.wait_for_queued_writes();
        self.database.read(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {ROW_COLUMNS} FROM {TABLE_NAME}\nORDER BY {LIST_ORDER};"
            ))?;
            let rows = statement.query_map([], |row| {
                Ok(AssociationRow {
                    pair: AssociationPair {
                        previous: row.get(0)?,
                        previous_tl: row.get(1)?,
                        next: row.get(2)?,
                        next_tl: row.get(3)?,
                    },
                    count: row.get(4)?,
                })
            })?;
            rows.collect()
        })
    }

    /// Forgets every bigram, and reports how many went.
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

impl AssociationSink for UserAssociationStore {
    fn record(&self, pairs: &[AssociationPair]) {
        UserAssociationStore::record(self, pairs);
    }
}

/// The unique key carries `prev_tl` because a word is the `(漢字, canonical
/// TL)` pair on the bigram's PREVIOUS side as well (§24). The `DROP INDEX`
/// is the one migration: pre-convergence builds named the same index
/// `idx_user_prev`.
fn apply_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {TABLE_NAME} (\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    prev_word TEXT NOT NULL,\n    prev_tl TEXT DEFAULT '',\n    next_word TEXT NOT NULL,\n    next_tl TEXT DEFAULT '',\n    count INTEGER DEFAULT 1,\n    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,\n    UNIQUE(prev_word, prev_tl, next_word, next_tl)\n);\nDROP INDEX IF EXISTS idx_user_prev;\nCREATE INDEX IF NOT EXISTS idx_user_prev_word_tl\n    ON {TABLE_NAME}(prev_word, prev_tl);"
    ))?;
    connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}
