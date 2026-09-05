//! Lexicon ranking — candidate dedup, scoring, sorting.
//!
//! Pure-CPU stateless RPC. No I/O, no time reads inside the crate (caller
//! supplies `now_ms`), no logging on the hot path. Public entry point is
//! [`process_candidates`]; implementation modules are crate-private.
//!
//! # Cross-platform invariants
//!
//! Single source of truth for the score / dedup / sort math. The Android
//! `CandidateProcessor.kt` mirror was deleted via PR #192 (v3.5.3 Path G);
//! iOS retains only a 4-LOC residual (`capitalize` 1-line bridge +
//! `startsWithRomanLetter` predicate, see
//! `migration-inventory.csv` row for `CandidateProcessor.swift`). Both
//! production paths route through this crate via FFI.
//!
//! Score formula constants (`USER_FREQ_CAP=100`, `USER_FREQ_WEIGHT=100`,
//! `RECENCY_WINDOW_MS=3_600_000`, `RECENCY_BONUS=200`, `EXACT_BONUS=100`,
//! `COMPLETION_PENALTY=-1000`, `CLOSENESS_WEIGHT=500`,
//! `BASE_FREQ_DIVISOR=10`, `SOURCE_TIERS`, `TIER_DENOMINATOR=10`) are
//! pinned here and were verified byte-identical across platforms at the
//! v3.5.2 ranking-slice audit (`docs/engine/sort.md` documents the behaviour).

mod dedup;
mod process;
mod score;
mod sort;

pub use process::process_candidates;
pub use score::{
    build_frequency_map, calculate_continuous_score, decayed_user_weight_delta, recency_rank,
    source_tier_rank, user_freq_boost, FrequencyData, FrequencyMap, BOOST_ALPHA,
    CONTINUOUS_DEFAULT_SOURCE_RANK, MAX_BOOST, RECENCY_WINDOW_MS, USER_WEIGHT_DECAY_TAU_MS,
};
