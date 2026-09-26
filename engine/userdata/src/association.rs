//! Which word the user tends to type after which. Port of
//! `Storage/UserAssociationStore.swift`; SQL byte-identical.

use crate::capacity::LearningCapacity;
use crate::database::{
    has_column, immediate_transaction, table_exists, user_version, JournalMode, StoreSchema,
    UserDataDatabase, UserDataDatabaseError,
};
use crate::stores::AssociationSink;
use crate::types::AssociationPair;
use rusqlite::{params, Connection};
use std::path::PathBuf;

pub(crate) const FILE_NAME: &str = "user_association.db";
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

/// One word learned after a previous one, as a prediction reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FollowingRow {
    pub next: String,
    pub next_tl: String,
    pub count: i64,
    pub last_used_ms: i64,
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

    pub fn new(path: PathBuf, journal: JournalMode, capacity: LearningCapacity) -> Self {
        Self {
            database: UserDataDatabase::new(
                "UserAssociationStore",
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

    /// Merges a backup's bigrams: a pair already stored keeps the larger
    /// count, every imported one counts as used now — the phones' `.taigi`
    /// merge (iOS `NextWordRepository.batchImportAssociations`). One
    /// transaction, then the cap. Answers how many pairs were merged.
    pub fn import_merge(&self, rows: Vec<AssociationRow>) -> Result<usize, UserDataDatabaseError> {
        let capacity = self.capacity.detached();
        self.database.perform(move |connection| {
            immediate_transaction::<_, UserDataDatabaseError>(connection, |connection| {
                let mut statement = connection.prepare(&format!(
                    "INSERT INTO {TABLE_NAME}\n    (prev_word, prev_tl, next_word, next_tl, count, last_used)\nVALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)\nON CONFLICT(prev_word, prev_tl, next_word, next_tl) DO UPDATE SET\n    count = MAX(count, excluded.count),\n    last_used = CURRENT_TIMESTAMP;"
                ))?;
                let mut merged = 0;
                for AssociationRow { pair, count } in rows
                    .iter()
                    .filter(|row| !row.pair.previous.is_empty() && !row.pair.next.is_empty())
                {
                    statement.execute(params![
                        pair.previous,
                        pair.previous_tl,
                        pair.next,
                        pair.next_tl,
                        count
                    ])?;
                    merged += 1;
                }
                capacity.enforce(connection)?;
                Ok(merged)
            })
        })
    }

    /// The words learned after `previous`, best evidence first, at most
    /// `limit`; `None` when the store could not be read (not open, busy).
    ///
    /// **Row order is load-bearing** (behavioral-invariants §24): rows whose
    /// `prev_tl` equals `previous_tl` first, then untagged ones, then the
    /// other readings of the same Hanji — never dropped — each tier by count,
    /// recency, then `id`. The next-word filter keeps only the FIRST row per
    /// predicted `(hanzi, tl)`, so this order decides whose evidence counts,
    /// and `ORDER BY` ranks before `LIMIT` truncates. CROSS-PLATFORM
    /// INVARIANT — mirrors iOS `NextWordRepository.swift` `fetchUserRows` and
    /// Android `NextWordService.kt` `USER_PREDICT_SQL`.
    pub fn rows_following(
        &self,
        previous: &str,
        previous_tl: &str,
        limit: usize,
    ) -> Option<Vec<FollowingRow>> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        self.database.read(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT next_word, next_tl, count, CAST(strftime('%s', last_used) AS INTEGER) * 1000\nFROM {TABLE_NAME}\nWHERE prev_word = ?1\nORDER BY\n    CASE WHEN prev_tl = ?2 THEN 0 WHEN prev_tl = '' THEN 1 ELSE 2 END,\n    count DESC, last_used DESC, id ASC\nLIMIT ?3;"
            ))?;
            let rows = statement.query_map(params![previous, previous_tl, limit], |row| {
                Ok(FollowingRow {
                    next: row.get(0)?,
                    next_tl: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    count: row.get(2)?,
                    last_used_ms: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                })
            })?;
            rows.collect()
        })
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

/// Converges any known shape to v6. A v6 file (this engine, macOS, the
/// phones since v6) only re-asserts the `IF NOT EXISTS` DDL; an older one is
/// migrated, re-created and stamped in ONE immediate transaction, so a file
/// that says v6 always has the v6 table. Mirrors iOS
/// `NextWordSchema.swift` `ensureTables` and Android `NextWordService.kt`
/// `ensureUserAssocSchema`.
fn apply_schema(connection: &Connection) -> rusqlite::Result<()> {
    if user_version(connection)? >= SCHEMA_VERSION {
        return create_current(connection);
    }
    immediate_transaction::<_, rusqlite::Error>(connection, |connection| {
        // Another process may have migrated it between the check and the lock.
        let version = user_version(connection)?;
        if version < SCHEMA_VERSION {
            migrate(connection, version)?;
        }
        create_current(connection)
    })?;
    // Refresh the planner's statistics once, after the DDL; best-effort.
    connection.execute_batch("PRAGMA optimize;").ok();
    Ok(())
}

/// The unique key carries `prev_tl` because a word is the `(Hanji, canonical
/// TL)` pair on the bigram's PREVIOUS side as well (§24). `idx_user_prev` is
/// the name pre-convergence desktop builds gave the read index.
fn create_current(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(&table_ddl(TABLE_NAME))?;
    connection.execute_batch(&format!(
        "DROP INDEX IF EXISTS idx_user_prev;\nCREATE INDEX IF NOT EXISTS idx_user_prev_word_tl\n    ON {TABLE_NAME}(prev_word, prev_tl);"
    ))?;
    connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

fn table_ddl(name: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {name} (\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    prev_word TEXT NOT NULL,\n    prev_tl TEXT DEFAULT '',\n    next_word TEXT NOT NULL,\n    next_tl TEXT DEFAULT '',\n    count INTEGER DEFAULT 1,\n    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,\n    UNIQUE(prev_word, prev_tl, next_word, next_tl)\n);"
    )
}

/// Brings a pre-v6 table to the v6 shape, inside the caller's transaction.
/// v2 is dropped, as both phones do: its stamp is ambiguous (Android's
/// `migrateV0ToV2` table vs one written as v2). Any other old table that
/// carries the columns a row needs is rebuilt with its rows; one that does
/// not is dropped. That subsumes iOS v3–v5 and Android v0/v1 + v3–v5. The
/// old single-column `idx_user_prev_word` goes with its table either way.
/// NAMED DIVERGENCE (user-data-engine-roadmap U7): iOS dropped every table
/// below v3; one that has the columns is now kept, as Android keeps v0/v1.
fn migrate(connection: &Connection, version: i64) -> rusqlite::Result<()> {
    if !table_exists(connection, TABLE_NAME)? {
        return Ok(());
    }
    if version == 2 || !has_row_columns(connection)? {
        connection.execute_batch(&format!("DROP TABLE {TABLE_NAME};"))
    } else {
        rebuild_to_v6(connection)
    }
}

/// Rebuilds under the v6 key, every row kept. Widening a UNIQUE key cannot
/// conflict (the old key is a strict subset), so no merge step. `id` is
/// copied: it is the read query's final tie-break. A table without
/// `prev_tl` / `next_tl` (v3, Android's v0/v1 ladder) gets `''`; a NULL gets
/// `''`. Mirrors iOS / Android `rebuildToV6`.
fn rebuild_to_v6(connection: &Connection) -> rusqlite::Result<()> {
    let tl_column = |column: &str| -> rusqlite::Result<String> {
        Ok(if has_column(connection, TABLE_NAME, column)? {
            format!("COALESCE({column}, '')")
        } else {
            "''".to_owned()
        })
    };
    let prev_tl = tl_column("prev_tl")?;
    let next_tl = tl_column("next_tl")?;
    connection.execute_batch(&table_ddl(&format!("{TABLE_NAME}_new")))?;
    connection.execute_batch(&format!(
        "INSERT INTO {TABLE_NAME}_new\n    (id, prev_word, prev_tl, next_word, next_tl, count, last_used)\nSELECT id, prev_word, {prev_tl}, next_word, {next_tl}, count, last_used\nFROM {TABLE_NAME};\nDROP TABLE {TABLE_NAME};\nALTER TABLE {TABLE_NAME}_new RENAME TO {TABLE_NAME};"
    ))
}

/// The columns a rebuilt row is copied from.
fn has_row_columns(connection: &Connection) -> rusqlite::Result<bool> {
    for column in ["id", "prev_word", "next_word", "count", "last_used"] {
        if !has_column(connection, TABLE_NAME, column)? {
            return Ok(false);
        }
    }
    Ok(true)
}
