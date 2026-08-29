//! The three learning stores against real SQLite files in temporary
//! directories. Ported from `macos/Tests/TaigiInputMethodCoreTests/
//! {LearningStore,LearningCapacity,CustomDictionaryStore}Tests.swift`.

use std::sync::{Arc, Mutex};
use taigi_windows_core::composing::{AssociationSink, CustomDictionarySource, FrequencySource};
use taigi_windows_core::engine::{
    derive_custom_query_key, derive_custom_search_keys, AssociationPair, CustomSearchKey,
};
use taigi_windows_core::settings::InputMode;
use taigi_windows_storage::{
    CustomDictionaryError, CustomDictionaryRow, CustomDictionaryStore, LearningCapacity,
    SearchKeyDeriver, UserAssociationStore, UserDataStores, UserFrequencyStore,
};

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn frequency_store(
    directory: &tempfile::TempDir,
    capacity: LearningCapacity,
) -> UserFrequencyStore {
    let store = UserFrequencyStore::new(directory.path().to_path_buf(), capacity);
    store.open_blocking();
    assert!(store.is_ready());
    store
}

/// A stub derivation: every roman yields one TL and one POJ notone key
/// with `prefix` in front, so a test can tell which derivation wrote a row.
fn stub_deriver(prefix: &'static str) -> SearchKeyDeriver {
    Arc::new(move |roman: &str| {
        Some(vec![
            CustomSearchKey {
                family: "tl".into(),
                form: "notone".into(),
                key: format!("{prefix}{roman}"),
            },
            CustomSearchKey {
                family: "poj".into(),
                form: "notone".into(),
                key: format!("{prefix}{roman}"),
            },
        ])
    })
}

fn custom_store(
    directory: &tempfile::TempDir,
    deriver: SearchKeyDeriver,
    limit: usize,
) -> CustomDictionaryStore {
    let store = CustomDictionaryStore::new(directory.path().to_path_buf(), deriver, limit);
    store.open_blocking();
    store
}

fn query_key(key: &str, family: &str) -> CustomSearchKey {
    CustomSearchKey {
        family: family.into(),
        form: "notone".into(),
        key: key.into(),
    }
}

fn hanzi_of(rows: &[CustomDictionaryRow]) -> Vec<&str> {
    rows.iter().map(|r| r.hanzi.as_str()).collect()
}

// Frequency

#[test]
fn recording_counts_the_pair_and_reads_back_by_word() {
    // trace: LearningStoreTests.swift — a word is (漢字, canonical TL).
    let directory = scratch();
    let store = frequency_store(&directory, UserFrequencyStore::shipped_capacity());
    store.record("重", "tāng");
    store.record("重", "tāng");
    store.record("重", "tîng");
    store.record("", "tîng");
    let rows = store.all_rows().unwrap();
    assert_eq!(
        rows.len(),
        2,
        "two readings, two rows; the empty word is refused"
    );
    assert_eq!(
        (rows[0].word.as_str(), rows[0].tl.as_str(), rows[0].count),
        ("重", "tāng", 2)
    );
    assert_eq!(rows[1].count, 1);
    assert!(
        rows[0].last_used_ms > 1_600_000_000_000,
        "milliseconds since the epoch"
    );
    let by_word =
        FrequencySource::rows_for_words(&store, &["重".to_owned(), "無".to_owned()]).unwrap();
    assert_eq!(by_word.len(), 2);
    assert_eq!(store.rows_for_words(&[]).unwrap().len(), 0);
    assert_eq!(store.delete_all().unwrap(), 2);
    assert!(store.all_rows().unwrap().is_empty());
}

#[test]
fn a_store_that_is_not_open_answers_none_rather_than_waiting() {
    let directory = scratch();
    let store = UserFrequencyStore::new(
        directory.path().to_path_buf(),
        UserFrequencyStore::shipped_capacity(),
    );
    assert!(!store.is_ready());
    assert_eq!(store.rows_for_words(&["字".to_owned()]), None);
    store.record("字", "ji");
    assert!(
        store.delete_all().is_err(),
        "the user's action reports not-open"
    );
}

#[test]
fn recording_past_the_cap_prunes_down_below_it_least_used_first() {
    // trace: LearningCapacityTests.swift:29-73.
    let directory = scratch();
    let store = frequency_store(
        &directory,
        LearningCapacity::new("user_frequency", 10, 4, 1),
    );
    for index in 0..20 {
        store.record(&format!("字{index}"), &format!("tsi{index}"));
    }
    let rows = store.all_rows().unwrap();
    assert!(
        rows.len() <= 10,
        "the table grew past its cap: {}",
        rows.len()
    );
    assert!(
        !rows.is_empty(),
        "pruning emptied the table instead of trimming it"
    );

    let directory = scratch();
    let store = frequency_store(&directory, LearningCapacity::new("user_frequency", 3, 2, 1));
    for _ in 0..5 {
        store.record("常", "siông");
    }
    let once_used: Vec<String> = (0..8).map(|i| format!("罕{i}")).collect();
    for word in &once_used {
        store.record(word, "hán");
    }
    let survivors: Vec<String> = store
        .all_rows()
        .unwrap()
        .into_iter()
        .map(|r| r.word)
        .collect();
    assert!(
        survivors.contains(&"常".to_owned()),
        "the most-used row was pruned: {survivors:?}"
    );
    assert!(
        once_used.iter().filter(|w| survivors.contains(w)).count() < once_used.len(),
        "nothing was pruned: {survivors:?}"
    );
}

// Association

fn pair(previous: &str, previous_tl: &str, next: &str, next_tl: &str) -> AssociationPair {
    AssociationPair {
        previous: previous.into(),
        previous_tl: previous_tl.into(),
        next: next.into(),
        next_tl: next_tl.into(),
    }
}

#[test]
fn bigrams_are_keyed_on_both_readings_and_written_in_order() {
    let directory = scratch();
    let store = UserAssociationStore::new(
        directory.path().to_path_buf(),
        UserAssociationStore::shipped_capacity(),
    );
    store.open_blocking();
    AssociationSink::record(
        &store,
        &[
            pair("重", "tîng", "複", "hok"),
            pair("重", "tāng", "複", "hok"),
        ],
    );
    store.record(&[
        pair("重", "tîng", "複", "hok"),
        pair("", "x", "複", "hok"),
        pair("a", "a", "", ""),
    ]);
    let rows = store.all_rows().unwrap();
    assert_eq!(
        rows.len(),
        2,
        "different prev_tl = different rows; empty halves refused"
    );
    assert_eq!(rows[0].pair.previous_tl, "tîng");
    assert_eq!(rows[0].count, 2);
    assert_eq!(store.delete_all().unwrap(), 2);
}

#[test]
fn recording_associations_past_the_cap_prunes_down_below_it() {
    // trace: LearningCapacityTests.swift:79-100 — a compound commit counts
    // several rows towards one throttle tick.
    let directory = scratch();
    let store = UserAssociationStore::new(
        directory.path().to_path_buf(),
        LearningCapacity::new("user_association", 10, 4, 1),
    );
    store.open_blocking();
    for index in 0..10 {
        store.record(&[
            pair(&format!("前{index}"), "a", &format!("後{index}"), "b"),
            pair(&format!("後{index}"), "b", &format!("再{index}"), "c"),
        ]);
    }
    let rows = store.all_rows().unwrap();
    assert!(rows.len() <= 10, "{}", rows.len());
    assert!(!rows.is_empty());
}

// Custom dictionary

#[test]
fn an_added_entry_is_found_by_its_key_from_another_romanization_and_by_prefix() {
    // trace: CustomDictionaryStoreTests.swift:74-107.
    let directory = scratch();
    let store = custom_store(
        &directory,
        stub_deriver(""),
        CustomDictionaryStore::MAX_ENTRIES,
    );
    store
        .upsert(&CustomDictionaryRow::new("gua", "我"))
        .unwrap();
    store
        .upsert(&CustomDictionaryRow::new("taigi", "台語"))
        .unwrap();
    assert_eq!(
        hanzi_of(&store.rows_matching(&query_key("gua", "tl"), 20)),
        ["我"]
    );
    assert_eq!(store.rows_matching(&query_key("gua", "poj"), 20).len(), 1);
    assert_eq!(
        hanzi_of(&store.rows_matching(&query_key("tai", "tl"), 20)),
        ["台語"]
    );
    let via_trait = CustomDictionarySource::rows_matching(&store, "tl", "notone", "ta");
    assert_eq!(via_trait.len(), 1);
    assert_eq!(via_trait[0].roman, "taigi");
    assert_eq!(store.count().unwrap(), 2);
}

#[test]
fn editing_the_romanization_drops_the_old_keys_and_deleting_removes_the_entry() {
    // trace: CustomDictionaryStoreTests.swift:109-135.
    let directory = scratch();
    let store = custom_store(
        &directory,
        stub_deriver(""),
        CustomDictionaryStore::MAX_ENTRIES,
    );
    let mut entry = CustomDictionaryRow::new("gua", "我");
    store.upsert(&entry).unwrap();
    entry.roman = "goa".into();
    store.upsert(&entry).unwrap();
    assert_eq!(store.rows_matching(&query_key("goa", "tl"), 20).len(), 1);
    assert!(
        store.rows_matching(&query_key("gua", "tl"), 20).is_empty(),
        "still answering to the old romanization"
    );
    assert_eq!(store.count().unwrap(), 1, "an edit is not a second row");
    assert!(store.delete(&entry.id).unwrap());
    assert!(
        !store.delete(&entry.id).unwrap(),
        "nothing left with that id"
    );
    assert!(store.rows_matching(&query_key("goa", "tl"), 20).is_empty());
}

#[test]
fn the_capacity_refuses_a_new_entry_but_never_an_edit() {
    let directory = scratch();
    let store = custom_store(&directory, stub_deriver(""), 2);
    let first = CustomDictionaryRow::new("a", "甲");
    store.upsert(&first).unwrap();
    store.upsert(&CustomDictionaryRow::new("b", "乙")).unwrap();
    assert!(matches!(
        store.upsert(&CustomDictionaryRow::new("c", "丙")),
        Err(CustomDictionaryError::CapacityReached { limit: 2 })
    ));
    let mut edited = first.clone();
    edited.hanzi = "假".into();
    store.upsert(&edited).unwrap();
    assert_eq!(store.count().unwrap(), 2);
    assert!(
        store.rows_matching(&query_key("c", "tl"), 20).is_empty(),
        "the refused row left no keys"
    );
}

#[test]
fn a_roman_that_derives_no_keys_is_refused_before_anything_is_written() {
    let directory = scratch();
    let refusing: SearchKeyDeriver = Arc::new(|_| Some(Vec::new()));
    let store = custom_store(&directory, refusing, CustomDictionaryStore::MAX_ENTRIES);
    assert!(matches!(
        store.upsert(&CustomDictionaryRow::new("???", "問")),
        Err(CustomDictionaryError::SearchKeyDerivationFailed { .. })
    ));
    assert_eq!(store.count().unwrap(), 0);
}

#[test]
fn listing_pages_filters_and_counts_with_one_predicate() {
    let directory = scratch();
    let store = custom_store(
        &directory,
        stub_deriver(""),
        CustomDictionaryStore::MAX_ENTRIES,
    );
    for (roman, hanzi) in [("gua", "我"), ("li", "你"), ("i", "伊"), ("guan_a", "阮仔")] {
        store
            .upsert(&CustomDictionaryRow::new(roman, hanzi))
            .unwrap();
    }
    assert_eq!(store.rows("", 2, 0).unwrap().len(), 2);
    assert_eq!(store.rows("", 10, 3).unwrap().len(), 1);
    assert_eq!(store.count_matching("gua").unwrap(), 2, "gua + guan_a");
    assert_eq!(store.rows("gua", 10, 0).unwrap().len(), 2);
    assert_eq!(
        store.count_matching("_a").unwrap(),
        1,
        "`_` is literal, not a wildcard"
    );
    assert_eq!(
        store.count_matching("你").unwrap(),
        1,
        "the hanzi column filters too"
    );
    assert_eq!(
        store.count_matching("  ").unwrap(),
        4,
        "blank filter = everything"
    );
    assert_eq!(store.all_rows().unwrap().len(), 4);
    assert_eq!(store.delete_all().unwrap(), 4);
    assert_eq!(store.count().unwrap(), 0);
}

#[test]
fn seeds_land_only_in_an_untouched_dictionary() {
    // trace: CustomDictionaryStoreTests.swift seed cases.
    let directory = scratch();
    let store = custom_store(
        &directory,
        stub_deriver(""),
        CustomDictionaryStore::MAX_ENTRIES,
    );
    store.seed_if_empty().unwrap();
    assert_eq!(store.count().unwrap(), 2);
    assert!(store.delete("default-gau-tsa").unwrap());
    store.seed_if_empty().unwrap();
    assert_eq!(
        store.count().unwrap(),
        1,
        "deleting one seed and relaunching must not bring it back"
    );
}

#[test]
fn batch_import_skips_duplicates_stops_at_the_cap_and_refuses_an_oversize_file() {
    // trace: CustomDictionaryStoreTests.swift import cases.
    let directory = scratch();
    let store = custom_store(&directory, stub_deriver(""), 3);
    store
        .upsert(&CustomDictionaryRow::new("gua", "我"))
        .unwrap();
    let rows = [
        CustomDictionaryRow::new("gua", "我"),
        CustomDictionaryRow::new("li", "你"),
        CustomDictionaryRow::new("li", "你"),
    ];
    let result = store.batch_import(&rows).unwrap();
    assert_eq!(
        (result.imported, result.skipped),
        (1, 2),
        "a stored duplicate and an in-file duplicate"
    );
    let more = [
        CustomDictionaryRow::new("i", "伊"),
        CustomDictionaryRow::new("in", "𪜶"),
    ];
    let result = store.batch_import(&more).unwrap();
    assert_eq!(
        (result.imported, result.skipped),
        (1, 1),
        "the cap stops the import partway"
    );
    assert_eq!(store.count().unwrap(), 3);
    let too_many: Vec<_> = (0..4)
        .map(|i| CustomDictionaryRow::new(&format!("r{i}"), ""))
        .collect();
    assert!(matches!(
        store.batch_import(&too_many),
        Err(CustomDictionaryError::CapacityReached { limit: 3 })
    ));
    assert_eq!(store.batch_import(&[]).unwrap().imported, 0);
}

#[test]
fn keys_written_by_an_older_derivation_are_rederived_once() {
    // trace: CustomDictionaryStoreTests.swift:322-412.
    let directory = scratch();
    let old_store = custom_store(
        &directory,
        stub_deriver("old-"),
        CustomDictionaryStore::MAX_ENTRIES,
    );
    let entry = CustomDictionaryRow::new("gua", "我");
    old_store.upsert(&entry).unwrap();
    assert_eq!(
        hanzi_of(&old_store.rows_matching(&query_key("old-gua", "tl"), 20)),
        ["我"]
    );
    drop(old_store);
    set_user_version(&directory, 2);

    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let recording = {
        let calls = Arc::clone(&calls);
        let inner = stub_deriver("new-");
        Arc::new(move |roman: &str| {
            calls.lock().unwrap().push(roman.to_owned());
            inner(roman)
        }) as SearchKeyDeriver
    };
    let new_store = custom_store(&directory, recording, CustomDictionaryStore::MAX_ENTRIES);
    new_store.rederive_search_keys_if_needed().unwrap();
    assert_eq!(
        hanzi_of(&new_store.rows_matching(&query_key("new-gua", "tl"), 20)),
        ["我"]
    );
    assert!(
        new_store
            .rows_matching(&query_key("old-gua", "tl"), 20)
            .is_empty(),
        "the superseded key has to be gone"
    );
    assert_eq!(calls.lock().unwrap().as_slice(), ["gua"]);
    // A store at the current shape is not re-derived.
    new_store.rederive_search_keys_if_needed().unwrap();
    assert_eq!(
        calls.lock().unwrap().len(),
        1,
        "a second launch derived nothing"
    );
}

#[test]
fn an_unversioned_store_with_rows_is_rederived_and_an_edit_in_flight_keeps_its_own_keys() {
    let directory = scratch();
    let old_store = custom_store(
        &directory,
        stub_deriver("old-"),
        CustomDictionaryStore::MAX_ENTRIES,
    );
    let entry = CustomDictionaryRow::new("gua", "我");
    old_store.upsert(&entry).unwrap();
    drop(old_store);
    set_user_version(&directory, 0);

    let new_store = custom_store(
        &directory,
        stub_deriver("new-"),
        CustomDictionaryStore::MAX_ENTRIES,
    );
    // The edit landing between the snapshot and the re-derivation write.
    let mut edited = entry.clone();
    edited.roman = "goa".into();
    new_store.upsert(&edited).unwrap();
    new_store.rederive_search_keys_if_needed().unwrap();
    assert_eq!(
        hanzi_of(&new_store.rows_matching(&query_key("new-goa", "tl"), 20)),
        ["我"]
    );
    assert!(new_store
        .rows_matching(&query_key("new-gua", "tl"), 20)
        .is_empty());
}

/// Stamp the version an older build would have recorded, from a second
/// connection to the same file.
fn set_user_version(directory: &tempfile::TempDir, version: i64) {
    let connection =
        rusqlite::Connection::open(directory.path().join("custom_dictionary.db")).unwrap();
    connection
        .pragma_update(None, "user_version", version)
        .unwrap();
}

#[test]
fn a_store_that_is_not_open_answers_no_rows_rather_than_waiting() {
    let directory = scratch();
    let store = CustomDictionaryStore::new(
        directory.path().to_path_buf(),
        stub_deriver(""),
        CustomDictionaryStore::MAX_ENTRIES,
    );
    assert!(store.rows_matching(&query_key("gua", "tl"), 20).is_empty());
    assert!(store.count().is_err());
}

#[test]
fn the_engine_derivation_finds_a_poj_entry_typed_as_tl() {
    // The real seam: keys from the engine, query key from the engine.
    let directory = scratch();
    let stores = UserDataStores::new(directory.path().to_path_buf());
    stores.custom_dictionary.open_blocking();
    stores.custom_dictionary.seed_if_empty().unwrap();
    let query = derive_custom_query_key("tsiahpa", InputMode::Tl).expect("a query key");
    let found = stores.custom_dictionary.rows_matching(&query, 20);
    assert_eq!(hanzi_of(&found), ["食飽未"]);
    assert!(derive_custom_search_keys("gâu-tsá").is_some_and(|keys| !keys.is_empty()));
    let poj_query = derive_custom_query_key("chiahpa", InputMode::Poj).expect("a POJ query key");
    assert_eq!(
        hanzi_of(&stores.custom_dictionary.rows_matching(&poj_query, 20)),
        ["食飽未"]
    );
}

// Codex PR4 post-impl: the oracle cases the first pass left out.

#[test]
fn the_throttle_lets_the_table_overshoot_between_checks_and_never_prunes_under_the_cap() {
    // trace: LearningCapacityTests — shipped throttle 100 keeps the COUNT off
    // the keystroke path; only every Nth record checks.
    let directory = scratch();
    let store = frequency_store(
        &directory,
        LearningCapacity::new("user_frequency", 5, 2, 10),
    );
    for index in 0..9 {
        store.record(&format!("字{index}"), "tsi");
    }
    assert_eq!(
        store.all_rows().unwrap().len(),
        9,
        "nine writes, no check yet: overshoot allowed"
    );
    store.record("字9", "tsi");
    // trace: count=10 > max=5 → LIMIT min(batch=2, 10-5+2=7) = 2 → 8 rows.
    // One check deletes ONE batch; the cap is reached over later checks.
    assert_eq!(
        store.all_rows().unwrap().len(),
        8,
        "the tenth write checks and prunes one batch"
    );

    let directory = scratch();
    let store = frequency_store(
        &directory,
        LearningCapacity::new("user_frequency", 50, 2, 1),
    );
    for index in 0..10 {
        store.record(&format!("字{index}"), "tsi");
    }
    assert_eq!(
        store.all_rows().unwrap().len(),
        10,
        "under the cap nothing is pruned"
    );
}

#[test]
fn clearing_one_learning_store_leaves_the_others_alone() {
    let directory = scratch();
    let stores = UserDataStores::new(directory.path().to_path_buf());
    stores.frequency.open_blocking();
    stores.association.open_blocking();
    stores.custom_dictionary.open_blocking();
    stores.frequency.record("字", "ji");
    stores.association.record(&[pair("字", "ji", "典", "tian")]);
    stores.custom_dictionary.seed_if_empty().unwrap();
    assert_eq!(stores.association.delete_all().unwrap(), 1);
    assert_eq!(stores.frequency.all_rows().unwrap().len(), 1);
    assert_eq!(stores.custom_dictionary.count().unwrap(), 2);
}

#[test]
fn an_import_crosses_the_chunk_boundary_and_derives_each_romanization_once() {
    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let recording = {
        let calls = Arc::clone(&calls);
        let inner = stub_deriver("");
        Arc::new(move |roman: &str| {
            calls.lock().unwrap().push(roman.to_owned());
            inner(roman)
        }) as SearchKeyDeriver
    };
    let directory = scratch();
    let store = custom_store(&directory, recording, CustomDictionaryStore::MAX_ENTRIES);
    // 1203 rows: two full chunks of 500 and a partial third; every third row
    // shares a romanization with the one before it (same word, other hanzi).
    let rows: Vec<CustomDictionaryRow> = (0..1203)
        .map(|i| {
            CustomDictionaryRow::new(
                &format!("r{}", i - (i % 3 == 2) as usize),
                &format!("字{i}"),
            )
        })
        .collect();
    let result = store.batch_import(&rows).unwrap();
    assert_eq!((result.imported, result.skipped), (1203, 0));
    assert_eq!(store.count().unwrap(), 1203);
    let distinct: std::collections::HashSet<&String> = rows.iter().map(|r| &r.roman).collect();
    assert_eq!(
        calls.lock().unwrap().len(),
        distinct.len(),
        "one derivation per distinct romanization"
    );
}

#[test]
fn an_import_whose_derivation_fails_writes_nothing() {
    let directory = scratch();
    let refusing: SearchKeyDeriver = Arc::new(|roman: &str| {
        if roman == "bad" {
            Some(Vec::new())
        } else {
            stub_deriver("")(roman)
        }
    });
    let store = custom_store(&directory, refusing, CustomDictionaryStore::MAX_ENTRIES);
    let rows = [
        CustomDictionaryRow::new("gua", "我"),
        CustomDictionaryRow::new("bad", "壞"),
    ];
    assert!(matches!(
        store.batch_import(&rows),
        Err(CustomDictionaryError::SearchKeyDerivationFailed { .. })
    ));
    assert_eq!(
        store.count().unwrap(),
        0,
        "the failure happens before the first row lands"
    );
}

#[test]
fn a_romanization_only_entry_is_stored_and_found_and_the_seeds_carry_their_ids() {
    let directory = scratch();
    let store = custom_store(
        &directory,
        stub_deriver(""),
        CustomDictionaryStore::MAX_ENTRIES,
    );
    store
        .upsert(&CustomDictionaryRow::new("tsiah", ""))
        .unwrap();
    let found = store.rows_matching(&query_key("tsiah", "tl"), 20);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].hanzi, "", "an empty 漢字 column is legitimate");
    store.delete_all().unwrap();
    store.seed_if_empty().unwrap();
    let mut seeded = store.all_rows().unwrap();
    seeded.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(
        seeded
            .iter()
            .map(|r| (r.id.as_str(), r.roman.as_str(), r.hanzi.as_str()))
            .collect::<Vec<_>>(),
        [
            ("default-gau-tsa", "gâu-tsá", "𠢕早"),
            ("default-tsiah-pa-bue", "tsia̍h-pá--buē", "食飽未")
        ],
        "same ids as iOS, so the same word is the same row on every platform"
    );
    assert_eq!(seeded[0].created_at.len(), 19, "yyyy-MM-dd HH:mm:ss");
}

#[test]
fn perform_from_inside_the_worker_is_refused_rather_than_deadlocking() {
    // trace: Codex PR4 BLOCK — a job that re-enters `perform` would wait for
    // a barrier the worker can never reach.
    let directory = scratch();
    let database = taigi_windows_storage::UserDataDatabase::new(
        "probe.db",
        "Probe",
        directory.path().to_path_buf(),
        |_| Ok(()),
    );
    database.open_blocking();
    let outcome: Result<
        Result<(), taigi_windows_storage::UserDataDatabaseError>,
        taigi_windows_storage::UserDataDatabaseError,
    > = database.perform(|_| Ok(Ok(())));
    assert!(outcome.is_ok());
    let handle = std::sync::Arc::new(database);
    let inner = std::sync::Arc::clone(&handle);
    let (tx, rx) = std::sync::mpsc::channel();
    handle.write(move |_| {
        let nested: Result<(), taigi_windows_storage::UserDataDatabaseError> =
            inner.perform(|_| Ok(()));
        tx.send(nested.is_err()).ok();
        Ok(())
    });
    assert!(
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap(),
        "re-entrant perform is an error"
    );
    // A write queued before open is not lost: the open is a job in the same queue.
    let directory = scratch();
    let store = UserFrequencyStore::new(
        directory.path().to_path_buf(),
        UserFrequencyStore::shipped_capacity(),
    );
    store.open();
    store.record("先", "sian");
    assert_eq!(
        store.all_rows().unwrap().len(),
        1,
        "queued behind the open, not dropped"
    );
}
