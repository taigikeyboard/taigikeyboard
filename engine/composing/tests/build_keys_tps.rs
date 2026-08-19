//! v3.5.9 D / C-3b — TPS continuous first-class behaviour pin.
//!
//! Mirror of `build_keys_tl_lattice.rs` for the TPS family. Confirms
//! `build_keys_tl_with_inventory` (despite its legacy name, the
//! production seam test wrapper for the shared `build_shadow_lattice`
//! and `left_anchored_keys_from_lattice` path) emits
//! `tps:<bopomofo_toneless>` keys against a `tps:`-tagged hermetic
//! inventory when called with `InputMode::Tps`.
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

use composing::dispatch::{build_continuous_keys_with_inventory, build_keys_tl_with_inventory};
use fst::SetBuilder;
use lexicon::SyllableInventory;

fn mapped(keys: &[((u32, u32), String)]) -> Vec<((u32, u32), String)> {
    keys.iter().map(|(s, k)| (*s, k.clone())).collect()
}

#[test]
fn tps_lattice_emits_tps_prefix_for_tone_marked_input() {
    // B2 (§17 TPS) — `ㄉㄞˋㆣㄧˊ`, 2 syllables both with explicit tone marks
    // (tone-2 ˋ U+02CB then tone-5 ˊ U+02CA). Both syllables are fully toned,
    // so the left-anchored keys KEEP their tone marks verbatim (the toned
    // `tps:<tps_num>` family) — the candidate set is then filtered to exactly
    // the typed tones. Pre-B2 the marks were unconditionally stripped to
    // `tps:ㄉㄞ` / `tps:ㄉㄞㆣㄧ` (the bug). Per-syllable byte counts: ㄉ ㄞ ㆣ ㄧ
    // each 3 bytes (Bopomofo block / extended); ˋ ˊ each 2 bytes (modifier
    // letters U+02CA / U+02CB).
    let inv = build_tps_inventory(&[
        "ㄉㄞ", "ㄉㄞˋ", // tone-2 / toneless ㄉㄞ
        "ㆣㄧ", "ㆣㄧˊ", // tone-5 / toneless ㆣㄧ
    ]);
    let keys = build_keys_tl_with_inventory(
        "\u{3109}\u{311e}\u{02cb}\u{31a3}\u{3127}\u{02ca}",
        &inv,
        phonetics::InputMode::Tps,
    );
    let texts: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        texts.contains(&"tps:\u{3109}\u{311e}\u{02cb}"),
        "expected toned tps:ㄉㄞˋ left-anchored key, got {texts:?}",
    );
    assert!(
        texts.contains(&"tps:\u{3109}\u{311e}\u{02cb}\u{31a3}\u{3127}\u{02ca}"),
        "expected toned tps:ㄉㄞˋㆣㄧˊ phrase key, got {texts:?}",
    );
    // No interior keys (start != 0) — left-anchored projection only.
    for ((start, _), key) in &keys {
        assert_eq!(*start, 0, "interior key {key:?} leaked");
    }
}

#[test]
fn tps_lattice_tone1_no_mark_emits_key() {
    // Tone-1 syllable carries no Bopomofo tone mark. The inv-driven
    // BFS probes every byte boundary against the `tps:` family and
    // accepts whatever the inventory recognises; here `ㄉㄞ` (3+3) and
    // `ㄉㄞㆣㄧ` (3+3+3+3) both hit, so the left-anchored projection
    // emits the atomic and phrase keys.
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
fn tps_lattice_spans_tone1_separator_space_into_phrase_key() {
    // INVARIANT_TPS_SPACE_SOFT_SEPARATOR — `ㄍㄠ ㄉㄞ` (kau-tai, the
    // user's 交代 case). First tone has no Bopomofo tone mark, so the
    // keyboard appends an ASCII space as the syllable boundary. The TPS
    // shadow strips that space (`build_separator_shadow`) so the lattice
    // chains `ㄍㄠ` + `ㄉㄞ` into a cross-space phrase edge instead of
    // dead-ending at the space. Codepoints: ㄍ U+310D, ㄠ U+3120,
    // space U+0020, ㄉ U+3109, ㄞ U+311E (each Bopomofo char 3 bytes,
    // space 1 byte → raw len 13; shadow len 12).
    let inv = build_tps_inventory(&["\u{310d}\u{3120}", "\u{3109}\u{311e}"]);
    let raw = "\u{310d}\u{3120}\u{0020}\u{3109}\u{311e}";
    assert_eq!(raw.len(), 13, "raw byte length precondition");
    let keys = build_keys_tl_with_inventory(raw, &inv, phonetics::InputMode::Tps);

    // First-syllable key — span ends at raw 6 (before the space), space
    // left pending for a mid-commit.
    assert!(
        keys.contains(&((0u32, 6u32), "tps:\u{310d}\u{3120}".to_string())),
        "first-syllable key tps:ㄍㄠ@(0,6) missing: {:?}",
        mapped(&keys),
    );
    // Cross-space phrase key — body is space-FREE (`tps:ㄍㄠㄉㄞ`) and the
    // consumed span ends at raw len 13, so a full commit replaces the
    // whole preedit INCLUDING the separator space.
    assert!(
        keys.contains(&(
            (0u32, 13u32),
            "tps:\u{310d}\u{3120}\u{3109}\u{311e}".to_string()
        )),
        "cross-space phrase key tps:ㄍㄠㄉㄞ@(0,13) missing: {:?}",
        mapped(&keys),
    );
    // No interior keys leaked.
    for ((start, _), key) in &keys {
        assert_eq!(*start, 0, "interior key {key:?} leaked");
    }
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
    // Tone-1 chain — the inv-driven BFS accepts each `ㄉㄞ` as a single
    // inventory hit, one hop per syllable.
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

// ---- INVARIANT_TPS_DEFOLD_ENUMERATE (§35) alternate readings ---------
//
// The per-keystroke auto-correct (§32/§33) picks one of two locally
// indistinguishable glyph readings; when it picks the wrong one the word is
// structurally hidden from the segmenter. These pin the recovery paths and,
// just as importantly, the readings that must NOT be rewritten.
//
// Fixture rule (`.claude/rules/taigi-incidents.md` § Trace before assert):
// every inventory below carries the BASE reading's syllables as well as the
// alternate's, so the alternate is proven to be an ADDITION rather than a
// silent replacement.

// 中文: §35 替代讀法 — auto-correct 選錯局部無法分辨的 glyph 讀法時,詞會被結構性藏起來。
// 中文:   以下測試釘住復原路徑,以及「絕對不可改寫」的讀法;每個 inventory 都同時含 base 音節。

fn key_texts(keys: &[((u32, u32), String)]) -> Vec<String> {
    keys.iter().map(|(_, k)| k.clone()).collect()
}

#[test]
fn alternate_reading_surfaces_khokng_as_kho_kng() {
    // 考卷 khó-kǹg typed toneless: ㄎ ㄛ ㄍ ㄫ. The ㄍ folds to the coda ㆻ
    // (khok is a real syllable so the §32 gate correctly keeps the fold) and
    // the ㄫ stays an onset form, giving `ㄎㄛㆻㄫ` — a reading the inventory
    // cannot segment. Part B terminalizes the nasal (`ㄎㄛㆻㆭ`) and part C
    // de-folds the coda in front of it (`ㄎㄛㄍㆭ`), which segments as kho|kng.
    // `ㄎㄛㆻ` (khok) is in the inventory so the base reading keeps its own
    // single-syllable key — the alternate does not displace it.
    let inv = build_tps_inventory(&["ㄎㄛ", "ㄎㄛㆻ", "ㄍㆭ"]);
    let keys = build_continuous_keys_with_inventory("ㄎㄛㆻㄫ", &inv, phonetics::InputMode::Tps);
    let texts = key_texts(&keys);
    assert!(
        texts.contains(&"tps:ㄎㄛㄍㆭ".to_string()),
        "expected the kho|kng alternate span key, got {texts:?}",
    );
    assert!(
        texts.contains(&"tps:ㄎㄛㆻ".to_string()),
        "base reading's khok key must survive, got {texts:?}",
    );
}

#[test]
fn alternate_reading_keys_come_after_the_base_reading_keys() {
    // Append order is load-bearing: `dedupe_by_roman_hanji_span` keeps the
    // EARLIER index on a source-rank tie, so a genuine collision must resolve
    // to the natural reading. Pinning the order, not just presence.
    let inv = build_tps_inventory(&["ㄎㄛ", "ㄎㄛㆻ", "ㄍㆭ"]);
    let keys = build_continuous_keys_with_inventory("ㄎㄛㆻㄫ", &inv, phonetics::InputMode::Tps);
    let texts = key_texts(&keys);
    let base = texts
        .iter()
        .position(|k| k == "tps:ㄎㄛㆻ")
        .expect("base khok key");
    let alternate = texts
        .iter()
        .position(|k| k == "tps:ㄎㄛㄍㆭ")
        .expect("alternate kho|kng key");
    assert!(
        base < alternate,
        "base keys must precede alternate keys, got {texts:?}",
    );
}

#[test]
fn alternate_reading_surfaces_msi_as_m_si() {
    // 毋是 m̄-sī typed toneless: ㄇ ㄒ ㄧ. `ㄇㄒ` is not a syllable, so the ㄇ
    // cannot be an onset; part B reads it as the syllabic ㆬ.
    let inv = build_tps_inventory(&["ㆬ", "ㄒㄧ"]);
    let keys = build_continuous_keys_with_inventory("ㄇㄒㄧ", &inv, phonetics::InputMode::Tps);
    let texts = key_texts(&keys);
    assert!(
        texts.contains(&"tps:ㆬㄒㄧ".to_string()),
        "expected the m|si alternate span key, got {texts:?}",
    );
}

#[test]
fn alternate_reading_leaves_a_nasal_that_opens_a_syllable_alone() {
    // 博雅 phok-ngá `ㄆㆦㆻㄫㄚ` — here the ㆻ really IS a coda and the ㄫ really
    // IS an onset (nga). Rewriting either would destroy the word.
    let inv = build_tps_inventory(&["ㄆㆦㆻ", "ㄫㄚ"]);
    let keys = build_continuous_keys_with_inventory("ㄆㆦㆻㄫㄚ", &inv, phonetics::InputMode::Tps);
    assert!(
        key_texts(&keys).contains(&"tps:ㄆㆦㆻㄫㄚ".to_string()),
        "phok|nga must still segment, got {keys:?}",
    );

    // 門 mn̂g `ㄇㆭ` — onset + syllabic-ng nucleus. `ㆭ` is not vowel material,
    // so a positional eligibility test would rewrite the ㄇ and lose the word.
    let inv = build_tps_inventory(&["ㄇㆭ"]);
    let keys = build_continuous_keys_with_inventory("ㄇㆭ", &inv, phonetics::InputMode::Tps);
    assert!(
        key_texts(&keys).contains(&"tps:ㄇㆭ".to_string()),
        "mng must still segment, got {keys:?}",
    );
}

#[test]
fn alternate_reading_does_not_cross_a_user_separator() {
    // `ㄎㄛㆻ`␣`ㄫ` — the space is the user's explicit syllable boundary (§31),
    // so the ㆻ closes the FIRST syllable and part C must not reach across it
    // to reconstruct 考卷. Same inventory as the no-space case, which is what
    // makes the absence meaningful.
    let inv = build_tps_inventory(&["ㄎㄛ", "ㄎㄛㆻ", "ㄍㆭ"]);
    let keys = build_continuous_keys_with_inventory("ㄎㄛㆻ ㄫ", &inv, phonetics::InputMode::Tps);
    let texts = key_texts(&keys);
    assert!(
        !texts.contains(&"tps:ㄎㄛㄍㆭ".to_string()),
        "must not de-fold across the user's separator, got {texts:?}",
    );
}

#[test]
fn alternate_reading_skips_a_bare_nasal_so_partial_prefix_still_runs() {
    // A buffer that is nothing but ㄇ / ㄫ must produce NO span key, so
    // `continuous.rs` keeps falling through to the partial-prefix branch that
    // lists every m- / ng- initial word. Terminalizing it would emit a
    // full-span key, make `keys.is_empty()` false, and drop that branch.
    let inv = build_tps_inventory(&["ㆬ", "ㆭ", "ㄇㆭ"]);
    for bare in ["ㄇ", "ㄫ"] {
        let keys = build_continuous_keys_with_inventory(bare, &inv, phonetics::InputMode::Tps);
        assert!(
            keys.is_empty(),
            "bare {bare} must emit no span key so partial-prefix runs, got {keys:?}",
        );
    }
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
