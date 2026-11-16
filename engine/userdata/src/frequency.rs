//! How often the user has committed each word, and how recently. Port of
//! the former Swift `UserFrequencyStore`; SQL byte-identical apart from the
//! `.taigi` v4 restore (`import_merge`, #1032).

use crate::capacity::LearningCapacity;
use crate::database::{
    has_column, immediate_transaction, migrate_to_version, normalize_khinsiann_rows,
    strip_khinsiann_if_changed, table_exists, JournalMode, StoreSchema, UserDataDatabase,
    UserDataDatabaseError,
};
use crate::timestamp::RESTORED_TIMESTAMP_SQL;
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection};
use std::path::PathBuf;

const TABLE_NAME: &str = "user_frequency";
const SCHEMA_VERSION: i64 = 3;
/// The columns every `FrequencyRow` read selects, next to the decoder that
/// reads them: the two agree by position.
const ROW_COLUMNS: &str = "word, tl, count, CAST(strftime('%s', last_used) AS INTEGER) * 1000, abbrev_count, COALESCE(CAST(strftime('%s', abbrev_last_used) AS INTEGER) * 1000, 0)";
/// One order, most-used first, `(word, tl)` as the final tie-break so equal
/// counts keep a stable order between two reads.
const LIST_ORDER: &str = "count DESC, last_used DESC, word ASC, tl ASC";
/// Words bound per `rows_for_words` statement — one parameter each, well
/// under SQLite's limit on bound parameters.
const WORDS_PER_QUERY: usize = 500;

/// The table as the Learning Records page reads it.
pub(crate) const LEARNING_TABLE: crate::learning_records::Table = crate::learning_records::Table {
    name: TABLE_NAME,
    identity: ["word", "tl"],
    count: "count",
    last_used: "last_used",
    delete_dependents: None,
};

/// One learned frequency row, as the store hands it over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrequencyRow {
    pub word: String,
    pub tl: String,
    /// Every pick, abbreviation picks included.
    pub count: i64,
    pub last_used_ms: i64,
    /// The picks of [`count`](Self::count) made through a whole-buffer
    /// abbreviation. Rows from before the split hold 0: every earlier pick
    /// reads as a reading pick.
    pub abbrev_count: i64,
    /// Last abbreviation pick; `0` = never.
    pub abbrev_last_used_ms: i64,
}

/// Records candidate commits and answers what the ranker should boost. The
/// engine does the ranking (`engine/ranking/src/score.rs`); this store only
/// hands it counts. A word is the `(Hanji, canonical TL)` PAIR (Core Principle
/// #7): 重/tîng and 重/tāng are different rows.
pub struct UserFrequencyStore {
    pub(crate) database: UserDataDatabase,
    capacity: LearningCapacity,
}

impl UserFrequencyStore {
    /// The cap every platform ships.
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
        self.write_pick(word, tl, false);
    }

    /// Counts one commit of `word` picked through a whole-buffer
    /// abbreviation: one more pick in `count` and in `abbrev_count`.
    pub fn record_abbreviation(&self, word: &str, tl: &str) {
        self.write_pick(word, tl, true);
    }

    fn write_pick(&self, word: &str, tl: &str, abbreviation: bool) {
        let word = phonetics::api::learning_hanji_key(word);
        if word.is_empty() {
            return;
        }
        let word = word.into_owned();
        let should_check_capacity = self.capacity.should_enforce(1);
        let tl = tl.to_owned();
        // The capacity's SQL needs no state from `self`; the check itself is
        // rebuilt on the writer thread from the same literal numbers.
        let capacity = if should_check_capacity {
            Some(self.capacity_for_writer())
        } else {
            None
        };
        self.database.write(move |connection| {
            let sql = if abbreviation {
                format!(
                    "INSERT INTO {TABLE_NAME} (word, tl, count, last_used, abbrev_count, abbrev_last_used)\nVALUES (?, ?, 1, CURRENT_TIMESTAMP, 1, CURRENT_TIMESTAMP)\nON CONFLICT(word, tl) DO UPDATE SET\n    count = count + 1,\n    last_used = CURRENT_TIMESTAMP,\n    abbrev_count = abbrev_count + 1,\n    abbrev_last_used = CURRENT_TIMESTAMP;"
                )
            } else {
                format!(
                    "INSERT INTO {TABLE_NAME} (word, tl, count, last_used)\nVALUES (?, ?, 1, CURRENT_TIMESTAMP)\nON CONFLICT(word, tl) DO UPDATE SET\n    count = count + 1,\n    last_used = CURRENT_TIMESTAMP;"
                )
            };
            connection.execute(&sql, params![word, tl])?;
            if let Some(capacity) = capacity {
                capacity.enforce(connection)?;
            }
            Ok(())
        });
    }

    /// Merges a backup's `(word, tl, count, last_used)` rows: a `(word, tl)`
    /// already stored keeps the larger count and the later use, so the
    /// walker's decay reads the same history it read on the device that
    /// wrote the file. `last_used` is the file's text, stored through
    /// [`RESTORED_TIMESTAMP_SQL`] — empty (a backup before version 4) counts
    /// as used now. One transaction, then the cap, so no reader sees the
    /// table over it. Answers how many rows were merged.
    pub fn import_merge(
        &self,
        rows: Vec<(String, String, i64, String)>,
    ) -> Result<usize, UserDataDatabaseError> {
        let capacity = self.capacity_for_writer();
        self.database.perform(move |connection| {
            immediate_transaction::<_, UserDataDatabaseError>(connection, |connection| {
                let mut statement = connection.prepare(&format!(
                    "INSERT INTO {TABLE_NAME} (word, tl, count, last_used)\nVALUES (?, ?, ?, {RESTORED_TIMESTAMP_SQL})\nON CONFLICT(word, tl) DO UPDATE SET\n    count = MAX(count, excluded.count),\n    last_used = MAX(COALESCE(last_used, excluded.last_used), excluded.last_used);"
                ))?;
                let mut merged = 0;
                for (word, tl, count, last_used) in rows {
                    let word = phonetics::api::strip_khinsiann_hanji(&word);
                    if word.is_empty() {
                        continue;
                    }
                    statement.execute(params![word, tl, count, last_used])?;
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
    /// not be read. Any number of words: they are bound
    /// [`WORDS_PER_QUERY`] at a time, one read.
    pub fn rows_for_words(&self, words: &[String]) -> Option<Vec<FrequencyRow>> {
        if words.is_empty() {
            return Some(Vec::new());
        }
        let mut normalized: Vec<std::borrow::Cow<'_, str>> = Vec::with_capacity(words.len());
        let mut seen = std::collections::HashSet::with_capacity(words.len());
        for word in words {
            let stripped = phonetics::api::learning_hanji_key(word);
            if seen.insert(stripped.clone()) {
                normalized.push(stripped);
            }
        }
        self.database.read(|connection| {
            let mut rows = Vec::new();
            for chunk in normalized.chunks(WORDS_PER_QUERY) {
                let placeholders = vec!["?"; chunk.len()].join(",");
                let mut statement = connection.prepare(&format!(
                    "SELECT {ROW_COLUMNS} FROM {TABLE_NAME}\nWHERE word IN ({placeholders});"
                ))?;
                let bound = params_from_iter(chunk.iter().map(AsRef::as_ref));
                for row in statement.query_map(bound, decode_row)? {
                    rows.push(row?);
                }
            }
            Ok(rows)
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

    /// Forgets everything.
    pub fn delete_all(&self) -> Result<(), UserDataDatabaseError> {
        self.database.perform(|connection| {
            connection.execute(&format!("DELETE FROM {TABLE_NAME};"), [])?;
            connection.execute("VACUUM;", []).ok();
            Ok(())
        })
    }
}

fn decode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FrequencyRow> {
    Ok(FrequencyRow {
        word: row.get(0)?,
        tl: row.get(1)?,
        count: row.get(2)?,
        last_used_ms: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
        abbrev_count: row.get(4)?,
        abbrev_last_used_ms: row.get(5)?,
    })
}

/// The current shape: a pre-pair-key table rebuilt first, then created
/// directly when absent, then any pure-Hanji neutral-tone rows normalized
/// and merged into v3.
fn apply_schema(connection: &Connection) -> rusqlite::Result<()> {
    migrate_to_pair_key_if_needed(connection)?;
    connection.execute_batch(&table_ddl(TABLE_NAME))?;
    add_abbreviation_columns_if_needed(connection)?;
    // No index on `word` alone: the UNIQUE constraint's automatic index
    // already has it as the leftmost column, so it serves `WHERE word IN`.
    connection.execute_batch(&format!(
        "CREATE INDEX IF NOT EXISTS idx_count ON {TABLE_NAME}(count DESC);\nCREATE INDEX IF NOT EXISTS idx_last_used ON {TABLE_NAME}(last_used DESC);"
    ))?;
    migrate_khinsiann_hanji_if_needed(connection)?;
    Ok(())
}

/// Normalizes pure-Hanji neutral-tone keys (`會記--得` -> `會記得`, `--轉-去` -> `轉去`)
/// inside an `immediate_transaction` and stamps `SCHEMA_VERSION` (`3`).
/// Han-Lo entries (`邊--a`) are left unchanged by `strip_khinsiann_hanji`.
fn migrate_khinsiann_hanji_if_needed(connection: &Connection) -> rusqlite::Result<()> {
    let select_sql = format!(
        "SELECT id, word, tl, count, last_used, created_at FROM {TABLE_NAME} WHERE word LIKE '%--%' ORDER BY id ASC;"
    );
    let update_sql = format!("UPDATE OR IGNORE {TABLE_NAME} SET word = ?1 WHERE id = ?2;");
    let merge_sql = format!(
        "UPDATE {TABLE_NAME}\nSET count = COALESCE(count, 0) + ?1,\n    last_used = CASE WHEN last_used IS NULL THEN ?2 WHEN ?2 IS NULL THEN last_used ELSE MAX(last_used, ?2) END,\n    created_at = CASE WHEN created_at IS NULL THEN ?3 WHEN ?3 IS NULL THEN created_at ELSE MIN(created_at, ?3) END\nWHERE word = ?4 AND tl = ?5;"
    );
    migrate_to_version(connection, SCHEMA_VERSION, |connection| {
        normalize_khinsiann_rows(
            connection,
            TABLE_NAME,
            &select_sql,
            |row| {
                Ok((
                    row.get(0)?,
                    (
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, Value>(4)?,
                        row.get::<_, Value>(5)?,
                    ),
                ))
            },
            |(word, ..)| strip_khinsiann_if_changed(word),
            |connection, id, stripped| {
                connection
                    .execute(&update_sql, params![stripped, id])
                    .map(|_| ())
            },
            |connection, _id, (_word, tl, count, last_used, created_at), stripped| {
                connection
                    .execute(
                        &merge_sql,
                        params![count.unwrap_or(0), last_used, created_at, stripped, tl],
                    )
                    .map(|_| ())
            },
        )
    })
}

fn table_ddl(name: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {name} (\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    word TEXT NOT NULL,\n    tl TEXT NOT NULL DEFAULT '',\n    count INTEGER DEFAULT 1,\n    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,\n    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,\n    abbrev_count INTEGER NOT NULL DEFAULT 0,\n    abbrev_last_used TIMESTAMP,\n    UNIQUE(word, tl)\n);"
    )
}

/// Rebuilds a pre-pair-key table (inline `word UNIQUE`, no `tl` column —
/// iOS / Android before R5) into the `(word, tl)` shape: `tl = ''` for every
/// old row, `id` / `count` / `last_used` / `created_at` kept. SQLite cannot
/// drop an inline UNIQUE, so create-new / copy / drop / rename in one
/// immediate transaction (the old `idx_word` goes with the old table).
/// Gated on the SHAPE, not `user_version`: iOS can leave a new-shape file at
/// 0. Ported from iOS `migrateToPairKeyIfNeeded` / Android `migrateToPairKey` (both removed).
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

/// Adds the abbreviation-pick columns to a table from before the split.
/// Additive, so an older build reading the file ignores them; every
/// existing row gets `abbrev_count = 0` — all its picks read as readings.
/// Gated on the shape, like the pair-key rebuild, with no version bump.
fn add_abbreviation_columns_if_needed(connection: &Connection) -> rusqlite::Result<()> {
    if has_column(connection, TABLE_NAME, "abbrev_count")? {
        return Ok(());
    }
    immediate_transaction::<_, rusqlite::Error>(connection, |connection| {
        // Another process may have added them between the check and the lock.
        if has_column(connection, TABLE_NAME, "abbrev_count")? {
            return Ok(());
        }
        connection.execute_batch(&format!(
            "ALTER TABLE {TABLE_NAME} ADD COLUMN abbrev_count INTEGER NOT NULL DEFAULT 0;\nALTER TABLE {TABLE_NAME} ADD COLUMN abbrev_last_used TIMESTAMP;"
        ))
    })
}
