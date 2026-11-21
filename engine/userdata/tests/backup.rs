//! The `.taigi` codec against backups the phones wrote (roadmap P4b): an
//! iOS export and an Android one must restore, merging with what is there,
//! and what the engine writes must restore on the other side.

use crate::common::{pair, paths, scratch};
use phonetics::api::derive_custom_query_key;
use std::sync::Arc;
use userdata::{
    derive_custom_search_keys, export_backup, import_backup, unix_millis_now, BackupError,
    CustomDictionaryRow, CustomDictionaryStore, JournalMode, LearnedPhraseStore, UserDataStores,
    BACKUP_VERSION,
};

fn stores(directory: &tempfile::TempDir) -> UserDataStores {
    let stores = UserDataStores::at(paths(directory), JournalMode::Delete);
    stores.open_blocking();
    stores
}

/// iOS `BackupService.exportAll` — sorted keys, `tl` present, `lastUsed` "".
const IOS_BACKUP: &str = r#"{
  "appVersion" : "3.6.8",
  "customDictionary" : [
    { "hanzi" : "台灣", "roman" : "tâi-uân" },
    { "hanzi" : "記起來", "origin" : 1, "roman" : "kì--khí-lâi" }
  ],
  "exportedAt" : "2026-09-01T00:00:00Z",
  "platform" : "ios",
  "userAssociation" : [
    { "count" : 3, "lastUsed" : "", "nextTl" : "hok", "nextWord" : "複", "prevTl" : "tîng", "prevWord" : "重" }
  ],
  "userFrequency" : [
    { "count" : 5, "lastUsed" : "", "tl" : "tâi-uân", "word" : "台灣" }
  ],
  "version" : 2
}"#;

/// Android `BackupService.exportAll` (org.json, insertion order) plus what
/// the lenient reader must tolerate: a pre-pair-key frequency row without
/// `tl`, a POJ bigram reading, a Hanji-less custom row, an unknown key.
const ANDROID_BACKUP: &str = r#"{
  "version": 2,
  "exportedAt": "2026-09-02T00:00:00Z",
  "platform": "android",
  "appVersion": "3.6.8",
  "someFutureKey": true,
  "customDictionary": [
    { "roman": "gâu-tsá", "hanzi": "𠢕早" },
    { "roman": "tsiah", "hanzi": "" }
  ],
  "userFrequency": [
    { "word": "食飯", "count": 2, "lastUsed": "" }
  ],
  "userAssociation": [
    { "prevWord": "食", "prevTl": "chia̍h", "nextWord": "飯", "nextTl": "pn̄g", "count": 4, "lastUsed": "" }
  ]
}"#;

#[test]
fn an_ios_backup_restores_and_skips_learned_rows() {
    let directory = scratch();
    let stores = stores(&directory);
    let seeded = stores.custom_dictionary.count().unwrap();

    let imported = import_backup(&stores, IOS_BACKUP.as_bytes()).unwrap();

    assert_eq!(
        imported.custom_dictionary, 1,
        "the origin = 1 row is skipped"
    );
    assert_eq!(imported.frequency, 1);
    assert_eq!(imported.association, 1);
    assert_eq!(stores.custom_dictionary.count().unwrap(), seeded + 1);
    let rows = stores
        .frequency
        .rows_for_words(&["台灣".to_owned()])
        .unwrap();
    assert_eq!((rows[0].tl.as_str(), rows[0].count), ("tâi-uân", 5));
    let following = stores.association.rows_following("重", "tîng", 10).unwrap();
    assert_eq!(following[0].next, "複");
}

#[test]
fn an_android_backup_restores_leniently() {
    let directory = scratch();
    let stores = stores(&directory);
    stores
        .custom_dictionary
        .upsert(&CustomDictionaryRow::new("gâu-tsá", "𠢕早"))
        .unwrap();

    let imported = import_backup(&stores, ANDROID_BACKUP.as_bytes()).unwrap();

    // 𠢕早 is already there; the Hanji-less row is skipped.
    assert_eq!(imported.custom_dictionary, 0);
    assert_eq!(stores.custom_dictionary.count().unwrap(), 1);
    let rows = stores
        .frequency
        .rows_for_words(&["食飯".to_owned()])
        .unwrap();
    assert_eq!(rows[0].tl, "", "no `tl` → the legacy bucket");
    let all = stores.association.all_rows().unwrap();
    assert_eq!(
        all[0].pair,
        pair("食", "tsia̍h", "飯", "pn̄g"),
        "POJ reading normalized to TL"
    );
}

/// The 台灣 row's `(count, abbrev_count, abbrev_last_used_ms)`.
fn taiwan_picks(stores: &UserDataStores) -> (i64, i64, i64) {
    let rows = stores
        .frequency
        .rows_for_words(&["台灣".to_owned()])
        .unwrap();
    assert_eq!(rows.len(), 1);
    (
        rows[0].count,
        rows[0].abbrev_count,
        rows[0].abbrev_last_used_ms,
    )
}

/// Records `readings` reading picks and `abbreviations` abbreviation picks
/// of 台灣, then waits for the writes.
fn stores_picks(stores: &UserDataStores, readings: usize, abbreviations: usize) {
    for _ in 0..readings {
        stores.frequency.record("台灣", "tâi-uân");
    }
    for _ in 0..abbreviations {
        stores.frequency.record_abbreviation("台灣", "tâi-uân");
    }
    stores.frequency.all_rows(); // flush the queued writes
}

/// A version-4 file with one 台灣 row of `count` picks, `abbrev` of them
/// through an abbreviation, last used 2025-01-02 03:04:05;
/// `abbrev_last_used` `None` leaves the `abbrevLastUsed` key out.
fn abbreviation_backup(count: i64, abbrev: i64, abbrev_last_used: Option<&str>) -> String {
    let abbrev_last_used = abbrev_last_used
        .map(|time| format!(r#""abbrevLastUsed": "{time}", "#))
        .unwrap_or_default();
    format!(
        r#"{{ "version": 4, "userFrequency": [{{ "abbrevCount": {abbrev}, {abbrev_last_used}"count": {count}, "lastUsed": "2025-01-02 03:04:05", "tl": "tâi-uân", "word": "台灣" }}] }}"#
    )
}

const V4_TIME: Option<&str> = Some("2025-01-02 03:04:05");

#[test]
fn a_restore_keeps_the_larger_count_when_neither_side_has_abbreviation_picks() {
    let directory = scratch();
    let stores = stores(&directory);
    stores_picks(&stores, 9, 0);

    import_backup(&stores, IOS_BACKUP.as_bytes()).unwrap();

    assert_eq!(taiwan_picks(&stores).0, 9, "MAX(9, 5)");
}

#[test]
fn abbreviation_picks_survive_an_export_and_restore() {
    let source_directory = scratch();
    let source = stores(&source_directory);
    stores_picks(&source, 4, 3);
    let (_, _, abbreviated_at) = taiwan_picks(&source);
    assert!(abbreviated_at > 0);

    let bytes = export_backup(&source, "ios", "3.10.0", 1_800_000_000).unwrap();
    let target_directory = scratch();
    let target = stores(&target_directory);
    import_backup(&target, &bytes).unwrap();
    assert_eq!(
        taiwan_picks(&target),
        (7, 3, abbreviated_at),
        "abbreviation picks stay abbreviation picks"
    );
}

#[test]
fn a_file_without_abbreviation_keys_restores_a_fresh_row_without_abbreviation_picks() {
    let directory = scratch();
    let stores = stores(&directory);
    import_backup(&stores, IOS_BACKUP.as_bytes()).unwrap();
    assert_eq!(taiwan_picks(&stores), (5, 0, 0), "none, never");
}

#[test]
fn a_file_without_abbreviation_keys_leaves_the_local_abbreviation_picks() {
    let directory = scratch();
    let stores = stores(&directory);
    stores_picks(&stores, 2, 3);
    let (_, _, abbreviated_at) = taiwan_picks(&stores);
    import_backup(&stores, IOS_BACKUP.as_bytes()).unwrap();
    // trace: local 5 picks / 3 abbreviation, file 5 / 0 → abbrev MAX(3, 0)
    // = 3, readings MAX(5 - 3, 5 - 0) = 5, count 5 + 3 = 8.
    assert_eq!(
        taiwan_picks(&stores),
        (8, 3, abbreviated_at),
        "the local abbreviation picks and their time stay"
    );
}

#[test]
fn a_restore_keeps_the_larger_reading_and_abbreviation_picks_apart() {
    let directory = scratch();
    let stores = stores(&directory);
    stores_picks(&stores, 8, 2);
    let (count, _, abbreviated_at) = taiwan_picks(&stores);
    assert_eq!(count, 10);

    let backup = abbreviation_backup(11, 9, V4_TIME);
    import_backup(&stores, backup.as_bytes()).unwrap();
    // trace: local 10 / 2 (8 readings), file 11 / 9 (2 readings) →
    // abbrev MAX(2, 9) = 9, readings MAX(8, 2) = 8, count 8 + 9 = 17.
    // The file's 2025 abbreviation time is older than the local one.
    let merged = taiwan_picks(&stores);
    assert_eq!(merged, (17, 9, abbreviated_at));

    import_backup(&stores, backup.as_bytes()).unwrap();
    assert_eq!(
        taiwan_picks(&stores),
        merged,
        "a second restore changes nothing"
    );
}

#[test]
fn a_restore_takes_the_file_abbreviation_time_when_the_local_row_has_none() {
    // Local row with reading picks only: `abbrev_last_used` is NULL.
    let directory = scratch();
    let stores = stores(&directory);
    stores_picks(&stores, 4, 0);
    assert_eq!(taiwan_picks(&stores), (4, 0, 0));
    import_backup(&stores, abbreviation_backup(5, 2, V4_TIME).as_bytes()).unwrap();
    // trace: local 4 / 0 (4 readings), file 5 / 2 (3 readings) → abbrev
    // MAX(0, 2) = 2, readings MAX(4, 3) = 4, count 4 + 2 = 6; time
    // MAX(NULL, file) = NULL → COALESCE(NULL, NULL, file) = file 2025 time.
    assert_eq!(taiwan_picks(&stores), (6, 2, V4_LAST_USED_MS));
}

#[test]
fn a_restore_takes_the_file_abbreviation_time_when_it_is_later() {
    // Local abbreviation time 2025 (seeded by a restore: a future time is
    // capped at now by `RESTORED_TIMESTAMP_SQL`); the file's is 2026.
    let directory = scratch();
    let stores = stores(&directory);
    import_backup(&stores, abbreviation_backup(3, 1, V4_TIME).as_bytes()).unwrap();
    assert_eq!(taiwan_picks(&stores), (3, 1, V4_LAST_USED_MS));
    let backup = abbreviation_backup(5, 3, Some("2026-01-02 03:04:05"));
    import_backup(&stores, backup.as_bytes()).unwrap();
    // trace: local 3 / 1 (2 readings), file 5 / 3 (2 readings) → abbrev
    // MAX(1, 3) = 3, readings MAX(2, 2) = 2, count 2 + 3 = 5; time
    // MAX(2025-01-02, 2026-01-02 03:04:05 UTC) = 1_767_323_045_000 ms.
    assert_eq!(taiwan_picks(&stores), (5, 3, 1_767_323_045_000));
}

#[test]
fn a_negative_hand_edited_abbreviation_count_restores_none() {
    let directory = scratch();
    let stores = stores(&directory);
    import_backup(&stores, abbreviation_backup(5, -3, V4_TIME).as_bytes()).unwrap();
    assert_eq!(taiwan_picks(&stores), (5, 0, 0), "none, no time");
}

#[test]
fn a_hand_edited_abbreviation_count_above_the_count_is_clamped_to_it() {
    let directory = scratch();
    let stores = stores(&directory);
    import_backup(&stores, abbreviation_backup(5, 99, V4_TIME).as_bytes()).unwrap();
    assert_eq!(
        taiwan_picks(&stores),
        (5, 5, V4_LAST_USED_MS),
        "never more abbreviation picks than picks"
    );
}

#[test]
fn abbreviation_picks_without_a_time_count_as_now() {
    let directory = scratch();
    let stores = stores(&directory);
    let before = unix_millis_now() / 1000 * 1000;
    import_backup(&stores, abbreviation_backup(5, 2, None).as_bytes()).unwrap();
    let (count, abbrev, abbreviated_at) = taiwan_picks(&stores);
    assert_eq!((count, abbrev), (5, 2));
    assert!(abbreviated_at >= before);
}

#[test]
fn what_the_engine_writes_restores_on_a_fresh_install() {
    let source_directory = scratch();
    let source = stores(&source_directory);
    import_backup(&source, IOS_BACKUP.as_bytes()).unwrap();

    let bytes = export_backup(&source, "macos", "3.6.10", 1_800_000_000).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains(&format!("\"version\": {BACKUP_VERSION}")));
    assert!(text.contains("\"exportedAt\": \"2027-01-15T08:00:00Z\""));
    assert!(
        !text.contains("\"lastUsed\": \"\""),
        "every learned row carries its last use"
    );
    assert!(
        text.find("\"appVersion\"").unwrap() < text.find("\"version\"").unwrap(),
        "sorted keys"
    );
    assert!(!text.contains("origin"), "never written");
    assert!(
        !text.contains("abbrev"),
        "a row without abbreviation picks writes no abbreviation keys"
    );
    // Persisted JSON key names: the code says `hanji` / `previous`, the
    // `.taigi` format keeps what older apps wrote and read.
    for key in [
        "\"hanzi\": \"台灣\"",
        "\"prevWord\": \"重\"",
        "\"prevTl\": \"tîng\"",
    ] {
        assert!(text.contains(key), "export keeps the {key} key");
    }

    let target_directory = scratch();
    let target = stores(&target_directory);
    let imported = import_backup(&target, &bytes).unwrap();
    assert_eq!(imported.frequency, 1);
    assert_eq!(imported.association, 1);
    assert!(target
        .custom_dictionary
        .all_rows()
        .unwrap()
        .iter()
        .any(|row| row.hanji == "台灣"));
}

#[test]
fn an_unreadable_file_or_version_zero_is_refused_before_any_write() {
    let directory = scratch();
    let stores = stores(&directory);

    assert!(matches!(
        import_backup(&stores, b"not json"),
        Err(BackupError::Unreadable(_))
    ));
    assert_eq!(
        import_backup(
            &stores,
            br#"{ "version": 0, "userFrequency": [{ "word": "x", "count": 1 }] }"#
        ),
        Err(BackupError::UnsupportedVersion(0))
    );
    assert!(stores
        .frequency
        .rows_for_words(&["x".to_owned()])
        .unwrap()
        .is_empty());
}

/// One bigram 蔣經國 → 德國簫 in a backup of `version`.
// trace: dictionary.csv rows 蔣經國/tsiúnn-keng-kok, 德國簫/tek-kok-siau —
// TL special finals `eng` / `ek` (taigi-phonetics-reference §3.2.6).
fn special_finals_backup(version: i64) -> String {
    format!(
        r#"{{ "version": {version}, "userAssociation": [{{ "prevWord": "蔣經國", "prevTl": "tsiúnn-keng-kok", "nextWord": "德國簫", "nextTl": "tek-kok-siau", "count": 2 }}] }}"#
    )
}

#[test]
fn a_version_3_backup_restores_canonical_readings_as_written() {
    let directory = scratch();
    let stores = stores(&directory);

    import_backup(&stores, special_finals_backup(3).as_bytes()).unwrap();

    assert_eq!(
        stores.association.all_rows().unwrap()[0].pair,
        pair("蔣經國", "tsiúnn-keng-kok", "德國簫", "tek-kok-siau")
    );
}

// Versions 1 and 2 may carry POJ, so their readings still fold; the fold
// cannot tell TL `eng` / `ek` from POJ, so it reads them as POJ `ing` / `ik`.
// trace: poj_display_to_tl_display — eng→ing, ek→ik.
#[test]
fn a_version_2_backup_still_folds_its_readings() {
    let directory = scratch();
    let stores = stores(&directory);

    import_backup(&stores, special_finals_backup(2).as_bytes()).unwrap();

    assert_eq!(
        stores.association.all_rows().unwrap()[0].pair,
        pair("蔣經國", "tsiúnn-king-kok", "德國簫", "tik-kok-siau")
    );
}

#[test]
fn what_the_engine_writes_keeps_tl_special_finals_through_a_restore() {
    let source_directory = scratch();
    let source = stores(&source_directory);
    source
        .association
        .record(&[pair("蔣經國", "tsiúnn-keng-kok", "德國簫", "tek-kok-siau")]);

    let bytes = export_backup(&source, "ios", "3.6.11", 1_800_000_000).unwrap();
    assert!(String::from_utf8(bytes.clone())
        .unwrap()
        .contains(&format!("\"version\": {BACKUP_VERSION}")));

    let target_directory = scratch();
    let target = stores(&target_directory);
    import_backup(&target, &bytes).unwrap();
    assert_eq!(
        target.association.all_rows().unwrap()[0].pair,
        pair("蔣經國", "tsiúnn-keng-kok", "德國簫", "tek-kok-siau")
    );
}

#[test]
fn a_restored_decomposed_roman_is_stored_and_exported_in_nfc() {
    // A backup written before romans were normalised (2026-10-07) restores
    // as NFC; the next export carries NFC.
    let directory = scratch();
    let stores = stores(&directory);
    let backup =
        r#"{ "version": 3, "customDictionary": [{ "roman": "li\u0301", "hanzi": "李" }] }"#;
    let imported = import_backup(&stores, backup.as_bytes()).unwrap();
    assert_eq!(imported.custom_dictionary, 1);
    let rows = stores.custom_dictionary.all_rows().unwrap();
    assert_eq!(rows[0].roman, "l\u{ed}");
    let exported =
        String::from_utf8(export_backup(&stores, "macos", "3.6.13", 1_800_000_000).unwrap())
            .unwrap();
    assert!(exported.contains("l\u{ed}") && !exported.contains("li\u{301}"));
}

/// A version-4 file with every store and old times on every learned row.
// trace: 2025-01-02 03:04:05 UTC = 1_735_689_600 (2025-01-01) + 86_400
// + 3·3600 + 4·60 + 5 = 1_735_787_045 s.
const V4_BACKUP: &str = r#"{
  "customDictionary": [
    { "createdAt": "2025-01-02 03:04:05", "hanzi": "屢", "roman": "lí", "updatedAt": "2025-01-02 03:04:05" },
    { "createdAt": "2025-03-01 00:00:00", "hanzi": "李", "roman": "lí", "updatedAt": "2025-03-01 00:00:00" }
  ],
  "learnedPhrases": [
    { "hanzi": "記起來", "learnCount": 3, "roman": "kì--khí-lâi", "updatedAt": "2025-01-02 03:04:05" }
  ],
  "userAssociation": [
    { "count": 3, "lastUsed": "2025-01-02 03:04:05", "nextTl": "hok", "nextWord": "複", "prevTl": "tîng", "prevWord": "重" }
  ],
  "userFrequency": [
    { "abbrevCount": 2, "abbrevLastUsed": "2025-01-02 03:04:05", "count": 5, "lastUsed": "2025-01-02 03:04:05", "tl": "tâi-uân", "word": "台灣" }
  ],
  "version": 4
}"#;
const V4_LAST_USED_MS: i64 = 1_735_787_045_000;

/// What typing reads from `stores`, times included: frequency
/// `(word, count, last_used_ms, abbrev_count, abbrev_last_used_ms)`,
/// association `(count, last_used_ms)`, learned phrases
/// `(hanji, canonical_tl, learn_count, updated_at_ms)`.
type TypingState = (
    Vec<(String, i64, i64, i64, i64)>,
    Vec<(i64, i64)>,
    Vec<(String, String, i64, i64)>,
);

/// The Hanji the keystroke path hands over for `typed`, in order.
fn keystroke_hanji(stores: &UserDataStores, typed: &str) -> Vec<String> {
    let query = derive_custom_query_key(typed, "tl").expect("a query key");
    stores
        .custom_dictionary
        .keystroke_rows(&query, CustomDictionaryStore::KEYSTROKE_LIMIT)
        .into_iter()
        .map(|row| row.hanji)
        .collect()
}

fn typing_state(stores: &UserDataStores) -> TypingState {
    let frequency = stores
        .frequency
        .all_rows()
        .unwrap()
        .into_iter()
        .map(|row| {
            (
                row.word,
                row.count,
                row.last_used_ms,
                row.abbrev_count,
                row.abbrev_last_used_ms,
            )
        })
        .collect();
    let association = stores
        .association
        .all_rows()
        .unwrap()
        .into_iter()
        .map(|row| (row.count, row.last_used_ms))
        .collect();
    let learned = stores
        .learned_phrases
        .backup_rows()
        .unwrap()
        .into_iter()
        .map(|(row, updated_at_ms)| (row.hanji, row.canonical_tl, row.learn_count, updated_at_ms))
        .collect();
    (frequency, association, learned)
}

#[test]
fn a_version_4_backup_round_trips_every_store_with_its_times() {
    let source_directory = scratch();
    let source = stores(&source_directory);
    let imported = import_backup(&source, V4_BACKUP.as_bytes()).unwrap();
    assert_eq!(imported.learned_phrases, 1);
    assert_eq!(
        typing_state(&source),
        (
            vec![("台灣".to_owned(), 5, V4_LAST_USED_MS, 2, V4_LAST_USED_MS)],
            vec![(3, V4_LAST_USED_MS)],
            vec![(
                "記起來".to_owned(),
                "kì--khí-lâi".to_owned(),
                3,
                V4_LAST_USED_MS
            )],
        ),
        "the file's times are kept, not stamped now"
    );

    let bytes = export_backup(&source, "android", "3.10.0", 1_800_000_000).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("\"lastUsed\": \"2025-01-02 03:04:05\""));
    assert!(text.contains("\"abbrevCount\": 2"));
    assert!(text.contains("\"abbrevLastUsed\": \"2025-01-02 03:04:05\""));
    assert!(text.contains("\"learnedPhrases\""));
    assert!(text.contains("\"createdAt\": \"2025-03-01 00:00:00\""));

    let target_directory = scratch();
    let target = stores(&target_directory);
    import_backup(&target, &bytes).unwrap();
    assert_eq!(typing_state(&target), typing_state(&source));
}

#[test]
fn a_restored_learned_phrase_is_found_by_its_whole_buffer_key() {
    let directory = scratch();
    let stores = stores(&directory);
    import_backup(&stores, V4_BACKUP.as_bytes()).unwrap();

    let query = derive_custom_query_key("kikhilai", "tl").expect("a query key");
    let found = stores
        .learned_phrases
        .rows_matching(&query, LearnedPhraseStore::KEYSTROKE_LIMIT);
    assert_eq!(found.len(), 1, "a restored phrase carries its search keys");
    assert_eq!(found[0].hanji, "記起來");
}

#[test]
fn homophone_custom_words_keep_their_keystroke_order_through_a_restore() {
    let source_directory = scratch();
    let source = stores(&source_directory);
    import_backup(&source, V4_BACKUP.as_bytes()).unwrap();
    assert_eq!(
        keystroke_hanji(&source, "li"),
        ["屢", "李"],
        "filed first leads"
    );

    let bytes = export_backup(&source, "ios", "3.10.0", 1_800_000_000).unwrap();
    let target_directory = scratch();
    let target = stores(&target_directory);
    import_backup(&target, &bytes).unwrap();
    assert_eq!(
        keystroke_hanji(&target, "li"),
        ["屢", "李"],
        "new ids, same order"
    );
}

#[test]
fn a_restore_keeps_the_later_use_and_a_file_without_times_counts_as_now() {
    let directory = scratch();
    let stores = stores(&directory);
    import_backup(&stores, V4_BACKUP.as_bytes()).unwrap();
    let last_used = |stores: &UserDataStores| stores.frequency.all_rows().unwrap()[0].last_used_ms;
    assert_eq!(last_used(&stores), V4_LAST_USED_MS);

    // The iOS version-2 file has `lastUsed` "": its 台灣 counts as used now.
    let before = unix_millis_now() / 1000 * 1000;
    import_backup(&stores, IOS_BACKUP.as_bytes()).unwrap();
    let now_stamped = last_used(&stores);
    assert!(now_stamped >= before, "later use wins");

    // The older time in the file never moves it back.
    import_backup(&stores, V4_BACKUP.as_bytes()).unwrap();
    assert_eq!(last_used(&stores), now_stamped);
}

#[test]
fn a_restore_past_the_learned_phrase_cap_keeps_the_most_composed() {
    let directory = scratch();
    let store = LearnedPhraseStore::new(
        paths(&directory).learned_phrases,
        JournalMode::Delete,
        Arc::new(derive_custom_search_keys),
        2,
    );
    store.open_blocking();
    let row = |hanji: &str, tl: &str, count: i64| {
        (
            hanji.to_owned(),
            tl.to_owned(),
            count,
            "2025-01-02 03:04:05".to_owned(),
        )
    };
    let merged = store
        .import_merge(vec![
            row("記起來", "kì--khí-lâi", 5),
            row("食飽未", "tsia̍h-pá--buē", 1),
            row("台灣人", "tâi-uân-lâng", 3),
        ])
        .unwrap();
    assert_eq!(merged, 3);
    let kept: Vec<String> = store
        .all_rows()
        .unwrap()
        .into_iter()
        .map(|row| row.hanji)
        .collect();
    assert_eq!(kept, ["記起來", "台灣人"]);
}

#[test]
fn a_restored_learned_phrase_already_stored_keeps_the_larger_count_and_later_touch() {
    let directory = scratch();
    let stores = stores(&directory);
    for _ in 0..7 {
        stores.learned_phrases.learn_phrase("記起來", "kì--khí-lâi");
    }
    let stored = |stores: &UserDataStores| stores.learned_phrases.backup_rows().unwrap();
    let (_, learned_now_ms) = stored(&stores)[0].clone();

    // The file holds 記起來 at 3 and an old touch: 7 and the local touch stay.
    import_backup(&stores, V4_BACKUP.as_bytes()).unwrap();
    let rows = stored(&stores);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0.learn_count, 7, "MAX(7, 3)");
    assert_eq!(rows[0].1, learned_now_ms, "the later touch stays");
    let query = derive_custom_query_key("kikhilai", "tl").expect("a query key");
    assert_eq!(
        stores
            .learned_phrases
            .rows_matching(&query, LearnedPhraseStore::KEYSTROKE_LIMIT)
            .len(),
        1,
        "its keys are written once, not doubled"
    );
}

#[test]
fn a_restored_bigram_keeps_the_later_use() {
    let directory = scratch();
    let stores = stores(&directory);
    let last_used =
        |stores: &UserDataStores| stores.association.all_rows().unwrap()[0].last_used_ms;
    import_backup(&stores, V4_BACKUP.as_bytes()).unwrap();
    assert_eq!(last_used(&stores), V4_LAST_USED_MS);

    let before = unix_millis_now() / 1000 * 1000;
    stores
        .association
        .record(&[pair("重", "tîng", "複", "hok")]);
    let recorded = last_used(&stores);
    assert!(recorded >= before);
    import_backup(&stores, V4_BACKUP.as_bytes()).unwrap();
    assert_eq!(
        last_used(&stores),
        recorded,
        "the older file time never wins"
    );
}

#[test]
fn homophone_custom_words_filed_in_one_second_order_by_hanji() {
    // A CSV import or a version-1 to -3 restore files a batch within one
    // second; the tie falls to the Hanji, never the per-device UUID. The
    // times are fixed here so the test never straddles a second.
    // trace: SQLite compares UTF-8 bytes — 屢 U+5C62 (E5 B1 A2) before
    // 李 U+674E (E6 9D 8E), whatever order the file lists them in.
    let directory = scratch();
    let stores = stores(&directory);
    let backup = r#"{ "version": 4, "customDictionary": [
        { "createdAt": "2025-01-02 03:04:05", "hanzi": "李", "roman": "lí" },
        { "createdAt": "2025-01-02 03:04:05", "hanzi": "屢", "roman": "lí" }
    ] }"#;
    import_backup(&stores, backup.as_bytes()).unwrap();
    assert_eq!(keystroke_hanji(&stores, "li"), ["屢", "李"]);
}
