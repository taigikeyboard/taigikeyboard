//! Candidate Display = Romanization Only display-dedup integration test (§44 /
//! `INVARIANT_ROMAN_ONLY_CELLS_COLLAPSE_SAME_ROMAN`). Roman-only cells hide
//! the hanji, so rows that differ only in hanji — homophones `食/tsia̍h` +
//! `𤆬/tsia̍h`, and the §34 literal `tsiah` beside dict `隻/tsiah` — are
//! visible duplicates. `composing::requests` collapses them by the RENDERED
//! ROMAN alone AFTER the literal prepend; first-seen wins (top-ranked sorted
//! row, or the literal). Side-by-side (explicit, proto default `0`, or an
//! unknown value) keeps every row; TPS ignores the setting.
//!
//! The consumed span left the key on 2026-09-03 (§44). No fixture here can
//! produce the case it used to separate — two rows rendering the SAME roman
//! over DIFFERENT spans — because a row's toneless FST key is derived from
//! its own reading, so equal romans mean equal keys and equal consumed
//! slices; an attempt to install a hand-built row with a longer key and a
//! shorter reading does not surface at all. The span-agnostic key is pinned
//! on the helper instead
//! (`composing::requests::tests::dedupe_display_roman_collapses_same_roman_across_spans`).
//!
//! Hermetic `LexiconHandle` install comes from `tests/common/mod.rs`.

use protos::engine::{CandidateDisplayMode, CellScript};

use crate::common::Fetch;
use crate::common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v4, config_with_display_mode,
    empty_association_bin, fetch_at_pos_response, install_lexicon, Row,
};
use test_support::{engine_install_lock, write_temp};

fn fixture_rows() -> Vec<Row> {
    // Toneless key `tsiah` carries three production-shaped rows: two homophone
    // readings `tsia̍h` (食 high-freq, 𤆬 low-freq) and the tone-4 `tsiah`
    // (隻) whose roman equals the §34 literal for raw input `tsiah`.
    vec![
        Row {
            toneless_key: "tsiah",
            hanji: "食",
            tl: "tsia̍h",
            syll: 1,
            freq: 9000,
        },
        Row {
            toneless_key: "tsiah",
            hanji: "𤆬",
            tl: "tsia̍h",
            syll: 1,
            freq: 3000,
        },
        Row {
            toneless_key: "tsiah",
            hanji: "隻",
            tl: "tsiah",
            syll: 1,
            freq: 5000,
        },
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v4(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let association_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(&["tsiah8", "tsiah4"]);
    install_lexicon(&fst_path, &dict_path, &association_path, &syllables_path);
}

const SIDE_BY_SIDE: i32 = CandidateDisplayMode::SideBySide as i32;
const ROMAN_ONLY: i32 = CandidateDisplayMode::RomanOnly as i32;

/// `(hanji, roman)` per candidate, strip order.
fn fetch(
    raw: &str,
    input_mode: &str,
    candidate_display_mode: i32,
) -> Vec<(Option<String>, String)> {
    fetch_with_identity(raw, input_mode, candidate_display_mode)
        .into_iter()
        .map(|(hanji, roman, _, _)| (hanji, roman))
        .collect()
}

fn rows_with_roman<'a>(
    candidates: &'a [(Option<String>, String)],
    roman: &str,
) -> Vec<&'a (Option<String>, String)> {
    candidates.iter().filter(|(_, r)| r == roman).collect()
}

#[test]
fn roman_only_collapses_same_roman_rows_keeping_the_top_ranked_one() {
    let _lock = engine_install_lock();
    install_fixture();
    let candidates = fetch("tsiah", "tl", ROMAN_ONLY);

    // 食 (freq 9000) outranks 𤆬 (freq 3000) → 食 is the survivor.
    let tsiah8 = rows_with_roman(&candidates, "tsia̍h");
    assert_eq!(
        tsiah8.len(),
        1,
        "羅馬字 must collapse 食/𤆬 `tsia̍h` to one cell; got {candidates:?}"
    );
    assert_eq!(
        tsiah8[0].0.as_deref(),
        Some("食"),
        "survivor = top-ranked row"
    );

    // The §34 literal `tsiah` is inserted first, so it absorbs dict 隻/tsiah.
    let literal = rows_with_roman(&candidates, "tsiah");
    assert_eq!(
        literal.len(),
        1,
        "literal `tsiah` and dict 隻/tsiah must be one cell; got {candidates:?}"
    );
    assert_eq!(
        literal[0].0, None,
        "the literal keeps slot 0 and wins the collapse"
    );
    assert_eq!(candidates[0].1, "tsiah", "literal stays at index 0");
    assert_eq!(
        candidates.len(),
        2,
        "exactly two visible cells; got {candidates:?}"
    );
}

/// `(hanji, roman, display_text, canonical_tl)` per candidate, strip order —
/// the identity sidechannels the platform round-trips on commit.
fn fetch_with_identity(
    raw: &str,
    input_mode: &str,
    candidate_display_mode: i32,
) -> Vec<(Option<String>, String, String, String)> {
    let cfg = config_with_display_mode(input_mode, candidate_display_mode);
    let resp = fetch_at_pos_response(&cfg, raw, Fetch::default());
    resp.continuous
        .map(|c| {
            c.candidates
                .into_iter()
                .map(|cand| (cand.hanji, cand.roman, cand.display_text, cand.canonical_tl))
                .collect()
        })
        .unwrap_or_default()
}

/// The §42 split cell of each listed row (`CandidateMessage.cell_script`).
fn fetch_cell_scripts(raw: &str, input_mode: &str, candidate_display_mode: i32) -> Vec<CellScript> {
    let cfg = config_with_display_mode(input_mode, candidate_display_mode);
    let resp = fetch_at_pos_response(&cfg, raw, Fetch::default());
    resp.continuous
        .map(|c| c.candidates.iter().map(|cand| cand.cell_script()).collect())
        .unwrap_or_default()
}

/// Under a single-script display the literal absorbs dict 隻/tsiah, so its
/// commit must still learn 隻: the literal inherits the absorbed row's
/// `display_text` (the NextWord / frequency key) and `canonical_tl` while it
/// keeps reading and committing the bare roman (`hanji` stays absent).
#[test]
fn single_script_display_literal_inherits_absorbed_dict_identity() {
    let _lock = engine_install_lock();
    install_fixture();
    let combined = CandidateDisplayMode::Combined as i32;
    for mode in [ROMAN_ONLY, combined] {
        let candidates = fetch_with_identity("tsiah", "tl", mode);
        let literal = &candidates[0];
        assert_eq!(
            literal.1, "tsiah",
            "literal stays at index 0; got {candidates:?}"
        );
        assert_eq!(
            literal.0, None,
            "literal stays a roman-only cell (mode {mode})"
        );
        assert_eq!(
            literal.2, "隻",
            "literal carries the absorbed 隻 identity (mode {mode}); got {candidates:?}"
        );
        assert_eq!(
            literal.3, "tsiah",
            "canonical TL follows the absorbed row (mode {mode})"
        );
        // Hanji with Romanization keeps the dict row itself: the engine split lists its Hanji row.
        assert_eq!(
            candidates.iter().any(|c| c.0.as_deref() == Some("隻")),
            mode == combined,
            "only COMBINED keeps 隻's own row; got {candidates:?}"
        );
    }
}

/// Pairing lists the dict row beside the literal, so the literal stays a pure
/// romanization commit with its own (roman) identity.
#[test]
fn side_by_side_literal_keeps_its_own_roman_identity() {
    let _lock = engine_install_lock();
    install_fixture();
    let candidates = fetch_with_identity("tsiah", "tl", SIDE_BY_SIDE);
    let literal = &candidates[0];
    assert_eq!(literal.1, "tsiah");
    assert_eq!(literal.0, None);
    assert_eq!(
        literal.2, "tsiah",
        "並排 literal identity = its roman; got {candidates:?}"
    );
}

#[test]
fn side_by_side_keeps_every_row_for_explicit_default_and_unknown_values() {
    let _lock = engine_install_lock();
    install_fixture();
    let explicit = fetch("tsiah", "tl", SIDE_BY_SIDE);

    // Today's list: literal + 隻 + 食 + 𤆬, hanji-bearing rows all distinct.
    assert_eq!(
        explicit.len(),
        4,
        "side-by-side keeps all rows; got {explicit:?}"
    );
    assert_eq!(rows_with_roman(&explicit, "tsia̍h").len(), 2);
    assert_eq!(rows_with_roman(&explicit, "tsiah").len(), 2);

    // proto3 default (un-wired builds) and an unknown value from a newer
    // platform both normalise to side-by-side — one fallback, one place.
    assert_eq!(
        fetch("tsiah", "tl", 0),
        explicit,
        "UNSPECIFIED = side-by-side"
    );
    assert_eq!(
        fetch("tsiah", "tl", 99),
        explicit,
        "unknown value = side-by-side"
    );
    // Hanji with Romanization keeps every row too and lists it as §42 one-script
    // cells, deduped per script: trace — literal tsiah (roman cell); 食 Hanji +
    // tsia̍h roman; 隻 Hanji (its tsiah absorbed by the literal); 𤆬 Hanji (its
    // tsia̍h already shown).
    assert_eq!(
        fetch("tsiah", "tl", CandidateDisplayMode::Combined as i32),
        vec![
            (None, "tsiah".to_owned()),
            (Some("食".to_owned()), "tsia̍h".to_owned()),
            (Some("食".to_owned()), "tsia̍h".to_owned()),
            (Some("隻".to_owned()), "tsiah".to_owned()),
            (Some("𤆬".to_owned()), "tsia̍h".to_owned()),
        ],
        "COMBINED = every row, split into one-script cells"
    );
    assert_eq!(
        fetch_cell_scripts("tsiah", "tl", CandidateDisplayMode::Combined as i32),
        vec![
            CellScript::Roman,
            CellScript::Hanji,
            CellScript::Roman,
            CellScript::Hanji,
            CellScript::Hanji,
        ],
        "each split row names the script its cell shows"
    );
    assert!(
        fetch_cell_scripts("tsiah", "tl", SIDE_BY_SIDE)
            .iter()
            .all(|script| *script == CellScript::Paired),
        "side-by-side rows are paired"
    );
}

#[test]
fn poj_roman_only_collapses_on_the_rendered_poj_roman() {
    let _lock = engine_install_lock();
    install_fixture();
    // POJ renders `tsia̍h` as `chia̍h`; the dedupe keys on what the user sees.
    let candidates = fetch("chiah", "poj", ROMAN_ONLY);
    let chiah8 = rows_with_roman(&candidates, "chia̍h");
    assert_eq!(
        chiah8.len(),
        1,
        "POJ 羅馬字 collapses 食/𤆬 `chia̍h`; got {candidates:?}"
    );
    assert_eq!(chiah8[0].0.as_deref(), Some("食"));
    assert_eq!(
        rows_with_roman(&candidates, "chiah").len(),
        1,
        "literal absorbs 隻/chiah"
    );
}

#[test]
fn tps_ignores_the_roman_only_setting() {
    let _lock = engine_install_lock();
    install_fixture();
    // Toneless Bopomofo for `tsiah`: `ㄐㄧㄚㆷ` (dispatch promotes the mode to
    // TPS from the buffer, so the "tl" string is irrelevant). TPS cells are
    // hanji-first, so 食 and 𤆬 stay distinct whatever the picker says.
    let tps: String = phonetics::tl_numeric_token_to_tps("tsiah4", false, true)
        .chars()
        .filter(|&c| !c.is_whitespace() && c != '-' && !phonetics::is_tps_tone_mark(c))
        .collect();
    assert!(!tps.is_empty());
    let roman_only = fetch(&tps, "tl", ROMAN_ONLY);
    let side_by_side = fetch(&tps, "tl", SIDE_BY_SIDE);
    assert_eq!(
        roman_only, side_by_side,
        "TPS output must not depend on 候選詞顯示"
    );
    let hanji: Vec<&str> = roman_only
        .iter()
        .filter_map(|(h, _)| h.as_deref())
        .collect();
    assert!(
        hanji.contains(&"食") && hanji.contains(&"𤆬"),
        "both 同音異字 stay in TPS; got {roman_only:?}"
    );
}
