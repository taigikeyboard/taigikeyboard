//! v3.5.8 連續輸入 (Continuous Input) Phase 5 — span-local candidate fetch
//! contract test.
//!
//! Pins the three roadmap-mandated cases (`docs/roadmap.md:339-341`):
//!
//! 1. `tsua` with endings={3, 4} must surface
//!    `紙(span=(0,4), syll=1)` + `珠仔(span=(0,4), syll=2)` + `珠(span=(0,3), syll=1)`
//!    — the "multi-syll same toneless key" + "shorter-span shadow" combo
//!    that motivates the whole BFS-with-multi-cut approach
//!    (`docs/roadmap.md:203`).
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
//! `tests/common/mod.rs::build_tkdb_v2`.

// 中文: Phase 5 fetch_candidates_for_endings 契約測試 — 鎖 roadmap §Phase 5 三條 case (tsua / taigikhipuann / taixyz)。

use std::path::PathBuf;

use fst::SetBuilder;
use lexicon::dictionary_reader::DictionaryReader;
use lexicon::prefix_index::PrefixIndex;
use lexicon::{fetch_candidates_for_endings, CandidateMode, RawCandidate, FORM_NOTONE};

mod common;
use common::{build_tkdb_v2, write_temp};

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
    let dict_bytes = build_tkdb_v2(b"TKDB", &dict_rows);
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
        &[3, 4],  // syllabifier emits span=3 (`tsu`) and span=4 (`tsua`).
        u32::MAX, // all sources enabled
        1.0,      // no user-freq boost
        &prefix_index,
        &dict,
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

    // v3.5.8 Phase 9.1 SortKey expected order (per
    // `docs/roadmap.md` § Phase 9 sort_key formula):
    //   (tier, -coverage_bytes, recency_rank, -adjusted_score, …)
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
        u32::MAX,
        1.0,
        &prefix_index,
        &dict,
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

    let out = fetch_candidates_for_endings("taixyz", 0, &[3], u32::MAX, 1.0, &prefix_index, &dict);

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
    let out = fetch_candidates_for_endings("tai", 0, &[], u32::MAX, 1.0, &prefix_index, &dict);
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
    let out = fetch_candidates_for_endings("tai", 3, &[3], u32::MAX, 1.0, &prefix_index, &dict);
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
    let out =
        fetch_candidates_for_endings("tai", 0, &[3, 99, 100], u32::MAX, 1.0, &prefix_index, &dict);
    assert_eq!(out.len(), 1, "only ending=3 valid: {out:#?}");
    assert_eq!(out[0].display_text, "台");
}

#[test]
fn nan_boost_does_not_break_descending_order() {
    // Contract violation: caller passes NaN. The sort comparator must
    // coerce NaN to f32::MIN so the descending-score invariant holds
    // (see continuous.rs sort_by). Result is implementation-defined
    // (NaN-scored candidates sink to the end) but never panics or
    // returns unsorted output.
    let (prefix_index, dict) = build_fixture(
        "nan-defense",
        &[
            Row {
                toneless_key: "tai",
                hanzi: "台",
                tl: "tâi",
                syll: 1,
                freq: 100,
            },
            Row {
                toneless_key: "tai",
                hanzi: "颱",
                tl: "thai",
                syll: 1,
                freq: 50,
            },
        ],
    );
    let out =
        fetch_candidates_for_endings("tai", 0, &[3], u32::MAX, f32::NAN, &prefix_index, &dict);
    // No panic; some deterministic ordering exists. We don't assert on
    // values because NaN multiplies poison everything; we just assert
    // the function returns and the sort completes.
    assert_eq!(out.len(), 2, "both rows still surface");
}

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
        u32::MAX,
        1.0,
        &prefix_index,
        &dict,
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
    let out =
        fetch_candidates_for_endings("tai1bak4", 0, &[4, 8], u32::MAX, 1.0, &prefix_index, &dict);
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
    // The syllabifier can't walk past `-` (the inventory has no
    // hyphenated entries), so hyphen-input handling is deferred to
    // Phase 9. This test pins the contract: feeding a hyphenated
    // segment into `fetch_candidates_for_endings` produces
    // `tl:tai-bak` (not `tl:taibak`), so a fixture that only stores
    // `tl:taibak` returns NO candidates. If anyone re-adds the hyphen
    // half of the strip, this test fails.
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
        u32::MAX,
        1.0,
        &prefix_index,
        &dict,
    );
    assert!(
        out.is_empty(),
        "hyphen MUST NOT be stripped at the lexicon strip layer; \
         got {out:#?} (likely a regression in `fetch_candidates_for_endings` \
         strip rule)",
    );
}

#[test]
fn user_freq_boost_amplifies_score_multiplicatively() {
    // boost=2.0 must double the score relative to boost=1.0 (which is
    // the noop baseline). Cross-checks the wiring between
    // fetch_candidates_for_endings and ranking::calculate_continuous_score.
    let (prefix_index, dict) = build_fixture(
        "boost",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 100,
        }],
    );
    let baseline =
        fetch_candidates_for_endings("tai", 0, &[3], u32::MAX, 1.0, &prefix_index, &dict);
    let boosted = fetch_candidates_for_endings("tai", 0, &[3], u32::MAX, 2.0, &prefix_index, &dict);
    assert!((baseline[0].score - 100.0).abs() < 1e-4);
    assert!((boosted[0].score - 200.0).abs() < 1e-4);
}

// ---------------------------------------------------------------------------
// v3.5.8 Phase 9.1 — Regression matrix (`taiuantaigi` / `e` / `taixyz`)
// pinned by `docs/roadmap.md` § Phase 9 「回歸守護矩陣」.
//
// These cases use synthetic dict fixtures that mirror the frequency
// disparity that drove the Phase 9 pivot (`docs/engine/continuous-input-
// ranking.md` §3.1): single-char freq orders of magnitude above
// multi-syllable phrases. The SortKey must surface the full-buffer
// phrase in slot #1 regardless of that disparity.
// ---------------------------------------------------------------------------

// 中文: Phase 9.1 回歸守護矩陣 — hermetic 重現 `taiuantaigi` 排序失敗場景,鎖死 Tier 1 政策。

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
        u32::MAX,
        1.0,
        &prefix_index,
        &dict,
    );

    let display_order: Vec<&str> = out.iter().map(|c| c.display_text.as_str()).collect();
    assert_eq!(
        display_order,
        vec!["臺灣台語", "台灣", "台"],
        "Tier 1 (full-buffer) 「臺灣台語」 must outrank Tier 2 partials \
         even though its score (15.6) is ~2000x lower than 「台」 (31281)"
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

    let out = fetch_candidates_for_endings("e", 0, &[1], u32::MAX, 1.0, &prefix_index, &dict);

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

    let out = fetch_candidates_for_endings("taixyz", 0, &[3], u32::MAX, 1.0, &prefix_index, &dict);

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
    // SortKey dimensions 0-5 (tier, coverage, recency_rank, adjusted_
    // score, raw freq, source_rank). Only `stable_idx` differentiates.
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
            Row {
                toneless_key: "tai",
                hanzi: "一",
                tl: "tai-a",
                syll: 1,
                freq: 100,
            },
            Row {
                toneless_key: "tai",
                hanzi: "二",
                tl: "tai-b",
                syll: 1,
                freq: 100,
            },
            Row {
                toneless_key: "tai",
                hanzi: "三",
                tl: "tai-c",
                syll: 1,
                freq: 100,
            },
        ],
    );

    let out = fetch_candidates_for_endings("tai", 0, &[3], u32::MAX, 1.0, &prefix_index, &dict);

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
    let out = fetch_candidates_for_endings("tai", 0, &[3], u32::MAX, 1.0, &prefix_index, &dict);
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
    // 中文: Phase 9.2 mode 端對端契約;HANT / TAILO (hanzi=None via len=0) / MIXED 三種來源全跑過 record_to_candidate。
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

    let hant = fetch_candidates_for_endings("tai", 0, &[3], u32::MAX, 1.0, &prefix_index, &dict);
    assert_eq!(hant.len(), 1);
    assert_eq!(hant[0].mode, CandidateMode::Hant);
    assert_eq!(hant[0].display_text, "台");

    let tailo = fetch_candidates_for_endings("li", 0, &[2], u32::MAX, 1.0, &prefix_index, &dict);
    assert_eq!(tailo.len(), 1);
    assert_eq!(
        tailo[0].mode,
        CandidateMode::Tailo,
        "empty hanzi (None) must derive TAILO; display_text falls back to TL"
    );
    assert_eq!(tailo[0].display_text, "lí");

    let mixed = fetch_candidates_for_endings("iausi", 0, &[5], u32::MAX, 1.0, &prefix_index, &dict);
    assert_eq!(mixed.len(), 1);
    assert_eq!(
        mixed[0].mode,
        CandidateMode::Mixed,
        "hanzi containing Latin letter (NFKD-normalized) must derive MIXED"
    );
    assert_eq!(mixed[0].display_text, "iáu是");
}
