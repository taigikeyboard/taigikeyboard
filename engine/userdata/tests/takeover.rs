//! The engine taking over files the platforms' native stores wrote
//! (user-data-engine-roadmap P2, U7 / U8). Every fixture is built from the
//! DDL the platform shipped, copied verbatim with its provenance, then
//! opened by the engine store: rows and ids must survive, the stamp must stay
//! a number that platform's own older app accepts, the takeover is marked
//! once, and a file from a later build is left alone.

use rusqlite::Connection;
mod common;

use common::{pair, paths, scratch};
use std::path::Path;
use std::sync::Arc;
use userdata::{
    derive_custom_search_keys, AssociationPair, CustomDictionaryStore, CustomSearchKey,
    JournalMode, LearnedPhraseStore, UserAssociationStore, UserFrequencyStore,
    TAIGI_APPLICATION_ID,
};

/// Writes a native store's file the way that platform left it.
fn build(path: &Path, sql: &str) {
    Connection::open(path).unwrap().execute_batch(sql).unwrap();
}

fn raw(path: &Path) -> Connection {
    Connection::open(path).unwrap()
}

fn pragma(path: &Path, name: &str) -> i64 {
    raw(path)
        .pragma_query_value(None, name, |row| row.get(0))
        .unwrap()
}

fn columns(path: &Path, table: &str) -> Vec<String> {
    let connection = raw(path);
    let mut statement = connection
        .prepare("SELECT name FROM pragma_table_info(?1);")
        .unwrap();
    statement
        .query_map([table], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn snapshot(path: &Path) -> std::path::PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".pre-engine");
    name.into()
}

fn frequency(path: &Path) -> UserFrequencyStore {
    let store = UserFrequencyStore::new(
        path.to_path_buf(),
        JournalMode::Delete,
        UserFrequencyStore::shipped_capacity(),
    );
    store.open_blocking();
    store
}

fn association(path: &Path) -> UserAssociationStore {
    let store = UserAssociationStore::new(
        path.to_path_buf(),
        JournalMode::Delete,
        UserAssociationStore::shipped_capacity(),
    );
    store.open_blocking();
    store
}

fn custom_dictionary(path: &Path) -> CustomDictionaryStore {
    let store = CustomDictionaryStore::new(
        path.to_path_buf(),
        JournalMode::Delete,
        Arc::new(derive_custom_search_keys),
        CustomDictionaryStore::MAX_ENTRIES,
    );
    store.open_blocking();
    store.rederive_search_keys_if_needed().unwrap();
    store
}

// ---------------------------------------------------------------- frequency

/// iOS before R5 — `ios/.../Lexicon/Database/UserFrequencySchema.swift` at
/// `1677e862^` (`createFrequencyTable`): `word UNIQUE`, no `tl`, stamp 0.
const IOS_FREQUENCY_V1: &str = "
CREATE TABLE IF NOT EXISTS user_frequency (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL UNIQUE,
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_word ON user_frequency(word);
CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value TEXT);
INSERT INTO metadata (key, value) VALUES ('schema_version', '1.0');
INSERT INTO user_frequency (id, word, count, last_used) VALUES
    (7, '台灣', 5, '2026-01-02 03:04:05'),
    (9, '食飯', 2, '2026-01-03 03:04:05');
";

#[test]
fn a_pre_pair_key_frequency_table_keeps_its_rows_ids_and_counts() {
    // trace: no `tl` column → migrate_to_pair_key_if_needed rebuilds,
    // tl = '' for both rows; ids 7 / 9 copied; idx_word dropped; stamp 2.
    let directory = scratch();
    let path = paths(&directory).frequency;
    build(&path, IOS_FREQUENCY_V1);

    let store = frequency(&path);

    assert!(store.is_ready());
    let rows = store
        .rows_for_words(&["台灣".into(), "食飯".into()])
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.tl.is_empty()));
    assert_eq!(rows.iter().find(|row| row.word == "台灣").unwrap().count, 5);
    let ids: Vec<i64> = raw(&path)
        .prepare("SELECT id FROM user_frequency ORDER BY id;")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(ids, vec![7, 9], "ids survive the rebuild");
    assert!(columns(&path, "user_frequency").contains(&"tl".to_owned()));
    assert_eq!(
        pragma(&path, "user_version"),
        2,
        "iOS / Android's own number"
    );
    assert_eq!(
        pragma(&path, "application_id"),
        i64::from(TAIGI_APPLICATION_ID)
    );
    let copy = snapshot(&path);
    assert!(copy.exists(), "the pre-takeover copy is taken");
    assert!(
        !columns(&copy, "user_frequency").contains(&"tl".to_owned()),
        "and it is the file as the platform left it"
    );
}

/// Android before R5 — `UserFrequencyService.kt` at `1677e862^`
/// (`createUserFrequencyTable` + `createUserFrequencyIndexes` +
/// `createMetadataTable`), stamped 1 by `SQLiteOpenHelper`: the iOS shape
/// plus two indexes on the columns the rebuild copies.
const ANDROID_FREQUENCY_V1: &str = "
CREATE TABLE user_frequency (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL UNIQUE,
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_word ON user_frequency(word);
CREATE INDEX idx_count ON user_frequency(count DESC);
CREATE INDEX idx_last_used ON user_frequency(last_used DESC);
CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value TEXT);
INSERT INTO metadata (key, value) VALUES ('app_version', '3.4.2');
INSERT INTO user_frequency (id, word, count, last_used) VALUES
    (3, '台灣', 4, '2026-01-02 03:04:05');
PRAGMA user_version = 1;
";

#[test]
fn an_android_v1_frequency_table_keeps_its_rows_and_its_metadata() {
    // trace: no `tl` → rebuild; idx_count / idx_last_used go with the old
    // table; id 3 and count 4 copied; `metadata` untouched; stamp 1 → 2.
    let directory = scratch();
    let path = paths(&directory).frequency;
    build(&path, ANDROID_FREQUENCY_V1);

    let store = frequency(&path);

    assert!(store.is_ready());
    let rows = store.rows_for_words(&["台灣".into()]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].count, 4);
    assert_eq!(pragma(&path, "user_version"), 2);
    let app_version: String = raw(&path)
        .query_row(
            "SELECT value FROM metadata WHERE key = 'app_version';",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        app_version, "3.4.2",
        "the old app's own table is left alone"
    );
}

#[test]
fn a_pair_key_frequency_table_left_at_version_zero_is_not_rebuilt() {
    // trace: iOS manages user_version by hand, so a new-shape file can sit
    // at 0 — the shape gate (tl present) skips the rebuild.
    let directory = scratch();
    let path = paths(&directory).frequency;
    build(
        &path,
        "CREATE TABLE user_frequency (
            id INTEGER PRIMARY KEY AUTOINCREMENT, word TEXT NOT NULL,
            tl TEXT NOT NULL DEFAULT '', count INTEGER DEFAULT 1,
            last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, UNIQUE(word, tl));
         INSERT INTO user_frequency (id, word, tl, count) VALUES (3, '重', 'tîng', 4);",
    );

    let store = frequency(&path);

    let rows = store.rows_for_words(&["重".into()]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].tl, "tîng");
    assert_eq!(pragma(&path, "user_version"), 2);
}

#[test]
fn a_frequency_file_from_a_later_build_stays_closed_and_untouched() {
    let directory = scratch();
    let path = paths(&directory).frequency;
    build(
        &path,
        "CREATE TABLE user_frequency (id INTEGER PRIMARY KEY, word TEXT);
         PRAGMA user_version = 3;",
    );

    let store = frequency(&path);

    assert!(!store.is_ready(), "closed: neutral ranking, not a guess");
    assert_eq!(pragma(&path, "user_version"), 3);
    assert_eq!(pragma(&path, "application_id"), 0);
    assert!(!snapshot(&path).exists(), "nothing copied, nothing changed");
}

// -------------------------------------------------------------- association

/// iOS v5 — `ios/.../NextWord/Repository/NextWordSchema.swift` at
/// `c2b8a581^` (`createTables`): the key lacks `prev_tl`.
const IOS_ASSOCIATION_V5: &str = "
CREATE TABLE IF NOT EXISTS user_association (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    prev_word TEXT NOT NULL,
    prev_tl TEXT DEFAULT '',
    next_word TEXT NOT NULL,
    next_tl TEXT DEFAULT '',
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(prev_word, next_word, next_tl)
);
CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl ON user_association(prev_word, prev_tl);
INSERT INTO user_association (id, prev_word, prev_tl, next_word, next_tl, count) VALUES
    (11, '重', 'tîng', '複', 'hok', 3);
PRAGMA user_version = 5;
";

/// iOS / Android v3 — the v5 table before v3 → v4's
/// `ALTER TABLE user_association ADD COLUMN prev_tl` (same file, `c2b8a581^`).
const ASSOCIATION_V3: &str = "
CREATE TABLE user_association (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    prev_word TEXT NOT NULL,
    next_word TEXT NOT NULL,
    next_tl TEXT DEFAULT '',
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(prev_word, next_word, next_tl)
);
CREATE INDEX idx_user_prev_word ON user_association(prev_word);
INSERT INTO user_association (id, prev_word, next_word, next_tl, count) VALUES
    (4, '台', '灣', NULL, 2);
PRAGMA user_version = 3;
";

/// Android v1 — the table `migrateV0ToV2` (`NextWordService.kt` at
/// `c2b8a581^`) read from: `next_poj` / `delimiter` alongside the rest.
const ANDROID_ASSOCIATION_V1: &str = "
CREATE TABLE user_association (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    prev_word TEXT NOT NULL,
    next_word TEXT NOT NULL,
    next_tl TEXT,
    next_poj TEXT,
    delimiter TEXT,
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO user_association (id, prev_word, next_word, next_tl, next_poj, delimiter, count) VALUES
    (21, '你', '好', 'hó', 'hó', ' ', 6);
PRAGMA user_version = 1;
";

fn pairs(store: &UserAssociationStore) -> Vec<(AssociationPair, i64)> {
    store
        .all_rows()
        .unwrap()
        .into_iter()
        .map(|row| (row.pair, row.count))
        .collect()
}

#[test]
fn a_v5_association_table_is_rebuilt_under_the_v6_key_with_its_rows() {
    // trace: v5 → migrate → rebuild_to_v6 (prev_tl, next_tl present →
    // COALESCE); id 11 copied; the key now carries prev_tl, so 重/tāng → 複
    // is a second row, not a bump of the first.
    let directory = scratch();
    let path = paths(&directory).association;
    build(&path, IOS_ASSOCIATION_V5);

    let store = association(&path);
    store.record(&[pair("重", "tāng", "複", "hok")]);

    assert_eq!(
        pairs(&store),
        vec![
            (pair("重", "tîng", "複", "hok"), 3),
            (pair("重", "tāng", "複", "hok"), 1),
        ]
    );
    let id: i64 = raw(&path)
        .query_row(
            "SELECT id FROM user_association WHERE prev_tl = 'tîng';",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(id, 11);
    assert_eq!(pragma(&path, "user_version"), 6);
    assert_eq!(
        pragma(&path, "application_id"),
        i64::from(TAIGI_APPLICATION_ID)
    );
}

#[test]
fn a_v3_association_table_without_prev_tl_gets_empty_readings() {
    let directory = scratch();
    let path = paths(&directory).association;
    build(&path, ASSOCIATION_V3);

    let store = association(&path);

    assert_eq!(pairs(&store), vec![(pair("台", "", "灣", ""), 2)]);
    let indexes: Vec<String> = raw(&path)
        .prepare("SELECT name FROM sqlite_master WHERE type = 'index' AND name LIKE 'idx_%';")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(indexes, vec!["idx_user_prev_word_tl".to_owned()]);
}

#[test]
fn an_android_v1_association_table_is_kept_as_android_keeps_it() {
    let directory = scratch();
    let path = paths(&directory).association;
    build(&path, ANDROID_ASSOCIATION_V1);

    let store = association(&path);

    assert_eq!(pairs(&store), vec![(pair("你", "", "好", "hó"), 6)]);
    assert!(!columns(&path, "user_association").contains(&"next_poj".to_owned()));
}

#[test]
fn a_v2_association_table_is_dropped_as_both_phones_drop_it() {
    // trace: v2's stamp is ambiguous (Android NextWordService.kt migrateToV6).
    let directory = scratch();
    let path = paths(&directory).association;
    build(
        &path,
        "CREATE TABLE user_association (id INTEGER PRIMARY KEY, prev_word TEXT, next_word TEXT,
             next_tl TEXT, count INTEGER, last_used TIMESTAMP, UNIQUE(prev_word, next_word));
         INSERT INTO user_association VALUES (1, '台', '灣', '', 2, '2026-01-01 00:00:00');
         PRAGMA user_version = 2;",
    );

    let store = association(&path);

    assert!(pairs(&store).is_empty());
    assert!(columns(&path, "user_association").contains(&"prev_tl".to_owned()));
    assert!(
        snapshot(&path).exists(),
        "the dropped rows are still in the copy"
    );
}

#[test]
fn an_old_association_table_without_the_row_columns_is_dropped() {
    let directory = scratch();
    let path = paths(&directory).association;
    build(
        &path,
        "CREATE TABLE user_association (word TEXT, following TEXT);
         INSERT INTO user_association VALUES ('台', '灣');
         PRAGMA user_version = 1;",
    );

    let store = association(&path);

    assert!(store.is_ready());
    assert!(pairs(&store).is_empty());
}

// -------------------------------------------------------- custom dictionary

/// iOS v6 / Android v10 — `CustomDictionaryService.kt` `CREATE_TABLE_SQL` +
/// `CREATE_SEARCH_KEY_TABLE_SQL`: the legacy derived columns the phones still
/// write, and keys the engine did not derive.
const PHONE_CUSTOM_DICTIONARY: &str = "
CREATE TABLE custom_dictionary (
    id TEXT PRIMARY KEY,
    roman TEXT NOT NULL,
    hanzi TEXT NOT NULL,
    notone TEXT DEFAULT '',
    abbrev TEXT DEFAULT '',
    roman_num TEXT DEFAULT '',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_custom_notone ON custom_dictionary(notone);
CREATE TABLE IF NOT EXISTS custom_search_key (entry_id TEXT NOT NULL, family TEXT NOT NULL, form TEXT NOT NULL, key TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_csk_lookup ON custom_search_key(family, form, key);
CREATE INDEX IF NOT EXISTS idx_csk_entry ON custom_search_key(entry_id);
INSERT INTO custom_dictionary (id, roman, hanzi, notone) VALUES ('E1', 'gâu-tsá', '𠢕早', 'gautsa');
INSERT INTO custom_search_key VALUES ('E1', 'tl', 'notone', 'stale');
";

fn stored_keys(path: &Path, id: &str) -> Vec<CustomSearchKey> {
    let connection = raw(path);
    let mut statement = connection
        .prepare(
            "SELECT family, form, key FROM custom_search_key WHERE entry_id = ?1 ORDER BY rowid;",
        )
        .unwrap();
    statement
        .query_map([id], |row| {
            Ok(CustomSearchKey {
                family: row.get(0)?,
                form: row.get(1)?,
                key: row.get(2)?,
            })
        })
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn an_android_v10_custom_dictionary_is_re_derived_and_keeps_its_stamp() {
    // trace: v10 ≥ 4 would skip re-derivation by stamp alone; the missing
    // application_id forces it (Android v7 == iOS v3). Stamp stays 10 so an
    // older Android app sees no downgrade; the legacy columns stay because
    // it still writes them.
    let directory = scratch();
    let path = paths(&directory).custom_dictionary;
    build(
        &path,
        &format!("{PHONE_CUSTOM_DICTIONARY}PRAGMA user_version = 10;"),
    );

    let store = custom_dictionary(&path);

    assert_eq!(store.count().unwrap(), 1);
    assert_eq!(
        stored_keys(&path, "E1"),
        derive_custom_search_keys("gâu-tsá").unwrap()
    );
    assert_eq!(pragma(&path, "user_version"), 10);
    assert_eq!(
        pragma(&path, "application_id"),
        i64::from(TAIGI_APPLICATION_ID)
    );
    assert!(columns(&path, "custom_dictionary").contains(&"notone".to_owned()));
}

#[test]
fn an_ios_v6_custom_dictionary_keeps_version_six() {
    let directory = scratch();
    let path = paths(&directory).custom_dictionary;
    build(
        &path,
        &format!("{PHONE_CUSTOM_DICTIONARY}PRAGMA user_version = 6;"),
    );

    custom_dictionary(&path);

    assert_eq!(pragma(&path, "user_version"), 6);
    assert_eq!(
        stored_keys(&path, "E1"),
        derive_custom_search_keys("gâu-tsá").unwrap()
    );
}

#[test]
fn a_v9_leftover_with_only_one_of_its_two_columns_still_opens() {
    // trace: drop_learned_rows_if_present checks each column on its own.
    let directory = scratch();
    let path = paths(&directory).custom_dictionary;
    build(
        &path,
        &format!(
            "{PHONE_CUSTOM_DICTIONARY}
             ALTER TABLE custom_dictionary ADD COLUMN origin INTEGER NOT NULL DEFAULT 0;
             INSERT INTO custom_dictionary (id, roman, hanzi, origin) VALUES ('L1', 'kì--khí-lâi', '記起來', 1);
             PRAGMA user_version = 9;"
        ),
    );

    let store = custom_dictionary(&path);

    assert!(store.is_ready());
    assert_eq!(store.count().unwrap(), 1, "the learned row went");
    assert!(!columns(&path, "custom_dictionary").contains(&"origin".to_owned()));
}

#[test]
fn a_v9_leftover_with_only_learn_count_still_opens() {
    let directory = scratch();
    let path = paths(&directory).custom_dictionary;
    build(
        &path,
        &format!(
            "{PHONE_CUSTOM_DICTIONARY}
             ALTER TABLE custom_dictionary ADD COLUMN learn_count INTEGER NOT NULL DEFAULT 0;
             PRAGMA user_version = 9;"
        ),
    );

    let store = custom_dictionary(&path);

    assert!(store.is_ready());
    assert_eq!(store.count().unwrap(), 1);
    assert!(!columns(&path, "custom_dictionary").contains(&"learn_count".to_owned()));
}

/// macOS / Windows / Linux v4 — `macos/.../Storage/CustomDictionaryStore.swift`
/// `applySchema` (the SQL the desktop stores ported byte-identically): no
/// legacy derived columns, keys the engine itself derived, stamp 4.
/// Android v3 — `CustomDictionaryService.kt` at `392d0283` (v3.4.2, the
/// first release with the dictionary): no `roman_num`, no side table.
const ANDROID_CUSTOM_DICTIONARY_V3: &str = "
CREATE TABLE custom_dictionary (
    id TEXT PRIMARY KEY,
    roman TEXT NOT NULL,
    hanzi TEXT NOT NULL,
    notone TEXT DEFAULT '',
    abbrev TEXT DEFAULT '',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_custom_roman ON custom_dictionary(roman);
CREATE INDEX idx_custom_notone ON custom_dictionary(notone);
CREATE INDEX idx_custom_abbrev ON custom_dictionary(abbrev);
INSERT INTO custom_dictionary (id, roman, hanzi, notone) VALUES ('E1', 'gâu-tsá', '𠢕早', 'gautsa');
PRAGMA user_version = 3;
";

/// Android v5 — `CustomDictionaryService.kt` at `da836ea5^` (v3.4.7 …
/// v3.6.0): `roman_num` added, still no side table.
const ANDROID_CUSTOM_DICTIONARY_V5: &str = "
CREATE TABLE custom_dictionary (
    id TEXT PRIMARY KEY,
    roman TEXT NOT NULL,
    hanzi TEXT NOT NULL,
    notone TEXT DEFAULT '',
    abbrev TEXT DEFAULT '',
    roman_num TEXT DEFAULT '',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_custom_roman ON custom_dictionary(roman);
CREATE INDEX idx_custom_notone ON custom_dictionary(notone);
CREATE INDEX idx_custom_abbrev ON custom_dictionary(abbrev);
CREATE INDEX idx_custom_roman_num ON custom_dictionary(roman_num);
INSERT INTO custom_dictionary (id, roman, hanzi, notone) VALUES ('E1', 'gâu-tsá', '𠢕早', 'gautsa');
PRAGMA user_version = 5;
";

#[test]
fn an_android_custom_dictionary_from_before_the_side_table_gets_one() {
    // trace: no custom_search_key table → apply_schema creates it; the
    // missing application_id forces re-derivation; stamp max(v, 4) — v3 → 4
    // (Android's own numbering, the "notone regenerated" step), v5 stays 5.
    for (sql, stamp) in [
        (ANDROID_CUSTOM_DICTIONARY_V3, 4),
        (ANDROID_CUSTOM_DICTIONARY_V5, 5),
    ] {
        let directory = scratch();
        let path = paths(&directory).custom_dictionary;
        build(&path, sql);

        let store = custom_dictionary(&path);

        assert_eq!(store.count().unwrap(), 1);
        assert_eq!(
            stored_keys(&path, "E1"),
            derive_custom_search_keys("gâu-tsá").unwrap()
        );
        assert_eq!(pragma(&path, "user_version"), stamp);
    }
}

#[test]
fn an_android_v7_custom_dictionary_the_last_release_wrote_keeps_version_seven() {
    // trace: mobile-3.6.8 ships DATABASE_VERSION = 7 over the side-table
    // shape (`ce99e22c`); stale keys re-derived, stamp untouched.
    let directory = scratch();
    let path = paths(&directory).custom_dictionary;
    build(
        &path,
        &format!("{PHONE_CUSTOM_DICTIONARY}PRAGMA user_version = 7;"),
    );

    let store = custom_dictionary(&path);

    assert_eq!(store.count().unwrap(), 1);
    assert_eq!(
        stored_keys(&path, "E1"),
        derive_custom_search_keys("gâu-tsá").unwrap()
    );
    assert_eq!(pragma(&path, "user_version"), 7);
}

const DESKTOP_CUSTOM_DICTIONARY_V4: &str = "
CREATE TABLE IF NOT EXISTS custom_dictionary (
    id TEXT PRIMARY KEY,
    roman TEXT NOT NULL,
    hanzi TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_custom_roman ON custom_dictionary(roman);
CREATE TABLE IF NOT EXISTS custom_search_key (entry_id TEXT NOT NULL, family TEXT NOT NULL, form TEXT NOT NULL, key TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_csk_lookup ON custom_search_key(family, form, key);
CREATE INDEX IF NOT EXISTS idx_csk_entry ON custom_search_key(entry_id);
INSERT INTO custom_dictionary (id, roman, hanzi) VALUES ('D1', 'tsia̍h-pá--buē', '食飽未');
INSERT INTO custom_search_key VALUES ('D1', 'tl', 'notone', 'stale');
PRAGMA user_version = 4;
";

#[test]
fn a_desktop_v4_custom_dictionary_is_re_derived_once_and_stays_v4() {
    // trace: v4 ≥ 4, but no application_id yet → re-derived once; the stamp
    // stays 4 and no legacy column appears.
    let directory = scratch();
    let path = paths(&directory).custom_dictionary;
    build(&path, DESKTOP_CUSTOM_DICTIONARY_V4);

    custom_dictionary(&path);

    assert_eq!(
        stored_keys(&path, "D1"),
        derive_custom_search_keys("tsia̍h-pá--buē").unwrap()
    );
    assert_eq!(pragma(&path, "user_version"), 4);
    assert!(!columns(&path, "custom_dictionary").contains(&"notone".to_owned()));
    assert!(snapshot(&path).exists());
}

#[test]
fn a_custom_dictionary_newer_than_every_platform_stays_closed() {
    let directory = scratch();
    let path = paths(&directory).custom_dictionary;
    build(
        &path,
        &format!("{PHONE_CUSTOM_DICTIONARY}PRAGMA user_version = 11;"),
    );

    let store = CustomDictionaryStore::new(
        path.clone(),
        JournalMode::Delete,
        Arc::new(derive_custom_search_keys),
        CustomDictionaryStore::MAX_ENTRIES,
    );
    store.open_blocking();

    assert!(!store.is_ready());
    assert!(store.rederive_search_keys_if_needed().is_err());
    assert_eq!(stored_keys(&path, "E1")[0].key, "stale", "untouched");
}

// ---------------------------------------------------------- the takeover

#[test]
fn a_fresh_install_takes_no_copy_and_is_marked() {
    let directory = scratch();
    let files = paths(&directory);

    frequency(&files.frequency);
    association(&files.association);
    custom_dictionary(&files.custom_dictionary);

    for path in [
        &files.frequency,
        &files.association,
        &files.custom_dictionary,
    ] {
        assert!(!snapshot(path).exists(), "{}", path.display());
        assert_eq!(
            pragma(path, "application_id"),
            i64::from(TAIGI_APPLICATION_ID)
        );
    }
    assert_eq!(pragma(&files.custom_dictionary, "user_version"), 4);
}

#[test]
fn a_fresh_install_whose_directory_does_not_exist_yet_still_opens() {
    // Android's `databases/` exists only once something wrote there.
    let directory = scratch();
    let path = directory.path().join("databases").join("user_frequency.db");

    let store = frequency(&path);

    assert!(store.is_ready());
    assert!(path.exists());
}

#[test]
fn the_copy_is_taken_once_and_a_taken_over_file_is_not_copied_again() {
    let directory = scratch();
    let path = paths(&directory).frequency;
    build(&path, IOS_FREQUENCY_V1);
    drop(frequency(&path));
    std::fs::remove_file(snapshot(&path)).unwrap();

    let store = frequency(&path);

    assert!(store.is_ready());
    assert!(
        !snapshot(&path).exists(),
        "already taken over: nothing to protect"
    );
}

#[test]
fn a_learned_phrase_file_keeps_its_rows_and_is_marked() {
    let directory = scratch();
    let path = paths(&directory).learned_phrases;
    // iOS `LearnedPhraseSchema.swift` / Android `LearnedPhraseService.kt` v1.
    build(
        &path,
        "CREATE TABLE learned_phrases (id INTEGER PRIMARY KEY, roman TEXT NOT NULL, hanzi TEXT NOT NULL,
             learn_count INTEGER NOT NULL DEFAULT 1, updated_at TEXT NOT NULL, UNIQUE(hanzi, roman));
         CREATE TABLE learned_search_key (phrase_id INTEGER NOT NULL, family TEXT NOT NULL, form TEXT NOT NULL, key TEXT NOT NULL);
         INSERT INTO learned_phrases VALUES (1, 'tsò tsìn-tshut-kháu', '做進出口', 2, '2026-09-23 00:00:00');
         PRAGMA user_version = 1;",
    );
    let store = LearnedPhraseStore::new(
        path.clone(),
        JournalMode::Delete,
        Arc::new(derive_custom_search_keys),
        LearnedPhraseStore::MAX_ENTRIES,
    );
    store.open_blocking();

    assert_eq!(store.all_rows().unwrap().len(), 1);
    assert_eq!(
        pragma(&path, "application_id"),
        i64::from(TAIGI_APPLICATION_ID)
    );
}

#[test]
fn the_journal_mode_is_the_platform_s_choice() {
    let directory = scratch();
    let files = paths(&directory);
    frequency(&files.frequency);
    let wal = UserAssociationStore::new(
        files.association.clone(),
        JournalMode::Wal,
        UserAssociationStore::shipped_capacity(),
    );
    wal.open_blocking();

    let mode = |path: &Path| -> String {
        raw(path)
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap()
    };
    assert_eq!(mode(&files.frequency), "delete");
    assert_eq!(mode(&files.association), "wal");
}
