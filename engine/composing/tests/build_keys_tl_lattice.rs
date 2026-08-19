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

// 中文: S1 — 行為中性契約測試:lattice 已建,但對外只發左錨投影 (start==0),
// 中文:   逐 byte 等同 S1 前;內段留 S2 (commit 僅帶 consumed_bytes、dedupe 非 span-aware)。

use std::path::PathBuf;

use composing::dispatch::{build_continuous_keys_with_inventory, build_keys_tl_with_inventory};
use fst::SetBuilder;
use lexicon::SyllableInventory;
use phonetics::canonicalize_syllable;

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
    // 「免調也壓制」). Input `tai` with BOTH `ta` and `tai` valid single
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

// 中文: §35 替代讀法產生器僅限 TPS,故 TL/POJ/English 的完整鍵接縫必須與 base 接縫「完全相等」。
// 中文:   用 exact equality 而非 contains:替代讀法若洩漏到非 TPS 模式會多出鍵。

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

fn build_inventory(samples: &[&str]) -> SyllableInventory {
    let pairs: Vec<(String, String)> = samples
        .iter()
        .map(|s| {
            canonicalize_syllable(s)
                .unwrap_or_else(|| panic!("sample {s:?} failed canonicalize_syllable"))
        })
        .collect();

    // v3.5.9 B-1: tagged-single-FST — emit keys with `tl:` prefix.
    let mut keys: Vec<String> = Vec::new();
    for (canonical, tone) in &pairs {
        if tone.is_empty() {
            keys.push(format!("tl:{canonical}"));
        } else {
            keys.push(format!("tl:{canonical}{tone}"));
            keys.push(format!("tl:{canonical}"));
        }
    }
    keys.sort();
    keys.dedup();

    let path = unique_temp_path();
    let file = std::fs::File::create(&path).expect("create fst");
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
    for key in &keys {
        builder.insert(key.as_bytes()).expect("insert");
    }
    builder.finish().expect("finish");
    SyllableInventory::open(&path).expect("open inventory")
}

fn unique_temp_path() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("taigi_lattice_inv_{pid}_{n}.fst"))
}
