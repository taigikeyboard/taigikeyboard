//! Slot 0 follows the user's selection — covers the `kingsin` bug (USER
//! 2026-09-14: picking 更新 repeatedly never moved it off second place).
//!
//! Two decisions had to change together:
//!
//! 1. **Which homophone fills a lattice edge.** The walker's edge pick
//!    (`lexicon::best_candidate_for_key_with_barriers`) ranked by
//!    `freq × boost` with the boost capped at `ranking::MAX_BOOST` (×5),
//!    so 更新 (freq 1) could never beat 敬神 (freq 25) whatever the count.
//!    User weight is now the leading `CandidateSortKey` dimension for the edge pick
//!    and the span-local list alike.
//! 2. **What the edge costs.** The walker priced an edge on the chosen
//!    word's own frequency; swapping in the rarer 更新 made the whole-buffer
//!    edge costlier than the 經+身 single-syllable split, so 更新 lost slot 0
//!    to a wrong segmentation instead. The key's cheapest homophone
//!    (`EdgeBest::span_walker_cost` since E1 P3, the key's max frequency
//!    before) now prices the edge — "is this span a word" is decoupled from
//!    "which word".
//!
//! Fixture mirrors the pre-E1 production frequencies (walker costs derived
//! from them, `test_support::walker_cost_from_fixture_frequency`): 敬神/kìng-sîn
//! 25, 更新/king-sin 1, and the single syllables 經/king 9218 + 身/sin 10865
//! whose split undercuts a freq-1 two-syllable edge under the khiin cost model.

use crate::common::{
    build_dictionary_fst_tl_toned, build_syllables_fst_tl, build_tkdb_v4, cell_with_hanji, config,
    empty_association_bin, fetch_cells, fetch_hanji, install_lexicon, selected, Fetch, Row,
    Selected, NOW_MS,
};
use lexicon::CustomEntry;
use test_support::{engine_install_lock, write_temp};

const TWO_HOURS_MS: i64 = 2 * 60 * 60 * 1000;

fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "kingsin",
            hanji: "敬神",
            tl: "kìng-sîn",
            syll: 2,
            freq: 25,
        },
        Row {
            toneless_key: "kingsin",
            hanji: "更新",
            tl: "king-sin",
            syll: 2,
            freq: 1,
        },
        Row {
            toneless_key: "king",
            hanji: "經",
            tl: "king",
            syll: 1,
            freq: 9218,
        },
        Row {
            toneless_key: "sin",
            hanji: "身",
            tl: "sin",
            syll: 1,
            freq: 10865,
        },
    ]
}

fn install_rows(rows: &[Row], syllables: &[&str]) {
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v4(rows));
    let fst_path = build_dictionary_fst_tl_toned(rows);
    let association_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst_tl(syllables);
    install_lexicon(&fst_path, &dict_path, &association_path, &syllables_path);
}

fn install_fixture() {
    install_rows(&fixture_rows(), &["king1", "king3", "sin1", "sin5"]);
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
    // ONLY word under `kingsin`, so the span's cost is its own (freq 1)
    // and cold start segments as 經+身 (synth 經身). One selection must
    // still lift 更新 to slot 0 — user-entry cost cap in `edge_cost`.
    let _lock = engine_install_lock();
    let rows: Vec<Row> = fixture_rows()
        .into_iter()
        .filter(|r| r.hanji != "敬神")
        .collect();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v4(&rows));
    let fst_path = build_dictionary_fst_tl_toned(&rows);
    let association_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst_tl(&["king1", "king3", "sin1", "sin5"]);
    install_lexicon(&fst_path, &dict_path, &association_path, &syllables_path);

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

// The picked word's key extends past the input, so only the Step 4b scan
// finds it (`tsua` is a strict prefix of `tsuann`, `ko` of `kong`).
fn partial_pick_rows() -> Vec<Row> {
    let row = |toneless_key, hanji, tl, syll, freq| Row {
        toneless_key,
        hanji,
        tl,
        syll,
        freq,
    };
    vec![
        row("an", "安", "an", 1, 9000),
        row("tsua", "紙", "tsuá", 1, 9000),
        row("tsuann", "怎", "tsuánn", 1, 20),
        row("ko", "哥", "ko", 1, 9000),
        row("kong", "講", "kóng", 1, 9000),
        row("antsuann", "按怎", "án-tsuánn", 2, 500),
        row("anko", "安哥", "an-ko", 2, 500),
        row("ankong", "按講", "àn-kóng", 2, 500),
        row("a", "阿", "a", 1, 9000),
        row("tsu", "資", "tsu", 1, 9000),
        row("tsuan", "全", "tsuân", 1, 20),
        row("si", "是", "sī", 1, 9000),
        row("sian", "先", "sian", 1, 20),
        row("hoo", "予", "hōo", 1, 9000),
        row("gua", "我", "guá", 1, 9000),
        row("hoogua", "予我", "hōo--guá", 2, 1),
    ]
}

fn install_partial_pick_fixture() {
    install_rows(
        &partial_pick_rows(),
        &[
            "a1", "an1", "an3", "tsu1", "tsua2", "tsuan5", "tsuann2", "ko1", "kong2", "si7",
            "sian1", "hoo7", "gua2", "guan2",
        ],
    );
}

#[test]
fn never_picked_partial_word_stays_behind_the_sentence() {
    let _lock = engine_install_lock();
    install_partial_pick_fixture();
    let hanji = fetch_hanji_with_freq("antsua", Vec::new(), 0);
    assert_eq!(
        hanji[0], "安紙",
        "cold start keeps the walker; got {hanji:?}"
    );
    // trace: `an` + `tsua` is the walk; Step 4b adds 按怎; `an` alone is the
    // one front row. Fixture rule: the strict-prefix syllables `a` (leaves
    // `ntsua` unwalkable), `tsu` and `tsuan` reach no row of their own.
    assert_eq!(hanji, ["安紙", "按怎", "安"]);
}

#[test]
fn picked_partial_word_takes_slot0_from_the_sentence() {
    let _lock = engine_install_lock();
    install_partial_pick_fixture();
    let hanji = fetch_hanji_with_freq(
        "antsua",
        vec![selected("按怎", "án-tsuánn", 1, TWO_HOURS_MS)],
        NOW_MS,
    );
    assert_eq!(hanji[0], "按怎", "picked word must lead; got {hanji:?}");
    assert_eq!(hanji[1], "安紙", "sentence directly behind; got {hanji:?}");
}

#[test]
fn picked_partial_word_stays_behind_a_dictionary_word_at_slot0() {
    let _lock = engine_install_lock();
    install_partial_pick_fixture();
    // `anko` is an exact dictionary word: predictive stays behind exact.
    let hanji = fetch_hanji_with_freq(
        "anko",
        vec![selected("按講", "àn-kóng", 10, TWO_HOURS_MS)],
        NOW_MS,
    );
    assert_eq!(hanji[0], "安哥", "exact word keeps slot 0; got {hanji:?}");
    assert_eq!(hanji[1], "按講", "pick still leads the rest; got {hanji:?}");
}

#[test]
fn picked_partial_word_stays_behind_a_dictionary_word_the_custom_path_spells() {
    let _lock = engine_install_lock();
    install_partial_pick_fixture();
    // A rare 予我 loses the walk to its split; the custom 予 then walks
    // `hōo guá`, which no promotion may replace, yet it is still that word.
    let hanji = fetch_hanji(
        "hoogua",
        "tl",
        Fetch {
            custom: vec![
                CustomEntry {
                    roman: "hōo".to_string(),
                    hanji: Some("予".to_string()),
                },
                CustomEntry {
                    roman: "hōo-guán".to_string(),
                    hanji: Some("予阮".to_string()),
                },
            ],
            frequency: vec![selected("予阮", "hōo-guán", 5, TWO_HOURS_MS)],
            now_ms: NOW_MS,
            ..Default::default()
        },
    );
    assert_eq!(
        hanji[0], "予我",
        "dictionary word keeps slot 0; got {hanji:?}"
    );
}

#[test]
fn one_syllable_pick_stays_behind_a_sentence() {
    let _lock = engine_install_lock();
    install_partial_pick_fixture();
    // `sia` walks 是 + 阿; 先 `sian` extends the input by one letter.
    let hanji = fetch_hanji_with_freq(
        "sia",
        vec![selected("先", "sian", 10, TWO_HOURS_MS)],
        NOW_MS,
    );
    assert_eq!(hanji[0], "是阿", "sentence keeps slot 0; got {hanji:?}");
    assert_eq!(hanji[1], "先", "pick still leads the rest; got {hanji:?}");
}

#[test]
fn one_syllable_pick_stays_behind_the_exact_syllable() {
    let _lock = engine_install_lock();
    install_partial_pick_fixture();
    // The store keys 講 by its reading, so this row may come from `kong`.
    let hanji = fetch_hanji_with_freq("ko", vec![selected("講", "kóng", 10, TWO_HOURS_MS)], NOW_MS);
    assert_eq!(hanji[0], "哥", "exact syllable keeps slot 0; got {hanji:?}");
    assert_eq!(hanji[1], "講", "pick still leads the rest; got {hanji:?}");
}

// An auto-capitalized sentence start recases what the row shows, never its
// identity: a pick counted under `Kóng` would rank nothing typed `kong`.
#[test]
fn capitalized_input_keeps_the_walker_identity_lowercase() {
    let _lock = engine_install_lock();
    install_partial_pick_fixture();
    for (raw, hanji, shown, identity) in [
        ("Kong", "講", "Kóng", "kóng"),
        ("Antsua", "安紙", "An tsuá", "an tsuá"),
    ] {
        let cells = fetch_cells(&config("tl"), raw, Fetch::default());
        let (_, roman, _, canonical_tl) = cell_with_hanji(&cells, hanji);
        assert_eq!(roman, shown, "the row shows the typed case; got {cells:?}");
        assert_eq!(
            canonical_tl, identity,
            "identity stays lowercase; got {cells:?}"
        );
    }
}

#[test]
fn a_pick_typed_lowercase_ranks_the_same_word_typed_capitalized() {
    let _lock = engine_install_lock();
    install_partial_pick_fixture();
    let hanji = fetch_hanji_with_freq(
        "Antsua",
        vec![selected("按怎", "án-tsuánn", 1, TWO_HOURS_MS)],
        NOW_MS,
    );
    assert_eq!(
        hanji[0], "按怎",
        "pick leads under auto-capitalization; got {hanji:?}"
    );
}
