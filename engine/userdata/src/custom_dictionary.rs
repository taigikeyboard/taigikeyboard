//! The words the user added themselves, and the keys that make them findable
//! from any romanization. Port of `Storage/CustomDictionaryStore.swift` +
//! `CustomDictionaryRow.swift`; SQL byte-identical.

use crate::database::{
    has_column, immediate_transaction, is_taken_over, mark_taken_over, user_version, JournalMode,
    StoreSchema, UserDataDatabase, UserDataDatabaseError,
};
use crate::stores::CustomDictionarySource;
use crate::timestamp::utc_timestamp_now;
use crate::types::{CustomEntry, CustomSearchKey};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) const FILE_NAME: &str = "custom_dictionary.db";
const TABLE_NAME: &str = "custom_dictionary";
const SEARCH_KEY_TABLE_NAME: &str = "custom_search_key";
/// v4 (2026-09-18): search-key abbreviation face = leading spelling unit per
/// syllable (`behavioral-invariants.md` §46) — `rederive_search_keys_if_needed`
/// rebuilds every entry's keys once.
const SCHEMA_VERSION: i64 = 4;
/// The highest stamp any writer of this file ever used — the three platforms
/// number one schema differently (Android `DATABASE_VERSION` 10, iOS 6,
/// macOS / desktop 4; portability D5). Above it the file is from a later
/// build and stays closed.
const HIGHEST_KNOWN_VERSION: i64 = 10;
/// One transaction per this many accepted rows, so a large import never
/// holds the write lock for its whole run. CROSS-PLATFORM INVARIANT —
/// mirrors iOS `CustomDictionaryRepository.swift:180`.
const IMPORT_CHUNK_SIZE: usize = 500;

/// How a stored roman becomes the keys it is findable under. Injected so a
/// test can drive the store without the engine, and because the shipped
/// implementation is an engine round-trip that must happen OUTSIDE the
/// write transaction.
pub type SearchKeyDeriver = Arc<dyn Fn(&str) -> Option<Vec<CustomSearchKey>> + Send + Sync>;

/// Why a custom-dictionary write did not happen.
#[derive(Debug, thiserror::Error)]
pub enum CustomDictionaryError {
    /// The dictionary already holds `limit` words and this would be one
    /// more. Editing an entry that is already there is never refused.
    #[error("custom dictionary is full (max {limit} entries)")]
    CapacityReached { limit: usize },
    /// The engine could not derive the search keys. The row is not written:
    /// an entry with no side keys is visible in the list and unreachable
    /// from the keyboard, which is worse than a refusal.
    #[error("could not derive search keys for {roman}")]
    SearchKeyDerivationFailed { roman: String },
    #[error(transparent)]
    Database(#[from] UserDataDatabaseError),
}

impl From<rusqlite::Error> for CustomDictionaryError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(UserDataDatabaseError::Sqlite(error))
    }
}

/// What a CSV import did. `skipped` does not say why — a duplicate, the cap
/// and a rejected row all land in one bucket, as on iOS and Android.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustomDictionaryImportResult {
    pub imported: usize,
    pub skipped: usize,
}

/// What makes two entries the same word to an import: the same word spelled
/// the same way (a file has no ids).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CustomDictionaryIdentity {
    pub roman: String,
    pub hanzi: String,
}

/// A word the user added: the romanization exactly as typed (TL or POJ
/// display form — nothing here folds it; the engine canonicalises per mode)
/// and the Hanji it stands for, which may be empty. Timestamps are the stored
/// `yyyy-MM-dd HH:mm:ss` UTC text. CROSS-PLATFORM INVARIANT — the stored
/// shape mirrors iOS `CustomDictionaryEntry.swift:11-31` and Android's table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomDictionaryRow {
    /// Stable across edits so the side table can be replaced rather than
    /// accumulated — a UUID, because editing either column keeps identity.
    pub id: String,
    pub roman: String,
    pub hanzi: String,
    pub created_at: String,
    pub updated_at: String,
}

impl CustomDictionaryRow {
    pub fn new(roman: &str, hanzi: &str) -> Self {
        Self::with_id(
            &uuid::Uuid::new_v4().to_string().to_uppercase(),
            roman,
            hanzi,
        )
    }

    pub fn with_id(id: &str, roman: &str, hanzi: &str) -> Self {
        let now = utc_timestamp_now();
        Self {
            id: id.to_owned(),
            roman: roman.to_owned(),
            hanzi: hanzi.to_owned(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn identity(&self) -> CustomDictionaryIdentity {
        CustomDictionaryIdentity {
            roman: self.roman.clone(),
            hanzi: self.hanzi.clone(),
        }
    }
}

/// The user's own dictionary: reads it on the keystroke path, writes it
/// when the user asks.
pub struct CustomDictionaryStore {
    database: UserDataDatabase,
    derive_search_keys: SearchKeyDeriver,
    entry_limit: usize,
}

impl CustomDictionaryStore {
    /// CROSS-PLATFORM INVARIANT — mirrors iOS
    /// `CustomDictionaryCapacityPolicy.swift:18` and Android `MAX_ENTRIES`.
    pub const MAX_ENTRIES: usize = 30_000;
    /// What the keystroke path is handed — the iOS call site's 20.
    pub const KEYSTROKE_LIMIT: usize = 20;

    /// What a fresh install can find before the user has added anything.
    /// Ids included, so the same word is the same row on every platform
    /// (iOS `CustomDictionaryService.swift:22-25`).
    pub fn seed_entries() -> [CustomDictionaryRow; 2] {
        [
            CustomDictionaryRow::with_id("default-gau-tsa", "gâu-tsá", "𠢕早"),
            CustomDictionaryRow::with_id("default-tsiah-pa-bue", "tsia̍h-pá--buē", "食飽未"),
        ]
    }

    /// `entry_limit` is injectable ONLY so a test can reach the cap without
    /// writing 30000 rows.
    pub fn new(
        path: PathBuf,
        journal: JournalMode,
        derive_search_keys: SearchKeyDeriver,
        entry_limit: usize,
    ) -> Self {
        Self {
            database: UserDataDatabase::new(
                "CustomDictionaryStore",
                path,
                journal,
                StoreSchema {
                    apply: apply_schema,
                    max_known_version: HIGHEST_KNOWN_VERSION,
                    // Taken over once the keys are re-derived.
                    marks_takeover_on_open: false,
                },
            ),
            derive_search_keys,
            entry_limit,
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

    // Keystroke path

    /// The entries matching `query_key`, for the composition being typed.
    /// Synchronous and best-effort: a store that is not open answers `[]`
    /// rather than making the keystroke wait. `form IN (?, 'abbrev')` lets
    /// an abbreviation row satisfy a query in the same family; `DISTINCT`
    /// because one entry owns several side rows. CROSS-PLATFORM INVARIANT —
    /// mirrors iOS `CustomDictionaryRepository.swift:332-367`.
    pub fn rows_matching(
        &self,
        query_key: &CustomSearchKey,
        limit: usize,
    ) -> Vec<CustomDictionaryRow> {
        self.database
            .read(|connection| {
                let mut statement = connection.prepare_cached(&format!(
                    "SELECT DISTINCT entry.id, entry.roman, entry.hanzi, entry.created_at, entry.updated_at\nFROM {TABLE_NAME} AS entry\nJOIN {SEARCH_KEY_TABLE_NAME} AS search_key ON search_key.entry_id = entry.id\nWHERE search_key.family = ?\n  AND search_key.form IN (?, 'abbrev')\n  AND search_key.key LIKE ? || '%'\nORDER BY entry.roman\nLIMIT ?;"
                ))?;
                let rows = statement.query_map(
                    params![query_key.family, query_key.form, query_key.key, limit as i64],
                    decode_row,
                )?;
                rows.collect()
            })
            .unwrap_or_default()
    }

    // User-driven writes

    /// Every entry, newest edit first — the order the settings list shows.
    /// Unbounded, for the export.
    pub fn all_rows(&self) -> Result<Vec<CustomDictionaryRow>, CustomDictionaryError> {
        self.database.perform::<_, CustomDictionaryError>(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT id, roman, hanzi, created_at, updated_at\nFROM {TABLE_NAME}\nORDER BY updated_at DESC;"
            ))?;
            let rows = statement.query_map([], decode_row)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    /// A page of entries for the settings list, newest edit first. The
    /// filter and the limit are both SQL. `LIKE` is case-insensitive for
    /// ASCII, which is the romanization; Hanji have no case to fold.
    pub fn rows(
        &self,
        filter: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<CustomDictionaryRow>, CustomDictionaryError> {
        let trimmed = filter.trim().to_owned();
        self.database.perform::<_, CustomDictionaryError>(move |connection| {
            if trimmed.is_empty() {
                let mut statement = connection.prepare(&format!(
                    "SELECT id, roman, hanzi, created_at, updated_at\nFROM {TABLE_NAME}\nORDER BY updated_at DESC\nLIMIT ? OFFSET ?;"
                ))?;
                let rows = statement.query_map(params![limit as i64, offset as i64], decode_row)?;
                return Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?);
            }
            let pattern = format!("%{}%", escaped_for_like(&trimmed));
            let mut statement = connection.prepare(&format!(
                "SELECT id, roman, hanzi, created_at, updated_at\nFROM {TABLE_NAME}\nWHERE roman LIKE ? ESCAPE '\\' OR hanzi LIKE ? ESCAPE '\\'\nORDER BY updated_at DESC\nLIMIT ? OFFSET ?;"
            ))?;
            let rows = statement.query_map(params![pattern, pattern, limit as i64, offset as i64], decode_row)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    /// The stored row with `id`, times included — what an edit page shows
    /// after a save.
    pub fn row(&self, id: &str) -> Result<Option<CustomDictionaryRow>, CustomDictionaryError> {
        let id = id.to_owned();
        self.database.perform::<_, CustomDictionaryError>(move |connection| {
            Ok(connection
                .query_row(
                    &format!(
                        "SELECT id, roman, hanzi, created_at, updated_at\nFROM {TABLE_NAME}\nWHERE id = ?;"
                    ),
                    [id],
                    decode_row,
                )
                .optional()?)
        })
    }

    pub fn count(&self) -> Result<usize, CustomDictionaryError> {
        self.database
            .perform::<_, CustomDictionaryError>(|connection| Ok(entry_count(connection)?))
    }

    /// How many entries `filter` matches — the number the pager divides
    /// into pages. The same two predicates `rows` filters on.
    pub fn count_matching(&self, filter: &str) -> Result<usize, CustomDictionaryError> {
        let trimmed = filter.trim();
        if trimmed.is_empty() {
            return self.count();
        }
        let pattern = format!("%{}%", escaped_for_like(trimmed));
        self.database.perform::<_, CustomDictionaryError>(move |connection| {
            let count: i64 = connection.query_row(
                &format!("SELECT COUNT(*)\nFROM {TABLE_NAME}\nWHERE roman LIKE ? ESCAPE '\\' OR hanzi LIKE ? ESCAPE '\\';"),
                params![pattern, pattern],
                |row| row.get(0),
            )?;
            Ok(count as usize)
        })
    }

    /// Adds `row`, or replaces the one that already carries its id. The
    /// search keys are derived first, outside the transaction.
    pub fn upsert(&self, row: &CustomDictionaryRow) -> Result<(), CustomDictionaryError> {
        let search_keys = self.derived_keys(&row.roman)?;
        let limit = self.entry_limit;
        let row = row.clone();
        self.database
            .perform::<_, CustomDictionaryError>(move |connection| {
                immediate_transaction(connection, |connection| {
                    if !entry_exists(connection, &row.id)? && entry_count(connection)? >= limit {
                        return Err(CustomDictionaryError::CapacityReached { limit });
                    }
                    write_row(connection, &row, &search_keys)
                })
            })
    }

    /// Removes one entry. `false` means there was nothing with that id.
    pub fn delete(&self, id: &str) -> Result<bool, CustomDictionaryError> {
        let id = id.to_owned();
        self.database
            .perform::<_, CustomDictionaryError>(move |connection| {
                immediate_transaction(connection, |connection| {
                    let existed = entry_exists(connection, &id)?;
                    connection.execute(
                        &format!("DELETE FROM {TABLE_NAME} WHERE id = ?;"),
                        params![id],
                    )?;
                    delete_search_keys(connection, &id)?;
                    Ok(existed)
                })
            })
    }

    /// Empties the dictionary and reports how many entries went.
    pub fn delete_all(&self) -> Result<usize, CustomDictionaryError> {
        self.database
            .perform::<_, CustomDictionaryError>(|connection| {
                let removed =
                    immediate_transaction::<_, CustomDictionaryError>(connection, |connection| {
                        let existing = entry_count(connection)?;
                        connection.execute(&format!("DELETE FROM {TABLE_NAME};"), [])?;
                        connection.execute(&format!("DELETE FROM {SEARCH_KEY_TABLE_NAME};"), [])?;
                        Ok(existing)
                    })?;
                // Outside the transaction, best-effort: a file that stays large
                // is not a failed clear.
                connection.execute("VACUUM;", []).ok();
                Ok(removed)
            })
    }

    /// Rebuilds every entry's search keys when the stored database predates
    /// the current derivation (v2 → v3: the POJ `o͘` / ⁿ fix; v3 → v4: the
    /// leading-unit abbreviation face) or the engine has not taken the file
    /// over yet, then records the shape and the takeover. A file another
    /// platform wrote is always re-derived once: its stamp cannot say which
    /// derivation its keys came from (Android v7 is iOS v3 — portability D5).
    /// The stamp only ever rises to 4, never falls: Android's 10 and iOS's 6
    /// stay, so an older app of that platform opening the file sees no
    /// downgrade (user-data-engine-roadmap U8). An entry whose roman will not
    /// derive keeps its keys.
    pub fn rederive_search_keys_if_needed(&self) -> Result<(), CustomDictionaryError> {
        let stored: Option<Vec<(String, String)>> = self
            .database
            .perform::<_, CustomDictionaryError>(|connection| {
                let version = user_version(connection)?;
                if version >= SCHEMA_VERSION && is_taken_over(connection)? {
                    return Ok(None);
                }
                Ok(Some(entry_romans(connection)?))
            })?;
        let Some(stored) = stored else {
            return Ok(());
        };
        let mut derived_by_roman: HashMap<String, Vec<CustomSearchKey>> = HashMap::new();
        for (_, roman) in &stored {
            if derived_by_roman.contains_key(roman) {
                continue;
            }
            if let Some(keys) = (self.derive_search_keys)(roman).filter(|keys| !keys.is_empty()) {
                derived_by_roman.insert(roman.clone(), keys);
            }
        }
        self.database
            .perform::<_, CustomDictionaryError>(move |connection| {
                immediate_transaction(connection, |connection| {
                    // Re-read inside the transaction: an edit between the snapshot
                    // and this write already left current keys behind.
                    let current: HashMap<String, String> =
                        entry_romans(connection)?.into_iter().collect();
                    for (id, roman) in &stored {
                        if current.get(id) != Some(roman) {
                            continue;
                        }
                        if let Some(keys) = derived_by_roman.get(roman) {
                            replace_search_keys(connection, id, keys)?;
                        }
                    }
                    let version = user_version(connection)?;
                    connection.pragma_update(None, "user_version", version.max(SCHEMA_VERSION))?;
                    mark_taken_over(connection)?;
                    Ok(())
                })
            })
    }

    /// What every launch runs once the file is open: the takeover's key
    /// re-derivation, then the seed entries. Blocks (both go through
    /// `perform`), so never on a UI thread or a store worker. A failure is
    /// logged and the store stays usable.
    pub fn finish_takeover(&self) {
        if let Err(error) = self.rederive_search_keys_if_needed() {
            log::error!("custom_dictionary.rederive_failed error={error}");
        }
        if let Err(error) = self.seed_if_empty() {
            log::error!("custom_dictionary.seed_failed error={error}");
        }
    }

    /// Writes the seed entries, but only into a dictionary nobody has
    /// touched — deleting one seed and relaunching must not bring it back.
    pub fn seed_if_empty(&self) -> Result<(), CustomDictionaryError> {
        let seeds: Vec<(CustomDictionaryRow, Vec<CustomSearchKey>)> = Self::seed_entries()
            .into_iter()
            .map(|row| self.derived_keys(&row.roman).map(|keys| (row, keys)))
            .collect::<Result<_, _>>()?;
        self.database
            .perform::<_, CustomDictionaryError>(move |connection| {
                immediate_transaction(connection, |connection| {
                    if entry_count(connection)? != 0 {
                        return Ok(());
                    }
                    for (row, keys) in &seeds {
                        write_row(connection, row, keys)?;
                    }
                    Ok(())
                })
            })
    }

    /// Imports parsed rows, skipping the ones already stored and stopping at
    /// the cap. Reaching the cap partway is not an error; a file whose own
    /// row count is over the cap is refused before anything is written. Both
    /// the cap and the duplicate check happen INSIDE each chunk's transaction.
    pub fn batch_import(
        &self,
        rows: &[CustomDictionaryRow],
    ) -> Result<CustomDictionaryImportResult, CustomDictionaryError> {
        let limit = self.entry_limit;
        if rows.len() > limit {
            return Err(CustomDictionaryError::CapacityReached { limit });
        }
        if rows.is_empty() {
            return Ok(CustomDictionaryImportResult {
                imported: 0,
                skipped: 0,
            });
        }
        let mut seen_in_file = HashSet::new();
        let candidates: Vec<&CustomDictionaryRow> = rows
            .iter()
            .filter(|row| seen_in_file.insert(row.identity()))
            .collect();
        let mut derived_by_roman: HashMap<&str, Vec<CustomSearchKey>> = HashMap::new();
        for row in &candidates {
            if !derived_by_roman.contains_key(row.roman.as_str()) {
                derived_by_roman.insert(&row.roman, self.derived_keys(&row.roman)?);
            }
        }
        let mut imported = 0;
        for chunk in candidates.chunks(IMPORT_CHUNK_SIZE) {
            // Paired here, owned: the worker runs this after the loop's
            // borrow of the file has to be over.
            let chunk: Vec<(CustomDictionaryRow, Vec<CustomSearchKey>)> = chunk
                .iter()
                .map(|row| ((*row).clone(), derived_by_roman[row.roman.as_str()].clone()))
                .collect();
            imported += self
                .database
                .perform::<_, CustomDictionaryError>(move |connection| {
                    immediate_transaction(connection, |connection| {
                        let mut stored_count = entry_count(connection)?;
                        let mut written = 0;
                        for (row, search_keys) in &chunk {
                            if stored_count >= limit {
                                break;
                            }
                            if row_exists(connection, &row.identity())? {
                                continue;
                            }
                            write_row(connection, row, search_keys)?;
                            stored_count += 1;
                            written += 1;
                        }
                        Ok(written)
                    })
                })?;
        }
        Ok(CustomDictionaryImportResult {
            imported,
            skipped: rows.len() - imported,
        })
    }

    fn derived_keys(&self, roman: &str) -> Result<Vec<CustomSearchKey>, CustomDictionaryError> {
        (self.derive_search_keys)(roman)
            .filter(|keys| !keys.is_empty())
            .ok_or_else(|| CustomDictionaryError::SearchKeyDerivationFailed {
                roman: roman.to_owned(),
            })
    }
}

impl CustomDictionarySource for CustomDictionaryStore {
    fn rows_matching(&self, family: &str, form: &str, key: &str) -> Vec<CustomEntry> {
        let query = CustomSearchKey {
            family: family.to_owned(),
            form: form.to_owned(),
            key: key.to_owned(),
        };
        CustomDictionaryStore::rows_matching(self, &query, Self::KEYSTROKE_LIMIT)
            .into_iter()
            .map(|row| CustomEntry {
                roman: row.roman,
                hanzi: row.hanzi,
            })
            .collect()
    }
}

/// `%` / `_` / `\` in user text, made literal for a `LIKE … ESCAPE '\'`.
fn escaped_for_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn row_exists(
    connection: &Connection,
    identity: &CustomDictionaryIdentity,
) -> rusqlite::Result<bool> {
    connection
        .query_row(
            &format!("SELECT 1 FROM {TABLE_NAME} WHERE roman = ? AND hanzi = ? LIMIT 1;"),
            params![identity.roman, identity.hanzi],
            |_| Ok(()),
        )
        .optional()
        .map(|found| found.is_some())
}

/// The entry and its search keys, written together. Must be called inside
/// a transaction.
fn write_row(
    connection: &Connection,
    row: &CustomDictionaryRow,
    search_keys: &[CustomSearchKey],
) -> Result<(), CustomDictionaryError> {
    connection.execute(
        &format!(
            "INSERT INTO {TABLE_NAME} (id, roman, hanzi, created_at, updated_at)\nVALUES (?, ?, ?, ?, ?)\nON CONFLICT(id) DO UPDATE SET\n    roman = excluded.roman,\n    hanzi = excluded.hanzi,\n    updated_at = excluded.updated_at;"
        ),
        params![row.id, row.roman, row.hanzi, row.created_at, row.updated_at],
    )?;
    // Replace rather than add: an edited roman must not stay findable under
    // the keys of the roman it replaced.
    replace_search_keys(connection, &row.id, search_keys)
}

fn replace_search_keys(
    connection: &Connection,
    entry_id: &str,
    search_keys: &[CustomSearchKey],
) -> Result<(), CustomDictionaryError> {
    delete_search_keys(connection, entry_id)?;
    for search_key in search_keys {
        connection.execute(
            &format!("INSERT INTO {SEARCH_KEY_TABLE_NAME} (entry_id, family, form, key)\nVALUES (?, ?, ?, ?);"),
            params![entry_id, search_key.family, search_key.form, search_key.key],
        )?;
    }
    Ok(())
}

fn delete_search_keys(connection: &Connection, entry_id: &str) -> rusqlite::Result<()> {
    connection.execute(
        &format!("DELETE FROM {SEARCH_KEY_TABLE_NAME} WHERE entry_id = ?;"),
        params![entry_id],
    )?;
    Ok(())
}

fn entry_romans(connection: &Connection) -> rusqlite::Result<Vec<(String, String)>> {
    let mut statement = connection.prepare(&format!("SELECT id, roman FROM {TABLE_NAME};"))?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

fn entry_count(connection: &Connection) -> rusqlite::Result<usize> {
    let count: i64 =
        connection.query_row(&format!("SELECT COUNT(*) FROM {TABLE_NAME};"), [], |row| {
            row.get(0)
        })?;
    Ok(count as usize)
}

fn entry_exists(connection: &Connection, id: &str) -> rusqlite::Result<bool> {
    connection
        .query_row(
            &format!("SELECT 1 FROM {TABLE_NAME} WHERE id = ? LIMIT 1;"),
            params![id],
            |_| Ok(()),
        )
        .optional()
        .map(|found| found.is_some())
}

fn decode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CustomDictionaryRow> {
    Ok(CustomDictionaryRow {
        id: row.get(0)?,
        roman: row.get(1)?,
        hanzi: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

/// The current shape, created directly. Deliberately does NOT stamp
/// `user_version`: creating tables says nothing about whether the ROWS were
/// derived by the current logic — `rederive_search_keys_if_needed` records
/// the shape once it has made the data match it.
fn apply_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {TABLE_NAME} (\n    id TEXT PRIMARY KEY,\n    roman TEXT NOT NULL,\n    hanzi TEXT NOT NULL,\n    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,\n    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\n);\nCREATE INDEX IF NOT EXISTS idx_custom_roman ON {TABLE_NAME}(roman);\nCREATE TABLE IF NOT EXISTS {SEARCH_KEY_TABLE_NAME} (\n    entry_id TEXT NOT NULL,\n    family   TEXT NOT NULL,\n    form     TEXT NOT NULL,\n    key      TEXT NOT NULL\n);\nCREATE INDEX IF NOT EXISTS idx_csk_lookup ON {SEARCH_KEY_TABLE_NAME}(family, form, key);\nCREATE INDEX IF NOT EXISTS idx_csk_entry ON {SEARCH_KEY_TABLE_NAME}(entry_id);"
    ))?;
    drop_learned_rows_if_present(connection)
}

/// The §50 leftovers of the unreleased 2026-09-20 shape (learned phrases
/// parked in this table under `origin` / `learn_count` + a partial unique
/// index; they live in `learned_phrases.db` since 2026-09-21): the index
/// goes, the rows the column marked as learned go with their side keys,
/// then the two columns themselves (the bundled SQLite has `DROP COLUMN` on
/// every platform, so an Android v10 file that reached v9 loses the inert
/// pair its own 3.22 could not drop) — after which the gate is false and
/// this is two read-only pragmas on every later open. A database that never
/// had the shape (every released one) runs nothing else. The branch is one
/// immediate transaction, as the IME and the settings app open the same
/// file; every statement in it is idempotent, so two processes both taking
/// it is harmless.
/// mirrors macos/.../Storage/CustomDictionaryStore.swift `dropLearnedRowsIfPresent`.
fn drop_learned_rows_if_present(connection: &Connection) -> rusqlite::Result<()> {
    if !has_column(connection, TABLE_NAME, "origin")?
        && !has_column(connection, TABLE_NAME, "learn_count")?
    {
        return Ok(());
    }
    immediate_transaction::<_, rusqlite::Error>(connection, |connection| {
        // Each column checked on its own: a file can carry one without the
        // other (a hand-edited or half-written v9), and `DROP COLUMN` on an
        // absent column fails the whole open.
        connection.execute_batch("DROP INDEX IF EXISTS idx_custom_learned_pair;")?;
        if has_column(connection, TABLE_NAME, "origin")? {
            connection.execute_batch(&format!(
                "DELETE FROM {SEARCH_KEY_TABLE_NAME} WHERE entry_id IN (SELECT id FROM {TABLE_NAME} WHERE origin = 1);\nDELETE FROM {TABLE_NAME} WHERE origin = 1;\nALTER TABLE {TABLE_NAME} DROP COLUMN origin;"
            ))?;
        }
        if has_column(connection, TABLE_NAME, "learn_count")? {
            connection.execute_batch(&format!(
                "ALTER TABLE {TABLE_NAME} DROP COLUMN learn_count;"
            ))?;
        }
        Ok(())
    })
}
