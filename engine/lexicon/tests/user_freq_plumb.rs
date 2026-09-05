//! v3.5.8 continuous-input Phase 9.3a — `user_frequency.db` plumb contract.
//!
//! Hermetic regression for the `FrequencyMap` + `now_ms` plumb landed
//! in PR-9.3a. Pins the closed Gap B from
//! `docs/engine/continuous-input-ranking.md`:
//!
//! - Non-empty `FrequencyMap` raises the matching candidate's score via
//!   `ranking::user_freq_boost(count)`.
//! - Boost saturates at `ranking::MAX_BOOST` (5.0) — 100 selections
//!   produce the same boost as 40, defending against stale-dominance.
//! - `SortKey.recency_rank` flips to `0` when the entry was selected
//!   strictly inside `RECENCY_WINDOW_MS`, and back to `1` for stale /
//!   never-used entries.
//! - Cold-start (empty map + `now_ms = 0`) reproduces pre-9.3a
//!   behaviour byte-identically — backward-compatible with PR-9.2
//!   platform builds that have not wired user-frequency snapshots.
//! - Clock skew (`now_ms < last_used_ms`) and `now_ms = 0` fall through
//!   to `recency_rank = 1` so a misbehaving platform clock cannot
//!   falsely promote stale entries.
//!
//! Fixture builder mirrors `tests/span_local_fetch.rs` (1:1 with the
//! shared `tests/common::build_tkdb_v3` helper). Phase 9.1 + 9.2
//! invariants (Tier 1 ordering, mode derive) are pinned in that file;
//! this file scopes to 9.3a-specific axes.

use std::path::PathBuf;

use fst::SetBuilder;
use lexicon::dictionary_reader::DictionaryReader;
use lexicon::prefix_index::PrefixIndex;
use lexicon::{fetch_candidates_for_endings, ContinuousFetchCtx};
use phonetics::InputMode;
use protos::engine::FrequencyEntry;
use ranking::{build_frequency_map, FrequencyMap, MAX_BOOST, RECENCY_WINDOW_MS};

/// v3.5.9 D7 — collapse the six-arg ctx into one literal per test
/// site. `fetch_candidates_for_endings` is the test-only entry per D8;
/// it always forces `custom = &[]` internally, so this helper omits
/// the custom field. `enabled_sources_bitmask = u32::MAX` matches
/// every other 9.3a test (filter narrowing is covered in
/// `span_local_fetch.rs`).
fn ctx<'a>(
    freq_map: &'a FrequencyMap,
    now_ms: i64,
    prefix_index: &'a PrefixIndex,
    dict: &'a DictionaryReader,
) -> ContinuousFetchCtx<'a> {
    // v3.5.9 B-4 — `mode` defaults to `Tl`; this suite pins TL freq
    // plumbing and never hits the canonicalize path (`custom = &[]`
    // forces the only B-4-touched site, `custom_entry_to_candidate`,
    // unreachable).
    ContinuousFetchCtx {
        enabled_sources_bitmask: u32::MAX,
        freq_map,
        now_ms,
        custom: &[],
        prefix_index,
        dict,
        mode: phonetics::InputMode::Tl,
        tps_space_pinned_body: None,
    }
}

mod common;
use common::{build_tkdb_v3, write_temp};

struct Row<'a> {
    toneless_key: &'a str,
    hanzi: &'a str,
    tl: &'a str,
    syll: u8,
    freq: u32,
}

fn build_fixture(name: &str, rows: &[Row<'_>]) -> (PrefixIndex, DictionaryReader) {
    let dict_rows: Vec<(u16, u32, u8, &str, &str)> = rows
        .iter()
        .map(|r| (1u16 << 11, r.freq, r.syll, r.hanzi, r.tl))
        .collect();
    let dict_bytes = build_tkdb_v3(b"TKDB", &dict_rows);
    let dict_path = write_temp(&format!("phase9-3a-{name}.dict.bin"), &dict_bytes);
    let dict = DictionaryReader::open(&dict_path).expect("dict.bin opens");

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
    std::env::temp_dir().join(format!("lexicon-test-phase9-3a-{name}-{pid}-{n}.fst"))
}

// ---------------------------------------------------------------------------
// 1. Boost amplification — single candidate, increasing selection counts.
// ---------------------------------------------------------------------------

#[test]
fn user_freq_boost_amplifies_score_for_matched_candidate() {
    let (prefix_index, dict) = build_fixture(
        "boost-amplifies",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 100,
        }],
    );

    // Cold start: empty map, neutral boost, score == freq.
    let cold = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx(&FrequencyMap::new(), 0, &prefix_index, &dict),
    );
    assert!((cold[0].score - 100.0).abs() < 1e-4);

    // 10 selections → boost = 1.0 + 10×0.1 = 2.0 → score = 200.0.
    let map_ten = build_frequency_map(&[FrequencyEntry {
        display_text_key: "台".into(),
        count: 10,
        last_used_ms: 1,
        canonical_tl: String::new(),
    }]);
    let warm = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx(&map_ten, 1_000_000_000_000, &prefix_index, &dict),
    );
    assert!((warm[0].score - 200.0).abs() < 1e-4);
}

#[test]
fn user_freq_boost_saturates_at_max_boost_when_count_high() {
    // 100 selections would naively produce boost = 11.0 → score = 1100.
    // Saturation pins it at MAX_BOOST = 5.0 → score = 500.0.
    let (prefix_index, dict) = build_fixture(
        "boost-saturates",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 100,
        }],
    );
    let map = build_frequency_map(&[FrequencyEntry {
        display_text_key: "台".into(),
        count: 100,
        last_used_ms: 1,
        canonical_tl: String::new(),
    }]);
    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx(&map, 1_000_000_000_000, &prefix_index, &dict),
    );
    let expected = 100.0_f32 * MAX_BOOST; // 500.0
    assert!(
        (out[0].score - expected).abs() < 1e-3,
        "expected saturated score {expected}, got {}",
        out[0].score
    );
}

// ---------------------------------------------------------------------------
// 2. Recency_rank — flips to 0 inside the window, 1 outside / clock skew.
// ---------------------------------------------------------------------------

#[test]
fn recency_rank_zero_when_last_used_is_within_window() {
    let (prefix_index, dict) = build_fixture(
        "recency-fresh",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 100,
        }],
    );
    let now_ms = 1_700_000_000_000_i64;
    let last_used_ms = now_ms - 1_000; // 1 second ago.
    let map = build_frequency_map(&[FrequencyEntry {
        display_text_key: "台".into(),
        count: 1,
        last_used_ms,
        canonical_tl: String::new(),
    }]);
    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx(&map, now_ms, &prefix_index, &dict),
    );
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].recency_rank, 0);
}

#[test]
fn recency_rank_one_when_last_used_is_outside_window() {
    let (prefix_index, dict) = build_fixture(
        "recency-stale",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 100,
        }],
    );
    let now_ms = 1_700_000_000_000_i64;
    // Strictly past the 1-hour window.
    let last_used_ms = now_ms - RECENCY_WINDOW_MS;
    let map = build_frequency_map(&[FrequencyEntry {
        display_text_key: "台".into(),
        count: 1,
        last_used_ms,
        canonical_tl: String::new(),
    }]);
    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx(&map, now_ms, &prefix_index, &dict),
    );
    assert_eq!(out[0].recency_rank, 1);
}

#[test]
fn recency_rank_one_when_clock_skew_now_before_last_used() {
    // Defensive: platform clock moved backwards. `recency_rank` must
    // not wrap a negative delta into the recency comparison.
    let (prefix_index, dict) = build_fixture(
        "recency-skew",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 100,
        }],
    );
    let last_used_ms = 1_700_000_000_000_i64;
    let now_ms = last_used_ms - 5_000; // 5 seconds before recorded selection.
    let map = build_frequency_map(&[FrequencyEntry {
        display_text_key: "台".into(),
        count: 1,
        last_used_ms,
        canonical_tl: String::new(),
    }]);
    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx(&map, now_ms, &prefix_index, &dict),
    );
    assert_eq!(out[0].recency_rank, 1);
}

#[test]
fn recency_rank_one_when_now_ms_is_zero() {
    // Backward-compat with PR-9.2 platform builds that pass `now_ms = 0`
    // (no wall clock injected yet). Every entry must fall through to
    // rank 1 — `recency_rank=0` should never appear with `now_ms=0`,
    // even if `last_used_ms` is positive.
    let (prefix_index, dict) = build_fixture(
        "recency-now-zero",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 100,
        }],
    );
    let map = build_frequency_map(&[FrequencyEntry {
        display_text_key: "台".into(),
        count: 1,
        last_used_ms: 1_700_000_000_000,
        canonical_tl: String::new(),
    }]);
    // platform shim has not injected a clock yet → now_ms = 0
    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx(&map, 0, &prefix_index, &dict),
    );
    assert_eq!(out[0].recency_rank, 1);
}

// ---------------------------------------------------------------------------
// 3. End-to-end: recency_rank reordering inside the same tier.
// ---------------------------------------------------------------------------

#[test]
fn recent_candidate_outranks_stale_within_same_tier_and_coverage() {
    // Two homophones under `tl:tai`, both Tier 0 (full-buffer `tai`),
    // same coverage. Identical raw freq. Only `recency_rank` differs.
    // The recent one must surface first.
    let (prefix_index, dict) = build_fixture(
        "recency-reorder",
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
                hanzi: "代",
                tl: "tāi",
                syll: 1,
                freq: 100,
            },
        ],
    );
    let now_ms = 1_700_000_000_000_i64;
    // 「代」 selected 5 minutes ago → recent. 「台」 has no entry → stale.
    let map = build_frequency_map(&[FrequencyEntry {
        display_text_key: "代".into(),
        count: 1,
        last_used_ms: now_ms - 5 * 60 * 1_000,
        canonical_tl: String::new(),
    }]);
    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx(&map, now_ms, &prefix_index, &dict),
    );
    assert_eq!(out.len(), 2);
    let displays: Vec<&str> = out.iter().map(|c| c.display_text.as_str()).collect();
    assert_eq!(
        displays,
        vec!["代", "台"],
        "recent entry 「代」 must precede stale 「台」 inside same tier + coverage; \
         got {displays:?}"
    );
    assert_eq!(out[0].recency_rank, 0);
    assert_eq!(out[1].recency_rank, 1);
}

// ---------------------------------------------------------------------------
// 4. Cold-start backward compatibility (= PR-9.2 behaviour).
// ---------------------------------------------------------------------------

#[test]
fn empty_freq_map_with_zero_now_matches_pre_9_3a_behaviour() {
    // The motivating Phase 9 case: `taiuantaigi`. PR-9.1 ranks
    // 「臺灣台語」 first via the Tier 0 rule even without user-frequency
    // plumbing. This test's real intent is cold-start parity: an empty
    // `FrequencyMap` + `now_ms = 0` must produce a deterministic order
    // with every recency rank stale.
    //
    // v3.5.8 整句 lattice + walker S8: the headline (Tier 0 phrase #1,
    // all recency = 1) is unchanged. Only the within-Tier-1 sub-order
    // flipped — coverage was demoted below freq, so the higher-freq
    // short 「台」 (31281) now precedes the lower-freq longer 「台灣」
    // (1379). Cold-start parity itself is intact (no user-freq effect).
    let (prefix_index, dict) = build_fixture(
        "cold-start-parity",
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
        &[3, 6, 11],
        InputMode::Tl,
        &ctx(&FrequencyMap::new(), 0, &prefix_index, &dict),
    );
    let displays: Vec<&str> = out.iter().map(|c| c.display_text.as_str()).collect();
    assert_eq!(
        displays,
        vec!["臺灣台語", "台", "台灣"],
        "cold-start ordering: Tier 0 phrase first; within Tier 1, \
         post-S8 higher-freq short 「台」 precedes lower-freq longer 「台灣」"
    );
    // All recency ranks should be `1` (stale/never) since `now_ms = 0`.
    assert!(
        out.iter().all(|c| c.recency_rank == 1),
        "cold-start: every recency_rank must be 1; got {:?}",
        out.iter().map(|c| c.recency_rank).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// 5. Headline acceptance — full-buffer phrase + selection → still slot #1.
// ---------------------------------------------------------------------------

#[test]
fn mismatched_display_text_key_leaves_score_neutral() {
    // Codex post-impl P3 #3: a `FrequencyEntry` whose `display_text_key`
    // does NOT match any fetched candidate's `display_text` must leave
    // every candidate at the cold-start neutral baseline. Pins the
    // contract that `record_to_candidate` only applies the boost when
    // the key actually hits — silent drops on miss, not "boost
    // anyway".
    let (prefix_index, dict) = build_fixture(
        "mismatched-key",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 100,
        }],
    );
    // Entry exists but for a different display text.
    let map = build_frequency_map(&[FrequencyEntry {
        display_text_key: "完全不一樣".into(),
        count: 50,
        last_used_ms: 1_700_000_000_000,
        canonical_tl: String::new(),
    }]);
    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx(&map, 1_700_000_000_500, &prefix_index, &dict),
    );
    assert_eq!(out.len(), 1);
    // boost = 1.0 → score == freq.
    assert!((out[0].score - 100.0).abs() < 1e-4);
    // No matching entry → recency_rank stays 1.
    assert_eq!(out[0].recency_rank, 1);
}

#[test]
fn duplicate_keys_in_freq_map_apply_last_write_winner_to_candidate() {
    // Codex post-impl P3 #3: integration-level pin for the duplicate-
    // key last-write-wins policy documented at
    // `ranking::build_frequency_map`. Two entries for the same display
    // text — the latter (`count = 7`) must win and reach
    // `record_to_candidate`. boost(7) = 1.7 → score = freq × 1.7.
    let (prefix_index, dict) = build_fixture(
        "duplicate-keys",
        &[Row {
            toneless_key: "tai",
            hanzi: "台",
            tl: "tâi",
            syll: 1,
            freq: 100,
        }],
    );
    let map = build_frequency_map(&[
        FrequencyEntry {
            display_text_key: "台".into(),
            count: 1, // would yield boost 1.1 → score 110.0
            last_used_ms: 1_700_000_000_000,
            canonical_tl: String::new(),
        },
        FrequencyEntry {
            display_text_key: "台".into(),
            count: 7, // last-write-winner: boost 1.7 → score 170.0
            last_used_ms: 1_700_000_000_500,
            canonical_tl: String::new(),
        },
    ]);
    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx(&map, 1_700_000_001_000, &prefix_index, &dict),
    );
    assert_eq!(out.len(), 1);
    let expected = 100.0_f32 * (1.0 + 7.0 * 0.1);
    assert!(
        (out[0].score - expected).abs() < 1e-3,
        "expected last-write boost score {expected}, got {}",
        out[0].score
    );
}

#[test]
fn taiuantaigi_phrase_keeps_slot_one_when_boosted() {
    // Phase 9 motivating case + Phase 9.3a user-freq plumb combined:
    // 「臺灣台語」 already wins on Tier 1; one selection raises its
    // score even further. Slot #1 must still be 「臺灣台語」 and its
    // boosted score must be strictly larger than the cold-start
    // baseline.
    let (prefix_index, dict) = build_fixture(
        "taiuantaigi-boosted",
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
    let now_ms = 1_700_000_000_000_i64;
    let map = build_frequency_map(&[FrequencyEntry {
        display_text_key: "臺灣台語".into(),
        count: 3,
        last_used_ms: now_ms - 30_000, // 30 seconds ago.
        canonical_tl: String::new(),
    }]);
    let out = fetch_candidates_for_endings(
        "taiuantaigi",
        0,
        &[3, 6, 11],
        InputMode::Tl,
        &ctx(&map, now_ms, &prefix_index, &dict),
    );
    assert_eq!(out[0].display_text, "臺灣台語");
    // 4-syll bias 1.3 × freq 12 × user_freq_boost(3) = 1.3 × 1.3 × 12 = 20.28.
    let expected = 12.0_f32 * 1.3 * 1.3;
    assert!(
        (out[0].score - expected).abs() < 1e-3,
        "expected boosted score {expected}, got {}",
        out[0].score
    );
    assert_eq!(out[0].recency_rank, 0, "selected within window → recent");
}
