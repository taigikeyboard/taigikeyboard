//! TPS visual-dedup integration test — covers the bug where TPS input
//! `ㄨㄢ` returned two `灣` candidates because two `dict.bin` rows
//! (`灣/uan` and `灣/uân`) share the toneless TPS key `tps:ㄨㄢ` and the
//! pre-sort `(roman, hanji, consumed_span)` dedupe legitimately keeps
//! both for TL/POJ. TPS mode hides romanization (hanji-only UI), so
//! `composing::continuous::dedupe_display_hanji_for_tps` collapses the
//! visible duplicate. TL/POJ paths must NOT collapse — distinct
//! romanizations are distinct rows in their UI.
//!
//! Hermetic install of `LexiconHandle` comes from `tests/common/mod.rs`
//! (this binary is its own process with its own singleton; the lock
//! guards in-binary `#[test]` parallelism). Fixture builders come from
//! `tests/common/mod.rs` — composing tests cannot import
//! `lexicon/tests/common/mod.rs` (test-private).

use protos::engine::FetchAtPos;

mod common;
use common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v3, config, empty_association_bin,
    engine_install_lock, fetch_at_pos_response, install_lexicon, write_temp, Row,
};

fn fixture_rows() -> Vec<Row> {
    // The reported bug: two 灣 rows differ only on tone (and hence
    // romanization), share toneless TPS key `tps:ㄨㄢ`. Mirrors production
    // `dictionary.csv:lines 灣,uan` + `灣,uân`.
    vec![
        Row {
            toneless_key: "uan",
            hanzi: "灣",
            tl: "uan",
            syll: 1,
            freq: 9981,
        },
        Row {
            toneless_key: "uan",
            hanzi: "灣",
            tl: "uân",
            syll: 1,
            freq: 7984,
        },
        // Sanity row to prove the dedupe is hanji-keyed and not roman-keyed:
        // distinct hanji on the same toneless key — must always survive.
        Row {
            toneless_key: "uan",
            hanzi: "彎",
            tl: "uan",
            syll: 1,
            freq: 100,
        },
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(&["uan1", "uan5"]);
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);
}

fn fetch(raw: &str, input_mode: &str) -> Vec<(Option<String>, String)> {
    let cfg = config(input_mode);
    let resp = fetch_at_pos_response(&cfg, raw, FetchAtPos::default());
    resp.continuous
        .map(|c| {
            c.candidates
                .into_iter()
                .map(|cand| (cand.hanji, cand.roman))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn tps_input_collapses_duplicate_hanji() {
    let _lock = engine_install_lock();
    install_fixture();
    // Raw `ㄨㄢ` (3+3=6 bytes UTF-8). `dispatch::handle` upgrades mode to
    // InputMode::Tps via `contains_tps`, so the `input_mode` string only
    // pins the non-Bopomofo path — we still pass "tl" to exercise the
    // production auto-detect.
    let candidates = fetch("\u{3128}\u{3122}", "tl");
    let wan_count = candidates
        .iter()
        .filter(|(h, _)| h.as_deref() == Some("灣"))
        .count();
    let wan2_count = candidates
        .iter()
        .filter(|(h, _)| h.as_deref() == Some("彎"))
        .count();
    assert_eq!(
        wan_count, 1,
        "TPS mode must collapse the two 灣 rows (uan + uân) to one visible \
         candidate; got {candidates:?}"
    );
    assert_eq!(
        wan2_count, 1,
        "distinct hanji 彎 must always survive the TPS dedupe; got {candidates:?}"
    );
}

#[test]
fn tl_input_keeps_both_uan_and_uan_diacritic_rows() {
    let _lock = engine_install_lock();
    install_fixture();
    // Same fixture, TL toneless input `uan` — both 灣 rows differ on the
    // displayed `roman` (`uan` vs `uân`) so the TL UI shows distinct rows
    // and must NOT collapse. Exact-count + roman-set pin guards against a
    // duplicate explosion or a roman-render regression sneaking past.
    let candidates = fetch("uan", "tl");
    let mut wan_romans: Vec<&str> = candidates
        .iter()
        .filter(|(h, _)| h.as_deref() == Some("灣"))
        .map(|(_, r)| r.as_str())
        .collect();
    wan_romans.sort();
    assert_eq!(
        wan_romans,
        vec!["uan", "uân"],
        "TL mode must keep exactly the two 灣 rows with romans {{uan, uân}}; \
         got {candidates:?}"
    );
}

#[test]
fn poj_input_keeps_both_oan_and_oan_diacritic_rows() {
    let _lock = engine_install_lock();
    install_fixture();
    // POJ input `oan` — same fixture, two 灣 rows derive POJ display
    // `oan` / `oân`. POJ UI shows romanization so both must survive.
    let candidates = fetch("oan", "poj");
    let mut wan_romans: Vec<&str> = candidates
        .iter()
        .filter(|(h, _)| h.as_deref() == Some("灣"))
        .map(|(_, r)| r.as_str())
        .collect();
    wan_romans.sort();
    assert_eq!(
        wan_romans,
        vec!["oan", "oân"],
        "POJ mode must keep exactly the two 灣 rows with romans {{oan, oân}}; \
         got {candidates:?}"
    );
}
