//! The `.taigi` codec against backups the phones wrote (roadmap P4b): an
//! iOS export and an Android one must restore, merging with what is there,
//! and what the engine writes must restore on the other side.

mod common;

use common::{pair, paths, scratch};
use userdata::{
    export_backup, import_backup, BackupError, JournalMode, UserDataStores, BACKUP_VERSION,
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
    let seeded = stores.custom_dictionary.count().unwrap();

    let imported = import_backup(&stores, ANDROID_BACKUP.as_bytes()).unwrap();

    // 𠢕早 is a seed entry already there; the Hanji-less row is skipped.
    assert_eq!(imported.custom_dictionary, 0);
    assert_eq!(stores.custom_dictionary.count().unwrap(), seeded);
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

#[test]
fn a_restore_keeps_the_larger_count() {
    let directory = scratch();
    let stores = stores(&directory);
    for _ in 0..9 {
        stores.frequency.record("台灣", "tâi-uân");
    }
    stores.frequency.all_rows(); // flush the queued writes

    import_backup(&stores, IOS_BACKUP.as_bytes()).unwrap();

    let rows = stores
        .frequency
        .rows_for_words(&["台灣".to_owned()])
        .unwrap();
    assert_eq!(rows[0].count, 9, "MAX(9, 5)");
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
    assert!(text.contains("\"lastUsed\": \"\""));
    assert!(
        text.find("\"appVersion\"").unwrap() < text.find("\"version\"").unwrap(),
        "sorted keys"
    );
    assert!(!text.contains("origin"), "never written");

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
        .any(|row| row.hanzi == "台灣"));
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
