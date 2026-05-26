//! v3.5.9 D / C-3b — TPS continuous first-class behaviour pin.
//!
//! Mirror of `build_keys_tl_lattice.rs` for the TPS family. Confirms
//! `build_keys_tl_with_inventory` (despite its legacy name, the
//! production seam test wrapper for the shared `build_shadow_lattice`
//! + `left_anchored_keys_from_lattice` path) emits `tps:<bopomofo_toneless>`
//! keys against a `tps:`-tagged hermetic inventory when called with
//! `InputMode::Tps`.
//!
//! This file is the SHIPPED replacement for the retired
//! `composing/src/continuous.rs::build_keys_tps_*` in-crate tests; the
//! legacy TPS short-circuit (which folded TPS into `tl:` keys via
//! `phonetics::tps_to_tl` before C-3b) is gone. The new path:
//! 1. Bopomofo shadow walks `canonicalize_poj_shadow` (no-op for
//!    non-Latin), `build_hyphen_shadow` (no `-` typically), and the
//!    multi-start lattice via `crate::syllabifier::tps::valid_span_endings_lowered`.
//! 2. The mode-aware `strip_tones_for_mode(.., Tps)` drops the 8
//!    Bopomofo tone scalars per
//!    `phonetics::tps::is_tps_tone_mark`.
//! 3. `mode_key_prefix(Tps)` emits the `tps:` family prefix matching
//!    the `tps_notone` axis of the build pipeline.

// 中文: D / C-3b — TPS 連續輸入 first-class 行為定錨測試。對應 TL 版的
// 中文:   build_keys_tl_lattice.rs;舊 build_keys_tps_* in-crate 測試已退役。
// 中文: 新路徑:Bopomofo shadow → canonicalize_poj_shadow no-op → hyphen-shadow
// 中文:   → lattice (走 syllabifier::tps::valid_span_endings_lowered) →
// 中文:   strip_tones_for_mode(.., Tps) → tps: 家族前綴。

use std::path::PathBuf;

use composing::dispatch::build_keys_tl_with_inventory;
use fst::SetBuilder;
use lexicon::SyllableInventory;

fn mapped(keys: &[((u32, u32), String)]) -> Vec<((u32, u32), String)> {
    keys.iter().map(|(s, k)| (*s, k.clone())).collect()
}

#[test]
fn tps_lattice_emits_tps_prefix_for_tone_marked_input() {
    // `ㄉㄞˋㆣㄧˊ` — 2 syllables, both with explicit tone marks
    // (tone-2 ˋ U+02CB then tone-5 ˊ U+02CA). After mode-aware tone
    // strip the toneless syllables `ㄉㄞ` (6 bytes) and `ㆣㄧ` (6 bytes)
    // remain; the lattice walks both as `(0, end_of_first_syllable)`
    // and `(0, end_of_phrase)` left-anchored edges. Per-syllable byte
    // counts: ㄉ ㄞ ㆣ ㄧ each 3 bytes (Bopomofo block / extended);
    // ˋ ˊ each 2 bytes (modifier letters U+02CA / U+02CB).
    //
    // First syllable shadow ending: 3+3+2 = 8.
    // Phrase shadow ending: 8 + 3+3+2 = 16.
    let inv = build_tps_inventory(&[
        "ㄉㄞ", "ㄉㄞˋ", // tone-2 / toneless ㄉㄞ
        "ㆣㄧ", "ㆣㄧˊ", // tone-5 / toneless ㆣㄧ
    ]);
    let keys = build_keys_tl_with_inventory(
        "\u{3109}\u{311e}\u{02cb}\u{31a3}\u{3127}\u{02ca}",
        &inv,
        phonetics::InputMode::Tps,
    );
    // Strip-tones drops ˋ (2 bytes) and ˊ (2 bytes) → body = ㄉㄞ then ㄉㄞㆣㄧ.
    let texts: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        texts.contains(&"tps:\u{3109}\u{311e}"),
        "expected tps:ㄉㄞ left-anchored key, got {texts:?}",
    );
    assert!(
        texts.contains(&"tps:\u{3109}\u{311e}\u{31a3}\u{3127}"),
        "expected tps:ㄉㄞㆣㄧ phrase key, got {texts:?}",
    );
    // No interior keys (start != 0) — left-anchored projection only.
    for ((start, _), key) in &keys {
        assert_eq!(*start, 0, "interior key {key:?} leaked");
    }
}

#[test]
fn tps_lattice_tone1_no_mark_emits_key() {
    // Tone-1 syllable carries no Bopomofo tone mark. The terminator
    // scan's implicit "next-initial-seen" rule splits the buffer at
    // each new initial; here `ㄉㄞ` (3+3) followed by `ㆣㄧ` (3+3).
    // After inventory gating both spans hit `tps:ㄉㄞ` / `tps:ㄉㄞㆣㄧ`.
    let inv = build_tps_inventory(&["ㄉㄞ", "ㆣㄧ"]);
    let keys = build_keys_tl_with_inventory(
        "\u{3109}\u{311e}\u{31a3}\u{3127}",
        &inv,
        phonetics::InputMode::Tps,
    );
    let texts: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        texts.contains(&"tps:\u{3109}\u{311e}"),
        "tone-1 first syllable key missing: {texts:?}",
    );
    assert!(
        texts.contains(&"tps:\u{3109}\u{311e}\u{31a3}\u{3127}"),
        "tone-1 phrase key missing: {texts:?}",
    );
}

#[test]
fn tps_lattice_inventory_gate_rejects_unknown_syllable() {
    // Inventory only has `ㄉㄞ`; the second span `ㆣㄧ` is NOT in the
    // inventory so the lattice walker rejects it. Only the first
    // (`(0, 6) tps:ㄉㄞ`) edge survives; no phrase key emitted.
    let inv = build_tps_inventory(&["ㄉㄞ"]);
    let keys = build_keys_tl_with_inventory(
        "\u{3109}\u{311e}\u{31a3}\u{3127}",
        &inv,
        phonetics::InputMode::Tps,
    );
    let texts: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert_eq!(
        texts,
        vec!["tps:\u{3109}\u{311e}"],
        "inventory gate should reject ㆣㄧ; full output: {:?}",
        mapped(&keys),
    );
}

#[test]
fn tps_lattice_empty_input_yields_no_keys() {
    let inv = build_tps_inventory(&["ㄉㄞ"]);
    let keys = build_keys_tl_with_inventory("", &inv, phonetics::InputMode::Tps);
    assert!(keys.is_empty(), "{keys:?}");
}

/// v3.5.9 D / C-3b — Codex post-impl R3 regression pin. Restores the
/// `build_keys_tps_caps_at_max_syllables` shape from the retired
/// `build_keys_tps` in-crate test. With the unified shadow-lattice path
/// the cap is enforced by `build_lattice`'s `max_syllables` arg
/// (=`shadow::MAX_SYLLABLES` = 8) flowing into
/// `syllabifier::tps::valid_span_endings_lowered`'s BFS depth.
/// A buffer with `MAX_SYLLABLES + 1` well-formed TPS syllables must
/// emit at most `MAX_SYLLABLES` left-anchored phrase endings (no
/// over-consumption past byte position N*MAX_SYLLABLES).
#[test]
fn tps_lattice_caps_at_max_syllables_via_lattice_bfs() {
    // Each `ㄉㄞ` syllable = 6 bytes (ㄉ 3 + ㄞ 3); 9 syllables = 54 bytes.
    // Tone-1 chain — implicit next-initial-seen rule splits between
    // each ㄉ-ㄞ pair. Inventory contains only the single-syllable form
    // so the BFS depth path is one hop per syllable.
    const SYLLABLE_BYTES: usize = 6;
    const MAX_SYLLABLES: usize = 8;
    let inv = build_tps_inventory(&["ㄉㄞ"]);
    let raw = "\u{3109}\u{311e}".repeat(MAX_SYLLABLES + 1);
    let keys = build_keys_tl_with_inventory(&raw, &inv, phonetics::InputMode::Tps);
    // Left-anchored phrase endings emitted by the BFS up to depth N.
    // The largest end must NOT exceed MAX_SYLLABLES * syllable_bytes.
    let max_end = keys
        .iter()
        .map(|((_, e), _)| *e as usize)
        .max()
        .unwrap_or(0);
    assert!(
        max_end <= MAX_SYLLABLES * SYLLABLE_BYTES,
        "TPS cap violated: max end {max_end} exceeds {} ({MAX_SYLLABLES} × {SYLLABLE_BYTES}); keys = {keys:?}",
        MAX_SYLLABLES * SYLLABLE_BYTES,
    );
    // And at least one phrase-length key was emitted (so the cap is
    // a ceiling, not a noop that yielded nothing).
    assert!(
        keys.iter()
            .any(|((_, e), _)| *e as usize >= SYLLABLE_BYTES * 2),
        "expected at least one multi-syllable key, got {keys:?}",
    );
}

// ---- Hermetic TPS SyllableInventory builder -------------------------
// Pattern mirrors `engine/composing/tests/build_keys_tl_lattice.rs`'s
// `build_inventory`. Samples are pre-stripped Bopomofo syllables (with
// or without tone mark trailing); each sample is emitted as a single
// `tps:<sample>` FST key plus, when a tone mark is present, the
// toneless body (`tps:<body>`) — mirroring how the production C-0 build
// pipeline emits both `tps_num` (with tones) and `tps_notone` (without)
// per dictionary row. No phonotactic gating here; the caller picks the
// sample set that exercises the path under test.

fn build_tps_inventory(samples: &[&str]) -> SyllableInventory {
    let mut keys: Vec<String> = Vec::new();
    for s in samples {
        keys.push(format!("tps:{s}"));
        // If the sample ends in a TPS tone mark, also emit the
        // toneless body so the inventory contains the `tps_notone`
        // form — matches the build pipeline's per-row dual emit.
        if let Some(last) = s.chars().last() {
            if phonetics::is_tps_tone_mark(last) {
                let body_bytes = s.len() - last.len_utf8();
                let body = &s[..body_bytes];
                if !body.is_empty() {
                    keys.push(format!("tps:{body}"));
                }
            }
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
    std::env::temp_dir().join(format!("taigi_lattice_tps_inv_{pid}_{n}.fst"))
}
