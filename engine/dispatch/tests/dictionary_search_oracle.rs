//! `DictionarySearch` answers what the shells' own orchestration of the
//! older ops answers today (`IsHanji` → `DictionaryFilters` →
//! `SearchByHanji` / `SearchWithSources` → `TlToPoj` → badge retag, custom
//! entries first via `SearchCustomEntries` — desktop-core
//! `engine/dictionary_search.rs`, iOS / Android `DictionarySearchService`),
//! over the production lexicon. Temporary: deleted with those ops once the
//! last shell moves onto `DictionarySearch` (roadmap C3).
//! Its own process: the user-data handle and lexicon handle are process-wide.
#![cfg(feature = "user-data")]

mod common;

use common::{open_user_data, production_lexicon_ready, roundtrip, tl_config, user_data};
use protos::engine::{
    dictionary_search_row::Id, lexicon_request, lexicon_response, phonetics_request,
    phonetics_response, request, response, user_data_request, user_data_response,
    DictionaryFiltersRequest, DictionarySearchRequest, DictionarySearchRow,
    DictionarySourceToggles, InputMode, IsHanjiRequest, KautianSubcollectionToggles,
    LexiconRequest, PhoneticsRequest, SaveCustomEntry, SearchByHanjiRequest, SearchCustomEntries,
    SearchWithSourcesRequest, TaigiWord, TlToPoj,
};

fn lexicon(method: lexicon_request::Method) -> Option<lexicon_response::Result> {
    let answer = roundtrip(
        tl_config(false),
        0,
        request::Payload::Lexicon(LexiconRequest {
            method: Some(method),
        }),
    );
    match answer.payload {
        Some(response::Payload::Lexicon(lexicon)) => lexicon.result,
        _ => None,
    }
}

fn tl_to_poj(tl: &str) -> String {
    let answer = roundtrip(
        tl_config(false),
        0,
        request::Payload::Phonetics(PhoneticsRequest {
            method: Some(phonetics_request::Method::TlToPoj(TlToPoj {
                input: tl.to_owned(),
            })),
        }),
    );
    match answer.payload {
        Some(response::Payload::Phonetics(phonetics)) => match phonetics.result {
            Some(phonetics_response::Result::StringResult(result)) => result.output,
            other => panic!("TlToPoj answered {other:?}"),
        },
        other => panic!("TlToPoj answered {other:?}"),
    }
}

/// iOS `DictionarySearchService.lexiconMode` (unknown modes read TL).
fn lexicon_mode(mode: &str) -> InputMode {
    match mode {
        "poj" => InputMode::Poj,
        "tps" => InputMode::Tps,
        _ => InputMode::Tl,
    }
}

/// The shells' orchestration, step for step (desktop-core
/// `dictionary_search::search`).
fn old_path(
    query: &str,
    mode: &str,
    toggles: &DictionarySourceToggles,
    custom_enabled: bool,
    limit: u32,
) -> Vec<DictionarySearchRow> {
    if query.is_empty() {
        return Vec::new();
    }
    let Some(lexicon_response::Result::IsHanjiResult(hanji)) =
        lexicon(lexicon_request::Method::IsHanji(IsHanjiRequest {
            text: query.to_owned(),
        }))
    else {
        panic!("IsHanji failed");
    };
    let Some(lexicon_response::Result::DictionaryFiltersResult(filters)) = lexicon(
        lexicon_request::Method::DictionaryFilters(DictionaryFiltersRequest {
            toggles: Some(*toggles),
        }),
    ) else {
        panic!("DictionaryFilters failed");
    };
    let bitmask = filters.dictionary_filter_bitmask;
    let input_mode = lexicon_mode(mode) as i32;
    let words: Vec<TaigiWord> = if hanji.is_hanji {
        match lexicon(lexicon_request::Method::SearchByHanji(
            SearchByHanjiRequest {
                query: query.to_owned(),
                input_mode,
                limit,
                enabled_sources_bitmask: bitmask,
            },
        )) {
            Some(lexicon_response::Result::SearchByHanjiResult(found)) => found.rows,
            _ => Vec::new(),
        }
    } else {
        match lexicon(lexicon_request::Method::SearchWithSources(
            SearchWithSourcesRequest {
                input: query.to_owned(),
                input_mode,
                limit,
                enabled_sources_bitmask: bitmask,
            },
        )) {
            Some(lexicon_response::Result::SearchWithSourcesResult(found)) => found.rows,
            _ => Vec::new(),
        }
    };
    let system = words.into_iter().map(|word| DictionarySearchRow {
        id: Some(Id::SystemId(word.id)),
        roman: if mode == "poj" {
            tl_to_poj(&word.roman)
        } else {
            word.roman.clone()
        },
        lookup_tl: word.roman,
        hanji: word.hanji,
        sources: word
            .sources
            .into_iter()
            .filter(|source| filters.enabled_source_codes.contains(source))
            .collect(),
    });
    let mut rows = Vec::new();
    if !hanji.is_hanji && custom_enabled {
        let answer = user_data(user_data_request::Method::SearchCustomEntries(
            SearchCustomEntries {
                query: query.to_owned(),
                input_mode: mode.to_owned(),
                limit,
            },
        ));
        if let Some(response::Payload::UserData(found)) = answer.payload {
            let Some(user_data_response::Result::CustomEntryMatches(matches)) = found.result else {
                panic!("SearchCustomEntries answered {found:?}");
            };
            rows.extend(
                matches
                    .entries
                    .into_iter()
                    .map(|entry| DictionarySearchRow {
                        id: Some(Id::CustomId(entry.id)),
                        lookup_tl: entry.roman.clone(),
                        roman: entry.roman,
                        hanji: (!entry.hanji.is_empty()).then_some(entry.hanji),
                        sources: vec![
                            protos::engine::DictionarySourceCode::DictSourceCustom as i32,
                        ],
                    }),
            );
        }
    }
    rows.extend(system);
    rows
}

fn new_op(
    query: &str,
    mode: &str,
    toggles: &DictionarySourceToggles,
    custom_enabled: bool,
    limit: u32,
) -> Vec<DictionarySearchRow> {
    let Some(lexicon_response::Result::DictionarySearchResult(found)) = lexicon(
        lexicon_request::Method::DictionarySearch(DictionarySearchRequest {
            query: query.to_owned(),
            input_mode: mode.to_owned(),
            toggles: Some(*toggles),
            custom_dictionary_enabled: custom_enabled,
            limit,
        }),
    ) else {
        panic!("DictionarySearch failed for {query:?}");
    };
    found.rows
}

fn all_on() -> DictionarySourceToggles {
    DictionarySourceToggles {
        kautian: true,
        taigitv: true,
        itaigi: true,
        sitbut: true,
        taihoa: true,
        taijit: true,
        kungge: true,
        stti: true,
        khpoo: true,
        variant: true,
        khiin: true,
        lkk: true,
        dev: true,
        kautian_subcollections: None,
    }
}

fn toggle_sets() -> Vec<DictionarySourceToggles> {
    let mut kautian_off = all_on();
    kautian_off.kautian = false;
    let mut dev_and_khiin_off = all_on();
    dev_and_khiin_off.dev = false;
    dev_and_khiin_off.khiin = false;
    let mut subcollections = all_on();
    subcollections.kautian_subcollections = Some(KautianSubcollectionToggles {
        accent_tainan: true,
        ..KautianSubcollectionToggles::default()
    });
    let only_itaigi = DictionarySourceToggles {
        itaigi: true,
        ..DictionarySourceToggles::default()
    };
    vec![
        all_on(),
        kautian_off,
        dev_and_khiin_off,
        subcollections,
        only_itaigi,
        DictionarySourceToggles::default(),
    ]
}

const QUERIES: &[&str] = &[
    "",
    "食",
    "台語",
    "食飽",
    "tsiah",
    "tsia̍h",
    "tsiah8",
    "chia̍h",
    "chiah",
    "ㄐㄧㄚ",
    "tai-gi",
    "taigi",
    "gi",
    "lí-hó",
    "Tsiah",
];
/// Wide prefixes: each lookup scans most of the index (~0.1–0.6 s in a
/// debug build), so they are compared once each instead of in every
/// combination.
const WIDE_QUERIES: &[&str] = &["ts", "-"];
const MODES: &[&str] = &["tl", "poj", "tps", "english", "unknown"];

fn compare_everything(custom_enabled: bool) {
    for toggles in toggle_sets() {
        for mode in MODES {
            for query in QUERIES {
                for limit in [20, 3] {
                    assert_eq!(
                        new_op(query, mode, &toggles, custom_enabled, limit),
                        old_path(query, mode, &toggles, custom_enabled, limit),
                        "query={query:?} mode={mode} custom={custom_enabled} limit={limit} \
                         toggles={toggles:?}"
                    );
                }
            }
        }
    }
    for query in WIDE_QUERIES {
        assert_eq!(
            new_op(query, "tl", &all_on(), custom_enabled, 20),
            old_path(query, "tl", &all_on(), custom_enabled, 20),
            "query={query:?} custom={custom_enabled}"
        );
    }
}

#[test]
fn dictionary_search_answers_what_the_shells_orchestrate() {
    if !production_lexicon_ready() {
        eprintln!("skip: production artifacts absent — run `make dict`");
        return;
    }

    // Before the platform opens its user data: no custom entries even when
    // Enable Custom Dictionary is on.
    compare_everything(true);

    let directory = tempfile::tempdir().unwrap();
    open_user_data(directory.path());
    for (roman, hanji) in [("tsia̍h-pá", "食飽"), ("tsiah-ko", ""), ("tâi-gí", "台語")] {
        let saved = user_data(user_data_request::Method::SaveCustomEntry(
            SaveCustomEntry {
                id: None,
                roman: roman.to_owned(),
                hanji: hanji.to_owned(),
            },
        ));
        assert!(
            matches!(saved.payload, Some(response::Payload::UserData(_))),
            "save {roman} answered {saved:?}"
        );
    }

    // Enable Custom Dictionary off with entries stored: the golden test
    // (`dictionary_search.rs`) covers it.
    compare_everything(true);
    // The custom entries really took part.
    assert!(
        new_op("tsiah", "tl", &all_on(), true, 20)
            .iter()
            .any(|row| matches!(row.id, Some(Id::CustomId(_))) && row.hanji.is_none()),
        "the romanization-only custom entry leads `tsiah`"
    );
}
