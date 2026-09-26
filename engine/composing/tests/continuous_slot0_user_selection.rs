//! Slot 0 follows the user's selection — covers the `kingsin` bug (USER
//! 2026-09-14: picking 更新 repeatedly never moved it off second place).
//!
//! Two decisions had to change together:
//!
//! 1. **Which homophone fills a lattice edge.** The walker's edge pick
//!    (`lexicon::best_candidate_for_key_with_barriers`) ranked by
//!    `freq × boost` with the boost capped at `ranking::MAX_BOOST` (×5),
//!    so 更新 (freq 1) could never beat 敬神 (freq 25) whatever the count.
//!    User weight is now the leading `SortKey` dimension for the edge pick
//!    and the span-local list alike.
//! 2. **What the edge costs.** The walker priced an edge on the chosen
//!    word's own frequency; swapping in the rarer 更新 made the whole-buffer
//!    edge costlier than the 經+身 single-syllable split, so 更新 lost slot 0
//!    to a wrong segmentation instead. `EdgeBest::span_frequency` (the
//!    key's max frequency) now prices the edge — "is this span a word" is
//!    decoupled from "which word".
//!
//! Fixture mirrors production frequencies: 敬神/kìng-sîn 25, 更新/king-sin
//! 1, and the single syllables 經/king 9218 + 身/sin 10865 whose split
//! undercuts a freq-1 two-syllable edge under the khiin cost model.

mod common;
use common::{
    build_dictionary_fst_tl_toned, build_syllables_fst_tl, build_tkdb_v3, empty_association_bin,
    engine_install_lock, fetch_hanji, install_lexicon, selected, write_temp, Fetch, Row, Selected,
    NOW_MS,
};

const TWO_HOURS_MS: i64 = 2 * 60 * 60 * 1000;

fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "kingsin",
            hanzi: "敬神",
            tl: "kìng-sîn",
            syll: 2,
            freq: 25,
        },
        Row {
            toneless_key: "kingsin",
            hanzi: "更新",
            tl: "king-sin",
            syll: 2,
            freq: 1,
        },
        Row {
            toneless_key: "king",
            hanzi: "經",
            tl: "king",
            syll: 1,
            freq: 9218,
        },
        Row {
            toneless_key: "sin",
            hanzi: "身",
            tl: "sin",
            syll: 1,
            freq: 10865,
        },
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst_tl_toned(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst_tl(&["king1", "king3", "sin1", "sin5"]);
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);
}

/// Candidate hanji in display order; the §34 literal-roman row at index 0
/// is dropped, so index 0 here = the walker's slot 0.
fn fetch_hanji_with_freq(raw: &str, freq: Vec<Selected>, now_ms: i64) -> Vec<String> {
    fetch_hanji(
        raw,
        "tl",
        Fetch {
            frequency: freq,
            now_ms,
            ..Default::default()
        },
    )
}

#[test]
fn cold_start_slot0_is_the_common_homophone() {
    let _lock = engine_install_lock();
    install_fixture();
    let hanji = fetch_hanji_with_freq("kingsin", Vec::new(), 0);
    assert_eq!(
        hanji[0], "敬神",
        "cold start: max-frequency homophone leads; got {hanji:?}"
    );
    assert!(hanji.contains(&"更新".to_string()));
}

#[test]
fn one_selection_moves_rare_homophone_to_slot0_even_hours_later() {
    let _lock = engine_install_lock();
    install_fixture();
    // One pick, two hours ago — past the retired 1-hour recency window.
    let hanji = fetch_hanji_with_freq(
        "kingsin",
        vec![selected("更新", "king-sin", 1, TWO_HOURS_MS)],
        NOW_MS,
    );
    assert_eq!(hanji[0], "更新", "selected word must lead; got {hanji:?}");
    assert_eq!(
        hanji[1], "敬神",
        "displaced homophone directly behind; got {hanji:?}"
    );
    // The whole-buffer edge must still win the segmentation: the 經+身
    // split would surface as a synthesized 經身 slot 0.
    assert!(
        !hanji.iter().any(|h| h == "經身"),
        "rare selected word must not price its span out of the best path; got {hanji:?}"
    );
}

#[test]
fn selected_rare_phrase_alone_under_its_key_beats_the_single_syllable_split() {
    // Device repro 2026-09-14 (敬神/警訊 sources toggled off): 更新 is the
    // ONLY word under `kingsin`, so the span's max frequency is its own 1
    // and cold start segments as 經+身 (synth 經身). One selection must
    // still lift 更新 to slot 0 — user-dict floor in `edge_cost`.
    let _lock = engine_install_lock();
    let rows: Vec<Row> = fixture_rows()
        .into_iter()
        .filter(|r| r.hanzi != "敬神")
        .collect();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst_tl_toned(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst_tl(&["king1", "king3", "sin1", "sin5"]);
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);

    let cold = fetch_hanji_with_freq("kingsin", Vec::new(), 0);
    assert_eq!(
        cold[0], "經身",
        "cold start splits the rare phrase; got {cold:?}"
    );

    let hanji = fetch_hanji_with_freq(
        "kingsin",
        vec![selected("更新", "king-sin", 1, TWO_HOURS_MS)],
        NOW_MS,
    );
    assert_eq!(hanji[0], "更新", "got {hanji:?}");
    assert!(!hanji.iter().any(|h| h == "經身"), "got {hanji:?}");
}

#[test]
fn more_selected_homophone_beats_less_selected_one() {
    let _lock = engine_install_lock();
    install_fixture();
    // Both selected; 敬神 heavier (10 picks, fresh) than 更新 (1 pick, old).
    let hanji = fetch_hanji_with_freq(
        "kingsin",
        vec![
            selected("更新", "king-sin", 1, TWO_HOURS_MS),
            selected("敬神", "kìng-sîn", 10, 1_000),
        ],
        NOW_MS,
    );
    assert_eq!(hanji[0], "敬神", "got {hanji:?}");
    assert_eq!(hanji[1], "更新", "got {hanji:?}");
}

#[test]
fn selection_without_wall_clock_is_neutral() {
    let _lock = engine_install_lock();
    install_fixture();
    // `now_ms = 0` (platform did not inject a clock) → no promotion.
    let hanji = fetch_hanji_with_freq("kingsin", vec![selected("更新", "king-sin", 50, 1_000)], 0);
    assert_eq!(
        hanji[0], "敬神",
        "no clock → cold-start order; got {hanji:?}"
    );
}
