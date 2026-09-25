//! v3.5.8 S1 — behavior-neutrality contract for the lattice rewire.
//!
//! S1 makes `build_keys_tl_with_inventory` build the segmentation
//! lattice (the multi-start DAG S2's whole-sentence walker will
//! traverse) but emit ONLY its left-anchored (`start == 0`)
//! projection as keys. That projection is byte-identical to the
//! pre-S1 single-start output, so S1 changes NOTHING the user can
//! see, tap, or commit.
//!
//! Interior (`start > 0`) spans are intentionally withheld until S2:
//! `CommitContinuous` carries only `consumed_bytes` and the
//! `(roman, hanji)` dedupe is not span-aware, so a tappable interior
//! candidate would mis-commit (Codex post-impl 2026-05-16 P1 #1/#2).
//!
//! The exhaustive pre-S1 key/offset matrix is pinned by
//! `build_keys_tl_hyphen.rs` / `build_keys_tl_poj_diacritic.rs`
//! (those pass UNCHANGED, which is itself the byte-identical proof).
//! This file adds the explicit *negative* guard: no emitted span may
//! start above byte 0. The full DAG (interior + phrase edges +
//! topological order) is unit-tested in-crate in
//! `composing/src/lattice/builder.rs`.

use composing::dispatch::{build_continuous_keys_with_inventory, build_keys_tl_with_inventory};

mod common;
use common::build_inventory;

fn mapped(keys: &[((u32, u32), String)]) -> Vec<((u32, u32), &str)> {
    keys.iter().map(|(s, k)| (*s, k.as_str())).collect()
}

#[test]
fn output_is_left_anchored_only_no_interior_spans() {
    // Even for a multi-syllable buffer whose lattice has interior
    // edges (e.g. `(3,6) tl:bak`, `(6,11) tl:taigi`), NONE may appear
    // in the user-facing key list — S1 is behavior-neutral.
    for (input, inv_samples) in [
        ("taibak", &["tai5", "bak4"][..]),
        ("tai-bak", &["tai5", "bak4"][..]),
        ("taiuantaigi", &["tai1", "uan1", "gi1"][..]),
    ] {
        let inv = build_inventory(inv_samples);
        let keys = build_keys_tl_with_inventory(input, &inv, phonetics::InputMode::Tl);
        for (span, key) in &keys {
            assert_eq!(
                span.0, 0,
                "S1 must emit only left-anchored keys; got interior span {span:?} ({key}) for input {input:?}",
            );
        }
    }
}

#[test]
fn hyphenless_output_is_byte_identical_to_pre_s1() {
    // `taibak` / inv {tai,bak}. Pre-S1 single-start emitted exactly
    // `[(0,3) tl:tai, (0,6) tl:taibak]`; the lattice's left-anchored
    // projection must reproduce that verbatim (no interior `(3,6)`).
    let inv = build_inventory(&["tai5", "bak4"]);
    let keys = build_keys_tl_with_inventory("taibak", &inv, phonetics::InputMode::Tl);
    assert_eq!(
        mapped(&keys),
        vec![((0, 3), "tl:tai"), ((0, 6), "tl:taibak")],
        "left-anchored projection must equal the pre-S1 single-start output",
    );
}

#[test]
fn internal_hyphen_output_is_byte_identical_to_pre_s1() {
    // `tai-bak` — pre-S1 emitted `[(0,3) tl:tai, (0,7) tl:taibak]`
    // (hyphen folded into the full-buffer span). The interior
    // `(3,7) tl:bak` edge exists in the lattice but is NOT emitted.
    let inv = build_inventory(&["tai5", "bak4"]);
    let keys = build_keys_tl_with_inventory("tai-bak", &inv, phonetics::InputMode::Tl);
    assert_eq!(
        mapped(&keys),
        vec![((0, 3), "tl:tai"), ((0, 7), "tl:taibak")],
    );
}

#[test]
fn longest_match_suppresses_shorter_single_syllable_prefix() {
    // §18 INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX (USER 2026-05-31
    // "suppress even without a tone"). Input `tai` with BOTH `ta` and `tai` valid single
    // syllables. The shorter `ta` (end 2) is a strict prefix of the longest
    // single `tai` (end 3) and has no multi-syllable phrase reading, so it is
    // suppressed — only `tl:tai` survives. This is the reported bug (`tai` /
    // `tai5` must not surface the 2-letter `ta` family).
    let inv = build_inventory(&["ta1", "tai5"]);
    let keys = build_keys_tl_with_inventory("tai", &inv, phonetics::InputMode::Tl);
    assert_eq!(
        mapped(&keys),
        vec![((0, 3), "tl:tai")],
        "shorter single-syllable `ta` must be suppressed under longest-match",
    );
}

#[test]
fn phrase_reachable_shorter_span_survives_suppression() {
    // §18 phrase guard. Input `ainn` against inv {a, i, ai, ainn}. The span
    // `ai` (end 2) is a NON-longest single syllable BUT also parses as the
    // phrase `a`+`i` (interior edge `(1, 2)`), so it is a legitimate
    // different-word candidate and MUST survive suppression. The bare `a`
    // (end 1) is a non-longest single with NO phrase reading → it is the only
    // shorter prefix dropped. Proves the suppression keys on
    // single-syllable-ONLY, never silently dropping a phrase.
    let inv = build_inventory(&["a1", "i1", "ai1", "ainn1"]);
    let keys = build_keys_tl_with_inventory("ainn", &inv, phonetics::InputMode::Tl);
    let m = mapped(&keys);
    assert!(
        m.iter().any(|(_, k)| *k == "tl:ai"),
        "phrase-reachable shorter span `ai` must survive, got {m:?}",
    );
    assert!(
        m.iter().any(|(_, k)| *k == "tl:ainn"),
        "longest single `ainn` must survive, got {m:?}",
    );
    assert!(
        m.iter().all(|(_, k)| *k != "tl:a"),
        "bare `a` (non-longest single, no phrase reading) must be suppressed, got {m:?}",
    );
}

#[test]
fn tl_space_stays_hard_boundary_not_collapsed() {
    // Scope guard for INVARIANT_TPS_SPACE_SOFT_SEPARATOR — the TPS
    // space-strip is TPS-ONLY. In TL (and POJ/English) an ASCII space is
    // a real word boundary / literal space, NOT a syllable separator, so
    // it must stay a HARD boundary: the lattice must NOT span it. `tai uan`
    // emits only the first-syllable `tl:tai@(0,3)`; no cross-space
    // `(0, 7)` phrase key (which would be the TPS behavior leaking into
    // TL). Pins that `build_separator_shadow` is identity for non-TPS.
    let inv = build_inventory(&["tai5", "uan5"]);
    let keys = build_keys_tl_with_inventory("tai uan", &inv, phonetics::InputMode::Tl);
    assert_eq!(
        mapped(&keys),
        vec![((0, 3), "tl:tai")],
        "TL space must stay a hard boundary; no cross-space phrase key allowed",
    );
}

// ---- Hermetic SyllableInventory builder -----------------------------
// Pattern mirrors `engine/composing/tests/build_keys_tl_hyphen.rs`;
// inline duplication preferred over a shared test-utils crate for the
// same reason documented there.

// INVARIANT_TPS_DEFOLD_ENUMERATE (§35) — the alternate-reading generators are
// TPS-only, so for TL / POJ / English the full-key seam must return EXACTLY the
// base seam's output. Exact equality, not `contains`: an alternate leaking into
// a non-TPS mode would show up as an extra key, and USER constraint for the
// round was that TL / POJ must not change at all.

#[test]
fn full_key_seam_equals_base_key_seam_for_non_tps_modes() {
    let inv = build_inventory(&["tai", "gi", "goa", "ai", "li", "hoo", "gua"]);
    for raw in ["taigi", "tai5gi2", "goa2ai3li2", "hoogua", "tai-gi"] {
        for mode in [
            phonetics::InputMode::Tl,
            phonetics::InputMode::Poj,
            phonetics::InputMode::English,
        ] {
            assert_eq!(
                build_continuous_keys_with_inventory(raw, &inv, mode),
                build_keys_tl_with_inventory(raw, &inv, mode),
                "{mode:?} {raw:?} must reach no alternate-reading generator",
            );
        }
    }
}

#[test]
fn closed_dead_end_prefix_is_not_rescued_by_its_phrase_reading() {
    // §18 guard (d) (USER report 2026-09-19: `iah8` trailed the whole `ia`
    // family). `ia` (end 2) is a non-longest single AND parses as `i`+`a`,
    // so guard (c) alone keeps it — but nothing leaves end 2 (`h8` is no
    // syllable) and the remainder carries a typed tone digit, so `ia` is a
    // closed dead end: committing it would strand `h8`. Only the longest
    // single `iah8` survives. Fixture rule: every production syllable that
    // is a strict prefix of `iah8` (`i`, `ia`, `iah`) plus the interior
    // chain `a` / `ah` / `ah8` is present, so the `i`+`a` phrase reading
    // and the `iah`-before-digit false boundary both actually fire.
    let inv = build_inventory(&["i1", "ia1", "iah4", "iah8", "a1", "ah4", "ah8"]);
    let keys = build_keys_tl_with_inventory("iah8", &inv, phonetics::InputMode::Tl);
    assert_eq!(
        mapped(&keys),
        vec![((0, 4), "tl:iah8")],
        "closed dead-end `ia` must be suppressed despite its `i`+`a` reading",
    );
}

#[test]
fn typed_hyphen_after_the_remainder_closes_the_dead_end_too() {
    // §52 — `iah-`: the same shape as `iah8`, closed by the typed `-`
    // instead of a tone digit. `ia` (end 2) still parses as `i`+`a`, but
    // nothing leaves end 2 and the `-` after `h` says the syllable is
    // finished, so `ia` is a closed dead end again.
    // `build_continuous_keys_with_inventory` is the barrier-carrying seam
    // (`build_keys_tl_with_inventory` discards barriers by design).
    let inv = build_inventory(&["i1", "ia1", "iah4", "iah8", "a1", "ah4", "ah8"]);
    let keys = build_continuous_keys_with_inventory("iah-", &inv, phonetics::InputMode::Tl);
    assert_eq!(
        mapped(&keys),
        vec![((0, 3), "tl:iah")],
        "closed dead-end `ia` must be suppressed when a typed `-` closes the remainder",
    );
    // Control: without the `-` the remainder `h` is still an open tail.
    let keys = build_continuous_keys_with_inventory("iah", &inv, phonetics::InputMode::Tl);
    assert!(
        mapped(&keys).iter().any(|(_, k)| *k == "tl:ia"),
        "got {keys:?}"
    );
}

#[test]
fn phrase_reachable_prefix_survives_while_the_remainder_is_open() {
    // Guard (d) negative controls — both halves of "closed dead end" are
    // required, so the mid-typing affordance is untouched:
    // - `iah` (no digit yet): `h` is an open pending tail that may still
    //   become `hoo`, so `ia` keeps surfacing exactly as before.
    // - `iakau3`: an edge leaves end 2 (`kau3`), so `ia` is no dead end
    //   even though the remainder carries a digit.
    let inv = build_inventory(&["i1", "ia1", "iah4", "iah8", "a1", "ah4", "ah8", "kau3"]);
    for input in ["iah", "iakau3"] {
        let keys = build_keys_tl_with_inventory(input, &inv, phonetics::InputMode::Tl);
        let m = mapped(&keys);
        assert!(
            m.iter().any(|(_, k)| *k == "tl:ia"),
            "{input}: phrase-reachable `ia` with an open remainder must survive, got {m:?}",
        );
    }
}
