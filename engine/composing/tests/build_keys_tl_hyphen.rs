//! v3.5.8 Phase 9 Item 8 — hyphenated TL shadow buffer integration matrix.
//!
//! Pins `build_keys_tl_with_inventory` against a hermetic
//! `SyllableInventory` for the seven shapes Codex's pre-impl
//! consultation called out as risky off-by-one territory:
//!
//! 1. `taibak` (hyphenless regression) — every `consumed_span` value
//!    must stay byte-for-byte identical to the pre-Item-8 behavior.
//! 2. `tai-bak` — canonical POJ/TL hyphenated multi-syllable.
//! 3. `tai5-bak4` — numeric-tone tokens with a separating hyphen.
//! 4. `goa--si` — POJ neutral-tone `--` collapses into the consumed
//!    prefix.
//! 5. `-tai` — leading hyphen folds into the consumed prefix.
//! 6. `tai-` — trailing hyphen left in the pending buffer (NOT
//!    consumed).
//! 7. `tai-bak-` — internal `-` consumed, trailing `-` left dangling.
//!
//! The hermetic inventory builder mirrors
//! `engine/composing/tests/syllabifier_tl.rs:204-247` to avoid a shared
//! test-utils crate.

// 中文: Phase 9 Item 8 — hyphen-shadow + offset map 的 end-to-end 測試。
// 中文: 七種型態:無連字 regression / 一般 / numeric tone / 雙連字 / 前導 / 後綴 / 內外混合。

use std::path::PathBuf;

use composing::dispatch::build_keys_tl_with_inventory;
use fst::SetBuilder;
use lexicon::SyllableInventory;
use phonetics::canonicalize_syllable;

#[test]
fn hyphenless_input_matches_pre_item8_consumed_span() {
    // Regression guard — `taibak` produced `(0, 3)` and `(0, 6)`
    // pre-Item 8. With the shadow pipeline in place those values must
    // not shift even though no hyphen is involved.
    let inv = build_inventory(&["tai5", "bak4"]);
    let keys = build_keys_tl_with_inventory("taibak", &inv, false);
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    assert_eq!(
        mapped,
        vec![((0, 3), "tl:tai"), ((0, 6), "tl:taibak")],
        "hyphenless consumed_span must stay byte-for-byte stable",
    );
}

#[test]
fn internal_hyphen_consumed_into_full_buffer_span() {
    // `tai-bak` (台北) — shadow `taibak`, endings on shadow={3,6}.
    // raw byte mapping: shadow_end=3 → raw_end=3 (just before `-`);
    // shadow_end=6 → raw_end=7 (after `k`). Full-buffer Tier-1 candidate
    // therefore reports consumed_span_end == raw_len == 7.
    let inv = build_inventory(&["tai5", "bak4"]);
    let keys = build_keys_tl_with_inventory("tai-bak", &inv, false);
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    assert_eq!(mapped, vec![((0, 3), "tl:tai"), ((0, 7), "tl:taibak")],);
}

#[test]
fn numeric_tone_with_hyphen_strips_both_in_key_only() {
    // `tai5-bak4` — shadow `tai5bak4`, endings on shadow={4,8} (the
    // syllabifier folds the trailing tone digit into the matched
    // syllable per `is_false_toneless_boundary`). Tone digits then get
    // stripped during `strip_ascii_tone_digits`; hyphens were stripped
    // up-front by `build_hyphen_shadow`. Final keys equal the hyphenless
    // case.
    let inv = build_inventory(&["tai5", "bak4"]);
    let keys = build_keys_tl_with_inventory("tai5-bak4", &inv, false);
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    assert_eq!(mapped, vec![((0, 4), "tl:tai"), ((0, 9), "tl:taibak")],);
}

#[test]
fn double_hyphen_collapses_into_consumed_prefix() {
    // POJ neutral-tone `--` marker (e.g. `goá--ê`) collapses to two
    // consumed raw bytes between the surviving shadow bytes. Test
    // shape uses `tai--bak` so the inventory stays canonical TL and
    // we do not pull POJ→TL canonicalization into this assertion;
    // the offset math is identical regardless of which syllables the
    // hyphens sit between.
    let inv = build_inventory(&["tai5", "bak4"]);
    let keys = build_keys_tl_with_inventory("tai--bak", &inv, false);
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    assert_eq!(mapped, vec![((0, 3), "tl:tai"), ((0, 8), "tl:taibak")],);
}

#[test]
fn leading_hyphen_consumed_into_first_span() {
    // `-tai` — the leading `-` is consumed by the only candidate's
    // span so the user sees the leading garbage swept along with the
    // commit (rather than being silently abandoned).
    let inv = build_inventory(&["tai5"]);
    let keys = build_keys_tl_with_inventory("-tai", &inv, false);
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    assert_eq!(mapped, vec![((0, 4), "tl:tai")]);
}

#[test]
fn trailing_hyphen_left_in_pending_buffer() {
    // `tai-` — user typed the hyphen anticipating another syllable.
    // consumed_span ends at the `i`, leaving `-` in the pending raw
    // buffer so the host app shows it after commit.
    let inv = build_inventory(&["tai5"]);
    let keys = build_keys_tl_with_inventory("tai-", &inv, false);
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    assert_eq!(mapped, vec![((0, 3), "tl:tai")]);
}

#[test]
fn internal_hyphen_consumed_trailing_hyphen_excluded() {
    // `tai-bak-` — the off-by-one regression hot-spot Codex flagged:
    // the inner `-` folds into the second span's prefix while the
    // outer trailing `-` is dropped from consumed_span. Combined: the
    // full match reports `(0, 7)`, not `(0, 8)`.
    let inv = build_inventory(&["tai5", "bak4"]);
    let keys = build_keys_tl_with_inventory("tai-bak-", &inv, false);
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    assert_eq!(mapped, vec![((0, 3), "tl:tai"), ((0, 7), "tl:taibak")],);
}

#[test]
fn all_hyphen_input_yields_no_keys() {
    // `---` → shadow empty → syllabifier returns no endings →
    // `build_keys_tl_with_inventory` short-circuits before allocating.
    let inv = build_inventory(&["tai5"]);
    let keys = build_keys_tl_with_inventory("---", &inv, false);
    assert!(keys.is_empty(), "expected no keys, got {keys:?}");
}

#[test]
fn multi_byte_chars_preserve_byte_correct_offset_map() {
    // POJ-display chars (e.g. `ō` U+014D, 2 bytes in UTF-8) are out of
    // scope for FST lookup today — the dictionary's `tl:` keys are
    // ASCII — but the shadow / offset-map helper MUST still be
    // byte-correct so a future POJ-canonicalization slice can layer on
    // without re-deriving the byte arithmetic. Lock the contract:
    // `tai-ōe-` (8 raw bytes total) yields one key `tl:tai` at raw
    // byte offset 3; the trailing `-ōe-` stays in the pending buffer
    // per trailing-hyphen semantics, and the multi-byte `ō` does not
    // skew the map. Codex post-impl P3 #3 regression guard.
    let inv = build_inventory(&["tai5"]);
    let keys = build_keys_tl_with_inventory("tai-ōe-", &inv, false);
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    assert_eq!(mapped, vec![((0, 3), "tl:tai")]);
}

// ---- Hermetic SyllableInventory builder -----------------------------
// Pattern mirrors `engine/composing/tests/syllabifier_tl.rs:204-247`;
// inline duplication preferred over a shared crate per the comment
// there ("small duplication is preferable to a shared test-utils
// crate for one reuse").

fn build_inventory(samples: &[&str]) -> SyllableInventory {
    let pairs: Vec<(String, String)> = samples
        .iter()
        .map(|s| {
            canonicalize_syllable(s)
                .unwrap_or_else(|| panic!("sample {s:?} failed canonicalize_syllable"))
        })
        .collect();

    let mut keys: Vec<String> = Vec::new();
    for (canonical, tone) in &pairs {
        if tone.is_empty() {
            keys.push(canonical.clone());
        } else {
            keys.push(format!("{canonical}{tone}"));
            keys.push(canonical.clone());
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
    std::env::temp_dir().join(format!("composing-build-keys-tl-hyphen-{pid}-{n}.fst"))
}
