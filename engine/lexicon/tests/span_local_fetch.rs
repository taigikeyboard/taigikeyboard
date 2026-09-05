//! v3.5.8 連續輸入 (Continuous Input) Phase 5 — span-local candidate fetch
//! contract test.
//!
//! Pins the three roadmap-mandated cases (`docs/releases/v3.5.8/plan.md` § Phase 5 — span-local candidate fetch / 走查範例):
//!
//! 1. `tsua` with endings={3, 4} must surface
//!    `紙(span=(0,4), syll=1)` + `珠仔(span=(0,4), syll=2)` + `珠(span=(0,3), syll=1)`
//!    — the "multi-syll same toneless key" + "shorter-span shadow" combo
//!    that motivates the whole BFS-with-multi-cut approach
//!    (`docs/releases/v3.5.8/plan.md` § Phase 3 — 純函數 syllabifier (TL + TPS) — design rationale).
//!
//! 2. `taigikhipuann` with endings={2, 4, 8, 13} must surface candidates
//!    for `台(syll=1)`, `台語(syll=2)`, and `台語齒盤(syll=4)` — the
//!    long-reach case that confirms the loop fans across distant endings,
//!    not just neighbouring ones.
//!
//! 3. `taixyz` with endings={3} only (the syllabifier can't proceed past
//!    `tai` because `xyz` isn't a valid syllable) must yield only
//!    single-syllable `tai` candidates with span=(0,3) and never any
//!    span ≥ 4 — proves the function trusts caller-supplied endings.
//!
//! Hermetic: builds a tiny `PrefixIndex` (3-row fst with `tl:` keys
//! pointing to a tiny `dictionary.bin` v2) per test so we never touch
//! the real packaged dictionary. FST builder pattern mirrors
//! `tests/syllables_fst.rs:186-207`; dict.bin v2 builder is shared
//! `tests/common/mod.rs::build_tkdb_v3`.

// Phase 5 fetch_candidates_for_endings 契約測試 — 鎖 roadmap §Phase 5 三條 case (tsua / taigikhipuann / taixyz)。

use std::path::PathBuf;

use fst::SetBuilder;
use lexicon::dictionary_reader::DictionaryReader;
use lexicon::prefix_index::PrefixIndex;
use lexicon::{
    best_candidate_for_key, fetch_candidates_for_endings, fetch_candidates_for_keys,
    fetch_partial_prefix_candidates, fetch_partial_prefix_candidates_unbounded, CandidateMode,
    ConsumedSpan, ContinuousFetchCtx, CustomEntry, RawCandidate, COVERAGE_KIND_FULL,
    COVERAGE_KIND_PARTIAL_PREFIX, FORM_NOTONE, PARTIAL_PREFIX_HYDRATE_CAP,
    PARTIAL_PREFIX_OUTPUT_CAP,
};
use phonetics::InputMode;
use ranking::FrequencyMap;

/// v3.5.9 D7 — build the shared `ContinuousFetchCtx` at a test site
/// with explicit `freq_map` / `now_ms` / `custom`. Pins
/// `enabled_sources_bitmask = u32::MAX` (matches every test in this
/// file — filter-narrowing cases live alongside their own ctx
/// construction). Use [`ctx_neutral`] for the cold-start
/// no-custom shorthand most cases need.
// D7 — 把 ContinuousFetchCtx 收進 helper,call site 由 8 個位置參數縮到 3 個 + 1 個 ctx 引用。
fn ctx<'a>(
    freq_map: &'a FrequencyMap,
    now_ms: i64,
    custom: &'a [CustomEntry],
    prefix_index: &'a PrefixIndex,
    dict: &'a DictionaryReader,
) -> ContinuousFetchCtx<'a> {
    // v3.5.9 B-4 — `mode` default is `Tl`; this test file pins TL
    // fixtures (every key starts with `tl:` in this suite). POJ-mode
    // canonicalization behavior is exercised by the inline tests in
    // `engine/lexicon/src/continuous.rs::item12_custom_dedupe_tests`
    // and the cross-mode parity tests in
    // `engine/phonetics/tests/canonical_tl_form.rs`.
    // B-4 — 此 test 套件全 TL fixture,mode 預設 Tl;POJ 行為由
    //   item12_custom_dedupe_tests + canonical_tl_form 跨 mode parity 測試覆蓋。
    ContinuousFetchCtx {
        enabled_sources_bitmask: u32::MAX,
        freq_map,
        now_ms,
        custom,
        prefix_index,
        dict,
        mode: phonetics::InputMode::Tl,
        tps_space_pinned_body: None,
    }
}

/// Cold-start shorthand: empty `freq_map`, `now_ms = 0`, `custom = &[]`.
/// Most Phase 5/9.1 regression cases need exactly this — the
/// frequency / recency / custom axes are pinned by dedicated tests.
// cold-start 簡寫 — 空 freq_map / now_ms=0 / 無 custom;絕大多數回歸測試用這個。
fn ctx_neutral<'a>(
    freq_map: &'a FrequencyMap,
    prefix_index: &'a PrefixIndex,
    dict: &'a DictionaryReader,
) -> ContinuousFetchCtx<'a> {
    ctx(freq_map, 0, &[], prefix_index, dict)
}

mod common;
use common::{build_tkdb_v3, write_temp};

/// Single dictionary fixture row: `(toneless_tl_key, hanzi, tl, syllable_count, frequency)`.
/// `bitmask` is fixed to `1 << 11` (the `lkk` source per
/// `dictionary/common/source_bits.py:35`); the per-source mask check
/// is short-circuited at `u32::MAX` filter input below, so any set
/// bit suffices to pass the default-enabled filter. Picking `lkk`
/// keeps the fixture outside of the Phase 9.1 `CONTINUOUS_SOURCE_BITS`
/// rank table so the resulting candidates always land at the
/// default source rank (5) and do not perturb tie-break ordering
/// tests that exercise other dimensions.
/// Rowid is implied by insertion order (1-based for dict.bin,
/// mirrored as 1-based in the FST).
struct Row<'a> {
    toneless_key: &'a str,
    hanzi: &'a str,
    tl: &'a str,
    syll: u8,
    freq: u32,
}

/// Build a `(PrefixIndex, DictionaryReader)` pair backed by tmpfiles.
/// Each row gets its 1-based rowid from the slice index. The FST is
/// keyed on `tl:<toneless_key> + 0xFF + rowid_le_4` and stays sorted by
/// inserting rows in ascending key order (caller-controlled).
fn build_fixture(name: &str, rows: &[Row<'_>]) -> (PrefixIndex, DictionaryReader) {
    // 1. dict.bin v2.
    let dict_rows: Vec<(u16, u32, u8, &str, &str)> = rows
        .iter()
        .map(|r| (1u16 << 11, r.freq, r.syll, r.hanzi, r.tl))
        .collect();
    let dict_bytes = build_tkdb_v3(b"TKDB", &dict_rows);
    let dict_path = write_temp(&format!("phase5-{name}.dict.bin"), &dict_bytes);
    let dict = DictionaryReader::open(&dict_path).expect("dict.bin opens");

    // 2. dictionary.fst — entries must be inserted in ascending byte
    // order. Sort by `tl:<key> + 0xFF + rowid` before insertion.
    let mut fst_keys: Vec<Vec<u8>> = Vec::new();
    for (idx, r) in rows.iter().enumerate() {
        let rowid = (idx + 1) as u32;
        let mut entry = Vec::with_capacity(r.toneless_key.len() + 4 + 5);
        entry.extend_from_slice(b"tl:");
        entry.extend_from_slice(r.toneless_key.as_bytes());
        entry.push(0xFF);
        entry.extend_from_slice(&rowid.to_le_bytes());
        fst_keys.push(entry);
    }
    fst_keys.sort();

    let fst_path = unique_temp_path(name);
    let file = std::fs::File::create(&fst_path).expect("create fst tmp");
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("fst builder");
    for entry in &fst_keys {
        builder.insert(entry).expect("fst insert");
    }
    builder.finish().expect("fst finish");
    let prefix_index = PrefixIndex::open(&fst_path).expect("dictionary.fst opens");

    (prefix_index, dict)
}

fn unique_temp_path(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("lexicon-test-phase5-{name}-{pid}-{n}.fst"))
}

/// Locate a candidate by `(display_text, consumed_span, syllable_count)`.
/// Asserts presence; returns the matched candidate so the caller can
/// inspect remaining fields (score, form).
fn find<'a>(
    out: &'a [RawCandidate],
    display: &str,
    span: (u32, u32),
    syll: u8,
) -> &'a RawCandidate {
    out.iter()
        .find(|c| c.display_text == display && c.consumed_span == span && c.syllable_count == syll)
        .unwrap_or_else(|| {
            panic!(
                "missing candidate (display={display:?}, span={span:?}, syll={syll}); got {out:#?}"
            )
        })
}

// ---------------------------------------------------------------------------
// Case 1 — `tsua` (roadmap §Phase 5 line 339)
// ---------------------------------------------------------------------------

#[test]
fn tsua_surfaces_zhi_zhuah_zhu_across_two_spans() {
    // Three rowids:
    //   1. 紙(tsuá, syll=1)   under tl:tsua  → span=(0,4)
    //   2. 珠仔(tsu-á, syll=2) under tl:tsua  → span=(0,4) — fused toneless
    //                                                       per Phase 1b
    //   3. 珠(tsu, syll=1)    under tl:tsu   → span=(0,3)
    let (prefix_index, dict) = build_fixture(
        "tsua",
        &[
            Row {
                toneless_key: "tsua",
                hanzi: "紙",
                tl: "tsuá",
                syll: 1,
                freq: 100,
            },
            Row {
                toneless_key: "tsua",
                hanzi: "珠仔",
                tl: "tsu-á",
                syll: 2,
                freq: 80,
            },
            Row {
                toneless_key: "tsu",
                hanzi: "珠",
                tl: "tsu",
                syll: 1,
                freq: 90,
            },
        ],
    );

    let out = fetch_candidates_for_endings(
        "tsua",
        0,
        &[3, 4], // syllabifier emits span=3 (`tsu`) and span=4 (`tsua`).
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );

    assert!(
        out.len() >= 3,
        "expected at least 3 candidates, got {}: {out:#?}",
        out.len()
    );
    let zhi = find(&out, "紙", (0, 4), 1);
    let zhuah = find(&out, "珠仔", (0, 4), 2);
    let zhu = find(&out, "珠", (0, 3), 1);

    // form is hard-coded to FORM_NOTONE for every Phase 5 candidate.
    assert_eq!(zhi.form, FORM_NOTONE);
    assert_eq!(zhuah.form, FORM_NOTONE);
    assert_eq!(zhu.form, FORM_NOTONE);

    // v3.5.8 Phase 9.2: pure-CJK hanji entries derive to CandidateMode::Hant
    // end-to-end through `record_to_candidate`. Pins integration plumbing
    // (per Codex post-impl finding #5, P3, 2026-05-11).
    assert_eq!(zhi.mode, CandidateMode::Hant, "紙 must derive HANT");
    assert_eq!(zhuah.mode, CandidateMode::Hant, "珠仔 must derive HANT");
    assert_eq!(zhu.mode, CandidateMode::Hant, "珠 must derive HANT");

    // Score sanity (`freq × syll_bias × user_freq_boost`, Phase 5 formula):
    //   紙   = 100 × 1.0 × 1.0 = 100.0  span=(0,4) → Tier 0 (full buffer "tsua")
    //   珠仔 = 80  × 1.1 × 1.0 =  88.0  span=(0,4) → Tier 0
    //   珠   = 90  × 1.0 × 1.0 =  90.0  span=(0,3) → Tier 1
    assert!((zhi.score - 100.0).abs() < 1e-4);
    assert!((zhuah.score - 88.0).abs() < 1e-4);
    assert!((zhu.score - 90.0).abs() < 1e-4);

    // v3.5.8 SortKey expected order (post-S8; per
    // `docs/releases/v3.5.8/plan.md` § Phase 9 sort_key formula):
    //   (coverage_kind, tier, recency_rank, -adjusted_score, -freq,
    //    -coverage_bytes, source_rank, stable_idx)
    //
    // - Tier 0 (full-buffer): 紙 + 珠仔, sorted by score desc → 紙 then 珠仔.
    // - Tier 1: 珠.
    // → expected display order: 紙(100, t=0) → 珠仔(88, t=0) → 珠(90, t=1).
    //
    // The deliberate behavior change vs Phase 5's pure-score-desc sort is
    // that Tier 0's 珠仔 (lower score) still surfaces ahead of Tier 1's 珠
    // (higher score), because Continuous prefers full-buffer coverage to
    // raw frequency when a multi-syllable phrase exactly matches the
    // pending buffer. See Codex R2 Q2.c rationale logged in
    // `/tmp/codex-v358-phase9-plan-r2-out.txt`.
    let display_order: Vec<&str> = out.iter().map(|c| c.display_text.as_str()).collect();
    assert_eq!(
        display_order,
        vec!["紙", "珠仔", "珠"],
        "Phase 9.1 SortKey: Tier 0 (full buffer) precedes Tier 1; \
         within Tier 0 sort by score desc"
    );
}

// ---------------------------------------------------------------------------
// Case 2 — `taigikhipuann` (roadmap §Phase 5 line 340)
// ---------------------------------------------------------------------------

#[test]
fn taigikhipuann_surfaces_long_reach_4_syllable_word() {
    // Endings simulate the syllabifier reaching:
    //   2  → `ta`?   No — TL minimal syllable is 2-3 chars; for this test
    //                we treat the syllabifier as having emitted endings at
    //                3 (tai), 5 (taigi), and 13 (taigikhipuann).
    //   13 lets us assert a 4-syll candidate surfaces under one toneless
    //   key.
    let (prefix_index, dict) = build_fixture(
        "taigikhipuann",
        &[
            Row {
                toneless_key: "tai",
                hanzi: "台",
                tl: "tâi",
                syll: 1,
                freq: 200,
            },
            Row {
                toneless_key: "taigi",
                hanzi: "台語",
                tl: "tâi-gí",
                syll: 2,
                freq: 150,
            },
            Row {
                toneless_key: "taigikhipuann",
                hanzi: "台語齒盤",
                tl: "tâi-gí-khí-puânn",
                syll: 4,
                freq: 5,
            },
        ],
    );

    let out = fetch_candidates_for_endings(
        "taigikhipuann",
        0,
        &[3, 5, 13],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );

    let tai = find(&out, "台", (0, 3), 1);
    let taigi = find(&out, "台語", (0, 5), 2);
    let quad = find(&out, "台語齒盤", (0, 13), 4);

    // Spans are at the requested ending offsets; syllable_count distinguishes the layers.
    assert_eq!(tai.consumed_span, (0, 3));
    assert_eq!(taigi.consumed_span, (0, 5));
    assert_eq!(quad.consumed_span, (0, 13));

    // 4-syll bias = 1.3:
    //   台語齒盤 = 5 × 1.3 = 6.5
    assert!((quad.score - 6.5).abs() < 1e-4);
}

// ---------------------------------------------------------------------------
// Case 3 — `taixyz` (roadmap §Phase 5 line 341)
// ---------------------------------------------------------------------------

#[test]
fn taixyz_emits_only_single_syllable_when_endings_capped() {
    // Syllabifier stops after `tai` because `xyz` has no valid TL initial
    // — it emits endings={3} only. The fetch must respect that and never
    // produce span >= 4 candidates even if the dictionary happens to
    // contain entries that would match longer toneless prefixes.
    let (prefix_index, dict) = build_fixture(
        "taixyz",
        &[
            Row {
                toneless_key: "tai",
                hanzi: "台",
                tl: "tâi",
                syll: 1,
                freq: 200,
            },
            // Decoy: a 2-syllable entry that WOULD match `tl:taix` if
            // endings included 4. Must NOT surface because endings={3}.
            Row {
                toneless_key: "taix",
                hanzi: "假詞",
                tl: "tai-x",
                syll: 2,
                freq: 999,
            },
        ],
    );

    let out = fetch_candidates_for_endings(
        "taixyz",
        0,
        &[3],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );

    assert!(!out.is_empty(), "expected at least the 台 candidate");
    for c in &out {
        assert_eq!(
            c.consumed_span,
            (0, 3),
            "no candidate should have span beyond ending=3, got {c:#?}"
        );
        assert_eq!(c.syllable_count, 1, "only single-syll candidates allowed");
    }
    // Decoy must not appear.
    assert!(
        out.iter().all(|c| c.display_text != "假詞"),
        "decoy entry leaked into result: {out:#?}"
    );
}

// ---------------------------------------------------------------------------
// Defensive guards
// ---------------------------------------------------------------------------

#[test]
fn empty_endings_yields_empty() {
    let (prefix_index, dict) = build_fixture(
        "empty-endings",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 1,
        }],
    );
    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert!(out.is_empty(), "no endings → no candidates");
}

#[test]
fn pos_at_or_past_input_end_yields_empty() {
    let (prefix_index, dict) = build_fixture(
        "pos-past-end",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 1,
        }],
    );
    // `pos == input.len()` is the natural "fully-consumed" state — no
    // span can extend past the input, so the function returns empty
    // without panicking.
    let out = fetch_candidates_for_endings(
        "tai",
        3,
        &[3],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert!(out.is_empty());
}

#[test]
fn out_of_range_endings_silently_skipped() {
    // Endings beyond input length must not panic / OOB; the matched
    // valid ending(s) still produce candidates.
    let (prefix_index, dict) = build_fixture(
        "oob-endings",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 50,
        }],
    );
    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[3, 99, 100],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert_eq!(out.len(), 1, "only ending=3 valid: {out:#?}");
    assert_eq!(out[0].display_text, "台");
}

// Phase 9.3a note: the pre-9.3a `nan_boost_does_not_break_descending_order`
// test injected `f32::NAN` directly through the `user_freq_boost: f32`
// parameter. That parameter is gone — `fetch_candidates_for_*` now
// builds the boost internally via `ranking::user_freq_boost(count)`,
// which always returns a finite value in `[1.0, MAX_BOOST]`. The
// `NonNanF32` defense inside `SortKey` is still pinned by
// `nan_score_is_coerced_to_minimum_not_panic` in `continuous.rs
// sort_key_tests`.

#[test]
fn numeric_tone_input_strips_to_fused_toneless_key() {
    // Codex bot PR #255 P1 finding 3214572227: numeric-tone TL/POJ
    // input must reach the fused-toneless FST key. `tsua7` (single
    // tone-7 syllable) was missing `紙(tsua)` because the lookup key
    // was being built as `tl:tsua7` instead of `tl:tsua`.
    let (prefix_index, dict) = build_fixture(
        "numeric-single",
        &[Row {
            toneless_key: "tsua",
            hanzi: "紙",
            tl: "tsuá",
            syll: 1,
            freq: 100,
        }],
    );
    let out = fetch_candidates_for_endings(
        "tsua7",
        0,
        &[5], // syllabifier emits one ending at end-of-input.
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert_eq!(out.len(), 1, "numeric-tone input must surface entry");
    assert_eq!(out[0].display_text, "紙");
    assert_eq!(out[0].consumed_span, (0, 5)); // span stays in raw bytes (incl. tone digit)
    assert_eq!(out[0].syllable_count, 1);
}

#[test]
fn numeric_tone_multi_syllable_strips_each_segment_to_fused_key() {
    // Multi-syllable numeric input: `tai1bak4` must reach `tl:taibak`
    // (Phase 1b fused toneless), not `tl:tai1bak4`.
    let (prefix_index, dict) = build_fixture(
        "numeric-multi",
        &[Row {
            toneless_key: "taibak",
            hanzi: "代墨",
            tl: "tâi-ba̍k",
            syll: 2,
            freq: 50,
        }],
    );
    let out = fetch_candidates_for_endings(
        "tai1bak4",
        0,
        &[4, 8],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    let multi = out
        .iter()
        .find(|c| c.display_text == "代墨" && c.consumed_span == (0, 8))
        .unwrap_or_else(|| panic!("expected 代墨(span=0..8); got {out:#?}"));
    assert_eq!(multi.syllable_count, 2);
}

#[test]
fn hyphen_in_input_is_not_stripped_at_lexicon_layer() {
    // Phase-6 Codex PR review (post-impl HIGH finding): the lexicon
    // toneless-key strip rule is intentionally `\d`-only, not `[\d\-]`.
    // Hyphenated TL input is folded upstream by
    // `composing::shadow::build_hyphen_shadow` (Phase 9 Item 8) so
    // segments arriving here are already hyphenless under the normal
    // dispatch path. This test pins the lower-layer invariant: if a
    // hyphenated segment somehow does reach `fetch_candidates_for_endings`
    // (defensive contract), the strip rule MUST NOT silently fold the
    // hyphen — it produces `tl:tai-bak` (not `tl:taibak`), and a
    // fixture that only stores `tl:taibak` returns NO candidates. If
    // anyone re-adds the hyphen half of the strip here, this test fails.
    let (prefix_index, dict) = build_fixture(
        "hyphen-no-strip",
        &[Row {
            toneless_key: "taibak",
            hanzi: "代墨",
            tl: "tâi-ba̍k",
            syll: 2,
            freq: 50,
        }],
    );
    let out = fetch_candidates_for_endings(
        "tai-bak",
        0,
        &[7], // hypothetical full-span ending (real syllabifier never emits this)
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert!(
        out.is_empty(),
        "hyphen MUST NOT be stripped at the lexicon strip layer; \
         got {out:#?} (likely a regression in `fetch_candidates_for_endings` \
         strip rule)",
    );
}

// Phase 9.3a: the pre-9.3a `user_freq_boost_amplifies_score_multiplicatively`
// test fed a raw `f32` boost through the public API. That signature is
// retired — boost is now derived from `FrequencyMap` entries. The
// new contract is covered end-to-end in
// `engine/lexicon/tests/user_freq_plumb.rs`.

// ---------------------------------------------------------------------------
// v3.5.8 Phase 9.1 — Regression matrix (`taiuantaigi` / `e` / `taixyz`)
// pinned by `docs/releases/v3.5.8/plan.md` § Phase 9 — 回歸守護矩陣.
//
// These cases use synthetic dict fixtures that mirror the frequency
// disparity that drove the Phase 9 pivot (`docs/engine/continuous-input-
// ranking.md` §3.1): single-char freq orders of magnitude above
// multi-syllable phrases. The SortKey must surface the full-buffer
// phrase in slot #1 regardless of that disparity.
// ---------------------------------------------------------------------------

// Phase 9.1 回歸守護矩陣 — hermetic 重現 `taiuantaigi` 排序失敗場景,鎖死 Tier 1 政策。

#[test]
fn taiuantaigi_full_buffer_phrase_outranks_high_freq_short_match() {
    // Reproduces the headline Phase 9 acceptance case:
    //   raw      = "taiuantaigi"  (len=11)
    //   syllabifier endings        → {3, 6, 9, 11}
    //   tl:tai          (3 chars)  → 「台」 freq=31281, syll=1
    //   tl:taiuan       (6 chars)  → 「台灣」 freq=1379, syll=2
    //   tl:taiuantaigi (11 chars)  → 「臺灣台語」 freq=12, syll=4
    //
    // Pre-Phase-9 (pure score desc): 「台」 (score=31281) outranks
    // 「臺灣台語」 (score=12×1.3=15.6) by ~2000x → user sees 「台」 at slot #1.
    // Phase 9.1 SortKey: 「臺灣台語」 is Tier 0 (consumed_span_end == raw_len),
    // 「台灣」 and 「台」 are Tier 1 → 「臺灣台語」 surfaces at #1.
    //
    // v3.5.8 整句 lattice + walker S8: the headline (Tier 0 phrase #1)
    // is unchanged — `tier` stays above score. Only the WITHIN-Tier-1
    // sub-order flipped: pre-S8 `-coverage_bytes` (dim 3) put the
    // 2-syllable 「台灣」 above the 1-syllable 「台」; post-S8 coverage is
    // the weak dim-6 tiebreak, so the higher-freq short 「台」 (31281)
    // now precedes the lower-freq longer 「台灣」 (1379). This is the
    // exact `guaikingkahuekhoo` dogfood fix — a high-freq single
    // syllable must not be buried below a longer lower-freq prefix.
    let (prefix_index, dict) = build_fixture(
        "taiuantaigi",
        &[
            Row {
                toneless_key: "tai",
                hanzi: "台",
                tl: "tâi",
                syll: 1,
                freq: 31281,
            },
            Row {
                toneless_key: "taiuan",
                hanzi: "台灣",
                tl: "tâi-uân",
                syll: 2,
                freq: 1379,
            },
            Row {
                toneless_key: "taiuantaigi",
                hanzi: "臺灣台語",
                tl: "tâi-uân-tâi-gí",
                syll: 4,
                freq: 12,
            },
        ],
    );

    let out = fetch_candidates_for_endings(
        "taiuantaigi",
        0,
        &[3, 6, 11], // syllabifier endings
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );

    let display_order: Vec<&str> = out.iter().map(|c| c.display_text.as_str()).collect();
    assert_eq!(
        display_order,
        vec!["臺灣台語", "台", "台灣"],
        "Tier 0 (full-buffer) 「臺灣台語」 must outrank Tier 1 partials \
         even though its score (15.6) is ~2000x lower than 「台」 (31281); \
         within Tier 1, post-S8 the higher-freq short 「台」 precedes the \
         lower-freq longer 「台灣」 (coverage demoted below freq)"
    );
}

#[test]
fn single_char_input_e_still_surfaces_de_at_slot_1() {
    // Regression guard for the inverse case: a single-char input must
    // not be hurt by Phase 9.1 tiering — the full buffer IS the
    // single char, so 「的」 lands in Tier 1 naturally.
    //
    //   raw     = "e"  (len=1)
    //   tl:e    → 「的」 freq=184693, syll=1, span=(0,1) → Tier 0
    //   tl:e    → 「鞋」 freq=500,    syll=1, span=(0,1) → Tier 0 (same)
    // Within Tier 0 + same coverage, score desc decides → 「的」 at #1.
    let (prefix_index, dict) = build_fixture(
        "e",
        &[
            Row {
                toneless_key: "e",
                hanzi: "的",
                tl: "ê",
                syll: 1,
                freq: 184693,
            },
            Row {
                toneless_key: "e",
                hanzi: "鞋",
                tl: "ê",
                syll: 1,
                freq: 500,
            },
        ],
    );

    let out = fetch_candidates_for_endings(
        "e",
        0,
        &[1],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );

    assert_eq!(out.len(), 2);
    assert_eq!(
        out[0].display_text, "的",
        "high-freq Tier 0 hanzi at slot 1"
    );
    assert_eq!(out[0].consumed_span, (0, 1));
    assert_eq!(
        out[1].display_text, "鞋",
        "low-freq Tier 0 hanzi at slot 2 (within-tier score desc)"
    );
}

#[test]
fn taixyz_invalid_tail_yields_empty_tier1_top() {
    // `xyz` cannot syllabify; syllabifier returns ending only at 3
    // (`tai`). `raw_len = 6` so no candidate has `consumed_span_end ==
    // raw_len = 6` → Tier 0 is empty, all candidates are Tier 1, sorted
    // by their normal score within Tier 1.
    //
    // This guards against the failure mode in Codex Q-E (R1): a Q1.b/c
    // (longest-reachable) Tier definition would have lifted 「台」 into
    // Tier 0 here, which the spec explicitly rejects.
    let (prefix_index, dict) = build_fixture(
        "taixyz-tier-rule",
        &[
            Row {
                toneless_key: "tai",
                hanzi: "台",
                tl: "tâi",
                syll: 1,
                freq: 31281,
            },
            Row {
                toneless_key: "tai",
                hanzi: "代",
                tl: "tāi",
                syll: 1,
                freq: 14215,
            },
        ],
    );

    let out = fetch_candidates_for_endings(
        "taixyz",
        0,
        &[3],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );

    assert!(!out.is_empty(), "Tier 1 partials must still surface");
    for cand in &out {
        assert_eq!(
            cand.consumed_span,
            (0, 3),
            "no candidate should claim more than the syllabifiable prefix"
        );
        assert_ne!(
            cand.consumed_span.1, 6,
            "Tier 0 must remain empty when no candidate covers raw_len"
        );
    }
    assert_eq!(
        out[0].display_text, "台",
        "within Tier 1, 「台」 (freq=31281) outranks 「代」 (freq=14215)"
    );
}

#[test]
fn stable_idx_preserves_insertion_order_at_fetch_boundary() {
    // Three candidates under the SAME toneless key "tai" with identical
    // SortKey dimensions 0..7 (coverage_kind, tier, recency_rank,
    // -score, -freq, -coverage, source_rank). Only `stable_idx`
    // (the last dim) differentiates.
    // The sort MUST keep them in pre-sort fetch order — which is FST
    // byte-sort over the encoded `tl:tai\xFF<rowid_le_u32>` suffix
    // (the `lookup_exact` enumeration order). For this fixture the
    // rowids 1/2/3 happen to coincide with builder-insertion order
    // because little-endian 1/2/3 differ only in the lowest byte and
    // therefore sort numerically.
    //
    // Pins Codex PR #262 r3216153007: earlier revisions stamped
    // `stable_idx` by incrementing a counter inside
    // `sort_by_cached_key`'s closure, which silently relied on
    // stdlib's call-order (non-contractual). The replacement uses
    // `enumerate()` over the pre-sort `Vec` so `stable_idx` reflects
    // position before any sorting machinery runs. This test guards
    // against future regressions of that pattern regardless of stdlib
    // internals.
    let (prefix_index, dict) = build_fixture(
        "stable-idx-insertion-order",
        &[
            // Distinct tones, all reducing to toneless "tai" so the
            // v3.5.8 abbrev-collision guard keeps them (a real dict
            // row's `tl` always normalizes back to its `tl_notone`);
            // they still tie on every SortKey dimension (freq / syll /
            // coverage / source / recency) so only `stable_idx`
            // separates them — the property under test.
            Row {
                toneless_key: "tai",
                hanzi: "一",
                tl: "tâi",
                syll: 1,
                freq: 100,
            },
            Row {
                toneless_key: "tai",
                hanzi: "二",
                tl: "tài",
                syll: 1,
                freq: 100,
            },
            Row {
                toneless_key: "tai",
                hanzi: "三",
                tl: "tāi",
                syll: 1,
                freq: 100,
            },
        ],
    );

    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );

    assert_eq!(out.len(), 3);
    let display_order: Vec<&str> = out.iter().map(|c| c.display_text.as_str()).collect();
    assert_eq!(
        display_order,
        vec!["一", "二", "三"],
        "stable_idx must preserve insertion order when higher SortKey \
         dimensions are tied; got {display_order:?}. Regression for \
         Codex PR #262 r3216153007."
    );
}

#[test]
fn raw_candidate_carries_dictionary_record_bitmask_for_sort_key() {
    // Phase 9.1 plumbs `DictionaryRecord.bitmask` through to
    // `RawCandidate.bitmask` so `SortKey` can derive source_tier_rank
    // at sort time without re-reading the dictionary. Verify the byte
    // identity (caller fixture sets bit 11 — see `build_fixture`).
    let (prefix_index, dict) = build_fixture(
        "bitmask-plumb",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 100,
        }],
    );
    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert_eq!(out.len(), 1);
    assert_eq!(
        out[0].bitmask,
        1u16 << 11,
        "bitmask must round-trip from DictionaryRecord to RawCandidate"
    );
    assert_eq!(out[0].frequency, 100, "raw freq must round-trip too");
}

// ---------------------------------------------------------------------------
// v3.5.8 Phase 9.2 — `CandidateMode` derive plumbing through fetch
// ---------------------------------------------------------------------------

#[test]
fn mode_carrier_propagates_through_fetch_for_hant_tailo_mixed() {
    // Integration regression for Codex post-impl finding #5 (P3, 2026-
    // 05-11): the in-crate `derive_mode` unit tests pin classification,
    // but they do not exercise the `DictionaryReader` → `RawCandidate`
    // plumbing. This test wires three fixture rows that hit all three
    // production-emittable `CandidateMode` arms and asserts the byte
    // identity through `record_to_candidate`.
    //
    // Empty `hanzi` ("") drives the v2 dict.bin header's `hanzi_len = 0`,
    // which `DictionaryReader::record` decodes as `hanzi: None` → TAILO.
    // Phase 9.2 mode 端對端契約;HANT / TAILO (hanzi=None via len=0) / MIXED 三種來源全跑過 record_to_candidate。
    let (prefix_index, dict) = build_fixture(
        "mode-plumb",
        &[
            Row {
                toneless_key: "tai",
                hanzi: "台",
                tl: "tâi",
                syll: 1,
                freq: 100,
            },
            Row {
                toneless_key: "li",
                hanzi: "",
                tl: "lí",
                syll: 1,
                freq: 50,
            },
            Row {
                toneless_key: "iausi",
                hanzi: "iáu是",
                tl: "iáu-sī",
                syll: 2,
                freq: 30,
            },
        ],
    );

    let hant = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert_eq!(hant.len(), 1);
    assert_eq!(hant[0].mode, CandidateMode::Hant);
    assert_eq!(hant[0].display_text, "台");

    let tailo = fetch_candidates_for_endings(
        "li",
        0,
        &[2],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert_eq!(tailo.len(), 1);
    assert_eq!(
        tailo[0].mode,
        CandidateMode::Tailo,
        "empty hanzi (None) must derive TAILO; display_text falls back to TL"
    );
    assert_eq!(tailo[0].display_text, "lí");

    let mixed = fetch_candidates_for_endings(
        "iausi",
        0,
        &[5],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert_eq!(mixed.len(), 1);
    assert_eq!(
        mixed[0].mode,
        CandidateMode::Mixed,
        "hanzi containing Latin letter (NFKD-normalized) must derive MIXED"
    );
    assert_eq!(mixed[0].display_text, "iáu是");
}

#[test]
fn roman_and_hanji_propagate_through_fetch_for_hant_tailo_mixed() {
    // v3.5.8 Phase 9 Item 5 — `roman` + `hanji` integration plumb.
    // Mirrors the in-crate `record_to_candidate_carrier_tests` hermetic
    // unit tests but exercises the full `DictionaryReader` → FST
    // lookup → `record_to_candidate` chain so the contract holds at
    // the integration boundary too. Empty `hanzi` ("") drives the v2
    // dict.bin header's `hanzi_len = 0`, which `DictionaryReader::record`
    // decodes as `hanzi: None` — the only TAILO path.
    // Item 5 — 端對端契約;roman 永等於 record.tl,hanji 與 record.hanzi 雙向同步(None ⇔ TAILO)。
    let (prefix_index, dict) = build_fixture(
        "item5-carrier",
        &[
            Row {
                toneless_key: "tai",
                hanzi: "台",
                tl: "tâi",
                syll: 1,
                freq: 100,
            },
            Row {
                toneless_key: "li",
                hanzi: "",
                tl: "lí",
                syll: 1,
                freq: 50,
            },
            Row {
                toneless_key: "iausi",
                hanzi: "iáu是",
                tl: "iáu-sī",
                syll: 2,
                freq: 30,
            },
        ],
    );

    let hant = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert_eq!(hant.len(), 1);
    assert_eq!(hant[0].roman, "tâi");
    assert_eq!(hant[0].hanji.as_deref(), Some("台"));

    let tailo = fetch_candidates_for_endings(
        "li",
        0,
        &[2],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert_eq!(tailo.len(), 1);
    assert_eq!(tailo[0].roman, "lí");
    assert_eq!(
        tailo[0].hanji, None,
        "empty hanzi (None) must wire as proto3 `optional` absent — NOT Some(empty)"
    );

    let mixed = fetch_candidates_for_endings(
        "iausi",
        0,
        &[5],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert_eq!(mixed.len(), 1);
    assert_eq!(mixed[0].roman, "iáu-sī");
    assert_eq!(mixed[0].hanji.as_deref(), Some("iáu是"));
}

// ---------------------------------------------------------------------------
// v3.5.8 Phase 9 Item 10 — partial-prefix engine path
// ---------------------------------------------------------------------------
//
// Pins `docs/engine/continuous-candidate-display.md` §15.3.D + §15.5
// behaviour: when the syllabifier returns no valid ending, the engine
// falls through to `prefix_index.lookup_prefix("tl:<lower-stripped>")`
// and emits candidates tagged with `coverage_kind = COVERAGE_KIND_PARTIAL_PREFIX`
// and `consumed_span = (0, raw.len())`.

/// Convenience: build the partial-prefix key tuple the way
/// `composing::shadow::build_partial_prefix_key` would for pure-ASCII
/// input (v3.5.9 B-2 dropped the `_tl` suffix when the emitter became
/// mode-aware). Hermetic tests at this layer cannot import the
/// composing crate (cyclic test seam), so we mirror the contract
/// inline; the composing side's unit tests pin the canonicalize +
/// hyphen-shadow + tone-digit-strip chain separately.
// ASCII 限定的 partial-prefix key helper;lexicon 測試不能反 import composing,
//   故在這裡 inline 一條同等的最小 pipeline (lower + 去 hyphen + 去 ASCII 數字)。
fn partial_prefix_key_for(raw: &str) -> (ConsumedSpan, String) {
    let toneless: String = raw
        .to_ascii_lowercase()
        .chars()
        .filter(|c| *c != '-' && !c.is_ascii_digit())
        .collect();
    ((0u32, raw.len() as u32), format!("tl:{toneless}"))
}

#[test]
fn partial_prefix_engine_path_surfaces_lookup_prefix_hits() {
    // Single-char partial buffer `g` matches every `tl:g*` entry in
    // the fixture via `prefix_index.lookup_prefix("tl:g")`. The
    // full-syllable path would have returned empty here (the
    // syllabifier needs at least one valid syllable boundary), so
    // this fetch entry is the one the dispatcher reaches.
    let (prefix_index, dict) = build_fixture(
        "item10-partial-prefix",
        &[
            Row {
                toneless_key: "gua",
                hanzi: "我",
                tl: "guá",
                syll: 1,
                freq: 200,
            },
            Row {
                toneless_key: "guan",
                hanzi: "阮",
                tl: "guán",
                syll: 1,
                freq: 50,
            },
            Row {
                toneless_key: "lin",
                hanzi: "恁",
                tl: "lín",
                syll: 1,
                freq: 30,
            },
        ],
    );

    let key = partial_prefix_key_for("g");
    let out = fetch_partial_prefix_candidates(
        &key,
        1, // raw_len = "g".len()
        &ctx(&FrequencyMap::new(), 0, &[], &prefix_index, &dict),
    );

    // Two `tl:g*` hits; `tl:lin` excluded by FST prefix range.
    assert_eq!(out.len(), 2, "expected 2 partial-prefix hits, got {out:#?}");
    let labels: Vec<&str> = out.iter().map(|c| c.display_text.as_str()).collect();
    assert!(
        labels.contains(&"我"),
        "missing 我 (tl:gua); got {labels:?}"
    );
    assert!(
        labels.contains(&"阮"),
        "missing 阮 (tl:guan); got {labels:?}"
    );

    // §15.5 invariant: every candidate is tagged partial.
    for c in &out {
        assert_eq!(
            c.coverage_kind, COVERAGE_KIND_PARTIAL_PREFIX,
            "partial-prefix candidate must carry COVERAGE_KIND_PARTIAL_PREFIX, got {c:?}"
        );
        // §15.5 + Q15.4: consumed_span covers the whole pending tail.
        assert_eq!(c.consumed_span, (0, 1));
    }

    // Within the partial bucket, the existing 8-dim SortKey policy
    // still applies — higher dict-freq wins on the `-neg_freq` dim
    // even after coverage_kind tied to 1.
    assert_eq!(out[0].display_text, "我", "higher-freq partial wins");
}

#[test]
fn partial_prefix_filters_abbrev_collisions() {
    // Codex PR #351 r3319500948 — `fetch_partial_prefix_candidates` must
    // mirror the span-local + walker guard at
    // `fetch_candidates_for_keys`:640 / `best_candidate_for_key`:922 and
    // reject rowids whose FST entry is an `tl_abbrev` / `poj_abbrev` /
    // `tps_abbrev` collision sharing the input prefix. The prefix-aware
    // variant `matches_continuous_toneless_prefix_key` reconstructs the
    // toneless from `record.tl` and keeps the rowid only when
    // `reconstructed.starts_with(key_body)`.
    //
    // Fixture: row 1 is a genuine `tl:taigi` toneless hit (`tâi-gí`/
    // 台語); row 2 is a synthetic acronym collision — a 5-syllable
    // phrase whose abbrev happens to start with `taigi`. We simulate the
    // collision by setting `toneless_key = "taigir"` on row 2 so its
    // FST entry `tl:taigir` falls inside the byte-range scan of
    // `lookup_prefix("tl:taigi")` but its `record.tl` reconstructs to a
    // toneless that does NOT start with `taigi` — the post-fix filter
    // rejects it. The pre-fix behaviour would have leaked row 2 as a
    // `COVERAGE_KIND_PARTIAL_PREFIX` candidate.
    let (prefix_index, dict) = build_fixture(
        "item10-abbrev-filter",
        &[
            Row {
                toneless_key: "taigi",
                hanzi: "台語",
                tl: "tâi-gí",
                syll: 2,
                freq: 100,
            },
            // Synthetic acronym collision row. `tl = "tó-â-iàu-gô-iàu"`
            // reconstructs (via `normalize_input` → strip digits) to
            // `toaiaugoiau`, which does NOT start with `taigi` → filter
            // rejects. The forged `toneless_key = "taigir"` only places
            // the FST entry inside the `lookup_prefix("tl:taigi")` range.
            Row {
                toneless_key: "taigir",
                hanzi: "X",
                tl: "tó-â-iàu-gô-iàu",
                syll: 5,
                freq: 50,
            },
        ],
    );

    let key = partial_prefix_key_for("taigi");
    let out = fetch_partial_prefix_candidates(
        &key,
        5,
        &ctx(&FrequencyMap::new(), 0, &[], &prefix_index, &dict),
    );

    // Row 1 (`tâi-gí`/台語) survives — genuine phonetic prefix-extension.
    // Row 2 (`tó-â-iàu-gô-iàu`/X) filtered — acronym-only collision.
    assert_eq!(
        out.len(),
        1,
        "post-fix: only the genuine prefix-extension survives, got {out:#?}"
    );
    let labels: Vec<&str> = out.iter().map(|c| c.display_text.as_str()).collect();
    assert_eq!(labels, vec!["台語"], "got {labels:?}");
    assert_eq!(out[0].coverage_kind, COVERAGE_KIND_PARTIAL_PREFIX);
}

#[test]
fn partial_prefix_unbounded_exposes_full_pool_for_cross_batch_dedupe() {
    // Codex PR #351 r3321758666 — when an input's exact key has many
    // homophones, the bounded fetcher's `PARTIAL_PREFIX_OUTPUT_CAP`
    // truncate is consumed by exact-match rows that the caller is
    // about to drop via cross-batch FULL/PARTIAL dedupe, leaving zero
    // visible extension candidates. The `_unbounded` variant must
    // return the sorted pool intact so the caller can exclude FULL
    // duplicates BEFORE truncating.
    //
    // Fixture: typed `hon`, 32 high-freq extensions at `tl:hong` (above
    // `PARTIAL_PREFIX_OUTPUT_CAP = 30`) + one lower-freq extension at
    // `tl:honn`. The bounded fetcher would return 30 `hong` rows (the
    // `honn` row does not survive the truncate); the unbounded fetcher
    // returns all 33 sorted rows so a caller-side exclude can drop the
    // 32 and surface the 1.
    //
    // Every row is single-syllable on purpose: the syllable-reach rule
    // (`typed_prefix_reaches_final_syllable`) drops any extension whose
    // final syllable the typed body never reaches, so a multi-syllable
    // extension of a one-syllable input can no longer stand in for "the
    // row the caller must be able to surface". Saturation and the
    // bounded/unbounded split are what this test is about, and both
    // reproduce with same-syllable extensions.
    let mut rows: Vec<Row> = (1..=32)
        .map(|i| Row {
            toneless_key: "hong",
            hanzi: HOMOPHONE_HANZI[i - 1],
            tl: HOMOPHONE_TL[i - 1],
            syll: 1,
            freq: 1000 + i as u32,
        })
        .collect();
    rows.push(Row {
        toneless_key: "honn",
        hanzi: "好",
        tl: "hònn",
        syll: 1,
        freq: 10,
    });
    let (prefix_index, dict) = build_fixture("item10-unbounded-pool", &rows);

    let key = partial_prefix_key_for("hon");
    let freq_map = FrequencyMap::new();
    let ctx_neutral = ctx(&freq_map, 0, &[], &prefix_index, &dict);

    // Bounded path: truncated to OUTPUT_CAP. The lower-freq extension
    // is squeezed out by the 32 homophones.
    let bounded = fetch_partial_prefix_candidates(&key, 3, &ctx_neutral);
    assert_eq!(
        bounded.len(),
        PARTIAL_PREFIX_OUTPUT_CAP,
        "bounded path saturates at OUTPUT_CAP, got {}",
        bounded.len()
    );
    assert!(
        !bounded.iter().any(|c| c.display_text == "好"),
        "extension `好`/hònn should NOT survive bounded truncate when 32 \
         `hong` rows outscore it; got display_texts: {:?}",
        bounded.iter().map(|c| &c.display_text).collect::<Vec<_>>()
    );

    // Unbounded path: full pool, no truncate. The extension is present
    // alongside all 32 `hong` rows for the caller to filter.
    let unbounded = fetch_partial_prefix_candidates_unbounded(&key, 3, &ctx_neutral);
    assert_eq!(
        unbounded.len(),
        33,
        "unbounded path keeps all 33 dedupe survivors, got {}",
        unbounded.len()
    );
    assert!(
        unbounded.iter().any(|c| c.display_text == "好"),
        "extension `好`/hònn MUST be present in the unbounded pool so \
         the caller can surface it after excluding FULL-block rows"
    );
}

// Hermetic hanzi + tl pools for the 32-homophone fixture above. Each
// hanzi/tl pair is unique so the per-row `(roman, hanji, span)` triple
// is distinct (no internal `dedupe_by_roman_hanji_span` collapse before
// the test's bounded/unbounded comparison runs).
const HOMOPHONE_HANZI: [&str; 32] = [
    "風", "封", "豐", "瘋", "蜂", "鋒", "峰", "烽", "馮", "逢", "縫", "奉", "鳳", "捧", "棒", "蓬",
    "篷", "鵬", "彭", "澎", "膨", "朋", "棚", "繃", "崩", "綳", "甭", "蓬", "鬃", "宏", "弘", "洪",
];
const HOMOPHONE_TL: [&str; 32] = [
    "hong-1", "hong-2", "hong-3", "hong-4", "hong-5", "hong-6", "hong-7", "hong-8", "hong-9",
    "hong-10", "hong-11", "hong-12", "hong-13", "hong-14", "hong-15", "hong-16", "hong-17",
    "hong-18", "hong-19", "hong-20", "hong-21", "hong-22", "hong-23", "hong-24", "hong-25",
    "hong-26", "hong-27", "hong-28", "hong-29", "hong-30", "hong-31", "hong-32",
];

#[test]
fn partial_prefix_returns_empty_when_no_dict_hits() {
    let (prefix_index, dict) = build_fixture(
        "item10-no-hits",
        &[Row {
            toneless_key: "gua",
            hanzi: "我",
            tl: "guá",
            syll: 1,
            freq: 200,
        }],
    );

    // `tl:zz` matches nothing in the fixture.
    let key = partial_prefix_key_for("zz");
    let out = fetch_partial_prefix_candidates(
        &key,
        2,
        &ctx(&FrequencyMap::new(), 0, &[], &prefix_index, &dict),
    );
    assert!(
        out.is_empty(),
        "no-hit prefix must yield empty, got {out:#?}"
    );
}

#[test]
fn partial_prefix_output_caps_at_output_cap() {
    // §15.8 risk row 1 — bound visible candidate count even when many
    // FST entries match a single-char prefix. Build a fixture with
    // `PARTIAL_PREFIX_OUTPUT_CAP + 5` `tl:t*` rows so the post-sort
    // truncate is exercised (well under `PARTIAL_PREFIX_HYDRATE_CAP`
    // so every row is hydrated and only the output cap is the
    // limiter). Toneless keys must be unique to land at distinct
    // rowids; we suffix 2-letter ASCII to keep them inside the fst.
    let rows: Vec<Row> = (0..(PARTIAL_PREFIX_OUTPUT_CAP + 5))
        .map(|i| {
            // Generate unique 3-char `t` + 2-letter lowercase suffix
            // (`taa`, `tab`, …, `tcz`).
            // Vowel as 2nd char so every key is a full reading, NOT an
            // all-consonant acronym surface that `is_roman_acronym_key`
            // would skip (this test exercises the OUTPUT cap, not that
            // filter).
            let vowels = [b'a', b'e', b'i', b'o', b'u'];
            let hi = vowels[(i / 26) % vowels.len()] as char;
            let lo = (b'a' + (i % 26) as u8) as char;
            Row {
                toneless_key: Box::leak(format!("t{hi}{lo}").into_boxed_str()),
                hanzi: Box::leak(format!("漢{i}").into_boxed_str()),
                tl: Box::leak(format!("t{hi}{lo}").into_boxed_str()),
                syll: 1,
                freq: 1,
            }
        })
        .collect();
    let (prefix_index, dict) = build_fixture("item10-output-cap", &rows);

    let key = partial_prefix_key_for("t");
    let out = fetch_partial_prefix_candidates(
        &key,
        1,
        &ctx(&FrequencyMap::new(), 0, &[], &prefix_index, &dict),
    );
    assert_eq!(
        out.len(),
        PARTIAL_PREFIX_OUTPUT_CAP,
        "partial-prefix output must be capped at PARTIAL_PREFIX_OUTPUT_CAP, got {}",
        out.len()
    );
}

#[test]
fn partial_prefix_high_freq_short_candidate_survives_past_legacy_byte_sort_cap() {
    // R6 regression — the pre-fix `take(PARTIAL_PREFIX_CAP=30)` cap
    // ran BEFORE hydration, in FST byte-sort order. For input `tl:k`,
    // the FST front-loaded `tl:ka-*` multi-syllable phrases and
    // evicted high-frequency single-syllable entries like `tl:ki`
    // before any SortKey scoring happened. This test pins the fix:
    // 35 low-freq `ka-XX` rows come first in byte order (rowids 1-35,
    // all with `frequency = 1`), then 1 high-freq `ki` row at rowid
    // 36 (`frequency = 50_000`). With HYDRATE_CAP=500 every row is
    // hydrated, sort runs, and `ki` wins on `-score` / `-freq` even
    // though it would have been dropped at rowid 31 under the old
    // pre-cap.
    //
    // Goal cited in PR description: "in normal case, 短候選排前面".
    let mut rows: Vec<Row> = (0..35)
        .map(|i| {
            // `ka-aa`, `ka-ab`, …, `ka-bi` — all multi-syllable
            // phrases sorting BEFORE `ki` in byte order under the
            // `tl:k` prefix scan.
            let hi = (b'a' + (i / 26) as u8) as char;
            let lo = (b'a' + (i % 26) as u8) as char;
            Row {
                toneless_key: Box::leak(format!("ka-{hi}{lo}").into_boxed_str()),
                hanzi: Box::leak(format!("加{i}").into_boxed_str()),
                tl: Box::leak(format!("ka-{hi}{lo}").into_boxed_str()),
                syll: 2,
                freq: 1,
            }
        })
        .collect();
    // The high-freq single-syllable target — would have been at
    // byte-rank 36, dropped by the old 30-row pre-cap.
    rows.push(Row {
        toneless_key: "ki",
        hanzi: "基",
        tl: "ki",
        syll: 1,
        freq: 50_000,
    });
    let (prefix_index, dict) = build_fixture("r6-short-candidate-survives", &rows);

    let key = partial_prefix_key_for("k");
    let out = fetch_partial_prefix_candidates(
        &key,
        1,
        &ctx(&FrequencyMap::new(), 0, &[], &prefix_index, &dict),
    );
    assert!(
        out.iter().any(|c| c.display_text == "基"),
        "high-freq `ki` (基) must reach the candidate list even though it sorts \
         past the legacy byte-rank 30; got {:?}",
        out.iter().map(|c| &c.display_text).collect::<Vec<_>>()
    );
    assert_eq!(
        out[0].display_text,
        "基",
        "high-freq single-syllable `ki` (基) must rank top-1 over freq=1 multi-syllable \
         phrases; got {:?}",
        out.iter().map(|c| &c.display_text).collect::<Vec<_>>()
    );
    assert!(
        out.len() <= PARTIAL_PREFIX_OUTPUT_CAP,
        "output must respect PARTIAL_PREFIX_OUTPUT_CAP, got {}",
        out.len()
    );
}

#[test]
fn partial_prefix_dedupe_runs_before_output_truncate() {
    // Pin pipeline order: hydrate → dedupe → sort → truncate. The
    // existing R6 regression test catches "truncate moved before
    // sort" (high-freq short candidate evicted by FST byte-sort), but
    // a duplicate-heavy fixture is needed to also catch "truncate
    // moved between sort and dedupe" — i.e., a hypothetical refactor
    // `hydrate → sort → truncate(OUTPUT_CAP) → dedupe` would silently
    // drop unique survivors when many rows share `(roman, hanji)`.
    //
    // Fixture: OUTPUT_CAP dict rows that all hydrate to the SAME
    // `(roman, hanji)` (distinct toneless_keys `kaa`..`kbd`, all with
    // `tl = "kaa"` / `hanzi = "加"`) plus 1 UNIQUE row at byte-rank
    // OUTPUT_CAP under toneless_key `kbe` with `tl = "kbe"` /
    // `hanzi = "基"`.
    // Correct order (dedupe → sort → truncate): dedupe collapses the
    // 30 dupes to 1; output = {"加", "基"} (2 candidates, truncate
    // no-op).
    // Broken order (sort → truncate → dedupe): sort puts the 30 dupes
    // at stable_idx 0..29 ahead of "基" at stable_idx 30; truncate(30)
    // keeps the dupes and drops "基"; dedupe then collapses to 1.
    // Output would be {"加"} alone — "基" silently evicted.
    let cap = PARTIAL_PREFIX_OUTPUT_CAP;
    let mut rows: Vec<Row> = (0..cap)
        .map(|i| {
            // Distinct toneless_keys (`kaa`, `kab`, …) so the FST
            // accepts them as separate entries, but `tl` + `hanzi`
            // collapse them to a single `(roman, hanji)` after
            // `dedupe_by_roman_hanji_span`.
            let hi = (b'a' + (i / 26) as u8) as char;
            let lo = (b'a' + (i % 26) as u8) as char;
            Row {
                toneless_key: Box::leak(format!("k{hi}{lo}").into_boxed_str()),
                hanzi: "加",
                tl: "kaa",
                syll: 1,
                freq: 1,
            }
        })
        .collect();
    // Byte-sort puts this last (after `kbd` at i=29 → `kbe` is i=30).
    rows.push(Row {
        toneless_key: "kbe",
        hanzi: "基",
        tl: "kbe",
        syll: 1,
        freq: 1,
    });
    let (prefix_index, dict) = build_fixture("partial-prefix-dedupe-order", &rows);

    let key = partial_prefix_key_for("k");
    let out = fetch_partial_prefix_candidates(
        &key,
        1,
        &ctx(&FrequencyMap::new(), 0, &[], &prefix_index, &dict),
    );
    let labels: Vec<&str> = out.iter().map(|c| c.display_text.as_str()).collect();
    assert!(
        labels.contains(&"基"),
        "unique survivor `基` (byte-rank {}) must reach output — dedupe must run \
         BEFORE truncate so the {} dupes of `加` collapse first and free a slot. \
         If truncate ran before dedupe, `基` would be evicted at stable_idx {}. \
         got {:?}",
        cap,
        cap,
        cap,
        labels
    );
    assert!(
        labels.contains(&"加"),
        "collapsed `加` must also reach output; got {:?}",
        labels
    );
    assert_eq!(
        out.len(),
        2,
        "post-dedupe pool is {{加, 基}} = 2 unique candidates; truncate is a no-op. \
         got len={} ({:?})",
        out.len(),
        labels
    );
}

#[test]
fn partial_prefix_hydrates_up_to_hydrate_cap_then_truncates() {
    // Worst-case stress: fixture has `PARTIAL_PREFIX_HYDRATE_CAP + 50`
    // rows under the same prefix. Sort survivor count must equal
    // `PARTIAL_PREFIX_OUTPUT_CAP` (the visible top-N stays stable);
    // hydration cap protects per-keystroke work without dropping the
    // visible candidate-strip size.
    let rows: Vec<Row> = (0..(PARTIAL_PREFIX_HYDRATE_CAP + 50))
        .map(|i| {
            // Encode i into a 3-letter lowercase suffix so all keys
            // share the `tl:s` prefix and stay unique up to 26^3 =
            // 17_576 rows. Suffix order: `aaa`, `aab`, …
            let a = (b'a' + ((i / 676) % 26) as u8) as char;
            let b = (b'a' + ((i / 26) % 26) as u8) as char;
            let c = (b'a' + (i % 26) as u8) as char;
            Row {
                toneless_key: Box::leak(format!("s{a}{b}{c}").into_boxed_str()),
                hanzi: Box::leak(format!("漢{i}").into_boxed_str()),
                tl: Box::leak(format!("s{a}{b}{c}").into_boxed_str()),
                syll: 1,
                freq: 1,
            }
        })
        .collect();
    let (prefix_index, dict) = build_fixture("r6-hydrate-cap-stress", &rows);

    let key = partial_prefix_key_for("s");
    let out = fetch_partial_prefix_candidates(
        &key,
        1,
        &ctx(&FrequencyMap::new(), 0, &[], &prefix_index, &dict),
    );
    assert_eq!(
        out.len(),
        PARTIAL_PREFIX_OUTPUT_CAP,
        "output stays bounded by OUTPUT_CAP even when hydration pool exceeds \
         HYDRATE_CAP, got {}",
        out.len()
    );
}

#[test]
fn partial_prefix_empty_key_returns_empty() {
    // Defensive: a literally empty FST key (`String::new()`) must
    // short-circuit before any FST range scan. Note this does NOT
    // pin behaviour for a bare namespace string `"tl:"` — that
    // input is treated as a legitimate "match everything under the
    // namespace" query (hydrated up to `PARTIAL_PREFIX_HYDRATE_CAP`,
    // output truncated to `PARTIAL_PREFIX_OUTPUT_CAP`), per the
    // caller-obligation contract on `fetch_partial_prefix_candidates`.
    // Production dispatch builds keys via
    // `composing::shadow::build_partial_prefix_key` which returns
    // `None` instead of emitting bare `"tl:"`.
    let (prefix_index, dict) = build_fixture(
        "item10-empty-key",
        &[Row {
            toneless_key: "x",
            hanzi: "X",
            tl: "x",
            syll: 1,
            freq: 1,
        }],
    );
    let key: (ConsumedSpan, String) = ((0u32, 0u32), String::new());
    let out = fetch_partial_prefix_candidates(
        &key,
        0,
        &ctx(&FrequencyMap::new(), 0, &[], &prefix_index, &dict),
    );
    assert!(out.is_empty());
}

#[test]
fn partial_prefix_coverage_kind_zero_unchanged_on_full_syllable_path() {
    // Item 10 invariant R1/R2: the existing `fetch_candidates_for_endings`
    // path must keep emitting `coverage_kind = COVERAGE_KIND_FULL` for
    // every candidate. Pin this at the integration boundary so any
    // future refactor that accidentally widens `record_to_candidate`'s
    // default fails here first.
    let (prefix_index, dict) = build_fixture(
        "item10-full-default",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 100,
        }],
    );

    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].coverage_kind, COVERAGE_KIND_FULL);
}

// ---------------------------------------------------------------------------
// v3.5.8 Phase 9 Item 12 — custom_dictionary.db merge + (roman, hanji)
// dedupe end-to-end through `fetch_candidates_for_keys`. Spec
// `docs/engine/continuous-input-ranking.md` §10.10 +
// `docs/engine/continuous-candidate-display.md` §15.
// ---------------------------------------------------------------------------

#[test]
fn item12_custom_only_entry_surfaces_full_buffer() {
    // No dict.bin hit for the span; a custom entry must still surface
    // as a full-buffer candidate (span = (0, raw_len), is_custom).
    let (prefix_index, dict) = build_fixture(
        "item12-custom-only",
        &[Row {
            toneless_key: "kah",
            hanzi: "甲",
            tl: "kah",
            syll: 1,
            freq: 100,
        }],
    );
    let keys: Vec<(ConsumedSpan, String)> = vec![((0, 6), "tl:taigi".to_owned())];
    let custom = vec![CustomEntry {
        roman: "tâi-gí".to_owned(),
        hanji: Some("台語".to_owned()),
    }];
    let out = fetch_candidates_for_keys(
        &keys,
        6,
        &ctx(&FrequencyMap::new(), 0, &custom, &prefix_index, &dict),
    );
    assert_eq!(out.len(), 1, "custom entry must surface, got {out:#?}");
    assert_eq!(out[0].display_text, "台語");
    assert_eq!(out[0].consumed_span, (0, 6));
    assert!(out[0].is_custom);
    assert_eq!(out[0].coverage_kind, COVERAGE_KIND_FULL);
}

#[test]
fn item12_custom_dedupes_and_wins_dict_collision() {
    // dict.bin and custom share `(tâi-gí, 台語)`. After the merge the
    // `(roman, hanji)` dedupe collapses them to one survivor — the
    // custom entry (source_tier_rank 0 beats the dict source tier),
    // even though the dict entry has a much higher raw frequency.
    let (prefix_index, dict) = build_fixture(
        "item12-collision",
        &[Row {
            toneless_key: "taigi",
            hanzi: "台語",
            tl: "tâi-gí",
            syll: 2,
            freq: 99999,
        }],
    );
    let keys: Vec<(ConsumedSpan, String)> = vec![((0, 6), "tl:taigi".to_owned())];
    let custom = vec![CustomEntry {
        roman: "tâi-gí".to_owned(),
        hanji: Some("台語".to_owned()),
    }];
    let out = fetch_candidates_for_keys(
        &keys,
        6,
        &ctx(&FrequencyMap::new(), 0, &custom, &prefix_index, &dict),
    );
    assert_eq!(
        out.len(),
        1,
        "duplicate (roman, hanji) must collapse to one, got {out:#?}"
    );
    assert!(
        out[0].is_custom,
        "custom (rank 0) must win the collision over the high-freq dict entry"
    );
    assert_eq!(out[0].display_text, "台語");
}

#[test]
fn item12_custom_roman_variant_not_deduped() {
    // Same hanji, different roman → distinct `(roman, hanji)` keys.
    // Both the dict entry and the custom entry survive (D1 dual-key).
    let (prefix_index, dict) = build_fixture(
        "item12-roman-variant",
        &[Row {
            toneless_key: "taigi",
            hanzi: "台語",
            tl: "tâi-gí",
            syll: 2,
            freq: 100,
        }],
    );
    let keys: Vec<(ConsumedSpan, String)> = vec![((0, 6), "tl:taigi".to_owned())];
    let custom = vec![CustomEntry {
        roman: "tai5-gi2".to_owned(), // numeric-tone variant of same hanji
        hanji: Some("台語".to_owned()),
    }];
    let out = fetch_candidates_for_keys(
        &keys,
        6,
        &ctx(&FrequencyMap::new(), 0, &custom, &prefix_index, &dict),
    );
    assert_eq!(
        out.len(),
        2,
        "roman variants must both survive, got {out:#?}"
    );
}

#[test]
fn item12_custom_merges_into_partial_prefix_path() {
    // D6: custom entries also surface in the partial-prefix path,
    // tagged COVERAGE_KIND_PARTIAL_PREFIX (NOT FULL) so §15.5's
    // "partial ranks below full" invariant is preserved.
    let (prefix_index, dict) = build_fixture(
        "item12-partial-custom",
        &[Row {
            toneless_key: "gua",
            hanzi: "我",
            tl: "guá",
            syll: 1,
            freq: 100,
        }],
    );
    let key: (ConsumedSpan, String) = ((0, 2), "tl:gu".to_owned());
    let custom = vec![CustomEntry {
        roman: "gún".to_owned(),
        hanji: Some("阮".to_owned()),
    }];
    let out = fetch_partial_prefix_candidates(
        &key,
        2,
        &ctx(&FrequencyMap::new(), 0, &custom, &prefix_index, &dict),
    );
    let custom_hit = out
        .iter()
        .find(|c| c.is_custom)
        .expect("custom entry must surface in partial-prefix path");
    assert_eq!(custom_hit.display_text, "阮");
    assert_eq!(
        custom_hit.coverage_kind, COVERAGE_KIND_PARTIAL_PREFIX,
        "custom in partial path must be PARTIAL_PREFIX, not FULL"
    );
}

#[test]
fn item12_empty_custom_is_noop() {
    // Backward-compat: empty custom slice reproduces pre-Item-12
    // behavior exactly (no synthesized candidates, dedupe no-op).
    let (prefix_index, dict) = build_fixture(
        "item12-empty-custom",
        &[Row {
            toneless_key: "taigi",
            hanzi: "台語",
            tl: "tâi-gí",
            syll: 2,
            freq: 100,
        }],
    );
    let keys: Vec<(ConsumedSpan, String)> = vec![((0, 6), "tl:taigi".to_owned())];
    let out = fetch_candidates_for_keys(
        &keys,
        6,
        &ctx(&FrequencyMap::new(), 0, &[], &prefix_index, &dict),
    );
    assert_eq!(out.len(), 1);
    assert!(!out[0].is_custom);
    assert_eq!(out[0].display_text, "台語");
}

// ----- v3.5.8 S2 — best_candidate_for_key (whole-sentence walker seam) -----

#[test]
fn best_candidate_for_key_returns_highest_score_on_collision() {
    // Two dict rows share the toneless key `tl:taiuan`. The walker's
    // edge provider must get the highest-`calculate_continuous_score`
    // one (freq-driven here) — `臺灣` (freq 5000), not `台灣`
    // (freq 100).
    let (prefix_index, dict) = build_fixture(
        "s2-best-collision",
        &[
            Row {
                toneless_key: "taiuan",
                hanzi: "台灣",
                tl: "tai5-uan5",
                syll: 2,
                freq: 100,
            },
            Row {
                toneless_key: "taiuan",
                hanzi: "臺灣",
                tl: "tâi-uân",
                syll: 2,
                freq: 5000,
            },
        ],
    );
    let best = best_candidate_for_key(
        "tl:taiuan",
        (0, 6),
        &FrequencyMap::new(),
        0,
        &prefix_index,
        &dict,
        u32::MAX,
    )
    .expect("key has dict hits");
    assert_eq!(best.display_text, "臺灣", "highest-score row must win");
    assert_eq!(best.hanji.as_deref(), Some("臺灣"));
    assert_eq!(best.roman, "tâi-uân");
    assert_eq!(best.syllable_count, 2);
    // consumed_span is stamped verbatim (the walker only reads
    // roman/hanji/freq/syll off it).
    assert_eq!(best.consumed_span, (0, 6));
    assert_eq!(best.coverage_kind, COVERAGE_KIND_FULL);
}

#[test]
fn best_candidate_for_key_none_when_key_absent() {
    // No dict hit → `None`, so the walker's edge provider falls back
    // to the synthesized toneless roman for that edge (the no-hanji
    // path is the walker's natural output, not a special fallback).
    let (prefix_index, dict) = build_fixture(
        "s2-best-absent",
        &[Row {
            toneless_key: "taigi",
            hanzi: "台語",
            tl: "tâi-gí",
            syll: 2,
            freq: 100,
        }],
    );
    assert!(
        best_candidate_for_key(
            "tl:zzz",
            (0, 3),
            &FrequencyMap::new(),
            0,
            &prefix_index,
            &dict,
            u32::MAX,
        )
        .is_none(),
        "absent key must return None"
    );
}

// ---------------------------------------------------------------------------
// v3.5.8 RC1 — `tl_abbrev` acronym collision must not surface in
// continuous span-local / walker fetch. Motivating bug: typing
// `ginalangtsiahpngbesai` surfaced 外夷 (`guā-î`) because its
// `tl_abbrev == "gi"` shares the `tl:gi` FST key with 語 (`gí`,
// real toneless "gi"). Normal-mode `lexicon::search` keeps acronym
// matching; continuous must not.
// ---------------------------------------------------------------------------

#[test]
fn continuous_drops_tl_abbrev_collision_keeps_genuine_toneless() {
    // Both rowids are indexed under `tl:gi`. 語 via real toneless;
    // 外夷 via its `tl_abbrev` ("gi") even though its toneless is
    // "guai". 外夷 is given a much higher freq so the pre-guard bug
    // (it dominating the strip and `best_candidate_for_key`) would be
    // obvious if the guard regressed.
    let (prefix_index, dict) = build_fixture(
        "rc1-abbrev",
        &[
            Row {
                toneless_key: "gi",
                hanzi: "語",
                tl: "gí",
                syll: 1,
                freq: 100,
            },
            Row {
                toneless_key: "gi",
                hanzi: "外夷",
                tl: "guā-î",
                syll: 2,
                freq: 5000,
            },
        ],
    );

    let out = fetch_candidates_for_endings(
        "gi",
        0,
        &[2],
        InputMode::Tl,
        &ctx_neutral(&FrequencyMap::new(), &prefix_index, &dict),
    );
    assert!(
        out.iter().any(|c| c.display_text == "語"),
        "genuine toneless 語 must be kept; got {out:#?}"
    );
    assert!(
        !out.iter().any(|c| c.display_text == "外夷"),
        "tl_abbrev collision 外夷 must be dropped; got {out:#?}"
    );

    // Walker edge provider: must pick the genuine record, never the
    // higher-freq acronym collision.
    let best = best_candidate_for_key(
        "tl:gi",
        (0, 2),
        &FrequencyMap::new(),
        0,
        &prefix_index,
        &dict,
        u32::MAX,
    )
    .expect("genuine toneless candidate exists");
    assert_eq!(
        best.display_text, "語",
        "best_candidate_for_key must skip the tl_abbrev collision"
    );
}
