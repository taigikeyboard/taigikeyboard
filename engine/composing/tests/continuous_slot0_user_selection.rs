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
    continuous_engine as composing_engine, custom, empty_association_bin, fetch_cells, fetch_hanji,
    frequency_map, install_lexicon, selected, Fetch, Row, Selected, NOW_MS,
};
use lexicon::CustomEntry;
use test_support::{engine_install_lock, write_temp};

const TWO_HOURS_MS: i64 = 2 * 60 * 60 * 1000;

fn row(
    toneless_key: &'static str,
    hanji: &'static str,
    tl: &'static str,
    syll: u8,
    freq: u32,
) -> Row {
    Row {
        toneless_key,
        hanji,
        tl,
        syll,
        freq,
    }
}

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
fn picked_partial_word_passes_a_one_word_path() {
    let _lock = engine_install_lock();
    install_partial_pick_fixture();
    // `anko` walks the one dictionary word 安哥. A one-word path joins the
    // sort (ranking-lab proposal, 2026-11), so the picked 按講 leads it on
    // `preference_class`, before `coverage_kind` holds a prefix back. The
    // Step 4d post-pass moves the path past one-syllable prefix picks only,
    // so a pick of two syllables keeps its lead.
    let hanji = fetch_hanji_with_freq(
        "anko",
        vec![selected("按講", "àn-kóng", 10, TWO_HOURS_MS)],
        NOW_MS,
    );
    assert_eq!(hanji[0], "按講", "the pick leads; got {hanji:?}");
    assert_eq!(hanji[1], "安哥", "the exact word follows; got {hanji:?}");
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
fn one_syllable_pick_stays_behind_a_one_word_path() {
    let _lock = engine_install_lock();
    install_partial_pick_fixture();
    // `ko` walks the one word 哥, which joins the sort (ranking-lab
    // proposal, 2026-11) and lands behind the picked 講 on
    // `preference_class`. The store keys 講 by its reading, so the pick may
    // have come from `kong`; a one-syllable prefix pick then stands aside
    // for the path (Step 4d post-pass), as it does for a sentence.
    let hanji = fetch_hanji_with_freq("ko", vec![selected("講", "kóng", 10, TWO_HOURS_MS)], NOW_MS);
    assert_eq!(hanji, ["哥", "講"]);
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

// A one-word path ahead of one-syllable prefix picks (Step 4d post-pass):
// `ko` walks 哥 and `sia` walks 寫, each one dictionary word. Fixture rule:
// `si` is a strict-prefix syllable of `sia` and `sian`; it leaves `a`
// unwalkable, so 是 never lists. `kong` extends `ko`; `k` is no syllable.
// These lists hold no two-syllable word. One can share a list with a
// one-syllable prefix pick once the input reaches its last syllable (`ia` →
// 以後 `í-āu` beside 抑 `ia̍h`); `mixed_prefix_picks_*` below covers that, and
// `picked_partial_word_passes_a_one_word_path` a two-syllable pick alone.
fn one_word_path_rows() -> Vec<Row> {
    vec![
        row("ko", "哥", "ko", 1, 9000),
        row("kong", "講", "kóng", 1, 9000),
        row("si", "是", "sī", 1, 9000),
        row("sia", "寫", "siá", 1, 9000),
        row("sian", "仙", "sian", 1, 9000),
    ]
}

fn install_one_word_path_fixture() {
    install_rows(
        &one_word_path_rows(),
        &["ko1", "kong2", "koo2", "si7", "sia2", "sian1"],
    );
}

fn fetch_with_custom(raw: &str, frequency: Vec<Selected>, custom: Vec<CustomEntry>) -> Vec<String> {
    fetch_hanji(
        raw,
        "tl",
        Fetch {
            frequency,
            custom,
            now_ms: NOW_MS,
            ..Default::default()
        },
    )
}

#[test]
fn one_word_path_leads_a_one_syllable_prefix_pick() {
    let _lock = engine_install_lock();
    install_one_word_path_fixture();
    // trace: `sia` walks 寫 (one edge); Step 4b adds 仙 `sian` (prefix). The
    // sort reads 仙, 寫 (`preference_class`); the post-pass moves 寫 ahead.
    let hanji = fetch_hanji_with_freq(
        "sia",
        vec![selected("仙", "sian", 10, TWO_HOURS_MS)],
        NOW_MS,
    );
    assert_eq!(hanji, ["寫", "仙"]);
}

#[test]
fn one_word_path_does_not_pass_a_never_picked_custom_word() {
    let _lock = engine_install_lock();
    install_one_word_path_fixture();
    // trace: picked 講 (class 0), the never-picked custom 姑 `kóo` (class 1,
    // a prefix extension), then 哥 (class 2). The row directly before 哥 is
    // the custom word, not a pick, so nothing moves: the narrow post-pass
    // keeps both earlier USER rules (a pick before a custom word, a custom
    // word before the rest).
    let hanji = fetch_with_custom(
        "ko",
        vec![selected("講", "kóng", 10, TWO_HOURS_MS)],
        vec![custom("kóo", "姑")],
    );
    assert_eq!(hanji, ["講", "姑", "哥"]);
    // With no pick the custom word leads the path as before (USER: keep).
    let hanji = fetch_with_custom("ko", Vec::new(), vec![custom("kóo", "姑")]);
    assert_eq!(hanji, ["姑", "哥", "講"]);
}

#[test]
fn a_picked_custom_word_on_the_whole_span_is_the_path() {
    let _lock = engine_install_lock();
    install_one_word_path_fixture();
    // trace: the walker's edge pick ranks with the list's key, so a picked
    // custom 糕 `ko` beats 哥 on the edge and is the path itself; nothing
    // stands before it.
    let hanji = fetch_with_custom(
        "ko",
        vec![
            selected("糕", "ko", 40, 1_000),
            selected("講", "kóng", 10, TWO_HOURS_MS),
        ],
        vec![custom("ko", "糕")],
    );
    assert_eq!(hanji, ["糕", "講", "哥"]);
}

/// The ranking lab's `(hanji, role, placed_by)` per row of `engine`'s list
/// under `picks` (`""` for the §34 literal).
fn lab_rows(
    engine: &composing::Engine,
    config: &protos::engine::AppConfig,
    picks: Vec<Selected>,
) -> Vec<(String, &'static str, &'static str)> {
    let rows = composing::UserRows {
        frequency: frequency_map(picks),
        custom: Vec::new(),
        learned: Vec::new(),
    };
    composing::explain::mechanism(
        engine,
        NOW_MS,
        &rows,
        &ranking::ContextRanks::default(),
        false,
        config,
    )
    .expect("a composition")
    .candidates
    .into_iter()
    .map(|row| (row.hanji.unwrap_or_default(), row.role, row.placed_by))
    .collect()
}

#[test]
fn the_lab_reports_a_moved_path_as_placed_by_the_post_pass() {
    let _lock = engine_install_lock();
    install_one_word_path_fixture();
    let config = config("tl");
    let engine = composing_engine(&config, "ko");
    // The moved path sits outside the twelve stages like the walker's
    // sentence; 講 reads as moved behind it, not as a row 哥 beats.
    assert_eq!(
        lab_rows(
            &engine,
            &config,
            vec![selected("講", "kóng", 10, TWO_HOURS_MS)]
        ),
        [
            (String::new(), "literal", "literal"),
            ("哥".to_string(), "sentence", "path_over_prefix_pick"),
            ("講".to_string(), "word", "behind_path"),
        ]
    );
    // Without a pick to stand aside from, the path stays an ordinary row.
    // trace: 哥 / 講 tie on tier and class; full before prefix.
    assert_eq!(
        lab_rows(&engine, &config, Vec::new()),
        [
            (String::new(), "literal", "literal"),
            ("哥".to_string(), "word", "lead"),
            ("講".to_string(), "word", "coverage_kind"),
        ]
    );
}

// `ia` walks the one word 也 `iā`. Step 4b reaches 抑 `ia̍h` (one syllable)
// and 以後 `í-āu` (two: the typed `ia` runs past `i` into `āu`). Fixture rule:
// `i` is a strict-prefix syllable of `ia` and `iah`, and `a` of `au`; `a` has
// no word, so `i a` is no path and 伊 `i` never lists as a front row (its
// remainder is unwalkable). `ia` is a strict prefix of `iah`.
fn mixed_prefix_picks_rows() -> Vec<Row> {
    vec![
        row("i", "伊", "i", 1, 9000),
        row("ia", "也", "iā", 1, 9000),
        row("iah", "抑", "ia̍h", 1, 9000),
        row("iau", "以後", "í-āu", 2, 500),
    ]
}

fn install_mixed_prefix_picks_fixture() {
    install_rows(
        &mixed_prefix_picks_rows(),
        &["a1", "au7", "i1", "i2", "ia7", "iah8"],
    );
}

#[test]
fn mixed_prefix_picks_put_the_one_word_path_ahead_of_the_one_syllable_pick() {
    let _lock = engine_install_lock();
    install_mixed_prefix_picks_fixture();
    // trace: tier 0 = 抑, 以後 (prefix picks, class 0, by weight), 也 (class
    // 2); 伊 is absent (fixture note). The post-pass takes the picks directly
    // before 也, moves the one-syllable 抑 behind it and keeps 以後 ahead.
    let hanji = fetch_hanji_with_freq(
        "ia",
        vec![
            selected("抑", "ia̍h", 40, TWO_HOURS_MS),
            selected("以後", "í-āu", 10, TWO_HOURS_MS),
        ],
        NOW_MS,
    );
    assert_eq!(hanji, ["以後", "也", "抑"]);
    // The two-syllable pick heavier: sorted 以後, 抑, 也 — the same result.
    let hanji = fetch_hanji_with_freq(
        "ia",
        vec![
            selected("抑", "ia̍h", 10, TWO_HOURS_MS),
            selected("以後", "í-āu", 40, TWO_HOURS_MS),
        ],
        NOW_MS,
    );
    assert_eq!(hanji, ["以後", "也", "抑"]);
}

#[test]
fn mixed_prefix_picks_the_lab_keeps_the_path_after_the_kept_pick() {
    let _lock = engine_install_lock();
    install_mixed_prefix_picks_fixture();
    let config = config("tl");
    let engine = composing_engine(&config, "ia");
    let picks = vec![
        selected("抑", "ia̍h", 40, TWO_HOURS_MS),
        selected("以後", "í-āu", 10, TWO_HOURS_MS),
    ];
    // 以後, kept ahead of the path, is the first row the sort chose; 抑 reads
    // as moved behind the path, not as a row ranked against 以後.
    assert_eq!(
        lab_rows(&engine, &config, picks),
        [
            (String::new(), "literal", "literal"),
            ("以後".to_string(), "word", "lead"),
            ("也".to_string(), "sentence", "path_over_prefix_pick"),
            ("抑".to_string(), "word", "behind_path"),
        ]
    );
}
