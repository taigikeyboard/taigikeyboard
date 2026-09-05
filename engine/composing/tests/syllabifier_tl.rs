//! v3.5.8 Phase 3 — TL syllabifier matrix (per `docs/releases/v3.5.8/plan.md`
//! § Phase 3 — Test 矩陣).
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

use std::path::PathBuf;

use composing::syllabifier::tl::valid_span_endings;
use fst::SetBuilder;
use lexicon::SyllableInventory;
use phonetics::{canonicalize_poj_syllable, canonicalize_syllable, InputMode};

const MAX_SYLLABLES: usize = 8;
// All tests in this matrix exercise the TL family; the `mode` parameter
// arrived with v3.5.9 B-1's mode-aware syllabifier surface.
const MODE: InputMode = InputMode::Tl;

#[test]
fn tsua_yields_3_and_4_key_multi_cut_case() {
    // tsu (3, 珠), tsua (4, 紙), and chain tsu+a (4) — all valid.
    // Matches `docs/releases/v3.5.8/plan.md` § Phase 3 — Test 矩陣 `tsua` → `{3, 4}` — the 關鍵 case.
    let inv = build_inventory(&["tsu1", "tsua7", "a2"]);
    assert_eq!(
        valid_span_endings("tsua", 0, &inv, MODE, MAX_SYLLABLES),
        vec![3, 4]
    );
}

#[test]
fn tai_yields_3_only_when_ta_absent() {
    // Hermetic inv contains `tai` but NOT `ta`; matches roadmap line 224.
    let inv = build_inventory(&["tai5"]);
    assert_eq!(
        valid_span_endings("tai", 0, &inv, MODE, MAX_SYLLABLES),
        vec![3]
    );
}

#[test]
fn taibak_yields_3_and_6_when_partials_absent() {
    // Hermetic inv contains tai + bak only; `ta`, `ba`, `taiba` etc.
    // are absent, so endings collapse to roadmap line 225's {3, 6}.
    let inv = build_inventory(&["tai5", "bak4"]);
    assert_eq!(
        valid_span_endings("taibak", 0, &inv, MODE, MAX_SYLLABLES),
        vec![3, 6]
    );
}

#[test]
fn khihthau_yields_4_and_8() {
    // Hermetic inv contains khih + thau only; `khi`, `tha`, `hu` etc.
    // are absent. Matches roadmap line 226.
    let inv = build_inventory(&["khih4", "thau5"]);
    assert_eq!(
        valid_span_endings("khihthau", 0, &inv, MODE, MAX_SYLLABLES),
        vec![4, 8]
    );
}

#[test]
fn taixyz_stops_at_3_when_tail_unparsable() {
    // Roadmap line 227 — `xyz` doesn't canonicalize to any syllable.
    let inv = build_inventory(&["tai1"]);
    assert_eq!(
        valid_span_endings("taixyz", 0, &inv, MODE, MAX_SYLLABLES),
        vec![3]
    );
}

#[test]
fn taigikhipuann_yields_running_totals() {
    // Roadmap line 228 — endings = running sums of 4-syllable greedy chain.
    let inv = build_inventory(&["tai5", "gi5", "khi3", "puann1"]);
    assert_eq!(
        valid_span_endings("taigikhipuann", 0, &inv, MODE, MAX_SYLLABLES),
        vec![3, 5, 8, 13]
    );
}

#[test]
fn pos_at_input_end_returns_empty() {
    let inv = build_inventory(&["tai1"]);
    assert!(valid_span_endings("tai", 3, &inv, MODE, MAX_SYLLABLES).is_empty());
}

#[test]
fn pos_beyond_input_end_returns_empty() {
    let inv = build_inventory(&["tai1"]);
    assert!(valid_span_endings("tai", 99, &inv, MODE, MAX_SYLLABLES).is_empty());
}

#[test]
fn max_syllables_zero_returns_empty() {
    // Cap of 0 means "no syllables allowed" — vacuously empty.
    let inv = build_inventory(&["tai1"]);
    assert!(valid_span_endings("tai", 0, &inv, MODE, 0).is_empty());
}

#[test]
fn max_syllables_one_excludes_chain_endings() {
    // tsua: depth 1 = {3 (tsu), 4 (tsua)} — chain tsu+a (depth 2) is capped out.
    let inv = build_inventory(&["tsu1", "tsua7", "a2"]);
    assert_eq!(valid_span_endings("tsua", 0, &inv, MODE, 1), vec![3, 4]);
}

#[test]
fn max_syllables_two_includes_chain_endings() {
    // taibak depth 2 — chain tai+bak reaches 6.
    let inv = build_inventory(&["tai5", "bak4"]);
    assert_eq!(valid_span_endings("taibak", 0, &inv, MODE, 2), vec![3, 6]);
}

#[test]
fn pos_offset_into_input_walks_only_remaining_suffix() {
    // From pos=3 of "taibak" (after `tai`), syllabifier sees only `bak`.
    let inv = build_inventory(&["tai5", "bak4"]);
    assert_eq!(
        valid_span_endings("taibak", 3, &inv, MODE, MAX_SYLLABLES),
        vec![6]
    );
}

#[test]
fn syllabic_consonant_m_and_ng_recognized() {
    // Single-letter syllables m/ng are valid TL syllables; FST has both.
    let inv = build_inventory(&["m7", "ng5"]);
    assert_eq!(
        valid_span_endings("m", 0, &inv, MODE, MAX_SYLLABLES),
        vec![1]
    );
    assert_eq!(
        valid_span_endings("ng", 0, &inv, MODE, MAX_SYLLABLES),
        vec![2]
    );
}

#[test]
fn empty_input_returns_empty() {
    let inv = build_inventory(&["tai1"]);
    assert!(valid_span_endings("", 0, &inv, MODE, MAX_SYLLABLES).is_empty());
}

#[test]
fn numeric_tone_input_suppresses_orphan_toneless_boundary() {
    // FST holds both `tai` (toneless) and `tai5` (numeric). Without
    // the suppression rule, `tai5` would yield {3, 4} and leave digit
    // `5` orphaned. With it, the toneless match at end=3 is suppressed
    // because input[3]='5' is a tone digit; only end=4 survives.
    let inv = build_inventory(&["tai5"]);
    assert_eq!(
        valid_span_endings("tai5", 0, &inv, MODE, MAX_SYLLABLES),
        vec![4]
    );
}

#[test]
fn numeric_tone_chain_yields_only_real_boundaries() {
    // `tai5gi2` chain: end=4 (tai5) and end=7 (gi2). Toneless ends 3 + 6
    // both suppressed by the trailing tone digit.
    let inv = build_inventory(&["tai5", "gi2"]);
    assert_eq!(
        valid_span_endings("tai5gi2", 0, &inv, MODE, MAX_SYLLABLES),
        vec![4, 7]
    );
}

#[test]
fn entering_coda_tone4_input_suppresses_toneless_boundary() {
    // `kak4` — toneless `kak` + numeric `kak4`. Without suppression, BFS
    // would yield {3, 4}; with it, only end=4 survives.
    let inv = build_inventory(&["kak4"]);
    assert_eq!(
        valid_span_endings("kak4", 0, &inv, MODE, MAX_SYLLABLES),
        vec![4]
    );
}

#[test]
fn toneless_input_unaffected_by_suppression_rule() {
    // No tone digit in input — toneless endings remain.
    let inv = build_inventory(&["tai5"]);
    assert_eq!(
        valid_span_endings("tai", 0, &inv, MODE, MAX_SYLLABLES),
        vec![3]
    );
}

#[test]
fn pos_at_non_char_boundary_returns_empty_safely() {
    // Multi-byte UTF-8 input — pos lands inside a codepoint. Must not panic.
    let inv = build_inventory(&["a1"]);
    let input = "\u{4e2d}a"; // 中a — `中` is 3 bytes (E4 B8 AD).
    assert!(valid_span_endings(input, 1, &inv, MODE, MAX_SYLLABLES).is_empty());
}

#[test]
fn non_ascii_input_yields_no_endings_without_panic() {
    // Pure non-ASCII input never matches the ASCII-only inventory.
    let inv = build_inventory(&["a1"]);
    assert!(valid_span_endings("\u{4e2d}", 0, &inv, MODE, MAX_SYLLABLES).is_empty());
}

#[test]
fn longer_syllable_dominates_shorter_chain_endings_dedupe() {
    // tsua (1 syl, end 4) + tsu+a (2 syl, end 4) → endings dedupe to {3, 4}.
    let inv = build_inventory(&["tsu1", "tsua7", "a2"]);
    let endings = valid_span_endings("tsua", 0, &inv, MODE, MAX_SYLLABLES);
    assert_eq!(endings, vec![3, 4], "endings should dedupe across depths");
}

// ---- v3.5.9 B-1 — POJ-mode syllabifier hermetic dual-inventory matrix.
//      Pins that `valid_span_endings(..., InputMode::Poj, ...)` routes
//      to the `poj:` family of the tagged-single-FST and recognises
//      POJ-shaped syllables (`chiah`, `chhia`, `goa`, `koe`, `peng`,
//      `pek`) without conflating them with their TL folds. Codex
//      pre-impl B-1 SHOULD, 2026-05-20.

#[test]
fn poj_mode_recognises_divergent_initial_chiah() {
    // `chiah` is a single valid POJ syllable; the TL fold is `tsiah`.
    // The POJ family must answer end=5 for "chiah"; the TL family
    // must NOT (the inventory builder canonicalizes via TL, so `chiah`
    // never appears as a tl: key).
    let inv = build_dual_inventory(&["tsiah4"], &["chiah4"]);
    assert_eq!(
        valid_span_endings("chiah", 0, &inv, InputMode::Poj, MAX_SYLLABLES),
        vec![5],
        "poj-mode must accept POJ-shape `chiah`"
    );
    assert!(
        valid_span_endings("chiah", 0, &inv, InputMode::Tl, MAX_SYLLABLES).is_empty(),
        "tl-mode must NOT accept POJ-shape `chiah`"
    );
    // Mirror: tl-mode accepts `tsiah`.
    assert_eq!(
        valid_span_endings("tsiah", 0, &inv, InputMode::Tl, MAX_SYLLABLES),
        vec![5],
    );
}

#[test]
fn poj_mode_aspirated_initial_chh_distinct_from_tsh() {
    // `chhia` (POJ) vs `tshia` (TL) — `chh→tsh` is in NORMALIZE_TO_TL_RULES,
    // so the divergent ASCII prefix is the discriminator the POJ family
    // must preserve.
    let inv = build_dual_inventory(&["tshia1"], &["chhia1"]);
    assert_eq!(
        valid_span_endings("chhia", 0, &inv, InputMode::Poj, MAX_SYLLABLES),
        vec![5],
    );
    assert_eq!(
        valid_span_endings("tshia", 0, &inv, InputMode::Tl, MAX_SYLLABLES),
        vec![5],
    );
    assert!(valid_span_endings("chhia", 0, &inv, InputMode::Tl, MAX_SYLLABLES).is_empty());
    assert!(valid_span_endings("tshia", 0, &inv, InputMode::Poj, MAX_SYLLABLES).is_empty());
}

#[test]
fn poj_mode_chains_divergent_finals_in_phrase() {
    // POJ `goa-koe` (`goa` + `koe`) vs TL `gua-kue`. End offsets must
    // be `{3, 6}` in POJ mode (chain depth 2) and only the matching
    // family resolves.
    let inv = build_dual_inventory(&["gua1", "kue1"], &["goa1", "koe1"]);
    assert_eq!(
        valid_span_endings("goakoe", 0, &inv, InputMode::Poj, MAX_SYLLABLES),
        vec![3, 6],
    );
    assert_eq!(
        valid_span_endings("guakue", 0, &inv, InputMode::Tl, MAX_SYLLABLES),
        vec![3, 6],
    );
    // Cross-family inputs collapse to empty.
    assert!(valid_span_endings("goakoe", 0, &inv, InputMode::Tl, MAX_SYLLABLES).is_empty());
    assert!(valid_span_endings("guakue", 0, &inv, InputMode::Poj, MAX_SYLLABLES).is_empty());
}

#[test]
fn poj_mode_nasal_oo_fold_preserved_as_ascii_poj() {
    // POJ non-ASCII `peⁿ` / `so͘` canonicalize to ASCII `penn` / `soo`
    // (encoding-only rule) and the POJ family stores `penn` / `soo` —
    // distinct from any TL phonotactic re-spelling like `oonn → onn`.
    let inv = build_dual_inventory(&["penn5", "soo3"], &["penn5", "soo3"]);
    // The post-NORMALIZE_TO_POJ form `penn` IS a valid TL syllable too
    // (both families list it for this row pair), so both modes match
    // independently — proving the lookups are mode-routed.
    assert_eq!(
        valid_span_endings("penn", 0, &inv, InputMode::Poj, MAX_SYLLABLES),
        vec![4]
    );
    assert_eq!(
        valid_span_endings("penn", 0, &inv, InputMode::Tl, MAX_SYLLABLES),
        vec![4]
    );
    assert_eq!(
        valid_span_endings("soo", 0, &inv, InputMode::Poj, MAX_SYLLABLES),
        vec![3]
    );
}

#[test]
fn poj_mode_english_routes_to_tl_family() {
    // InputMode::English shares the TL family per
    // SyllableInventory::contains_in routing (English buffers do not
    // need their own syllable inventory).
    let inv = build_dual_inventory(&["tai5"], &["chiah4"]);
    assert_eq!(
        valid_span_endings("tai", 0, &inv, InputMode::English, MAX_SYLLABLES),
        vec![3]
    );
    assert!(valid_span_endings("chiah", 0, &inv, InputMode::English, MAX_SYLLABLES).is_empty());
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

    // v3.5.9 B-1: emit keys with the `tl:` family prefix so the
    // hermetic inventory matches the tagged-single-FST format that
    // `SyllableInventory::contains_in(InputMode::Tl, …)` queries.
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

/// v3.5.9 B-1 — build a hermetic tagged-single-FST inventory carrying
/// BOTH the `tl:` and `poj:` families. TL samples canonicalize via
/// `canonicalize_syllable` (POJ→TL fold + nasal/o-dot ASCII fold); POJ
/// samples canonicalize via `canonicalize_poj_syllable` (encoding-only
/// fold; POJ ASCII spelling preserved). Mirrors the production
/// `fst-builder build-syllables --tl-input --poj-input` pipeline.
fn build_dual_inventory(tl_samples: &[&str], poj_samples: &[&str]) -> SyllableInventory {
    let mut keys: Vec<String> = Vec::new();
    for s in tl_samples {
        let (canonical, tone) = canonicalize_syllable(s)
            .unwrap_or_else(|| panic!("tl sample {s:?} failed canonicalize_syllable"));
        if tone.is_empty() {
            keys.push(format!("tl:{canonical}"));
        } else {
            keys.push(format!("tl:{canonical}{tone}"));
            keys.push(format!("tl:{canonical}"));
        }
    }
    for s in poj_samples {
        let (canonical, tone) = canonicalize_poj_syllable(s)
            .unwrap_or_else(|| panic!("poj sample {s:?} failed canonicalize_poj_syllable"));
        if tone.is_empty() {
            keys.push(format!("poj:{canonical}"));
        } else {
            keys.push(format!("poj:{canonical}{tone}"));
            keys.push(format!("poj:{canonical}"));
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
