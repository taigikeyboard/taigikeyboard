//! Whole-buffer abbreviation candidates end to end (`Start →
//! EnterContinuous → FetchAtPos`) — `behavioral-invariants.md` §46, USER
//! 2026-09-18 `ss` → 鎖匙 `só-sî`. Step 4c of
//! `composing::continuous::assemble_candidates` + `shadow::abbrev_query_key` +
//! `lexicon::fetch_abbrev_candidates`; the per-family face checks and the
//! sort live in `lexicon/tests/whole_buffer_abbrev.rs`, this file pins what
//! only the assembled vector shows: block order, cross-batch dedupe, the
//! span-local branch, recase, POJ render, the custom merge.
//!
//! `common::build_dictionary_fst` emits the `*_abbrev` keys like
//! `create_fst.py` does.

use protos::engine::CandidateMessage;

mod common;
use common::Fetch;
use common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v3, config, empty_association_bin,
    engine_install_lock, fetch_at_pos_response, fetch_hanji, fetch_hanji_with_custom,
    install_lexicon, write_temp, Row,
};
use lexicon::CustomEntry;

/// Abbreviation-only rows under `mk` beyond the ones that are also partial
/// extensions — enough that the block would overflow the output cap.
const MK_ABBREV_ONLY_ROWS: usize = 30;

fn fixture_rows() -> Vec<Row> {
    let mut rows = vec![
        // trace: abbrev "ss" (só + sî), toneless "sosi".
        Row {
            toneless_key: "sosi",
            hanzi: "鎖匙",
            tl: "só-sî",
            syll: 2,
            freq: 100,
        },
        // trace: abbrev "ss" too, higher freq → ranks ahead of 鎖匙.
        Row {
            toneless_key: "siansenn",
            hanzi: "先生",
            tl: "sian-senn",
            syll: 2,
            freq: 400,
        },
        // trace: single syllable, toneless "tse" — a partial-prefix hit for
        // `ts` that must stay ahead of every abbreviation hit.
        Row {
            toneless_key: "tse",
            hanzi: "這",
            tl: "tse",
            syll: 1,
            freq: 27_958,
        },
        // trace: abbrev "tss" (leading units ts + s — `ts` is one unit).
        Row {
            toneless_key: "tsasi",
            hanzi: "早時",
            tl: "tsá-sî",
            syll: 2,
            freq: 50,
        },
        // trace: `m` is a valid left-anchored syllable (毋), so buffer `mk`
        // takes the span-local branch; 物件 (abbrev "mk") must still surface.
        Row {
            toneless_key: "m",
            hanzi: "毋",
            tl: "m̄",
            syll: 1,
            freq: 39_475,
        },
        Row {
            toneless_key: "mihkiann",
            hanzi: "物件",
            tl: "mi̍h-kiānn",
            syll: 2,
            freq: 244,
        },
        // trace: TL abbrev "tsp" (leading units ts + p); POJ display
        // `chia̍h-pn̄g` → POJ abbrev "chp".
        Row {
            toneless_key: "tsiahpng",
            hanzi: "食飯",
            tl: "tsia̍h-pn̄g",
            syll: 2,
            freq: 144,
        },
    ];
    // trace: toneless `mk…` AND abbrev "mk" — under buffer `mk` each of
    // these is a Step 4b partial extension (`mk` reaches one glyph into
    // the second syllable, §43 admits it) AND an abbreviation hit. High
    // freq so they lead the abbreviation pool and would eat a pre-exclude
    // cap.
    for (hanzi, tl, toneless_key) in [
        ("毋肯", "m̄-khíng", "mkhing"),
        ("毋甘", "m̄-kam", "mkam"),
        ("毋管", "m̄-kuán", "mkuan"),
        ("毋驚", "m̄-kiann", "mkiann"),
        ("毋見", "m̄-kìnn", "mkinn"),
        ("毋過", "m̄-koh", "mkoh"),
        ("毋捌", "m̄-kok", "mkok"),
    ] {
        rows.push(Row {
            toneless_key,
            hanzi,
            tl,
            syll: 2,
            freq: 5_000,
        });
    }
    // trace: abbrev "mk" only (toneless `moka` does not start with `mk`).
    for i in 0..MK_ABBREV_ONLY_ROWS {
        rows.push(Row {
            toneless_key: "moka",
            hanzi: Box::leak(format!("詞{i}").into_boxed_str()),
            tl: "mó-ka",
            syll: 2,
            freq: 10,
        });
    }
    rows
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    // `m7` makes `m` a lattice syllable (the span-local branch for `mk`).
    let syllables_path = build_syllables_fst(&["m7", "tse1", "so2", "si5"]);
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);
}

fn fetch(raw: &str, input_mode: &str) -> Vec<CandidateMessage> {
    fetch_at_pos_response(&config(input_mode), raw, Fetch::default())
        .continuous
        .map(|c| c.candidates)
        .unwrap_or_default()
}

fn hanji(raw: &str, input_mode: &str) -> Vec<String> {
    fetch_hanji(raw, input_mode, Fetch::default())
}

// INVARIANT_CONTINUOUS_WHOLE_BUFFER_ABBREV (behavioral-invariants.md §46)
#[test]
fn ss_surfaces_abbreviated_words_after_the_literal() {
    let _lock = engine_install_lock();
    install_fixture();
    let cands = fetch("ss", "tl");
    // §34 literal keeps index 0; the abbreviation block follows, higher
    // dictionary frequency first, each committing the whole buffer.
    assert_eq!(
        (cands[0].roman.as_str(), cands[0].hanji.as_deref()),
        ("ss", None)
    );
    assert_eq!(hanji("ss", "tl"), vec!["先生", "鎖匙"], "{cands:?}");
    let sosi = &cands[2];
    assert_eq!(sosi.roman, "só-sî");
    assert_eq!((sosi.consumed_span_start, sosi.consumed_span_end), (0, 2));
}

#[test]
fn abbreviation_block_trails_partial_hits() {
    let _lock = engine_install_lock();
    install_fixture();
    // `ts` is a valid onset and one leading unit: the single-syllable
    // partial hit 這 keeps its place; 早時 abbreviates to `tss`, not `ts`,
    // and 食飯 to `tsp`, so neither joins the `ts` strip.
    assert_eq!(hanji("ts", "tl"), vec!["這"]);
    // `tss` has no syllabic reading: the abbreviation block is all there is.
    assert_eq!(hanji("tss", "tl"), vec!["早時"]);
    assert_eq!(hanji("tsp", "tl"), vec!["食飯"]);
}

#[test]
fn span_local_branch_dedupes_partial_overlap_before_capping() {
    let _lock = engine_install_lock();
    install_fixture();
    // `mk`: `m` is a lattice syllable so the fetch takes the span-local
    // branch (毋 first); the seven `m̄-k…` rows arrive as Step 4b partial
    // extensions AND as abbreviation hits and must show once; the
    // abbreviation block is then filled to the cap from the rows the
    // exclude did not touch (物件 + the 30 `mó-ka` rows → 30 survive).
    let cands = fetch("mk", "tl");
    let hanji: Vec<&str> = cands.iter().filter_map(|c| c.hanji.as_deref()).collect();
    assert_eq!(hanji[0], "毋", "{hanji:?}");
    let distinct: std::collections::HashSet<&str> = hanji.iter().copied().collect();
    assert_eq!(
        distinct.len(),
        hanji.len(),
        "no candidate shown twice: {hanji:?}"
    );
    let two_syllable = cands.iter().filter(|c| c.syllable_count == 2).count();
    assert_eq!(
        two_syllable,
        7 + lexicon::PARTIAL_PREFIX_OUTPUT_CAP,
        "7 partial overlaps + a full abbreviation block; got {hanji:?}"
    );
    let mihkiann = cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some("物件"))
        .expect("物件 via the span-local branch");
    assert_eq!(
        (mihkiann.consumed_span_start, mihkiann.consumed_span_end),
        (0, 2)
    );
    assert_eq!(mihkiann.syllable_count, 2, "record's own syllable count");
}

#[test]
fn uppercase_buffer_recases_the_abbreviated_roman() {
    let _lock = engine_install_lock();
    install_fixture();
    let cands = fetch("SS", "tl");
    let sosi = cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some("鎖匙"))
        .expect("鎖匙");
    assert_eq!(sosi.roman, "SÓ-SÎ");
}

#[test]
fn syllabic_and_separated_buffers_get_no_abbreviation_hits() {
    let _lock = engine_install_lock();
    install_fixture();
    // `s` alone: one glyph, no word abbreviates to it; `s-s` / `s s` / `ss2`
    // carry a non-consonant and stay on the syllable paths.
    for raw in ["s", "s-s", "s s", "ss2"] {
        let hanji = hanji(raw, "tl");
        assert!(
            !hanji.iter().any(|h| h == "鎖匙" || h == "先生"),
            "{raw:?} must not surface abbreviation hits; got {hanji:?}"
        );
    }
    // `sosi` (the syllables) reaches 鎖匙 through the toneless key as before.
    assert!(hanji("sosi", "tl").iter().any(|h| h == "鎖匙"));
}

#[test]
fn poj_mode_matches_the_poj_abbreviation_and_renders_poj() {
    let _lock = engine_install_lock();
    install_fixture();
    // POJ `chia̍h-pn̄g` abbreviates to `chp` (TL `tsp` is not a POJ key).
    let cands = fetch("chp", "poj");
    let tsiahpng = cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some("食飯"))
        .unwrap_or_else(|| panic!("食飯 via poj-abbrev:chp; got {cands:?}"));
    assert_eq!(tsiahpng.roman, "chia̍h-pn̄g", "POJ presentation");
    assert_eq!(tsiahpng.canonical_tl, "tsia̍h-pn̄g", "identity stays TL");
    assert!(!hanji("tsp", "poj").iter().any(|h| h == "食飯"));
    assert!(!hanji("chp", "tl").iter().any(|h| h == "食飯"));
    // `ss` is the same acronym in both scripts.
    assert!(hanji("ss", "poj").iter().any(|h| h == "鎖匙"));
}

#[test]
fn tps_initials_match_the_tps_abbreviation() {
    let _lock = engine_install_lock();
    install_fixture();
    let cands = fetch("ㄙㄒ", "tps");
    let sosi = cands
        .iter()
        .find(|c| c.hanji.as_deref() == Some("鎖匙"))
        .unwrap_or_else(|| panic!("鎖匙 via tps:ㄙㄒ; got {cands:?}"));
    assert_eq!(
        (sosi.consumed_span_start, sosi.consumed_span_end),
        (0, 6),
        "TPS span is UTF-8 bytes"
    );
    // A tone mark on the tail disqualifies the buffer.
    assert!(!hanji("ㄙㄒˊ", "tps").iter().any(|h| h == "鎖匙"));
}

#[test]
fn custom_entry_sharing_the_abbreviated_word_shows_once() {
    let _lock = engine_install_lock();
    install_fixture();
    // The platform pre-queries `custom_dictionary.db` with `form IN (?,
    // 'abbrev')`, so a custom 鎖匙 `só-sî` already answers `ss` through the
    // custom merge (whole-buffer synth, source rank 0). The dictionary's
    // own 鎖匙 must collapse onto it — same `(roman, hanji, span)` — and
    // the custom row stays ahead of the dictionary block.
    let hanji = fetch_hanji_with_custom(
        "ss",
        "tl",
        vec![CustomEntry {
            roman: "só-sî".into(),
            hanji: Some("鎖匙".into()),
        }],
    );
    assert_eq!(
        hanji,
        vec!["鎖匙", "先生"],
        "custom first, dictionary 鎖匙 deduped"
    );
}
