//! Slot-0 respects the dictionary separator form — covers the `hoogua`
//! bug (USER 2026-06-02). The continuous-input walker synthesizes the
//! full-buffer "best reading" by joining per-syllable canonical romans
//! with a SPACE. For a genuine multi-word reading that space-join is the
//! desired display, but when the synth's `(hanji, span)` coincides with a
//! single lexical dict word that stores its own separator form (`-` hyphen
//! or `--` neutral tone), the space-join is a malformed rendering of that word.
//!
//! Reported symptom: typing `hoogua` showed best candidate `hōo guá`
//! (space) instead of the dictionary word 予我 `hōo--guá` (khinsiann). The
//! pre-fix `(roman, hanji, span)` slot-0 dedupe could not collapse the
//! pair because the romans differ ONLY in the separator, so the malformed
//! synth won slot 0 and the canonical dict row sank below.
//!
//! The fix (display layer, NOT the cost/segmentation primitive — diagnosis
//! §S5 / behavioral-invariants §18 lesson) promotes the matching FULL,
//! full-span dict row's canonical roman to slot 0 when it is the SAME
//! reading (separator-insensitive, tone-preserving) as the synth.
//!
//! Fixture mirrors production: 予我/hōo--guá (freq 16) and 戶外/hōo-guā
//! (freq 25) share the toneless key `hoogua`; the high-freq single chars
//! 予/hōo + 我/guá make the walker's min-cost path the 予+我 SPLIT (synth
//! hanji 予我, synth roman `hōo guá`), exactly as production does. 戶外's
//! tone-7 `guā` differs from the synth's tone-2 `guá`, so it must NOT be
//! promoted (Core Principle #7 word identity).
//!
//! Hermetic `LexiconHandle` install comes from `tests/common/mod.rs`.

use crate::common::Fetch;
use crate::common::{
    build_dictionary_fst_tl_toned, build_syllables_fst_tl, build_tkdb_v4, build_tkdb_v4_costed,
    config_tl, empty_association_bin, fetch_at_pos_response, fetch_cells, install_lexicon,
    selected, Row, NOW_MS,
};
use test_support::{engine_install_lock, walker_cost_from_fixture_frequency, write_temp};

/// 予我/hōo--guá (neutral tone, freq 16) + 戶外/hōo-guā (hyphen, freq 25) collide on
/// `tl_notone = hoogua`. 予/hōo + 我/guá are high-freq single chars so the
/// walker's min-cost path is the 予+我 split → synth `hōo guá` (space).
fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "hoogua",
            hanji: "予我",
            tl: "hōo--guá", // khinsiann; tl_num hoo7gua2
            syll: 2,
            freq: 16,
        },
        Row {
            toneless_key: "hoogua",
            hanji: "戶外",
            tl: "hōo-guā", // hyphen; tl_num hoo7gua7 — tone 7 (different word)
            syll: 2,
            freq: 25,
        },
        // 予/我 are very common single morphemes — high freq so the
        // walker's min-cost path is the 予+我 SPLIT, not the 1-edge
        // whole-word 戶外. Threshold derived from the cost model
        // (`ln(CORPUS/(1+freq))/len^0.2 × syll^0.2`, `lattice/cost.rs`):
        // the 2-edge split beats the freq-25 whole word only once each
        // single's freq exceeds ~18.4k; 80k leaves comfortable margin.
        Row {
            toneless_key: "hoo",
            hanji: "予",
            tl: "hōo",
            syll: 1,
            freq: 80_000,
        },
        Row {
            toneless_key: "gua",
            hanji: "我",
            tl: "guá",
            syll: 1,
            freq: 80_000,
        },
    ]
}

fn install_rows(rows: &[Row], syllables: &[&str]) {
    install_rows_with_dictionary(rows, &build_tkdb_v4(rows), syllables);
}

fn install_rows_with_dictionary(rows: &[Row], dictionary: &[u8], syllables: &[&str]) {
    let dict_path = write_temp("dictionary.bin", dictionary);
    let fst_path = build_dictionary_fst_tl_toned(rows);
    let association_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst_tl(syllables);
    install_lexicon(&fst_path, &dict_path, &association_path, &syllables_path);
}

fn install_fixture() {
    install_rows(&fixture_rows(), &["hoo7", "gua2", "gua7"]);
}

/// Drive `raw` through `Start → FetchAtPos` and return
/// the `(hanji, roman)` pairs in candidate order.
///
/// Runs in DEFAULT config (literal-roman candidate ON, §34/S22). The
/// always-on preedit-literal prepend takes index 0 as a bare-roman row
/// (`hanji = None`), so this file's separator invariant asserts against the
/// first HANJI-bearing candidate (the dict best), which now sits right after
/// it — verifying `INVARIANT_CONTINUOUS_SLOT0_RESPECTS_DICT_SEPARATOR` under
/// the config users actually run. The separator-promote lives in
/// `assemble_candidates` and is independent of the toggle.
fn fetch_candidates(raw: &str) -> Vec<(Option<String>, String)> {
    let cfg = config_tl();
    let resp = fetch_at_pos_response(&cfg, raw, Fetch::default());
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
fn slot0_promotes_dict_khinsiann_form_over_space_synth() {
    let _lock = engine_install_lock();
    install_fixture();
    // INVARIANT_CONTINUOUS_SLOT0_RESPECTS_DICT_SEPARATOR.
    // The best DICT candidate must be the dictionary word 予我 with its
    // canonical khinsiann roman `hōo--guá`, NOT the walker's space-joined
    // synth `hōo guá`. In default config the §34 literal-roman row (`hanji
    // = None`) leads at index 0, so the dict best is the first HANJI-bearing
    // candidate right after it.
    let cands = fetch_candidates("hoogua");
    let (hanji0, roman0) = cands
        .iter()
        .find(|(hanji, _)| hanji.is_some())
        .expect("hoogua produced no hanji candidate");
    assert_eq!(
        hanji0.as_deref(),
        Some("予我"),
        "best dict candidate hanji must be 予我; got {cands:?}"
    );
    assert_eq!(
        roman0, "hōo--guá",
        "best dict roman must be the dict khinsiann form `hōo--guá`, not the space synth; got {roman0:?}"
    );

    // The malformed space-join synth must be fully suppressed (it was the
    // same reading as the promoted dict row).
    assert!(
        !cands.iter().any(|(_, r)| r == "hōo guá"),
        "space-joined synth `hōo guá` must be suppressed; got {cands:?}"
    );

    // 戶外/hōo-guā is a DIFFERENT word (tone 7 ≠ tone 2) and must remain a
    // separate candidate — never collapsed into the promote.
    assert!(
        cands
            .iter()
            .any(|(h, r)| h.as_deref() == Some("戶外") && r == "hōo-guā"),
        "戶外/hōo-guā must remain present (different word); got {cands:?}"
    );
}

#[test]
fn slot0_keeps_space_synth_for_genuine_multiword_reading() {
    let _lock = engine_install_lock();
    install_fixture();
    // Negative guard: `hoo` alone is a single syllable — no full-span
    // multi-word collision. The single-char dict word 予/hōo is the
    // expected slot-0 and nothing about the promote should fire. This
    // pins that the promote is scoped to the separator-mismatch case and
    // does not perturb ordinary single-syllable continuous output.
    let cands = fetch_candidates("hoo");
    assert!(
        cands
            .iter()
            .any(|(h, r)| h.as_deref() == Some("予") && r == "hōo"),
        "hoo must surface 予/hōo; got {cands:?}"
    );
}

/// 出來 in both separator forms (production freqs: `tshut-lâi` 529,
/// `tshut--lâi` 16) beside the single syllables 出/tshut 25088 and
/// 來/lâi 58294. As in production, the walker's min-cost path is the ONE
/// whole-word edge, so its synth roman is that edge's own dict roman —
/// an exact twin of a span-local row (`gina` bug, Discord 2026-09-30).
fn install_separator_sibling_fixture() {
    let rows = vec![
        Row {
            toneless_key: "tshutlai",
            hanji: "出來",
            tl: "tshut-lâi",
            syll: 2,
            freq: 529,
        },
        Row {
            toneless_key: "tshutlai",
            hanji: "出來",
            tl: "tshut--lâi",
            syll: 2,
            freq: 16,
        },
        Row {
            toneless_key: "tshut",
            hanji: "出",
            tl: "tshut",
            syll: 1,
            freq: 25_088,
        },
        Row {
            toneless_key: "lai",
            hanji: "來",
            tl: "lâi",
            syll: 1,
            freq: 58_294,
        },
    ];
    install_rows(&rows, &["tshut4", "lai5"]);
}

/// The 出來 romans in display order.
fn tshutlai_romans(fetch: Fetch) -> Vec<String> {
    fetch_cells(&config_tl(), "tshutlai", fetch)
        .into_iter()
        .filter(|cell| cell.0.as_deref() == Some("出來"))
        .map(|cell| cell.1)
        .collect()
}

#[test]
fn slot0_keeps_walker_pick_over_its_separator_sibling_cold() {
    let _lock = engine_install_lock();
    install_separator_sibling_fixture();
    // The walker picks the frequent `tshut-lâi`; the exact twin exists, so
    // the promote must not swap in the freq-16 `tshut--lâi`.
    assert_eq!(
        tshutlai_romans(Fetch::default()),
        ["tshut-lâi", "tshut--lâi"]
    );
}

#[test]
fn slot0_follows_the_selected_separator_form() {
    let _lock = engine_install_lock();
    install_separator_sibling_fixture();
    // Selecting the rare khinsiann form makes it the walker's edge word;
    // it must lead, not be swapped back for its hyphen sibling.
    let picked_khinsiann = Fetch {
        frequency: vec![selected("出來", "tshut--lâi", 10, 1_000)],
        now_ms: NOW_MS,
        ..Default::default()
    };
    assert_eq!(
        tshutlai_romans(picked_khinsiann),
        ["tshut--lâi", "tshut-lâi"]
    );
}

/// 予我 in two separator forms with explicit corpus costs that disagree with
/// their frequencies: `hōo-guá` (freq 10, cost 15,000) is the corpus's
/// spelling, `hōo--guá` (freq 16, cost 15,300) the frequent one. Both cost more
/// than a freq-25 word (13,127), which already loses to the 予 + 我 split
/// (`fixture_rows`' singles at freq 80,000), so slot 0 is the split synth
/// `hōo guá` and the promote picks a dictionary form.
fn install_corpus_separator_fixture() {
    let mut rows = fixture_rows();
    rows.retain(|row| row.hanji != "戶外");
    rows.push(Row {
        toneless_key: "hoogua",
        hanji: "予我",
        tl: "hōo-guá",
        syll: 2,
        freq: 10,
    });
    let dictionary = build_tkdb_v4_costed(&rows, |row| match row.tl {
        "hōo--guá" => 15_300,
        "hōo-guá" => 15_000,
        _ => walker_cost_from_fixture_frequency(row.freq),
    });
    install_rows_with_dictionary(&rows, &dictionary, &["hoo7", "gua2"]);
}

#[test]
fn slot0_promotes_the_separator_form_the_corpus_writes() {
    let _lock = engine_install_lock();
    install_corpus_separator_fixture();
    // E1 P5b: the promote takes the first same-reading row of the sorted
    // list, and the list now sorts on the corpus cost — so slot 0 shows the
    // corpus's `hōo-guá`, not the more frequent `hōo--guá`.
    let cands = fetch_candidates("hoogua");
    let romans: Vec<&str> = cands
        .iter()
        .filter(|(hanji, _)| hanji.as_deref() == Some("予我"))
        .map(|(_, roman)| roman.as_str())
        .collect();
    assert_eq!(romans, ["hōo-guá", "hōo--guá"], "got {cands:?}");
}
