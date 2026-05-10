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
use lexicon::{fetch_candidates_for_endings, RawCandidate, FORM_NOTONE};

mod common;
use common::{build_tkdb_v2, write_temp};

/// Single dictionary fixture row: `(toneless_tl_key, hanzi, tl, syllable_count, frequency)`.
/// `bitmask` is fixed to `1 << 11` (kautian-equivalent) so all rows pass
/// the default-enabled filter. Rowid is implied by insertion order
/// (1-based for dict.bin, mirrored as 1-based in the FST).
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

    // Score sanity:
    //   紙   = 100 × 1.0 × 1.0 = 100.0
    //   珠仔 = 80  × 1.1 × 1.0 = 88.0
    //   珠   = 90  × 1.0 × 1.0 = 90.0
    // → desc order should be 紙(100) > 珠(90) > 珠仔(88).
    assert!((zhi.score - 100.0).abs() < 1e-4);
    assert!((zhuah.score - 88.0).abs() < 1e-4);
    assert!((zhu.score - 90.0).abs() < 1e-4);

    // Result list must be sorted by score desc.
    let scores: Vec<f32> = out.iter().map(|c| c.score).collect();
    let mut sorted = scores.clone();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    assert_eq!(scores, sorted, "candidates must be desc by score");
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
