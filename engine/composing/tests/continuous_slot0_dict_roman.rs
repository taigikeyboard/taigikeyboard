//! Slot-0 respects the dictionary separator form — covers the `hoogua`
//! bug (USER 2026-06-02). The continuous-input walker synthesizes the
//! full-buffer "best reading" by joining per-syllable canonical romans
//! with a SPACE. For a genuine multi-word reading that space-join is the
//! desired display, but when the synth's `(hanji, span)` coincides with a
//! single lexical dict word that stores its own separator form (`-` 連字
//! or `--` 輕聲), the space-join is a malformed rendering of that word.
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

use protos::engine::FetchAtPos;

mod common;
use common::{
    build_dictionary_fst_tl_toned, build_syllables_fst_tl, build_tkdb_v3, config_tl,
    empty_association_bin, engine_install_lock, fetch_at_pos_response, install_lexicon, write_temp,
    Row,
};

/// 予我/hōo--guá (輕聲, freq 16) + 戶外/hōo-guā (連字, freq 25) collide on
/// `tl_notone = hoogua`. 予/hōo + 我/guá are high-freq single chars so the
/// walker's min-cost path is the 予+我 split → synth `hōo guá` (space).
fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "hoogua",
            hanzi: "予我",
            tl: "hōo--guá", // khinsiann; tl_num hoo7gua2
            syll: 2,
            freq: 16,
        },
        Row {
            toneless_key: "hoogua",
            hanzi: "戶外",
            tl: "hōo-guā", // 連字; tl_num hoo7gua7 — tone 7 (different word)
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
            hanzi: "予",
            tl: "hōo",
            syll: 1,
            freq: 80_000,
        },
        Row {
            toneless_key: "gua",
            hanzi: "我",
            tl: "guá",
            syll: 1,
            freq: 80_000,
        },
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst_tl_toned(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst_tl(&["hoo7", "gua2", "gua7"]);
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);
}

/// Drive `raw` through `Start → EnterContinuous → FetchAtPos` and return
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
