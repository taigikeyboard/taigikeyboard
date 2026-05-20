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

use composing::dispatch::build_keys_tl_with_inventory;
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

// ---- Hermetic SyllableInventory builder -----------------------------
// Pattern mirrors `engine/composing/tests/build_keys_tl_hyphen.rs`;
// inline duplication preferred over a shared test-utils crate for the
// same reason documented there.

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
