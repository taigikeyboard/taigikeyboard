//! The three learning stores against real SQLite files in temporary
//! directories. Ported from `macos/Tests/TaigiInputMethodCoreTests/
//! {LearningStore,LearningCapacity,CustomDictionaryStore}Tests.swift`.

mod common;

use common::{pair, paths, scratch};
use std::sync::{Arc, Mutex};
use userdata::{
    derive_custom_query_key, derive_custom_search_keys, CustomDictionaryError, CustomDictionaryRow,
    CustomDictionarySource, CustomDictionaryStore, CustomSearchKey, FrequencySource, JournalMode,
    LearnedPhraseRow, LearnedPhraseSource, LearnedPhraseStore, LearningCapacity, SearchKeyDeriver,
    UserAssociationStore, UserDataStores, UserFrequencyStore,
};

fn frequency_store(
    directory: &tempfile::TempDir,
    capacity: LearningCapacity,
) -> UserFrequencyStore {
    let store = UserFrequencyStore::new(paths(directory).frequency, JournalMode::Wal, capacity);
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
    let store = CustomDictionaryStore::new(
        paths(directory).custom_dictionary,
        JournalMode::Wal,
        deriver,
        limit,
    );
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
    // trace: LearningStoreTests.swift — a word is (Hanji, canonical TL).
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
        paths(&directory).frequency,
        JournalMode::Wal,
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

#[test]
fn bigrams_are_keyed_on_both_readings_and_written_in_order() {
    let directory = scratch();
    let store = UserAssociationStore::new(
        paths(&directory).association,
        JournalMode::Wal,
        UserAssociationStore::shipped_capacity(),
    );
    store.open_blocking();
    store.record(&[
        pair("重", "tîng", "複", "hok"),
        pair("重", "tāng", "複", "hok"),
    ]);
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
        paths(&directory).association,
        JournalMode::Wal,
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

// INVARIANT_CUSTOM_DICT_CAPACITY (behavioral-invariants.md §27)
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
        paths(&directory).custom_dictionary,
        JournalMode::Wal,
        stub_deriver(""),
        CustomDictionaryStore::MAX_ENTRIES,
    );
    assert!(store.rows_matching(&query_key("gua", "tl"), 20).is_empty());
    assert!(store.count().is_err());
}

// INVARIANT_CUSTOM_DICT_CROSS_MODE (behavioral-invariants.md §26)
#[test]
fn the_engine_derivation_finds_a_poj_entry_typed_as_tl() {
    // The real seam: keys from the engine, query key from the engine.
    let directory = scratch();
    let stores = UserDataStores::new(directory.path().to_path_buf());
    stores.custom_dictionary.open_blocking();
    stores.custom_dictionary.seed_if_empty().unwrap();
    let query = derive_custom_query_key("tsiahpa", "tl").expect("a query key");
    // The `Arc` also implements the trait (its keystroke-path shape); the
    // inherent, row-returning method is named explicitly.
    let found = CustomDictionaryStore::rows_matching(&stores.custom_dictionary, &query, 20);
    assert_eq!(hanzi_of(&found), ["食飽未"]);
    assert!(derive_custom_search_keys("gâu-tsá").is_some_and(|keys| !keys.is_empty()));
    let poj_query = derive_custom_query_key("chiahpa", "poj").expect("a POJ query key");
    assert_eq!(
        hanzi_of(&CustomDictionaryStore::rows_matching(
            &stores.custom_dictionary,
            &poj_query,
            20
        )),
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
    stores.learned_phrases.open_blocking();
    stores.frequency.record("字", "ji");
    stores.association.record(&[pair("字", "ji", "典", "tian")]);
    stores.custom_dictionary.seed_if_empty().unwrap();
    stores.learned_phrases.learn_phrase("記起來", "kì--khí-lâi");
    assert_eq!(stores.association.delete_all().unwrap(), 1);
    assert_eq!(stores.frequency.all_rows().unwrap().len(), 1);
    assert_eq!(stores.custom_dictionary.count().unwrap(), 2);
    assert_eq!(stores.learned_phrases.all_rows().unwrap().len(), 1);
}

// ---- Learned phrases (§50) — `learned_phrases.db` --------------------------

fn learned_store(directory: &tempfile::TempDir, limit: usize) -> LearnedPhraseStore {
    let store = LearnedPhraseStore::new(
        paths(directory).learned_phrases,
        JournalMode::Wal,
        Arc::new(derive_custom_search_keys),
        limit,
    );
    store.open_blocking();
    assert!(store.is_ready());
    store
}

fn learned_matches(store: &LearnedPhraseStore, input: &str, mode: &str) -> Vec<String> {
    // Learning is queued behind the writer; `all_rows` is the barrier.
    store.all_rows();
    let key = derive_custom_query_key(input, mode).expect("query key");
    LearnedPhraseSource::rows_matching(store, &key.family, &key.form, &key.key)
        .into_iter()
        .map(|row| row.hanzi)
        .collect()
}

#[test]
fn learning_the_same_pair_twice_is_one_row_with_count_two_found_by_the_whole_buffer_only() {
    let directory = scratch();
    let store = learned_store(&directory, LearnedPhraseStore::MAX_ENTRIES);
    store.learn_phrase("記起來", "kì--khí-lâi");
    store.learn_phrase("記起來", "kì--khí-lâi");
    store.learn_phrase("", "kì--khí-lâi");
    store.learn_phrase("記起來", "");
    assert_eq!(
        store.all_rows().unwrap(),
        vec![LearnedPhraseRow {
            hanzi: "記起來".into(),
            canonical_tl: "kì--khí-lâi".into(),
            learn_count: 2,
        }]
    );
    assert_eq!(learned_matches(&store, "kikhilai", "tl"), ["記起來"]);
    assert_eq!(learned_matches(&store, "ki3khi2lai5", "tl"), ["記起來"]);
    assert_eq!(learned_matches(&store, "kikhilai", "poj"), ["記起來"]);
    assert!(
        learned_matches(&store, "kikhi", "tl").is_empty(),
        "a prefix must not match"
    );
    assert!(
        learned_matches(&store, "kikhilaia", "tl").is_empty(),
        "a longer buffer must not match"
    );
}

#[test]
fn touching_bumps_a_known_pair_and_ignores_an_unknown_one_most_composed_first() {
    let directory = scratch();
    let store = learned_store(&directory, LearnedPhraseStore::MAX_ENTRIES);
    store.learn_phrase("機起來", "ki-khí-lâi");
    store.learn_phrase("記起來", "kì--khí-lâi");
    store.touch_phrase("記起來", "kì--khí-lâi");
    store.touch_phrase("台語", "tâi-gí");
    assert_eq!(
        store
            .all_rows()
            .unwrap()
            .iter()
            .map(|row| (row.hanzi.as_str(), row.learn_count))
            .collect::<Vec<_>>(),
        [("記起來", 2), ("機起來", 1)]
    );
    assert_eq!(
        learned_matches(&store, "kikhilai", "tl"),
        ["記起來", "機起來"]
    );
}

#[test]
fn learning_past_the_cap_evicts_the_fewest_composed_row_and_its_keys_never_the_newest() {
    let directory = scratch();
    let store = learned_store(&directory, 3);
    for i in 0..3 {
        store.learn_phrase(&format!("詞{i}"), &format!("su-{i}"));
    }
    store.learn_phrase("詞0", "su-0");
    store.learn_phrase("新詞", "sin-su");
    let rows = store.all_rows().unwrap();
    let hanzi: Vec<&str> = rows.iter().map(|row| row.hanzi.as_str()).collect();
    assert_eq!(rows.len(), 3, "rows stay at the cap; got {hanzi:?}");
    assert!(hanzi.contains(&"詞0"), "the twice-composed row survives");
    assert!(hanzi.contains(&"新詞"), "the newest learn is kept");
    let evicted = if hanzi.contains(&"詞1") {
        "su2"
    } else {
        "su1"
    };
    assert!(
        learned_matches(&store, evicted, "tl").is_empty(),
        "the evicted row's keys are gone"
    );
}

#[test]
fn wiping_learned_phrases_clears_rows_and_keys_and_the_store_learns_again() {
    let directory = scratch();
    let store = learned_store(&directory, LearnedPhraseStore::MAX_ENTRIES);
    store.learn_phrase("記起來", "kì--khí-lâi");
    assert_eq!(store.delete_all().unwrap(), 1);
    assert!(store.all_rows().unwrap().is_empty());
    assert!(learned_matches(&store, "kikhilai", "tl").is_empty());
    store.learn_phrase("記起來", "kì--khí-lâi");
    assert_eq!(learned_matches(&store, "kikhilai", "tl"), ["記起來"]);
}

#[test]
fn a_custom_dictionary_that_reached_the_parked_learned_shape_keeps_only_its_manual_rows() {
    let directory = scratch();
    {
        let connection =
            rusqlite::Connection::open(directory.path().join("custom_dictionary.db")).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE custom_dictionary (id TEXT PRIMARY KEY, roman TEXT NOT NULL, hanzi TEXT NOT NULL, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, origin INTEGER NOT NULL DEFAULT 0, learn_count INTEGER NOT NULL DEFAULT 0);
                 CREATE TABLE custom_search_key (entry_id TEXT NOT NULL, family TEXT NOT NULL, form TEXT NOT NULL, key TEXT NOT NULL);
                 CREATE UNIQUE INDEX idx_custom_learned_pair ON custom_dictionary(hanzi, roman) WHERE origin = 1;
                 INSERT INTO custom_dictionary (id, roman, hanzi) VALUES ('m1', 'tâi-gí', '台語');
                 INSERT INTO custom_search_key VALUES ('m1', 'tl', 'notone', 'taigi');
                 INSERT INTO custom_dictionary (id, roman, hanzi, origin, learn_count) VALUES ('l1', 'kì--khí-lâi', '記起來', 1, 3);
                 INSERT INTO custom_search_key VALUES ('l1', 'tl', 'notone', 'kikhilai');
                 PRAGMA user_version = 4;",
            )
            .unwrap();
    }
    let store = custom_store(
        &directory,
        stub_deriver(""),
        CustomDictionaryStore::MAX_ENTRIES,
    );
    assert_eq!(hanzi_of(&store.all_rows().unwrap()), ["台語"]);
    assert!(
        store
            .rows_matching(&query_key("kikhilai", "tl"), 20)
            .is_empty(),
        "the learned row's keys went with it"
    );
    assert_eq!(
        hanzi_of(&store.rows_matching(&query_key("taigi", "tl"), 20)),
        ["台語"]
    );
    let connection =
        rusqlite::Connection::open(directory.path().join("custom_dictionary.db")).unwrap();
    let columns: Vec<String> = connection
        .prepare("PRAGMA table_info(custom_dictionary);")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(
        !columns.iter().any(|c| c == "origin" || c == "learn_count"),
        "the columns are dropped; got {columns:?}"
    );
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

// INVARIANT_NEXTWORD_PREV_HANJI_LOOKUP (behavioral-invariants.md §24)
#[test]
fn predictions_read_the_exact_reading_first_then_untagged_then_the_others() {
    // trace: §24 tiers — prev_tl = query (0), '' (1), other (2); within a
    // tier count DESC. 重/tāng → 複 has the highest count but the wrong
    // reading, so it ranks last for a tîng query, and is still returned.
    let directory = scratch();
    let store = UserAssociationStore::new(
        paths(&directory).association,
        JournalMode::Wal,
        UserAssociationStore::shipped_capacity(),
    );
    store.open_blocking();
    for _ in 0..3 {
        store.record(&[pair("重", "tāng", "量", "liōng")]);
    }
    store.record(&[pair("重", "", "要", "iàu")]);
    store.record(&[pair("重", "tîng", "複", "hok")]);
    store.all_rows(); // flush the queued writes

    let rows = store.rows_following("重", "tîng", 10).unwrap();

    let order: Vec<&str> = rows.iter().map(|row| row.next.as_str()).collect();
    assert_eq!(order, vec!["複", "要", "量"]);
    assert_eq!(rows[2].count, 3);
    assert!(rows[0].last_used_ms > 0);
    assert_eq!(store.rows_following("重", "tîng", 1).unwrap().len(), 1);
}

#[test]
fn a_restore_fills_the_room_left_past_duplicates() {
    // trace: import_until_full — the stored row and the file's repeat are
    // skipped without using up the cap, so the later new rows still land
    // until the dictionary holds `limit`; the rest are skipped, no refusal.
    let directory = scratch();
    let store = custom_store(&directory, stub_deriver(""), 3);
    store.upsert(&CustomDictionaryRow::new("a", "甲")).unwrap();

    let rows: Vec<CustomDictionaryRow> = [
        ("a", "甲"),
        ("a", "甲"),
        ("b", "乙"),
        ("c", "丙"),
        ("d", "丁"),
    ]
    .iter()
    .map(|(roman, hanzi)| CustomDictionaryRow::new(roman, hanzi))
    .collect();
    let result = store.import_until_full(&rows).unwrap();

    assert_eq!(result.imported, 2);
    assert_eq!(store.count().unwrap(), 3);
    assert!(
        matches!(
            store.batch_import(&rows),
            Err(CustomDictionaryError::CapacityReached { .. })
        ),
        "a CSV over the cap is still refused whole"
    );
}
