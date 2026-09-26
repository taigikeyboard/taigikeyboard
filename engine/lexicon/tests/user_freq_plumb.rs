//! v3.5.8 continuous-input Phase 9.3a — `user_frequency.db` plumb contract.
//!
//! Hermetic regression for the `FrequencyMap` + `now_ms` plumb landed
//! in PR-9.3a. Pins the closed Gap B from
//! `docs/engine/continuous-input-ranking.md`:
//!
//! - `score` stays purely dictionary-derived (2026-09-14): a matching
//!   frequency row never changes it; the user signal lives only in
//!   `user_weight`.
//! - `user_weight` saturates at `ranking::MAX_BOOST − 1` — 100 selections
//!   weigh the same as 40, defending against stale-dominance.
//! - `RawCandidate.user_weight` (the leading `SortKey` user dim) is
//!   `ranking::decayed_user_weight_delta` of the entry — `> 0.0` for
//!   any selected word, still `> 0.0` past the old 1-hour window, and
//!   `0.0` for never-used entries.
//! - Cold-start (empty map + `now_ms = 0`) reproduces pre-9.3a
//!   behaviour byte-identically — a user with no frequency rows ranks
//!   exactly as the dictionary does.
//! - Clock skew (`now_ms < last_used_ms`) and `now_ms = 0` fall through
//!   to `user_weight = 0.0` so a misbehaving platform clock cannot
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
use lexicon::ContinuousFetchCtx;
use phonetics::InputMode;
use ranking::{FrequencyMap, BOOST_ALPHA, MAX_BOOST, USER_WEIGHT_DECAY_TAU_MS};

/// Length of the retired binary 1-hour recency window (epoch-ms). The
/// tests below pin that a selection older than it still counts.
const RETIRED_RECENCY_WINDOW_MS: i64 = 60 * 60 * 1000;

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
        learned: &[],
        prefix_index,
        dict,
        mode: phonetics::InputMode::Tl,
        tone_pin: lexicon::TonePin::None,
    }
}

mod common;
use common::{
    build_tkdb_v3, fetch_candidates_for_endings, frequency_map, write_temp, FrequencyFixture,
};

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
// 1. Score stays dictionary-only; user_weight grows with count and saturates.
// ---------------------------------------------------------------------------

#[test]
fn selections_leave_score_untouched_and_raise_user_weight() {
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

    // Cold start: empty map, score == freq, no user weight.
    let cold = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx(&FrequencyMap::new(), 0, &prefix_index, &dict),
    );
    assert!((cold[0].score - 100.0).abs() < 1e-4);
    assert_eq!(cold[0].user_weight, 0.0);

    // 10 fresh selections → score unchanged, user_weight = 10 × 0.1.
    let now_ms = 1_700_000_000_000_i64;
    let map_ten = frequency_map(&[FrequencyFixture {
        display_text_key: "台".into(),
        count: 10,
        last_used_ms: now_ms,
        canonical_tl: String::new(),
    }]);
    let warm = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx(&map_ten, now_ms, &prefix_index, &dict),
    );
    assert!((warm[0].score - 100.0).abs() < 1e-4);
    assert!((warm[0].user_weight - 1.0).abs() < 1e-6);
}

#[test]
fn user_weight_saturates_when_count_high() {
    // 100 selections would naively weigh 10.0; saturation pins the
    // weight at MAX_BOOST − 1 = 4.0.
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
    let now_ms = 1_700_000_000_000_i64;
    let map = frequency_map(&[FrequencyFixture {
        display_text_key: "台".into(),
        count: 100,
        last_used_ms: now_ms,
        canonical_tl: String::new(),
    }]);
    let out = fetch_candidates_for_endings(
        "tai",
        0,
        &[3],
        InputMode::Tl,
        &ctx(&map, now_ms, &prefix_index, &dict),
    );
    let expected = f64::from(MAX_BOOST) - 1.0; // 4.0
    assert!(
        (out[0].user_weight - expected).abs() < 1e-6,
        "expected saturated weight {expected}, got {}",
        out[0].user_weight
    );
}

// ---------------------------------------------------------------------------
// 2. user_weight — decayed selection weight; 0.0 on clock skew / no clock.
// ---------------------------------------------------------------------------

#[test]
fn user_weight_is_one_selection_delta_when_just_selected() {
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
    let map = frequency_map(&[FrequencyFixture {
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
    // One selection, ~no decay → delta ≈ BOOST_ALPHA.
    assert!((out[0].user_weight - f64::from(BOOST_ALPHA)).abs() < 1e-6);
}

#[test]
fn user_weight_persists_past_the_old_one_hour_window() {
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
    // Past the retired 1-hour recency window: the selection must still
    // count (the 2026-09-14 bug — 更新 sank below 警訊 after one hour).
    let last_used_ms = now_ms - RETIRED_RECENCY_WINDOW_MS;
    let map = frequency_map(&[FrequencyFixture {
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
    let expected = f64::from(BOOST_ALPHA)
        * (-(RETIRED_RECENCY_WINDOW_MS as f64) / USER_WEIGHT_DECAY_TAU_MS as f64).exp();
    // f32 boost arithmetic → ~2e-8 slack.
    assert!((out[0].user_weight - expected).abs() < 1e-6);
}

#[test]
fn user_weight_zero_when_clock_skew_now_before_last_used() {
    // Defensive: platform clock moved backwards. `user_weight` must
    // not turn a negative age into a boost.
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
    let map = frequency_map(&[FrequencyFixture {
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
    assert_eq!(out[0].user_weight, 0.0);
}

#[test]
fn user_weight_zero_when_now_ms_is_zero() {
    // Backward-compat with PR-9.2 platform builds that pass `now_ms = 0`
    // (no wall clock injected yet). Every entry must fall through to
    // weight 0.0 — no promotion without a clock, even if `last_used_ms`
    // is positive.
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
    let map = frequency_map(&[FrequencyFixture {
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
    assert_eq!(out[0].user_weight, 0.0);
}

// ---------------------------------------------------------------------------
// 3. End-to-end: user_weight reordering inside the same tier.
// ---------------------------------------------------------------------------

#[test]
fn selected_candidate_outranks_never_selected_within_same_tier_and_coverage() {
    // Two homophones under `tl:tai`, both Tier 0 (full-buffer `tai`),
    // same coverage. Identical raw freq. Only `user_weight` differs.
    // The selected one must surface first.
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
    let map = frequency_map(&[FrequencyFixture {
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
        "selected entry 「代」 must precede never-selected 「台」 inside same tier + coverage; \
         got {displays:?}"
    );
    assert!(out[0].user_weight > 0.0);
    assert_eq!(out[1].user_weight, 0.0);
}

#[test]
fn rare_selected_homophone_outranks_common_never_selected_after_hours() {
    // The 2026-09-14 bug shape (`kingsin` → 更新 freq 1 vs 敬神 freq 25):
    // a selected word 25× rarer than its never-selected homophone, last
    // picked two hours ago, must still lead — user preference is a
    // lexicographic dim above dictionary score, not a ×5-capped boost.
    let (prefix_index, dict) = build_fixture(
        "rare-selected-reorder",
        &[
            Row {
                toneless_key: "kingsin",
                hanzi: "敬神",
                tl: "kìng-sîn",
                syll: 2,
                freq: 25,
            },
            Row {
                toneless_key: "kingsin",
                hanzi: "更新",
                tl: "king-sin",
                syll: 2,
                freq: 1,
            },
        ],
    );
    let now_ms = 1_700_000_000_000_i64;
    let map = frequency_map(&[FrequencyFixture {
        display_text_key: "更新".into(),
        count: 1,
        last_used_ms: now_ms - 2 * RETIRED_RECENCY_WINDOW_MS,
        canonical_tl: "king-sin".into(),
    }]);
    let out = fetch_candidates_for_endings(
        "kingsin",
        0,
        &[7],
        InputMode::Tl,
        &ctx(&map, now_ms, &prefix_index, &dict),
    );
    let displays: Vec<&str> = out.iter().map(|c| c.display_text.as_str()).collect();
    assert_eq!(displays, vec!["更新", "敬神"], "got {displays:?}");
    // The capped boost alone could never do this: 1 × 1.1 × 1.1 < 27.5.
    assert!(out[0].score < out[1].score);
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
    // Every user_weight should be `0.0` (never) since `now_ms = 0`.
    assert!(
        out.iter().all(|c| c.user_weight == 0.0),
        "cold-start: every user_weight must be 0.0; got {:?}",
        out.iter().map(|c| c.user_weight).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// 5. Headline acceptance — full-buffer phrase + selection → still slot #1.
// ---------------------------------------------------------------------------

#[test]
fn mismatched_display_text_key_leaves_score_neutral() {
    // Codex post-impl P3 #3: a frequency row whose `display_text_key`
    // does NOT match any fetched candidate's `display_text` must leave
    // every candidate at the cold-start neutral baseline. Pins the
    // contract that `record_to_candidate` only applies the weight when
    // the key actually hits — silent drops on miss.
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
    let map = frequency_map(&[FrequencyFixture {
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
    assert!((out[0].score - 100.0).abs() < 1e-4);
    // No matching entry → user_weight stays 0.0.
    assert_eq!(out[0].user_weight, 0.0);
}

#[test]
fn duplicate_keys_in_freq_map_apply_last_write_winner_to_candidate() {
    // Codex post-impl P3 #3: integration-level pin for the duplicate-
    // key last-write-wins policy documented at
    // `ranking::FrequencyMap`. Two entries for the same display
    // text — the latter (`count = 7`) must win and reach
    // `record_to_candidate`: user_weight = 7 × 0.1 (fresh).
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
    let map = frequency_map(&[
        FrequencyFixture {
            display_text_key: "台".into(),
            count: 1, // would yield user_weight 0.1
            last_used_ms: 1_700_000_000_000,
            canonical_tl: String::new(),
        },
        FrequencyFixture {
            display_text_key: "台".into(),
            count: 7, // last-write-winner: user_weight 0.7
            last_used_ms: 1_700_000_001_000,
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
    let expected = 7.0 * f64::from(BOOST_ALPHA);
    assert!(
        (out[0].user_weight - expected).abs() < 1e-6,
        "expected last-write user_weight {expected}, got {}",
        out[0].user_weight
    );
}

#[test]
fn taiuantaigi_phrase_keeps_slot_one_when_selected() {
    // Phase 9 motivating case + user-freq plumb combined: 「臺灣台語」
    // already wins on Tier 0; selections leave its dictionary score
    // untouched and give it a positive user weight. Slot #1 must still
    // be 「臺灣台語」.
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
    let map = frequency_map(&[FrequencyFixture {
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
    // 4-syll bias 1.3 × freq 12 = 15.6 — no user term in the score.
    let expected = 12.0_f32 * 1.3;
    assert!(
        (out[0].score - expected).abs() < 1e-3,
        "expected dictionary-only score {expected}, got {}",
        out[0].score
    );
    assert!(out[0].user_weight > 0.0, "selected → positive user weight");
}
