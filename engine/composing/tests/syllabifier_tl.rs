//! v3.5.8 Phase 3 — TL syllabifier matrix (per `docs/roadmap.md` lines
//! 222-228).
//!
//! Tests use a hermetic `SyllableInventory` built per-case so that the
//! roadmap's pedagogical examples (`tai → {3}`, `taibak → {3, 6}`,
//! `khihthau → {4, 8}`) hold without depending on the production
//! `dictionary/output/syllables.fst` (which contains additional
//! syllables like `ta`, `tha`, `ba` that would expand the result set).
//! The hermetic builder mirrors `engine/lexicon/tests/syllables_fst.rs`
//! at line 186-207 — small duplication is preferable to a shared
//! test-utils crate for one reuse.
//!
//! Reuses production `phonetics::canonicalize_syllable` so that test
//! samples can be authored in either TL or POJ shape.

// 中文: Phase 3 TL syllabifier 測試 — 用 hermetic SyllableInventory 配合 roadmap 教學例子。
// 中文: Builder pattern 1:1 鏡射 lexicon::syllables_fst tests,以避免引入共用 test-utils crate。

use std::path::PathBuf;

use composing::syllabifier::tl::valid_span_endings;
use fst::SetBuilder;
use lexicon::SyllableInventory;
use phonetics::canonicalize_syllable;

const MAX_SYLLABLES: usize = 8;

#[test]
fn tsua_yields_3_and_4_key_multi_cut_case() {
    // tsu (3, 珠), tsua (4, 紙), and chain tsu+a (4) — all valid.
    // Matches `docs/roadmap.md` line 223 — the 關鍵 case.
    let inv = build_inventory(&["tsu1", "tsua7", "a2"]);
    assert_eq!(
        valid_span_endings("tsua", 0, &inv, MAX_SYLLABLES),
        vec![3, 4]
    );
}

#[test]
fn tai_yields_3_only_when_ta_absent() {
    // Hermetic inv contains `tai` but NOT `ta`; matches roadmap line 224.
    let inv = build_inventory(&["tai5"]);
    assert_eq!(valid_span_endings("tai", 0, &inv, MAX_SYLLABLES), vec![3]);
}

#[test]
fn taibak_yields_3_and_6_when_partials_absent() {
    // Hermetic inv contains tai + bak only; `ta`, `ba`, `taiba` etc.
    // are absent, so endings collapse to roadmap line 225's {3, 6}.
    let inv = build_inventory(&["tai5", "bak4"]);
    assert_eq!(
        valid_span_endings("taibak", 0, &inv, MAX_SYLLABLES),
        vec![3, 6]
    );
}

#[test]
fn khihthau_yields_4_and_8() {
    // Hermetic inv contains khih + thau only; `khi`, `tha`, `hu` etc.
    // are absent. Matches roadmap line 226.
    let inv = build_inventory(&["khih4", "thau5"]);
    assert_eq!(
        valid_span_endings("khihthau", 0, &inv, MAX_SYLLABLES),
        vec![4, 8]
    );
}

#[test]
fn taixyz_stops_at_3_when_tail_unparsable() {
    // Roadmap line 227 — `xyz` doesn't canonicalize to any syllable.
    let inv = build_inventory(&["tai1"]);
    assert_eq!(
        valid_span_endings("taixyz", 0, &inv, MAX_SYLLABLES),
        vec![3]
    );
}

#[test]
fn taigikhipuann_yields_running_totals() {
    // Roadmap line 228 — endings = running sums of 4-syllable greedy chain.
    let inv = build_inventory(&["tai5", "gi5", "khi3", "puann1"]);
    assert_eq!(
        valid_span_endings("taigikhipuann", 0, &inv, MAX_SYLLABLES),
        vec![3, 5, 8, 13]
    );
}

#[test]
fn pos_at_input_end_returns_empty() {
    let inv = build_inventory(&["tai1"]);
    assert!(valid_span_endings("tai", 3, &inv, MAX_SYLLABLES).is_empty());
}

#[test]
fn pos_beyond_input_end_returns_empty() {
    let inv = build_inventory(&["tai1"]);
    assert!(valid_span_endings("tai", 99, &inv, MAX_SYLLABLES).is_empty());
}

#[test]
fn max_syllables_zero_returns_empty() {
    // Cap of 0 means "no syllables allowed" — vacuously empty.
    let inv = build_inventory(&["tai1"]);
    assert!(valid_span_endings("tai", 0, &inv, 0).is_empty());
}

#[test]
fn max_syllables_one_excludes_chain_endings() {
    // tsua: depth 1 = {3 (tsu), 4 (tsua)} — chain tsu+a (depth 2) is capped out.
    let inv = build_inventory(&["tsu1", "tsua7", "a2"]);
    assert_eq!(valid_span_endings("tsua", 0, &inv, 1), vec![3, 4]);
}

#[test]
fn max_syllables_two_includes_chain_endings() {
    // taibak depth 2 — chain tai+bak reaches 6.
    let inv = build_inventory(&["tai5", "bak4"]);
    assert_eq!(valid_span_endings("taibak", 0, &inv, 2), vec![3, 6]);
}

#[test]
fn pos_offset_into_input_walks_only_remaining_suffix() {
    // From pos=3 of "taibak" (after `tai`), syllabifier sees only `bak`.
    let inv = build_inventory(&["tai5", "bak4"]);
    assert_eq!(
        valid_span_endings("taibak", 3, &inv, MAX_SYLLABLES),
        vec![6]
    );
}

#[test]
fn syllabic_consonant_m_and_ng_recognized() {
    // Single-letter syllables m/ng are valid TL syllables; FST has both.
    let inv = build_inventory(&["m7", "ng5"]);
    assert_eq!(valid_span_endings("m", 0, &inv, MAX_SYLLABLES), vec![1]);
    assert_eq!(valid_span_endings("ng", 0, &inv, MAX_SYLLABLES), vec![2]);
}

#[test]
fn empty_input_returns_empty() {
    let inv = build_inventory(&["tai1"]);
    assert!(valid_span_endings("", 0, &inv, MAX_SYLLABLES).is_empty());
}

#[test]
fn numeric_tone_input_suppresses_orphan_toneless_boundary() {
    // FST holds both `tai` (toneless) and `tai5` (numeric). Without
    // the suppression rule, `tai5` would yield {3, 4} and leave digit
    // `5` orphaned. With it, the toneless match at end=3 is suppressed
    // because input[3]='5' is a tone digit; only end=4 survives.
    let inv = build_inventory(&["tai5"]);
    assert_eq!(valid_span_endings("tai5", 0, &inv, MAX_SYLLABLES), vec![4]);
}

#[test]
fn numeric_tone_chain_yields_only_real_boundaries() {
    // `tai5gi2` chain: end=4 (tai5) and end=7 (gi2). Toneless ends 3 + 6
    // both suppressed by the trailing tone digit.
    let inv = build_inventory(&["tai5", "gi2"]);
    assert_eq!(
        valid_span_endings("tai5gi2", 0, &inv, MAX_SYLLABLES),
        vec![4, 7]
    );
}

#[test]
fn entering_coda_tone4_input_suppresses_toneless_boundary() {
    // `kak4` — toneless `kak` + numeric `kak4`. Without suppression, BFS
    // would yield {3, 4}; with it, only end=4 survives.
    let inv = build_inventory(&["kak4"]);
    assert_eq!(valid_span_endings("kak4", 0, &inv, MAX_SYLLABLES), vec![4]);
}

#[test]
fn toneless_input_unaffected_by_suppression_rule() {
    // No tone digit in input — toneless endings remain.
    let inv = build_inventory(&["tai5"]);
    assert_eq!(valid_span_endings("tai", 0, &inv, MAX_SYLLABLES), vec![3]);
}

#[test]
fn pos_at_non_char_boundary_returns_empty_safely() {
    // Multi-byte UTF-8 input — pos lands inside a codepoint. Must not panic.
    let inv = build_inventory(&["a1"]);
    let input = "\u{4e2d}a"; // 中a — `中` is 3 bytes (E4 B8 AD).
    assert!(valid_span_endings(input, 1, &inv, MAX_SYLLABLES).is_empty());
}

#[test]
fn non_ascii_input_yields_no_endings_without_panic() {
    // Pure non-ASCII input never matches the ASCII-only inventory.
    let inv = build_inventory(&["a1"]);
    assert!(valid_span_endings("\u{4e2d}", 0, &inv, MAX_SYLLABLES).is_empty());
}

#[test]
fn longer_syllable_dominates_shorter_chain_endings_dedupe() {
    // tsua (1 syl, end 4) + tsu+a (2 syl, end 4) → endings dedupe to {3, 4}.
    let inv = build_inventory(&["tsu1", "tsua7", "a2"]);
    let endings = valid_span_endings("tsua", 0, &inv, MAX_SYLLABLES);
    assert_eq!(endings, vec![3, 4], "endings should dedupe across depths");
}

// ---- Hermetic SyllableInventory builder -----------------------------

/// Build a `SyllableInventory` from raw POJ/TL-shaped sample tokens.
/// Pipeline: `phonetics::canonicalize_syllable` → emit numeric +
/// toneless keys → fst::SetBuilder → temp file → `SyllableInventory::open`.
/// Mirrors `engine/lexicon/tests/syllables_fst.rs:186-207`.
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
    std::env::temp_dir().join(format!("composing-syllabifier-tl-{pid}-{n}.fst"))
}
