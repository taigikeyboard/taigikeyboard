//! The phrases the user composed segment by segment (§50) — learning data
//! in its own `learned_phrases.db`, never the custom dictionary's (USER
//! 2026-09-21). Port of the former Swift `LearnedPhraseStore`; SQL
//! byte-identical apart from the `.taigi` v4 restore (`import_merge`,
//! #1032).

use crate::custom_dictionary::SearchKeyDeriver;
use crate::database::{
    immediate_transaction, migrate_to_version, normalize_khinsiann_rows,
    strip_khinsiann_if_changed, JournalMode, StoreSchema, UserDataDatabase, UserDataDatabaseError,
};
use crate::timestamp::RESTORED_TIMESTAMP_SQL;
use phonetics::api::CustomSearchKey;
use rusqlite::types::Value;
use rusqlite::{params, Connection};
use std::path::PathBuf;

const TABLE_NAME: &str = "learned_phrases";
const SEARCH_KEY_TABLE_NAME: &str = "learned_search_key";
/// `user_version` — a key-derivation change bumps it and adds a backfill step.
const SCHEMA_VERSION: i64 = 2;

/// The table as the Learning Records page reads it. A phrase's search keys
/// go with it.
pub(crate) const LEARNING_TABLE: crate::learning_records::Table = crate::learning_records::Table {
    name: TABLE_NAME,
    identity: ["hanzi", "roman"],
    count: "learn_count",
    count_share: None,
    last_used: "updated_at",
    delete_dependents: Some(delete_search_keys),
};

/// One learned phrase with its count, as a fetch reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LearnedPhraseRow {
    pub hanji: String,
    pub canonical_tl: String,
    pub learn_count: i64,
}

/// Keeps the phrases final commits taught, answers the exact whole-buffer query on
/// the keystroke path, and is wiped with the other learning data.
pub struct LearnedPhraseStore {
    pub(crate) database: UserDataDatabase,
    derive_search_keys: SearchKeyDeriver,
    limit: usize,
}

impl LearnedPhraseStore {
    /// Rows kept; past it the fewest-composed, then least recently touched,
    /// row goes (ChiaKey's policy) so a learn never fails. Ported
    /// from iOS `LearnedPhraseRepository.maxEntries` /
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

    /// Records one phrase a final commit taught: inserts the `(hanji, canonical
    /// TL)` pair or bumps its `learn_count`, in one statement on the
    /// `(hanji, roman)` unique constraint. A fresh row gets its keys and may
    /// evict past the cap; all in one transaction. Best-effort and off the
    /// keystroke path. The keys are derived before the write lock is taken
    /// (an FFI round-trip has no business holding it).
    pub fn learn_phrase(&self, hanji: &str, canonical_tl: &str) {
        let Some((hanji, search_keys)) = self.keyed_phrase(hanji, canonical_tl) else {
            return;
        };
        let limit = self.limit;
        let canonical_tl = canonical_tl.to_owned();
        self.database.write(move |connection| {
            immediate_transaction::<_, rusqlite::Error>(connection, |connection| {
                let (row_id, learn_count): (i64, i64) = connection.query_row(
                    &format!(
                        "INSERT INTO {TABLE_NAME} (roman, hanzi, learn_count, updated_at)\nVALUES (?, ?, 1, CURRENT_TIMESTAMP)\nON CONFLICT(hanzi, roman) DO UPDATE SET\n    learn_count = MIN(learn_count + 1, {}),\n    updated_at = CURRENT_TIMESTAMP\nRETURNING id, learn_count;",
                        Self::MAX_LEARN_COUNT
                    ),
                    params![canonical_tl, hanji],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                // `learn_count` reads 1 only for the row this statement just
                // inserted (a bump lands at 2 or more) — the one case that
                // needs keys and can push the table past its cap.
                if learn_count != 1 {
                    return Ok(());
                }
                write_search_keys(connection, row_id, &search_keys)?;
                evict_past_cap(connection, limit, Some(row_id))
            })
        });
    }

    /// Merges a backup's `(hanji, canonical TL, learn_count, updated_at)`
    /// rows: a pair already stored keeps the larger count and the later
    /// touch; `updated_at` is the file's text, stored through
    /// [`RESTORED_TIMESTAMP_SQL`]. Unlike [`learn_phrase`](Self::learn_phrase)
    /// every row writes its keys (`OR IGNORE` absorbs the ones a stored row
    /// already has) — a restored count is rarely 1 — and the cap is enforced
    /// once, after the batch. Rows whose Hanji or reading is empty, or whose
    /// keys cannot be derived, are skipped as `learn_phrase` skips them.
    /// The keys are derived before the transaction. Answers how many rows
    /// were merged.
    pub fn import_merge(
        &self,
        rows: Vec<(String, String, i64, String)>,
    ) -> Result<usize, UserDataDatabaseError> {
        let keyed: Vec<(String, String, i64, String, Vec<CustomSearchKey>)> = rows
            .into_iter()
            .filter_map(|(hanji, canonical_tl, learn_count, updated_at)| {
                let (hanji, search_keys) = self.keyed_phrase(&hanji, &canonical_tl)?;
                let learn_count = learn_count.clamp(1, Self::MAX_LEARN_COUNT);
                Some((hanji, canonical_tl, learn_count, updated_at, search_keys))
            })
            .collect();
        let limit = self.limit;
        self.database.perform(move |connection| {
            immediate_transaction::<_, UserDataDatabaseError>(connection, |connection| {
                let mut upsert = connection.prepare(&format!(
                    "INSERT INTO {TABLE_NAME} (roman, hanzi, learn_count, updated_at)\nVALUES (?, ?, ?, {RESTORED_TIMESTAMP_SQL})\nON CONFLICT(hanzi, roman) DO UPDATE SET\n    learn_count = MAX(learn_count, excluded.learn_count),\n    updated_at = MAX(updated_at, excluded.updated_at)\nRETURNING id;"
                ))?;
                for (hanji, canonical_tl, learn_count, updated_at, search_keys) in &keyed {
                    let row_id: i64 = upsert.query_row(
                        params![canonical_tl, hanji, learn_count, updated_at],
                        |row| row.get(0),
                    )?;
                    write_search_keys(connection, row_id, search_keys)?;
                }
                evict_past_cap(connection, limit, None)?;
                Ok(keyed.len())
            })
        })
    }

    /// The stored Hanji key and the search keys of a phrase worth storing;
    /// `None` when its Hanji or reading is empty or its keys cannot be
    /// derived.
    fn keyed_phrase(
        &self,
        hanji: &str,
        canonical_tl: &str,
    ) -> Option<(String, Vec<CustomSearchKey>)> {
        let hanji = phonetics::api::learning_hanji_key(hanji);
        if hanji.is_empty() || canonical_tl.is_empty() {
            return None;
        }
        let search_keys =
            (self.derive_search_keys)(canonical_tl).filter(|keys| !keys.is_empty())?;
        Some((hanji.into_owned(), search_keys))
    }

    /// Bumps a phrase the user just committed as one candidate, so a phrase
    /// that is used stays ahead of the eviction line. No-op for an unknown pair.
    pub fn touch_phrase(&self, hanji: &str, canonical_tl: &str) {
        let hanji = phonetics::api::learning_hanji_key(hanji);
        if hanji.is_empty() || canonical_tl.is_empty() {
            return;
        }
        let hanji = hanji.into_owned();
        let canonical_tl = canonical_tl.to_owned();
        self.database.write(move |connection| {
            connection.execute(
                &format!(
                    "UPDATE {TABLE_NAME}\nSET learn_count = MIN(learn_count + 1, {}), updated_at = CURRENT_TIMESTAMP\nWHERE hanzi = ? AND roman = ?;",
                    Self::MAX_LEARN_COUNT
                ),
                params![hanji, canonical_tl],
            )?;
            Ok(())
        });
    }

    /// The phrases whose derived key EQUALS `query_key` — the whole typed
    /// buffer, not a prefix — for `composing::UserRows.learned`, most composed
    /// first. Empty when the store cannot answer right now. Ported
    /// from iOS `exactMatchSQL` / Android `EXACT_MATCH_SQL`.
    pub fn rows_matching(
        &self,
        query_key: &CustomSearchKey,
        limit: usize,
    ) -> Vec<LearnedPhraseRow> {
        self.rows_matching_any(std::slice::from_ref(query_key), limit)
    }

    /// [`rows_matching`](Self::rows_matching) for each of `query_keys` — a
    /// TPS walk's edge bodies — each phrase once, in one read: the store
    /// answers every key or none.
    pub fn rows_matching_any(
        &self,
        query_keys: &[CustomSearchKey],
        limit: usize,
    ) -> Vec<LearnedPhraseRow> {
        self.database
            .read(|connection| {
                // Cached: the one statement the keystroke path runs here.
                let mut statement = connection.prepare_cached(&format!(
                    "SELECT p.hanzi, p.roman, p.learn_count\nFROM {TABLE_NAME} p\nJOIN {SEARCH_KEY_TABLE_NAME} k ON k.phrase_id = p.id\nWHERE k.family = ? AND k.form = ? AND k.key = ?\nORDER BY p.learn_count DESC, p.updated_at DESC\nLIMIT ?;"
                ))?;
                let mut rows: Vec<LearnedPhraseRow> = Vec::new();
                for query_key in query_keys {
                    let matched = statement.query_map(
                        params![query_key.family, query_key.form, query_key.key, limit as i64],
                        decode_row,
                    )?;
                    for row in matched {
                        let row = row?;
                        if !rows.contains(&row) {
                            rows.push(row);
                        }
                    }
                }
                Ok(rows)
            })
            .unwrap_or_default()
    }

    /// Every phrase with its last touch (epoch ms, 0 when the store has no
    /// time for it), most composed first — the `.taigi` export.
    pub fn backup_rows(&self) -> Option<Vec<(LearnedPhraseRow, i64)>> {
        self.database.wait_for_queued_writes();
        self.database.read(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT hanzi, roman, learn_count, CAST(strftime('%s', updated_at) AS INTEGER) * 1000\nFROM {TABLE_NAME}\nORDER BY learn_count DESC, updated_at DESC, id;"
            ))?;
            let rows = statement.query_map([], |row| {
                Ok((decode_row(row)?, row.get::<_, Option<i64>>(3)?.unwrap_or(0)))
            })?;
            rows.collect()
        })
    }

    /// Every phrase, most composed first. For the tests.
    pub fn all_rows(&self) -> Option<Vec<LearnedPhraseRow>> {
        self.backup_rows()
            .map(|rows| rows.into_iter().map(|(row, _)| row).collect())
    }

    /// Forgets everything — the learning-data wipe, beside the frequency and
    /// association stores'. One transaction: the keys and the rows go
    /// together.
    pub fn delete_all(&self) -> Result<(), UserDataDatabaseError> {
        self.database.perform(|connection| {
            immediate_transaction::<_, rusqlite::Error>(connection, |connection| {
                connection.execute(&format!("DELETE FROM {SEARCH_KEY_TABLE_NAME};"), [])?;
                connection.execute(&format!("DELETE FROM {TABLE_NAME};"), [])?;
                Ok(())
            })?;
            connection.execute("VACUUM;", []).ok();
            Ok(())
        })
    }
}

fn decode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LearnedPhraseRow> {
    Ok(LearnedPhraseRow {
        hanji: row.get(0)?,
        canonical_tl: row.get(1)?,
        learn_count: row.get(2)?,
    })
}

/// The keys of one phrase, deleted with it.
fn delete_search_keys(connection: &Connection, phrase_id: i64) -> rusqlite::Result<()> {
    connection.execute(
        &format!("DELETE FROM {SEARCH_KEY_TABLE_NAME} WHERE phrase_id = ?;"),
        params![phrase_id],
    )?;
    Ok(())
}

/// The full cross-mode key bundle for a fresh row. `OR IGNORE` on the
/// side-key unique index absorbs a repeated key.
fn write_search_keys(
    connection: &Connection,
    phrase_id: i64,
    search_keys: &[CustomSearchKey],
) -> rusqlite::Result<()> {
    let mut statement = connection.prepare_cached(&format!(
        "INSERT OR IGNORE INTO {SEARCH_KEY_TABLE_NAME} (phrase_id, family, form, key)\nVALUES (?, ?, ?, ?);"
    ))?;
    for key in search_keys {
        statement.execute(params![phrase_id, key.family, key.form, key.key])?;
    }
    Ok(())
}

/// Drops rows past `cap`, never `kept_id` (the row just inserted survives
/// whatever its timestamp ties with; a restore's batch keeps none aside).
/// A cheap `COUNT(*)` first — under the cap the ordered walk never runs.
/// Keys first, so the subquery still resolves against the intact main table;
/// the offset then selects exactly the rows past the cap. Ported from iOS
/// `evictPastCap` / Android `PAST_CAP_SQL`.
fn evict_past_cap(
    connection: &Connection,
    cap: usize,
    kept_id: Option<i64>,
) -> rusqlite::Result<()> {
    let count: i64 =
        connection.query_row(&format!("SELECT COUNT(*) FROM {TABLE_NAME};"), [], |row| {
            row.get(0)
        })?;
    if count <= cap as i64 {
        return Ok(());
    }
    let past_cap = format!(
        "SELECT id FROM {TABLE_NAME}\nWHERE id IS NOT ?\nORDER BY learn_count DESC, updated_at DESC, id\nLIMIT -1 OFFSET ?"
    );
    let offset = cap.saturating_sub(usize::from(kept_id.is_some())) as i64;
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

/// The current shape, created directly, plus v1 -> v2 neutral-tone Hanji
/// normalization. Ported from iOS `LearnedPhraseSchema` / Android
/// `LearnedPhraseService` DDL.
fn apply_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {TABLE_NAME} (\n    id INTEGER PRIMARY KEY,\n    roman TEXT NOT NULL,\n    hanzi TEXT NOT NULL,\n    learn_count INTEGER NOT NULL DEFAULT 1,\n    updated_at TEXT NOT NULL,\n    UNIQUE(hanzi, roman)\n);\nCREATE TABLE IF NOT EXISTS {SEARCH_KEY_TABLE_NAME} (\n    phrase_id INTEGER NOT NULL,\n    family TEXT NOT NULL,\n    form TEXT NOT NULL,\n    key TEXT NOT NULL\n);\nCREATE INDEX IF NOT EXISTS idx_lsk_lookup ON {SEARCH_KEY_TABLE_NAME}(family, form, key);\nCREATE UNIQUE INDEX IF NOT EXISTS idx_lsk_phrase ON {SEARCH_KEY_TABLE_NAME}(phrase_id, family, form, key);\nCREATE INDEX IF NOT EXISTS idx_learned_rank ON {TABLE_NAME}(learn_count, updated_at);"
    ))?;
    migrate_khinsiann_hanji_if_needed(connection)?;
    Ok(())
}

/// Normalizes pure-Hanji neutral-tone keys (`走--轉-去` -> `走轉去`) inside an
/// `immediate_transaction`, cleaning `learned_search_key` rows for deleted
/// collision losers, and stamps `SCHEMA_VERSION` (`2`).
fn migrate_khinsiann_hanji_if_needed(connection: &Connection) -> rusqlite::Result<()> {
    let select_sql = format!(
        "SELECT id, hanzi, roman, learn_count, updated_at FROM {TABLE_NAME} WHERE hanzi LIKE '%--%' ORDER BY id ASC;"
    );
    let update_sql = format!("UPDATE OR IGNORE {TABLE_NAME} SET hanzi = ?1 WHERE id = ?2;");
    let merge_sql = format!(
        "UPDATE {TABLE_NAME}\nSET learn_count = MIN(learn_count + ?1, {}),\n    updated_at = CASE WHEN updated_at IS NULL THEN ?2 WHEN ?2 IS NULL THEN updated_at ELSE MAX(updated_at, ?2) END\nWHERE hanzi = ?3 AND roman = ?4;",
        LearnedPhraseStore::MAX_LEARN_COUNT
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
                        row.get::<_, i64>(3)?,
                        row.get::<_, Value>(4)?,
                    ),
                ))
            },
            |(hanzi, ..)| strip_khinsiann_if_changed(hanzi),
            |connection, id, stripped| {
                connection
                    .execute(&update_sql, params![stripped, id])
                    .map(|_| ())
            },
            |connection, loser_id, (_hanzi, roman, learn_count, updated_at), stripped| {
                connection.execute(
                    &merge_sql,
                    params![learn_count, updated_at, stripped, roman],
                )?;
                delete_search_keys(connection, loser_id)
            },
        )
    })
}
