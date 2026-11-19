//! `DictionarySearch` through `process_request` over a synthetic lexicon:
//! the Hanji / romanization branch, the rendered romanization, the badges
//! the toggles leave and the custom entries joined in front.
//! Its own process: the user-data handle and lexicon handle are process-wide.
#![cfg(feature = "user-data")]

mod common;

use common::{open_user_data, roundtrip, tl_config, user_data};
use lexicon::LexiconPaths;
use protos::engine::{
    dictionary_search_row::Id, lexicon_request, lexicon_response, request, response,
    user_data_request, user_data_response, DictionarySearchRequest, DictionarySearchRow,
    DictionarySourceCode, DictionarySourceToggles, LexiconRequest, SaveCustomEntry,
    UserDataResponse,
};
use test_support::{
    build_tkdb, build_tkwa, fst_entry, walker_cost_from_fixture_frequency, write_fst_set,
    write_temp, TkdbRow, TKDB_VERSION,
};

const KAUTIAN_BIT: u16 = 1 << 0;
const ITAIGI_BIT: u16 = 1 << 2;

const KAUTIAN: i32 = DictionarySourceCode::DictSourceKautian as i32;
const ITAIGI: i32 = DictionarySourceCode::DictSourceItaigi as i32;
const CUSTOM: i32 = DictionarySourceCode::DictSourceCustom as i32;

/// Row ids are 1-based in insertion order: 1 食, 2 食飽, 3 赤.
fn install_synthetic_lexicon() {
    let rows = [
        ("tsiah", "食", "tsia̍h", KAUTIAN_BIT | ITAIGI_BIT, 900u32),
        ("tsiahpa", "食飽", "tsia̍h-pá", ITAIGI_BIT, 500u32),
        ("tshiah", "赤", "tshiah", KAUTIAN_BIT, 800u32),
    ];
    let tkdb_rows: Vec<TkdbRow<'_>> = rows
        .iter()
        .map(|&(_, hanji, tl, bitmask, frequency)| TkdbRow {
            bitmask,
            frequency,
            syllable_count: Some(tl.split('-').count() as u8),
            kautian_subtag: Some(0),
            walker_cost: Some(walker_cost_from_fixture_frequency(frequency)),
            hanji,
            tl,
        })
        .collect();
    let dict_path = write_temp(
        "dictionary-search-dict.bin",
        &build_tkdb(b"TKDB", TKDB_VERSION, &tkdb_rows),
    );
    let mut fst_entries = Vec::new();
    for (index, &(toneless, hanji, tl, ..)) in rows.iter().enumerate() {
        let rowid = (index + 1) as u32;
        fst_entries.push(fst_entry(b"tl:", toneless, rowid));
        fst_entries.push(fst_entry(b"hanzi:", hanji, rowid));
        let poj = phonetics::api::tl_display_to_poj_display(tl);
        if let Some((canon, _)) = phonetics::canonicalize_poj_syllable(&poj) {
            fst_entries.push(fst_entry(b"poj:", &canon.replace('-', ""), rowid));
        }
    }
    let fst_path = write_fst_set("dictionary-search-dict.fst", fst_entries);
    let assoc_path = write_temp("dictionary-search-assoc.bin", &build_tkwa(2, &[]));
    let syll_path = write_fst_set(
        "dictionary-search-syll.fst",
        vec![b"tl:tsiah".to_vec(), b"tl:pa".to_vec()],
    );
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

fn search(
    query: &str,
    mode: &str,
    toggles: impl Into<Option<DictionarySourceToggles>>,
    custom_enabled: bool,
) -> Vec<DictionarySearchRow> {
    let answer = roundtrip(
        tl_config(false),
        0,
        request::Payload::Lexicon(LexiconRequest {
            method: Some(lexicon_request::Method::DictionarySearch(
                DictionarySearchRequest {
                    query: query.to_owned(),
                    input_mode: mode.to_owned(),
                    toggles: toggles.into(),
                    custom_dictionary_enabled: custom_enabled,
                    limit: 20,
                },
            )),
        }),
    );
    match answer.payload {
        Some(response::Payload::Lexicon(lexicon)) => match lexicon.result {
            Some(lexicon_response::Result::DictionarySearchResult(found)) => found.rows,
            other => panic!("DictionarySearch answered {other:?}"),
        },
        _ => panic!("DictionarySearch answered {answer:?}"),
    }
}

fn kautian_and_itaigi() -> DictionarySourceToggles {
    DictionarySourceToggles {
        kautian: true,
        itaigi: true,
        ..DictionarySourceToggles::default()
    }
}

fn system(
    id: i64,
    roman: &str,
    lookup_tl: &str,
    hanji: &str,
    sources: &[i32],
) -> DictionarySearchRow {
    DictionarySearchRow {
        id: Some(Id::SystemId(id)),
        roman: roman.to_owned(),
        lookup_tl: lookup_tl.to_owned(),
        hanji: Some(hanji.to_owned()),
        sources: sources.to_vec(),
    }
}

#[test]
fn dictionary_search_branches_renders_badges_and_joins_custom_entries() {
    install_synthetic_lexicon();

    // trace: TL key `tsiah` → exact {1}, prefix {1, 2}; MOE rows first
    // (`lexicon::search`): 食 (kautian) then 食飽. Badges = row bits ∩ toggles.
    let tl_rows = vec![
        system(1, "tsia̍h", "tsia̍h", "食", &[KAUTIAN, ITAIGI]),
        system(2, "tsia̍h-pá", "tsia̍h-pá", "食飽", &[ITAIGI]),
    ];
    assert_eq!(search("tsiah", "tl", kautian_and_itaigi(), false), tl_rows);
    // English and an unknown mode read the TL family and show TL. (TPS reads
    // the `tps:` family, absent here; the production oracle covers it.)
    for mode in ["english", "unknown"] {
        assert_eq!(
            search("tsiah", mode, kautian_and_itaigi(), false),
            tl_rows,
            "{mode}"
        );
    }

    // trace: the fixture's `poj:` keys come from `canonicalize_poj_syllable`,
    // which reads one syllable, so only 食 (`chia̍h`) has one; roman rendered
    // by `tl_display_to_poj_display`, lookup stays TL.
    assert_eq!(
        search("chiah", "poj", kautian_and_itaigi(), false),
        vec![system(1, "chia̍h", "tsia̍h", "食", &[KAUTIAN, ITAIGI])]
    );

    // Kautian off: 赤 (kautian only) is filtered out by the lookup, 食 keeps
    // only the ITAIGI badge.
    let only_itaigi = DictionarySourceToggles {
        itaigi: true,
        ..DictionarySourceToggles::default()
    };
    let rows = search("tsiah", "tl", only_itaigi, false);
    assert!(rows.iter().all(|row| row.sources == [ITAIGI]), "{rows:?}");
    assert_eq!(rows.len(), 2, "{rows:?}");

    // trace: Hanji query → `hanzi:食` exact {1}, prefix {1, 2}.
    assert_eq!(
        search("食", "tl", kautian_and_itaigi(), true),
        tl_rows,
        "a Hanji query never lists custom entries"
    );
    assert!(search("", "tl", kautian_and_itaigi(), true).is_empty());
    // Absent toggles = every toggle off: no source enabled, no rows.
    assert_eq!(search("tsiah", "tl", None, false), []);

    // Before the platform opens its user data: no custom entries.
    assert_eq!(search("tsiah", "tl", kautian_and_itaigi(), true), tl_rows);

    let directory = tempfile::tempdir().unwrap();
    open_user_data(directory.path());
    let saved = user_data(user_data_request::Method::SaveCustomEntry(
        SaveCustomEntry {
            id: None,
            roman: "tsiah-ko".to_owned(),
            hanji: String::new(),
        },
    ));
    let Some(response::Payload::UserData(UserDataResponse {
        result: Some(user_data_response::Result::CustomEntrySaved(saved)),
    })) = saved.payload
    else {
        panic!("save answered {saved:?}");
    };
    let custom_id = saved.entry.expect("saved entry").id;

    let custom = DictionarySearchRow {
        id: Some(Id::CustomId(custom_id)),
        roman: "tsiah-ko".to_owned(),
        lookup_tl: "tsiah-ko".to_owned(),
        hanji: None,
        sources: vec![CUSTOM],
    };
    assert_eq!(
        search("tsiah", "tl", kautian_and_itaigi(), true),
        [vec![custom], tl_rows.clone()].concat()
    );
    // Enable Custom Dictionary off leaves them out.
    assert_eq!(search("tsiah", "tl", kautian_and_itaigi(), false), tl_rows);
}
