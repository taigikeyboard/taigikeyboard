//! Dispatch integration test verifying that `FrequencyMap` boosts and
//! `ContextRanks` bigrams are shared between neutral-tone `--` Hanji forms
//! (e.g. custom `會記--得` / opt-in `khinsiann_in_hanji` display) and plain
//! Hanji (`會記得`) via `process_request`.
//! Its own process: the user-data handle and lexicon handle are process-wide.
#![cfg(feature = "user-data")]

mod common;

use std::time::{Duration, Instant};

use common::{open_user_data, record_pick, roundtrip};
use lexicon::LexiconPaths;
use protos::engine::{
    composing_request, next_word_request, request, response, AppConfig, CandidateDisplayMode,
    CandidateMessage, ComposingRequest, DecisionInput, DictionarySourceToggles, FetchAtPos,
    NextWordRequest, Reset, Start, WordSelected,
};
use test_support::{
    build_tkdb, build_tkwa, fst_entry, walker_cost_from_fixture_frequency, write_fst_set,
    write_temp, TkdbRow, TKDB_VERSION,
};
use userdata::{CustomDictionaryRow, JournalMode, UserDataPaths, UserDataStores};

const NOW_MS: i64 = 1_800_000_000_000;
const GENERATION: u64 = 1;
const KAUTIAN_BIT: u16 = 1 << 0;

fn config(khinsiann_in_hanji: bool) -> AppConfig {
    AppConfig {
        input_mode: "tl".to_owned(),
        is_hanji_first: true,
        khinsiann_in_hanji,
        candidate_display_mode: CandidateDisplayMode::SideBySide as i32,
        dictionary_sources: Some(DictionarySourceToggles {
            kautian: true,
            taigitv: true,
            itaigi: true,
            sitbut: true,
            taihoa: true,
            taijit: true,
            kungge: true,
            stti: true,
            khpoo: true,
            ..DictionarySourceToggles::default()
        }),
        ..AppConfig::default()
    }
}

fn install_synthetic_lexicon() {
    let rows = [
        ("ekitit", "繪記直", "ē-kì-tīt", 3u8, 900u32),
        ("ekitit", "會記得", "ē-kì--tit", 3u8, 500u32),
        ("tngkhi", "當機", "tng-khi", 2u8, 900u32),
        ("tngkhi", "轉去", "tńg--khì", 2u8, 500u32),
        ("hoboo", "好某", "hó-bôo", 2u8, 900u32),
        ("hoboo", "好無", "hó--bôo", 2u8, 500u32),
    ];
    let tkdb_rows: Vec<TkdbRow<'_>> = rows
        .iter()
        .map(|&(.., hanji, tl, syll, freq)| TkdbRow {
            bitmask: KAUTIAN_BIT,
            frequency: freq,
            syllable_count: Some(syll),
            // Main entry (subtag bit 0), as every shipped kautian row carries one.
            kautian_subtag: Some(1),
            walker_cost: Some(walker_cost_from_fixture_frequency(freq)),
            hanji,
            tl,
        })
        .collect();
    let dict_path = write_temp(
        "khinsiann-rank-dict.bin",
        &build_tkdb(b"TKDB", TKDB_VERSION, &tkdb_rows),
    );

    let mut fst_entries = Vec::new();
    for (idx, &(toneless, hanji, tl, ..)) in rows.iter().enumerate() {
        let rowid = (idx + 1) as u32;
        fst_entries.push(fst_entry(b"tl:", toneless, rowid));
        fst_entries.push(fst_entry(b"hanzi:", hanji, rowid));
        let poj = phonetics::api::tl_display_to_poj_display(tl);
        if let Some((canon, _)) = phonetics::canonicalize_poj_syllable(&poj) {
            fst_entries.push(fst_entry(b"poj:", &canon.replace('-', ""), rowid));
        }
    }
    let fst_path = write_fst_set("khinsiann-rank-dict.fst", fst_entries);

    let assoc_path = write_temp("khinsiann-rank-assoc.bin", &build_tkwa(2, &[]));

    let mut syll_entries = Vec::new();
    for syll in [
        "e7", "e", "ki3", "ki", "tit4", "tit", "tng", "khi", "tng2", "khi3", "ho2", "ho", "boo5",
        "boo", "si", "ua",
    ] {
        syll_entries.push(format!("tl:{syll}").into_bytes());
    }
    let syll_path = write_fst_set("khinsiann-rank-syll.fst", syll_entries);

    let paths = LexiconPaths::validated(
        fst_path.to_str().unwrap(),
        dict_path.to_str().unwrap(),
        assoc_path.to_str().unwrap(),
        syll_path.to_str().unwrap(),
        0,
    )
    .expect("validated lexicon paths");
    lexicon::EngineHandle::install(paths).expect("install synthetic lexicon");
}

fn fetch_candidates(cfg: AppConfig, raw: &str) -> Vec<CandidateMessage> {
    roundtrip(
        cfg.clone(),
        GENERATION,
        request::Payload::Composing(ComposingRequest {
            next_word_context: None,
            method: Some(composing_request::Method::Reset(Reset {})),
        }),
    );
    roundtrip(
        cfg.clone(),
        GENERATION,
        request::Payload::Composing(ComposingRequest {
            next_word_context: None,
            method: Some(composing_request::Method::Start(Start {
                text: raw.to_owned(),
            })),
        }),
    );
    let response = roundtrip(
        cfg,
        GENERATION,
        request::Payload::Composing(ComposingRequest {
            next_word_context: None,
            method: Some(composing_request::Method::FetchAtPos(FetchAtPos {
                now_ms: NOW_MS,
                ..FetchAtPos::default()
            })),
        }),
    );
    match response.payload {
        Some(response::Payload::Composing(c)) => {
            c.continuous
                .expect("FetchAtPos returns continuous response")
                .candidates
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

#[test]
fn frequency_and_context_ranks_share_across_khinsiann_and_plain_hanji() {
    install_synthetic_lexicon();

    let directory = tempfile::tempdir().unwrap();
    let paths = UserDataPaths::in_directory(directory.path());
    let stores = UserDataStores::at(paths, JournalMode::Delete);
    stores.open_blocking();
    // Custom entries on `tngkhi` for testing custom `FrequencyMap` lookup,
    // and two equal-frequency custom entries on `siua` for testing custom
    // `ContextRanks` lookup.
    stores
        .custom_dictionary
        .upsert(&CustomDictionaryRow::new("tng-khi", "當--機"))
        .unwrap();
    stores
        .custom_dictionary
        .upsert(&CustomDictionaryRow::new("tńg--khì", "轉--去"))
        .unwrap();
    stores
        .custom_dictionary
        .upsert(&CustomDictionaryRow::new("sì-uā", "四--話"))
        .unwrap();
    stores
        .custom_dictionary
        .upsert(&CustomDictionaryRow::new("sì--uā", "是--話"))
        .unwrap();
    drop(stores);

    open_user_data(directory.path());

    // 1. Baseline: with no user frequency, higher-frequency builtin `繪記直`
    // (900) leads `會記得` (500) on `ekitit`.
    let before: Vec<String> = fetch_candidates(config(false), "ekitit")
        .into_iter()
        .filter_map(|c| c.hanji)
        .collect();
    assert_eq!(before.first().map(String::as_str), Some("繪記直"));

    // Record usage of `--` display form `會記--得`: persisted as plain `會記得`.
    record_pick("會記--得", "ē-kì--tit");

    let reader = UserDataStores::at(
        UserDataPaths::in_directory(directory.path()),
        JournalMode::Delete,
    );
    reader.open_blocking();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let rows = reader
            .frequency
            .rows_for_words(&["會記得".to_owned()])
            .unwrap_or_default();
        if rows.iter().any(|r| r.word == "會記得" && r.count == 1) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "frequency write timed out: {rows:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }

    // With `khinsiann_in_hanji = false`, builtin `會記得` gets the frequency
    // boost from committing `會記--得` and beats `繪記直` (900).
    let after_off: Vec<String> = fetch_candidates(config(false), "ekitit")
        .into_iter()
        .filter_map(|c| c.hanji)
        .collect();
    assert_eq!(after_off.first().map(String::as_str), Some("會記得"));

    // 2. Record usage of plain `轉去` (`tńg--khì`); verify custom `轉--去`
    // shares that `FrequencyMap` boost against custom `當--機`.
    record_pick("轉去", "tńg--khì");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let rows = reader
            .frequency
            .rows_for_words(&["轉去".to_owned()])
            .unwrap_or_default();
        if rows.iter().any(|r| r.word == "轉去" && r.count == 1) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "frequency write timed out: {rows:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }

    let tng_after_freq: Vec<String> = fetch_candidates(config(true), "tngkhi")
        .into_iter()
        .filter_map(|c| c.hanji)
        .collect();
    let pos_tng = tng_after_freq.iter().position(|h| h == "轉--去").unwrap();
    let pos_tng_other = tng_after_freq.iter().position(|h| h == "當--機").unwrap();
    assert!(
        pos_tng < pos_tng_other,
        "plain `轉去` frequency count must boost custom `轉--去` ahead of `當--機`: {tng_after_freq:?}"
    );

    // 3. Baseline for `hoboo` (with no context): `好某` (900) beats `好--無` (500).
    let hoboo_before: Vec<String> = fetch_candidates(config(true), "hoboo")
        .into_iter()
        .filter_map(|c| c.hanji)
        .collect();
    assert_eq!(hoboo_before.first().map(String::as_str), Some("好某"));

    // Baseline for custom `siua` (with no context): both have 0 frequency and
    // no context; capture whichever leads so we can pick the other as target.
    let siua_before: Vec<String> = fetch_candidates(config(true), "siua")
        .into_iter()
        .filter_map(|c| c.hanji)
        .collect();
    assert_eq!(
        siua_before.len(),
        2,
        "expected both custom rows: {siua_before:?}"
    );
    let second_custom = siua_before[1].clone();
    let (second_tl, second_plain) = if second_custom == "是--話" {
        ("sì--uā", "是話")
    } else {
        ("sì-uā", "四話")
    };

    // Record bigrams via WordSelected:
    //   `會記--得` -> `好--無` (builtin with khinsiann_in_hanji display)
    //   `會記--得` -> `second_custom` (custom `--` entry)
    // Then leave `會記--得` as the active last-selected word.
    for (word, tl, ts) in [
        ("會記--得", "ē-kì--tit", NOW_MS - 5_000),
        ("好--無", "hó--bôo", NOW_MS - 4_000),
        ("會記--得", "ē-kì--tit", NOW_MS - 3_000),
        (second_custom.as_str(), second_tl, NOW_MS - 2_000),
        ("會記--得", "ē-kì--tit", NOW_MS - 1_000),
    ] {
        roundtrip(
            config(true),
            0,
            request::Payload::Nextword(NextWordRequest {
                method: Some(next_word_request::Method::WordSelected(WordSelected {
                    text: word.to_owned(),
                    roman: tl.to_owned(),
                    require_roman_mode: false,
                    trigger_prediction: false,
                    input: Some(DecisionInput { now_ms: ts }),
                    preceding: Vec::new(),
                })),
            }),
        );
    }

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let assoc = reader
            .association
            .rows_following("會記得", "ē-kì--tit", 10)
            .unwrap_or_default();
        if assoc.iter().any(|r| r.next == "好無") && assoc.iter().any(|r| r.next == second_plain)
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "association write timed out: {assoc:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }

    // With `會記--得` as active context:
    // - Builtin `好--無` (freq 500) shares normalized `會記得 -> 好無` bigram
    //   and now beats higher-frequency `好某` (900).
    let hoboo_after: Vec<String> = fetch_candidates(config(true), "hoboo")
        .into_iter()
        .filter_map(|c| c.hanji)
        .collect();
    assert_eq!(
        hoboo_after.first().map(String::as_str),
        Some("好--無"),
        "normalized bigram `會記得 -> 好無` must boost `好--無` ahead of `好某`: {hoboo_after:?}"
    );

    // - Custom trailing candidate `second_custom` shares normalized
    //   `會記得 -> {second_plain}` bigram and now moves to position 0!
    let siua_after: Vec<String> = fetch_candidates(config(true), "siua")
        .into_iter()
        .filter_map(|c| c.hanji)
        .collect();
    assert_eq!(
        siua_after.first(),
        Some(&second_custom),
        "normalized bigram must promote {second_custom} to slot 0: {siua_after:?}"
    );
}
