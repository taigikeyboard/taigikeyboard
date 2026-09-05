//! A continuous candidate never carries a syllable the user has not typed
//! into (product owner 2026-08-21, all three platforms).
//!
//! `lookup_prefix` hydrates every dictionary row whose FST key merely STARTS
//! with the typed body, so before the rule `tsuisi` (two syllables typed)
//! surfaced 水社寮 `tsuí-siā-liâu` (three) and `kesithau` surfaced 家私頭仔
//! (four) — words whose last syllable was never typed at all.
//! `typed_prefix_reaches_final_syllable` (`lexicon::continuous`) rejects those
//! at hydration, per key family and per tone surface.
//!
//! Hermetic: every fixture derives its FST key bodies from the row's own `tl`
//! through the same `phonetics` helpers `dictionary/build/create_fst.py` uses,
//! so a fixture key can never drift from what production would store.
//!
//! What this file does NOT cover: exact-key hits (阿姨仔 `a-î-á` typed as
//! `aia`) never reach this fetcher — they arrive through `lookup_exact` in
//! `fetch_candidates_for_keys` — and the end-to-end candidate strip, which is
//! pinned by `composing/tests/golden/fetch_at_pos.golden`.

use fst::SetBuilder;
use lexicon::dictionary_reader::DictionaryReader;
use lexicon::prefix_index::PrefixIndex;
use lexicon::{
    fetch_partial_prefix_candidates, ConsumedSpan, ContinuousFetchCtx, CustomEntry,
    PARTIAL_PREFIX_HYDRATE_CAP,
};
use phonetics::InputMode;
use ranking::FrequencyMap;

mod common;
use common::{build_tkdb_v3, write_temp};

/// One dictionary fixture row. `bitmask` is fixed to the `lkk` source bit so
/// every row passes the all-sources filter the tests use.
struct Row<'a> {
    hanzi: &'a str,
    tl: &'a str,
    syll: u8,
    freq: u32,
}

/// The FST key bodies `create_fst.py` would emit for `tl` in `family` — the
/// numeric-tone face, the toneless face, and (TPS only) the `er`↔`or` dialect
/// variants. Deduplicated: a toneless reading derives the same body twice.
///
/// Built with the same derivations production uses, which is sound precisely
/// because those are pinned row-for-row against the shipped
/// `dictionary/output/dictionary.csv` by `tests/roman_num_face_parity.rs` and
/// `tests/tps_notone_parity.rs`. A hand-rolled fixture chain would only add a
/// fifth copy to keep in sync.
fn key_bodies(family: &str, tl: &str) -> Vec<String> {
    let mut bodies = match family {
        "tps:" => {
            let notone = phonetics::tps_notone_from_tl(tl);
            let num = phonetics::tps_num_from_tl(tl);
            let mut out = vec![num.clone(), notone.clone()];
            for primary in [num, notone] {
                let variant = phonetics::tps_notone_or_variant(&primary);
                if !variant.is_empty() {
                    out.push(variant);
                }
            }
            out
        }
        "poj:" => num_and_notone(phonetics::poj_num_syllable_ends_from_tl(tl).0),
        _ => num_and_notone(phonetics::tl_num_syllable_ends_from_tl(tl).0),
    };
    bodies.sort();
    bodies.dedup();
    bodies
}

/// The numeric-tone body and its toneless sibling — `create_fst.py` emits both,
/// and `notone.py::remove_tone` builds the second from the first.
fn num_and_notone(num: String) -> Vec<String> {
    let notone: String = num.chars().filter(|c| !c.is_ascii_digit()).collect();
    vec![num, notone]
}

/// A `(PrefixIndex, DictionaryReader)` holding `rows` under every key face
/// `family` stores them under. Rowids are 1-based insertion order.
fn build_fixture(name: &str, family: &str, rows: &[Row<'_>]) -> (PrefixIndex, DictionaryReader) {
    let dict_rows: Vec<(u16, u32, u8, &str, &str)> = rows
        .iter()
        .map(|r| (1u16 << 11, r.freq, r.syll, r.hanzi, r.tl))
        .collect();
    let dict_path = write_temp(
        &format!("syllable-reach-{name}.dict.bin"),
        &build_tkdb_v3(b"TKDB", &dict_rows),
    );
    let dict = DictionaryReader::open(&dict_path).expect("dict.bin opens");

    let mut entries: Vec<Vec<u8>> = Vec::new();
    for (idx, row) in rows.iter().enumerate() {
        let rowid = (idx + 1) as u32;
        for body in key_bodies(family, row.tl) {
            let mut entry = Vec::new();
            entry.extend_from_slice(family.as_bytes());
            entry.extend_from_slice(body.as_bytes());
            entry.push(0xFF);
            entry.extend_from_slice(&rowid.to_le_bytes());
            entries.push(entry);
        }
    }
    entries.sort();
    entries.dedup();

    let mut builder = SetBuilder::memory();
    for entry in &entries {
        builder.insert(entry).expect("fst insert");
    }
    let fst_path = write_temp(
        &format!("syllable-reach-{name}.dictionary.fst"),
        &builder.into_inner().expect("fst finish"),
    );
    (
        PrefixIndex::open(&fst_path).expect("dictionary.fst opens"),
        dict,
    )
}

fn ctx<'a>(
    prefix_index: &'a PrefixIndex,
    dict: &'a DictionaryReader,
    freq_map: &'a FrequencyMap,
    custom: &'a [CustomEntry],
    mode: InputMode,
) -> ContinuousFetchCtx<'a> {
    ContinuousFetchCtx {
        enabled_sources_bitmask: u32::MAX,
        freq_map,
        now_ms: 0,
        custom,
        prefix_index,
        dict,
        mode,
        tps_space_pinned_body: None,
    }
}

fn typed_key(family: &str, typed: &str) -> (ConsumedSpan, String) {
    ((0, typed.len() as u32), format!("{family}{typed}"))
}

/// The `display_text` of every candidate a typed prefix surfaces, in fetch
/// order — the same `hanji ?? roman` key the platform commits and learns under,
/// read off the candidate rather than re-derived here.
fn hanji_for(
    family: &str,
    typed: &str,
    mode: InputMode,
    rows: &[Row<'_>],
    name: &str,
) -> Vec<String> {
    let (prefix_index, dict) = build_fixture(name, family, rows);
    let freq_map = FrequencyMap::new();
    let fetch_ctx = ctx(&prefix_index, &dict, &freq_map, &[], mode);
    let key = typed_key(family, typed);
    fetch_partial_prefix_candidates(&key, key.0 .1, &fetch_ctx)
        .iter()
        .map(|candidate| candidate.display_text.clone())
        .collect()
}

fn tl_rows() -> Vec<Row<'static>> {
    vec![
        Row {
            hanzi: "水",
            tl: "tsuí",
            syll: 1,
            freq: 900,
        },
        Row {
            hanzi: "水手",
            tl: "tsuí-siú",
            syll: 2,
            freq: 500,
        },
        Row {
            hanzi: "水社寮",
            tl: "tsuí-siā-liâu",
            syll: 3,
            freq: 100,
        },
    ]
}

#[test]
fn tl_two_syllable_input_keeps_two_syllable_extension_and_drops_three() {
    // trace: typed body `tsuisi` (6 bytes). 水手 toneless faces
    // ["tsui", "siu"] → head "tsui" (4) < 6 → kept (its last syllable IS
    // being typed). 水社寮 faces ["tsui", "sia", "liau"] → head "tsuisia"
    // (7) ≥ 6 → dropped (`liau` never typed).
    let out = hanji_for("tl:", "tsuisi", InputMode::Tl, &tl_rows(), "tl-tsuisi");
    assert!(out.contains(&"水手".to_string()), "got {out:?}");
    assert!(!out.contains(&"水社寮".to_string()), "got {out:?}");
}

#[test]
fn tl_single_syllable_input_keeps_only_single_syllable_readings() {
    // trace: typed body `tsui` (4). 水 is single-syllable → head is empty
    // (0) < 4 → kept. 水手 head "tsui" (4) ≥ 4 → dropped: the input stops
    // exactly ON the boundary, so `siú` was never typed into.
    let out = hanji_for("tl:", "tsui", InputMode::Tl, &tl_rows(), "tl-tsui");
    assert_eq!(out, vec!["水".to_string()], "got {out:?}");
}

#[test]
fn tl_one_letter_into_the_second_syllable_admits_it() {
    // trace: typed body `tsuis` (5) > head "tsui" (4) → 水手 returns the
    // moment the user types the first letter of `siú`.
    let out = hanji_for("tl:", "tsuis", InputMode::Tl, &tl_rows(), "tl-tsuis");
    assert!(out.contains(&"水手".to_string()), "got {out:?}");
    assert!(!out.contains(&"水社寮".to_string()), "got {out:?}");
}

#[test]
fn tl_leading_initial_surfaces_only_single_syllable_readings() {
    // #441/#443 partial-prefix cluster, under the reach rule: a lone
    // initial still reaches single-character readings, and now nothing
    // longer. trace: typed `ts` (2); 水手 head "tsui" (4) ≥ 2 → dropped.
    let out = hanji_for("tl:", "ts", InputMode::Tl, &tl_rows(), "tl-ts");
    assert_eq!(out, vec!["水".to_string()], "got {out:?}");
}

#[test]
fn tl_multi_syllable_extension_drops_at_every_earlier_boundary() {
    // trace: 家私頭 toneless "kesithau", 家私頭仔 "kesithaua". Typed
    // `kesithau` (8) → 家私頭 head "kesi" (4) < 8 kept; 家私頭仔 head
    // "kesithau" (8) ≥ 8 dropped.
    let rows = vec![
        Row {
            hanzi: "家私頭",
            tl: "ke-si-thâu",
            syll: 3,
            freq: 300,
        },
        Row {
            hanzi: "家私頭仔",
            tl: "ke-si-thâu-á",
            syll: 4,
            freq: 300,
        },
    ];
    let out = hanji_for("tl:", "kesithau", InputMode::Tl, &rows, "tl-kesithau");
    assert!(out.contains(&"家私頭".to_string()), "got {out:?}");
    assert!(!out.contains(&"家私頭仔".to_string()), "got {out:?}");
}

#[test]
fn tl_numeric_tone_key_measures_reach_on_the_toned_face() {
    // The `tl:<tl_num>` family is a different surface: digits separate
    // syllables, so a toneless head length would be off by one digit per
    // syllable and could admit a word the user stopped short of.
    let rows = vec![
        Row {
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 900,
        },
        Row {
            hanzi: "台灣",
            tl: "tâi-uân",
            syll: 2,
            freq: 800,
        },
    ];
    let out = hanji_for("tl:", "tai5", InputMode::Tl, &rows, "tl-tai5");
    assert_eq!(out, vec!["台".to_string()], "got {out:?}");
}

#[test]
fn tl_numeric_tone_key_admits_a_word_typed_into_its_last_syllable() {
    // trace: typed `tai5u` (5) > head "tai5" (4) → 台灣 returns.
    let rows = vec![Row {
        hanzi: "台灣",
        tl: "tâi-uân",
        syll: 2,
        freq: 800,
    }];
    let out = hanji_for("tl:", "tai5u", InputMode::Tl, &rows, "tl-tai5u");
    assert_eq!(out, vec!["台灣".to_string()], "got {out:?}");
}

#[test]
fn an_unmarked_reading_is_measured_on_its_numeric_face_too() {
    // The regression this exists for: a reading whose syllables are ALL tone 1
    // or 4 carries no tone mark, and the build pipeline still writes the
    // default digits into `tl_num` (`kau-kuan` → `kau1kuan1`). A runtime mirror
    // that only supplies default tones when the reading has a mark somewhere
    // derives `kaukuan`, fails its own `starts_with`, and the guard fails open
    // — silently, on 17,976 of the shipped dictionary's rows. Every other toned
    // case in this file uses a marked reading (`tâi-uân`) and cannot see it.
    let rows = vec![
        Row {
            hanzi: "交",
            tl: "kau",
            syll: 1,
            freq: 900,
        },
        Row {
            hanzi: "交關",
            tl: "kau-kuan",
            syll: 2,
            freq: 400,
        },
    ];
    let out = hanji_for("tl:", "kau1", InputMode::Tl, &rows, "tl-kau1");
    assert_eq!(out, vec!["交".to_string()], "got {out:?}");

    // …and it comes back the moment the second syllable is started.
    let out = hanji_for("tl:", "kau1k", InputMode::Tl, &rows, "tl-kau1k");
    assert!(out.contains(&"交關".to_string()), "got {out:?}");

    // Same reading, stop-coda default tone 4 rather than 1.
    let rows = vec![
        Row {
            hanzi: "甲",
            tl: "kah",
            syll: 1,
            freq: 900,
        },
        Row {
            hanzi: "甲意",
            tl: "kah-ì",
            syll: 2,
            freq: 400,
        },
    ];
    let out = hanji_for("tl:", "kah4", InputMode::Tl, &rows, "tl-kah4");
    assert_eq!(out, vec!["甲".to_string()], "got {out:?}");
}

#[test]
fn poj_matches_tl_on_the_same_readings() {
    // Same three readings as `tl_two_syllable_input_*`, reached through the
    // `poj:` family: typed `chuisi` (6), 水手 head "chui" (4) < 6 kept,
    // 水社寮 head "chuisia" (7) ≥ 6 dropped.
    let out = hanji_for("poj:", "chuisi", InputMode::Poj, &tl_rows(), "poj-chuisi");
    assert!(out.contains(&"水手".to_string()), "got {out:?}");
    assert!(!out.contains(&"水社寮".to_string()), "got {out:?}");
}

#[test]
fn poj_numeric_tone_key_measures_reach_on_the_toned_face() {
    // trace: typed `goa2` (4). 我 face ["goa2"] kept; 我是 faces
    // ["goa2", "si7"] → head "goa2" (4) ≥ 4 dropped.
    let rows = vec![
        Row {
            hanzi: "我",
            tl: "guá",
            syll: 1,
            freq: 900,
        },
        Row {
            hanzi: "我是",
            tl: "guá-sī",
            syll: 2,
            freq: 400,
        },
    ];
    let out = hanji_for("poj:", "goa2", InputMode::Poj, &rows, "poj-goa2");
    assert_eq!(out, vec!["我".to_string()], "got {out:?}");
}

fn tps_rows() -> Vec<Row<'static>> {
    vec![
        Row {
            hanzi: "交",
            tl: "kau",
            syll: 1,
            freq: 900,
        },
        Row {
            hanzi: "交代",
            tl: "kau-tài",
            syll: 2,
            freq: 500,
        },
    ]
}

#[test]
fn tps_first_syllable_only_keeps_single_syllable_readings() {
    // trace: typed `ㄍㄠ` (6 bytes). 交 single-syllable kept; 交代 faces
    // ["ㄍㄠ", "ㄉㄞ"] → head 6 ≥ 6 dropped.
    let out = hanji_for("tps:", "ㄍㄠ", InputMode::Tps, &tps_rows(), "tps-kau");
    assert_eq!(out, vec!["交".to_string()], "got {out:?}");
}

#[test]
fn tps_one_glyph_into_the_second_syllable_admits_it() {
    // trace: typed `ㄍㄠㄉ` (9) > head 6 → 交代 returns (§31 / S18 keeps
    // working the moment the second syllable is started).
    let out = hanji_for("tps:", "ㄍㄠㄉ", InputMode::Tps, &tps_rows(), "tps-kaut");
    assert!(out.contains(&"交代".to_string()), "got {out:?}");
}

#[test]
fn tps_or_dialect_variant_face_is_measured_on_its_own_glyphs() {
    // C-3a emits a second `tps_*_var` key per `ㄜ`-carrying reading
    // (`ㄜ` → `ㄛ`), and a hit through it must be measured against THAT
    // face, not the primary. trace: 火爐 `hér-lôo` primary "ㄏㄜㄌㆦ",
    // variant "ㄏㄛㄌㆦ"; typed `ㄏㄛ` (6) → head "ㄏㄛ" (6) ≥ 6 → dropped,
    // while single-syllable 火 stays.
    let rows = vec![
        Row {
            hanzi: "火",
            tl: "hér",
            syll: 1,
            freq: 900,
        },
        Row {
            hanzi: "火爐",
            tl: "hér-lôo",
            syll: 2,
            freq: 400,
        },
    ];
    let out = hanji_for("tps:", "ㄏㄛ", InputMode::Tps, &rows, "tps-var-short");
    assert_eq!(out, vec!["火".to_string()], "got {out:?}");

    let out = hanji_for("tps:", "ㄏㄛㄌ", InputMode::Tps, &rows, "tps-var-long");
    assert!(out.contains(&"火爐".to_string()), "got {out:?}");
}

#[test]
fn tps_tone_marked_key_measures_reach_on_the_toned_face() {
    // 交 is tone 1, so its toned face is the bare `ㄍㄠ`; 交代's is
    // "ㄍㄠ" + "ㄉㄞ˪". trace: typed `ㄍㄠㄉㄞ˪` (15) > head 6 → 交代 kept,
    // proving the tone-marked surface resolves on its own face rather than
    // falling through to the toneless one.
    let out = hanji_for(
        "tps:",
        "ㄍㄠㄉㄞ˪",
        InputMode::Tps,
        &tps_rows(),
        "tps-toned",
    );
    assert!(out.contains(&"交代".to_string()), "got {out:?}");
}

#[test]
fn custom_dictionary_entries_are_exempt_from_the_reach_rule() {
    // Product owner 2026-08-21: a word the user added themselves stays
    // prefix-visible. The rule gates `dict.bin` rows only, so a
    // three-syllable custom entry surfaces from a one-glyph prefix that
    // would drop any dictionary row of the same length.
    let rows = tl_rows();
    let (prefix_index, dict) = build_fixture("custom-exempt", "tl:", &rows);
    let freq_map = FrequencyMap::new();
    let custom = vec![CustomEntry {
        roman: "tsuí-siā-liâu".to_string(),
        hanji: Some("水社寮".to_string()),
    }];
    let fetch_ctx = ctx(&prefix_index, &dict, &freq_map, &custom, InputMode::Tl);
    let key = typed_key("tl:", "tsui");
    let out: Vec<String> = fetch_partial_prefix_candidates(&key, key.0 .1, &fetch_ctx)
        .iter()
        .map(|candidate| candidate.display_text.clone())
        .collect();
    assert!(
        out.contains(&"水社寮".to_string()),
        "custom entry must stay visible, got {out:?}"
    );
    assert!(
        !out.contains(&"水手".to_string()),
        "the dictionary row of the same shape is still dropped, got {out:?}"
    );
}

#[test]
fn a_flood_of_rejected_rows_does_not_starve_the_readings_that_survive() {
    // The reach test needs `record.tl`, which is only readable after
    // `dict.record(rowid)` — so a rejected row still spends hydration
    // budget, and `PARTIAL_PREFIX_HYDRATE_CAP` keeps meaning "records
    // INSPECTED", not "eligible records collected". What stops that from
    // starving the strip is the shortest-matched-key-first policy: the
    // single-syllable readings carry the shortest keys and are hydrated
    // before the long multi-syllable extensions that the rule rejects.
    //
    // Fixture: more two-syllable `tsuí-…` rows than the whole hydrate
    // budget, plus one single-syllable 水. Typed `tsui` drops every
    // two-syllable row (head "tsui" ≥ 4) and must still return 水.
    let mut rows = vec![Row {
        hanzi: "水",
        tl: "tsuí",
        syll: 1,
        freq: 10,
    }];
    // Distinct readings so no two rows collapse in the dedupe: `tsuí-` +
    // one of 26×26 two-letter open syllables well past the budget.
    let tails: Vec<String> = (b'a'..=b'z')
        .flat_map(|v1| (b'a'..=b'z').map(move |v2| format!("{}{}", v1 as char, v2 as char)))
        .take(PARTIAL_PREFIX_HYDRATE_CAP + 50)
        .collect();
    let flood: Vec<String> = tails.iter().map(|tail| format!("tsuí-s{tail}")).collect();
    for (i, tl) in flood.iter().enumerate() {
        rows.push(Row {
            hanzi: FLOOD_HANZI,
            tl,
            syll: 2,
            freq: 1000 + i as u32,
        });
    }
    let out = hanji_for("tl:", "tsui", InputMode::Tl, &rows, "tl-flood");
    assert_eq!(
        out,
        vec!["水".to_string()],
        "the single-syllable reading must survive a hydrate-cap flood of \
         rejected two-syllable rows, got {out:?}"
    );
}

/// Filler 漢字 for the flood rows, deliberately DIFFERENT from the single
/// reading under test so a leaked flood row shows up in the assertion instead
/// of hiding behind an identical label.
const FLOOD_HANZI: &str = "汁";

#[test]
fn a_toned_tps_body_stopping_on_a_boundary_is_rejected() {
    // The toned TPS face is the one a toneless measurement would silently get
    // wrong, and the existing `ㄍㄠㄉㄞ˪` case cannot show it: that input runs
    // past every boundary, so it passes either way. This one stops EXACTLY on
    // the second-to-last boundary.
    // trace: 台語課 `tâi-gí-khò` → tps_num "ㄉㄞˊㆣㄧˋㄎㄛ˪" (`gí` is ㆣㄧ,
    // not ㄍㄧ), ends after ㄉㄞˊ (8 bytes) and ㆣㄧˋ (16). Typed `ㄉㄞˊㆣㄧˋ`
    // (16) → head 16 ≥ 16 → dropped; one glyph further (19) → kept.
    let rows = vec![
        Row {
            hanzi: "台語",
            tl: "tâi-gí",
            syll: 2,
            freq: 900,
        },
        Row {
            hanzi: "台語課",
            tl: "tâi-gí-khò",
            syll: 3,
            freq: 400,
        },
    ];
    let out = hanji_for(
        "tps:",
        "ㄉㄞˊㆣㄧˋ",
        InputMode::Tps,
        &rows,
        "tps-toned-boundary",
    );
    assert_eq!(out, vec!["台語".to_string()], "got {out:?}");

    let out = hanji_for(
        "tps:",
        "ㄉㄞˊㆣㄧˋㄎ",
        InputMode::Tps,
        &rows,
        "tps-toned-into-last",
    );
    assert!(out.contains(&"台語課".to_string()), "got {out:?}");
}

#[test]
fn a_toned_or_dialect_variant_body_is_measured_on_its_own_face() {
    // The `er`↔`or` variant of the TONED face — the toneless variant case
    // above cannot show that the boundaries carry over when tone marks are in
    // the string.
    // trace: 火爐 `hér-lôo` → tps_num "ㄏㄜˋㄌㆦˊ", variant "ㄏㄛˋㄌㆦˊ",
    // first boundary after ㄏㄛˋ (9 bytes). Typed `ㄏㄛˋ` (9) → dropped;
    // `ㄏㄛˋㄌ` (12) → kept. 火 stays either way.
    let rows = vec![
        Row {
            hanzi: "火",
            tl: "hér",
            syll: 1,
            freq: 900,
        },
        Row {
            hanzi: "火爐",
            tl: "hér-lôo",
            syll: 2,
            freq: 400,
        },
    ];
    let out = hanji_for(
        "tps:",
        "ㄏㄛˋ",
        InputMode::Tps,
        &rows,
        "tps-toned-var-short",
    );
    assert_eq!(out, vec!["火".to_string()], "got {out:?}");

    let out = hanji_for(
        "tps:",
        "ㄏㄛˋㄌ",
        InputMode::Tps,
        &rows,
        "tps-toned-var-long",
    );
    assert!(out.contains(&"火爐".to_string()), "got {out:?}");
}

#[test]
fn a_khinsiann_separator_does_not_add_a_syllable() {
    // 輕聲 `--` leaves an empty run between the hyphens. Counting it would make
    // 予我 look three syllables long and shift every boundary, so the guard
    // would drop it one keystroke late.
    // trace: 予我 `hōo--guá` → tl_num "hoo7gua2", ends at 4 and 8. Typed
    // `hoo7` (4) → head 4 ≥ 4 → dropped; `hoo7g` (5) → kept.
    let rows = vec![
        Row {
            hanzi: "予",
            tl: "hōo",
            syll: 1,
            freq: 900,
        },
        Row {
            hanzi: "予我",
            tl: "hōo--guá",
            syll: 2,
            freq: 400,
        },
    ];
    let out = hanji_for("tl:", "hoo7", InputMode::Tl, &rows, "tl-khinsiann-short");
    assert_eq!(out, vec!["予".to_string()], "got {out:?}");

    let out = hanji_for("tl:", "hoo7g", InputMode::Tl, &rows, "tl-khinsiann-long");
    assert!(out.contains(&"予我".to_string()), "got {out:?}");
}

#[test]
fn a_space_separated_reading_splits_at_the_space() {
    // A 詞組 reading separates its words with a space rather than a hyphen;
    // both are syllable boundaries to the build pipeline.
    // trace: 也是 `iā sī` → tl_num "ia7si7", ends at 4 and 8. Typed `ia7` (3)
    // → head 4 ≥ 3 → dropped; `ia7s` (4) → kept.
    let rows = vec![
        Row {
            hanzi: "也",
            tl: "iā",
            syll: 1,
            freq: 900,
        },
        Row {
            hanzi: "也是",
            tl: "iā sī",
            syll: 2,
            freq: 400,
        },
    ];
    let out = hanji_for("tl:", "ia7", InputMode::Tl, &rows, "tl-space-short");
    assert_eq!(out, vec!["也".to_string()], "got {out:?}");

    let out = hanji_for("tl:", "ia7s", InputMode::Tl, &rows, "tl-space-long");
    assert!(out.contains(&"也是".to_string()), "got {out:?}");
}
