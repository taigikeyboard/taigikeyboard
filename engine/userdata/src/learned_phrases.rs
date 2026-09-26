//! The phrases the user composed segment by segment (§50) — learning data
//! in its own `learned_phrases.db`, never the custom dictionary's (USER
//! 2026-09-21). Port of `Storage/LearnedPhraseStore.swift`; SQL
//! byte-identical.

use crate::custom_dictionary::SearchKeyDeriver;
use crate::database::{
    immediate_transaction, JournalMode, StoreSchema, UserDataDatabase, UserDataDatabaseError,
};
use crate::stores::LearnedPhraseSource;
use crate::types::{CustomSearchKey, LearnedPhrase};
use rusqlite::{params, Connection};
use std::path::PathBuf;

const TABLE_NAME: &str = "learned_phrases";
const SEARCH_KEY_TABLE_NAME: &str = "learned_search_key";
/// `user_version` — a key-derivation change bumps it and adds a backfill step.
const SCHEMA_VERSION: i64 = 1;

/// One learned phrase with its count, for the tests: no product surface
/// lists learned phrases.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LearnedPhraseRow {
    pub hanzi: String,
    pub canonical_tl: String,
    pub learn_count: i64,
}

/// Writes `Effect::PhraseLearned`, answers the exact whole-buffer query on
/// the keystroke path, and is wiped with the other learning data.
pub struct LearnedPhraseStore {
    database: UserDataDatabase,
    derive_search_keys: SearchKeyDeriver,
    limit: usize,
}

impl LearnedPhraseStore {
    /// Rows kept; past it the fewest-composed, then least recently touched,
    /// row goes (ChiaKey's policy) so a learn never fails. CROSS-PLATFORM
    /// INVARIANT — mirrors iOS `LearnedPhraseRepository.maxEntries` /
    /// Android `LearnedPhraseService.MAX_ENTRIES`.
    pub const MAX_ENTRIES: usize = 2_000;
    /// Largest `learn_count` a row can carry.
    pub const MAX_LEARN_COUNT: i64 = 1_000_000;
    /// Rows per fetch (exact whole-buffer match; homophone phrases).
    pub const KEYSTROKE_LIMIT: usize = 5;

    /// The cap is injectable ONLY so a test can reach it without writing
    /// 2000 rows.
    pub fn new(
        path: PathBuf,
        journal: JournalMode,
        derive_search_keys: SearchKeyDeriver,
        limit: usize,
    ) -> Self {
        Self {
            database: UserDataDatabase::new(
                "LearnedPhraseStore",
                path,
                journal,
                StoreSchema {
                    apply: apply_schema,
                    max_known_version: SCHEMA_VERSION,
                    marks_takeover_on_open: true,
                },
            ),
            derive_search_keys,
            limit,
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

    /// Records one `Effect::PhraseLearned`: inserts the `(hanzi, canonical
    /// TL)` pair or bumps its `learn_count`, in one statement on the
    /// `(hanzi, roman)` unique constraint. A fresh row gets its keys and may
    /// evict past the cap; all in one transaction. Best-effort and off the
    /// keystroke path. The keys are derived before the write lock is taken
    /// (an FFI round-trip has no business holding it).
    pub fn learn_phrase(&self, hanzi: &str, canonical_tl: &str) {
        if hanzi.is_empty() || canonical_tl.is_empty() {
            return;
        }
        let Some(search_keys) =
            (self.derive_search_keys)(canonical_tl).filter(|keys| !keys.is_empty())
        else {
            return;
        };
        let limit = self.limit;
        let hanzi = hanzi.to_owned();
        let canonical_tl = canonical_tl.to_owned();
        self.database.write(move |connection| {
            immediate_transaction::<_, rusqlite::Error>(connection, |connection| {
                let (row_id, learn_count): (i64, i64) = connection.query_row(
                    &format!(
                        "INSERT INTO {TABLE_NAME} (roman, hanzi, learn_count, updated_at)\nVALUES (?, ?, 1, CURRENT_TIMESTAMP)\nON CONFLICT(hanzi, roman) DO UPDATE SET\n    learn_count = MIN(learn_count + 1, {}),\n    updated_at = CURRENT_TIMESTAMP\nRETURNING id, learn_count;",
                        Self::MAX_LEARN_COUNT
                    ),
                    params![canonical_tl, hanzi],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                // `learn_count` reads 1 only for the row this statement just
                // inserted (a bump lands at 2 or more) — the one case that
                // needs keys and can push the table past its cap.
                if learn_count != 1 {
                    return Ok(());
                }
                write_search_keys(connection, row_id, &search_keys)?;
                evict_past_cap(connection, limit, row_id)
            })
        });
    }

    /// Bumps a phrase the user just committed as one candidate, so a phrase
    /// that is used stays ahead of the eviction line. No-op for an unknown pair.
    pub fn touch_phrase(&self, hanzi: &str, canonical_tl: &str) {
        if hanzi.is_empty() || canonical_tl.is_empty() {
            return;
        }
        let hanzi = hanzi.to_owned();
        let canonical_tl = canonical_tl.to_owned();
        self.database.write(move |connection| {
            connection.execute(
                &format!(
                    "UPDATE {TABLE_NAME}\nSET learn_count = MIN(learn_count + 1, {}), updated_at = CURRENT_TIMESTAMP\nWHERE hanzi = ? AND roman = ?;",
                    Self::MAX_LEARN_COUNT
                ),
                params![hanzi, canonical_tl],
            )?;
            Ok(())
        });
    }

    /// The phrases whose derived key EQUALS `query_key` — the whole typed
    /// buffer, not a prefix — for `FetchAtPos.learned_entries`, most composed
    /// first. Empty when the store cannot answer right now. CROSS-PLATFORM
    /// INVARIANT — mirrors iOS `exactMatchSQL` / Android `EXACT_MATCH_SQL`.
    pub fn rows_matching(
        &self,
        query_key: &CustomSearchKey,
        limit: usize,
    ) -> Vec<LearnedPhraseRow> {
        self.database
            .read(|connection| {
                // Cached: the one statement the keystroke path runs here.
                let mut statement = connection.prepare_cached(&format!(
                    "SELECT p.hanzi, p.roman, p.learn_count\nFROM {TABLE_NAME} p\nJOIN {SEARCH_KEY_TABLE_NAME} k ON k.phrase_id = p.id\nWHERE k.family = ? AND k.form = ? AND k.key = ?\nORDER BY p.learn_count DESC, p.updated_at DESC\nLIMIT ?;"
                ))?;
                let rows = statement.query_map(
                    params![query_key.family, query_key.form, query_key.key, limit as i64],
                    decode_row,
                )?;
                rows.collect()
            })
            .unwrap_or_default()
    }

    /// Every phrase, most composed first. For the tests — nothing in the
    /// shipped UI lists learned phrases.
    pub fn all_rows(&self) -> Option<Vec<LearnedPhraseRow>> {
        self.database.wait_for_queued_writes();
        self.database.read(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT hanzi, roman, learn_count FROM {TABLE_NAME}\nORDER BY learn_count DESC, updated_at DESC, id;"
            ))?;
            let rows = statement.query_map([], decode_row)?;
            rows.collect()
        })
    }

    /// Forgets everything, and reports how many rows went — the learning-data
    /// wipe, beside the frequency and association stores'. One transaction:
    /// the keys and the rows land together, and the count taken inside it is
    /// the number that went.
    pub fn delete_all(&self) -> Result<i64, UserDataDatabaseError> {
        self.database.perform(|connection| {
            let existing = immediate_transaction::<_, rusqlite::Error>(connection, |connection| {
                let existing: i64 = connection.query_row(
                    &format!("SELECT COUNT(*) FROM {TABLE_NAME};"),
                    [],
                    |row| row.get(0),
                )?;
                connection.execute(&format!("DELETE FROM {SEARCH_KEY_TABLE_NAME};"), [])?;
                connection.execute(&format!("DELETE FROM {TABLE_NAME};"), [])?;
                Ok(existing)
            })?;
            connection.execute("VACUUM;", []).ok();
            Ok(existing)
        })
    }
}

impl LearnedPhraseSource for LearnedPhraseStore {
    fn rows_matching(&self, family: &str, form: &str, key: &str) -> Vec<LearnedPhrase> {
        let query = CustomSearchKey {
            family: family.to_owned(),
            form: form.to_owned(),
            key: key.to_owned(),
        };
        LearnedPhraseStore::rows_matching(self, &query, Self::KEYSTROKE_LIMIT)
            .into_iter()
            .map(|row| LearnedPhrase {
                hanzi: row.hanzi,
                canonical_tl: row.canonical_tl,
            })
            .collect()
    }

    fn learn_phrase(&self, hanzi: &str, canonical_tl: &str) {
        LearnedPhraseStore::learn_phrase(self, hanzi, canonical_tl);
    }

    fn touch_phrase(&self, hanzi: &str, canonical_tl: &str) {
        LearnedPhraseStore::touch_phrase(self, hanzi, canonical_tl);
    }
}

fn decode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LearnedPhraseRow> {
    Ok(LearnedPhraseRow {
        hanzi: row.get(0)?,
        canonical_tl: row.get(1)?,
        learn_count: row.get(2)?,
    })
}

/// The full cross-mode key bundle for a fresh row. `OR IGNORE` on the
/// side-key unique index absorbs a repeated key.
fn write_search_keys(
    connection: &Connection,
    phrase_id: i64,
    search_keys: &[CustomSearchKey],
) -> rusqlite::Result<()> {
    let mut statement = connection.prepare(&format!(
        "INSERT OR IGNORE INTO {SEARCH_KEY_TABLE_NAME} (phrase_id, family, form, key)\nVALUES (?, ?, ?, ?);"
    ))?;
    for key in search_keys {
        statement.execute(params![phrase_id, key.family, key.form, key.key])?;
    }
    Ok(())
}

/// Drops rows past `cap`, never `kept_id` (the row just inserted survives
/// whatever its timestamp ties with). A cheap `COUNT(*)` first — under the
/// cap the ordered walk never runs. Keys first, so the subquery still
/// resolves against the intact main table; `OFFSET cap - 1` selects exactly
/// the rows past the cap once the kept row is set aside. CROSS-PLATFORM
/// INVARIANT — mirrors iOS `evictPastCap` / Android `PAST_CAP_SQL`.
fn evict_past_cap(connection: &Connection, cap: usize, kept_id: i64) -> rusqlite::Result<()> {
    let count: i64 =
        connection.query_row(&format!("SELECT COUNT(*) FROM {TABLE_NAME};"), [], |row| {
            row.get(0)
        })?;
    if count <= cap as i64 {
        return Ok(());
    }
    let past_cap = format!(
        "SELECT id FROM {TABLE_NAME}\nWHERE id <> ?\nORDER BY learn_count DESC, updated_at DESC, id\nLIMIT -1 OFFSET ?"
    );
    let offset = cap.saturating_sub(1) as i64;
    connection.execute(
        &format!("DELETE FROM {SEARCH_KEY_TABLE_NAME} WHERE phrase_id IN ({past_cap});"),
        params![kept_id, offset],
    )?;
    connection.execute(
        &format!("DELETE FROM {TABLE_NAME} WHERE id IN ({past_cap});"),
        params![kept_id, offset],
    )?;
    Ok(())
}

/// The current shape, created directly. CROSS-PLATFORM INVARIANT — mirrors
/// iOS `LearnedPhraseSchema` / Android `LearnedPhraseService` DDL.
fn apply_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {TABLE_NAME} (\n    id INTEGER PRIMARY KEY,\n    roman TEXT NOT NULL,\n    hanzi TEXT NOT NULL,\n    learn_count INTEGER NOT NULL DEFAULT 1,\n    updated_at TEXT NOT NULL,\n    UNIQUE(hanzi, roman)\n);\nCREATE TABLE IF NOT EXISTS {SEARCH_KEY_TABLE_NAME} (\n    phrase_id INTEGER NOT NULL,\n    family TEXT NOT NULL,\n    form TEXT NOT NULL,\n    key TEXT NOT NULL\n);\nCREATE INDEX IF NOT EXISTS idx_lsk_lookup ON {SEARCH_KEY_TABLE_NAME}(family, form, key);\nCREATE UNIQUE INDEX IF NOT EXISTS idx_lsk_phrase ON {SEARCH_KEY_TABLE_NAME}(phrase_id, family, form, key);\nCREATE INDEX IF NOT EXISTS idx_learned_rank ON {TABLE_NAME}(learn_count, updated_at);"
    ))?;
    connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}
