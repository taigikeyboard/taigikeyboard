//! `fetch_abbrev_candidates` — the whole-buffer abbreviation lookup
//! (`behavioral-invariants.md` §46; USER 2026-09-18 `ss` → 鎖匙 `só-sî`).
//!
//! The one continuous fetch that reads the `*-abbrev:` FST family — the
//! family tag is the authority, so every rowid under the key is a hit that
//! passes the source filter. Hermetic wire-format fixtures; the
//! production-artifact run is `composing/tests/candidate_dump.rs`.

use lexicon::dictionary_reader::{DictionaryReader, KAUTIAN_BIT};
use lexicon::search::{self, SearchParams};
use lexicon::{fetch_abbrev_candidates, COVERAGE_KIND_ABBREV};
use phonetics::{InputMode, KeyFamily};
use ranking::{FrequencyData, FrequencyMap};

use crate::common::{build_tkdb_v4, build_wire_index, hanji_of, neutral_ctx};
use test_support::write_temp;

const DEFAULT_SOURCE: u16 = 1 << 11;

type DictRow = (u16, u32, u8, &'static str, &'static str);

/// Rows shared by every test; rowid = 1-based position.
///
/// trace: 鎖匙 só-sî → tl_abbrev "ss", poj_abbrev "ss", tps_abbrev ㄙㄒ;
/// 先生 sian-senn → "ss" (higher freq); 總鎖匙 tsóng-só-sî → "tss";
/// 食飯 tsia̍h-pn̄g → tl "tp", poj (chia̍h-pn̄g) "cp", tps ㄗㄅ;
/// 毋是 m̄-sī → tps_abbrev ㆬㄒ (syllabic nasal ㆬ, not the initial ㄇ);
/// 燒 sio (single syllable, no abbreviation).
fn rows() -> Vec<DictRow> {
    vec![
        (DEFAULT_SOURCE, 100, 2, "鎖匙", "só-sî"),
        (DEFAULT_SOURCE, 400, 2, "先生", "sian-senn"),
        (DEFAULT_SOURCE, 16, 3, "總鎖匙", "tsóng-só-sî"),
        (KAUTIAN_BIT, 144, 2, "食飯", "tsia̍h-pn̄g"),
        (DEFAULT_SOURCE, 27, 2, "毋是", "m̄-sī"),
        (DEFAULT_SOURCE, 9000, 1, "燒", "sio"),
    ]
}

fn open_dict(name: &str, rows: &[DictRow]) -> DictionaryReader {
    let path = write_temp(name, &build_tkdb_v4(b"TKDB", rows));
    DictionaryReader::open(&path).expect("dict.bin opens")
}

#[test]
fn tl_abbreviation_family_hits_ranked_by_frequency_with_record_metadata() {
    let idx = build_wire_index(
        "abbrev-tl.fst",
        &[
            ("tl-abbrev:ss", 1),
            ("tl-abbrev:ss", 2),
            ("tl-abbrev:tss", 3),
            ("tl:sosi", 1),
            ("tl:ss", 6), // a phonetic-family key never reaches the abbrev fetch
        ],
    );
    let dict = open_dict("abbrev-tl.dict.bin", &rows());
    let freq = FrequencyMap::new();
    let ctx = neutral_ctx(&idx, &dict, &freq, InputMode::Tl);
    let out = fetch_abbrev_candidates("tl-abbrev:ss", 2, &ctx);
    assert_eq!(hanji_of(&out), vec!["先生", "鎖匙"], "{out:?}");
    let sosi = &out[1];
    assert_eq!(sosi.roman, "só-sî");
    assert_eq!(sosi.canonical_tl, "só-sî");
    assert_eq!(sosi.consumed_span, (0, 2));
    assert_eq!(sosi.syllable_count, 2, "record's own syllable count");
    assert_eq!(sosi.coverage_kind, COVERAGE_KIND_ABBREV);
    assert!(!sosi.is_custom);
}

#[test]
fn source_filter_applies() {
    let idx = build_wire_index("abbrev-filter.fst", &[("tl-abbrev:tp", 4)]);
    let dict = open_dict("abbrev-filter.dict.bin", &rows());
    let freq = FrequencyMap::new();
    // Kautian on → 食飯 surfaces; only the default source on → dropped.
    let all = neutral_ctx(&idx, &dict, &freq, InputMode::Tl);
    assert_eq!(
        hanji_of(&fetch_abbrev_candidates("tl-abbrev:tp", 2, &all)),
        vec!["食飯"]
    );
    let mut default_only = neutral_ctx(&idx, &dict, &freq, InputMode::Tl);
    default_only.enabled_sources_bitmask = u32::from(DEFAULT_SOURCE);
    assert!(fetch_abbrev_candidates("tl-abbrev:tp", 2, &default_only).is_empty());
}

/// 鎖匙 picked through `ss` before: its abbreviation picks lift it over
/// the dictionary-frequent 先生.
#[test]
fn abbreviation_selection_outranks_dictionary_frequency() {
    let idx = build_wire_index(
        "abbrev-freq.fst",
        &[("tl-abbrev:ss", 1), ("tl-abbrev:ss", 2)],
    );
    let dict = open_dict("abbrev-freq.dict.bin", &rows());
    let mut freq = FrequencyMap::new();
    freq.insert(
        "鎖匙".to_string(),
        "só-sî".to_string(),
        FrequencyData {
            abbrev_count: 3,
            abbrev_last_used_ms: 1_000,
            ..FrequencyData::default()
        },
    );
    let mut ctx = neutral_ctx(&idx, &dict, &freq, InputMode::Tl);
    ctx.now_ms = 2_000;
    let out = fetch_abbrev_candidates("tl-abbrev:ss", 2, &ctx);
    assert_eq!(hanji_of(&out), vec!["鎖匙", "先生"], "{out:?}");
}

/// 鎖匙 typed in full (`soo-si`) many times: those reading picks do not
/// lift it under the abbreviation `ss`, so 先生 keeps its dictionary lead
/// and 鎖匙 carries no user weight there.
#[test]
fn reading_selection_does_not_lift_an_abbreviation_hit() {
    let idx = build_wire_index(
        "abbrev-read.fst",
        &[("tl-abbrev:ss", 1), ("tl-abbrev:ss", 2)],
    );
    let dict = open_dict("abbrev-read.dict.bin", &rows());
    let mut freq = FrequencyMap::new();
    freq.insert(
        "鎖匙".to_string(),
        "só-sî".to_string(),
        FrequencyData {
            count: 200,
            last_used_ms: 1_000,
            ..FrequencyData::default()
        },
    );
    let mut ctx = neutral_ctx(&idx, &dict, &freq, InputMode::Tl);
    ctx.now_ms = 2_000;
    let out = fetch_abbrev_candidates("tl-abbrev:ss", 2, &ctx);
    assert_eq!(hanji_of(&out), vec!["先生", "鎖匙"], "{out:?}");
    assert!(out.iter().all(|c| c.user_weight == 0.0), "{out:?}");
}

#[test]
fn poj_family_is_read_independently_of_tl() {
    // 食飯: POJ display `chia̍h-pn̄g` abbreviates to `cp`; the TL acronym
    // `tp` lives under `tl-abbrev:` and is not a POJ key.
    let idx = build_wire_index(
        "abbrev-poj.fst",
        &[
            ("poj-abbrev:cp", 4),
            ("tl-abbrev:tp", 4),
            ("poj-abbrev:ss", 1),
        ],
    );
    let dict = open_dict("abbrev-poj.dict.bin", &rows());
    let freq = FrequencyMap::new();
    let ctx = neutral_ctx(&idx, &dict, &freq, InputMode::Poj);
    assert_eq!(
        hanji_of(&fetch_abbrev_candidates("poj-abbrev:cp", 2, &ctx)),
        vec!["食飯"]
    );
    assert!(fetch_abbrev_candidates("poj-abbrev:tp", 2, &ctx).is_empty());
    assert_eq!(
        hanji_of(&fetch_abbrev_candidates("poj-abbrev:ss", 2, &ctx)),
        vec!["鎖匙"]
    );
}

#[test]
fn tps_family_keys_are_bopomofo_glyph_exact() {
    // 毋是's abbreviation face is ㆬㄒ (syllabic nasal); the literal ㄇㄒ is a
    // different key — the §35 ambiguity path, not this fetch, is how a
    // typed `ㄇㄒ` reaches 毋是 as a reading.
    let idx = build_wire_index(
        "abbrev-tps.fst",
        &[("tps-abbrev:ㄙㄒ", 1), ("tps-abbrev:ㆬㄒ", 5)],
    );
    let dict = open_dict("abbrev-tps.dict.bin", &rows());
    let freq = FrequencyMap::new();
    let ctx = neutral_ctx(&idx, &dict, &freq, InputMode::Tps);
    let sosi = fetch_abbrev_candidates("tps-abbrev:ㄙㄒ", 6, &ctx);
    assert_eq!(hanji_of(&sosi), vec!["鎖匙"]);
    assert_eq!(sosi[0].consumed_span, (0, 6));
    assert_eq!(
        hanji_of(&fetch_abbrev_candidates("tps-abbrev:ㆬㄒ", 6, &ctx)),
        vec!["毋是"]
    );
    assert!(fetch_abbrev_candidates("tps-abbrev:ㄇㄒ", 6, &ctx).is_empty());
}

#[test]
fn pool_is_sorted_and_not_truncated() {
    // 40 rows all abbreviating to `kk`, ascending frequency → the whole pool
    // comes back most-frequent first; the caller truncates AFTER its
    // cross-batch exclude (Step 4c), so nothing may be cut here.
    let rows: Vec<DictRow> = (0..40u32)
        .map(|i| {
            let hanji: &'static str = Box::leak(format!("詞{i}").into_boxed_str());
            (DEFAULT_SOURCE, 100 + i, 2, hanji, "ka-ki")
        })
        .collect();
    let dict = open_dict("abbrev-pool.dict.bin", &rows);
    let entries: Vec<(&str, u32)> = (1..=40).map(|rowid| ("tl-abbrev:kk", rowid)).collect();
    let idx = build_wire_index("abbrev-pool.fst", &entries);
    let freq = FrequencyMap::new();
    let ctx = neutral_ctx(&idx, &dict, &freq, InputMode::Tl);
    let out = fetch_abbrev_candidates("tl-abbrev:kk", 2, &ctx);
    assert_eq!(out.len(), 40, "un-truncated pool");
    assert_eq!(out[0].hanji.as_deref(), Some("詞39"), "most frequent first");
    assert_eq!(out[39].hanji.as_deref(), Some("詞0"));
}

#[test]
fn tab3_search_unions_the_phonetic_and_abbreviation_families() {
    // The dictionary browser keeps acronym matching (typing `ss` lists 鎖匙
    // and 先生) although the acronym keys moved out of the `tl:` range:
    // `search` reads both families — main exact, abbrev exact, main
    // prefix, abbrev prefix.
    let idx = build_wire_index(
        "abbrev-tab3.fst",
        &[
            ("tl:sosi", 1),
            ("tl-abbrev:ss", 1),
            ("tl-abbrev:ss", 2),
            ("tl:siansenn", 2),
            ("tl-abbrev:sst", 3), // prefix hit through the abbrev family (synthetic)
            ("tl:sio", 6),
        ],
    );
    let dict = open_dict("abbrev-tab3.dict.bin", &rows());
    let params = SearchParams {
        input: "ss".to_string(),
        family: KeyFamily::Tl,
        limit: 10,
        enabled_sources_bitmask: u32::MAX,
    };
    let rows = search::search(&params, &idx, &dict).expect("search succeeds");
    let hanji: Vec<&str> = rows.iter().filter_map(|r| r.hanji.as_deref()).collect();
    assert_eq!(hanji, vec!["先生", "鎖匙", "總鎖匙"], "{rows:?}");
}
