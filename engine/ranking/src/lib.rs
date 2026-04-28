//! Lexicon ranking — candidate dedup, scoring, sorting.
//!
//! Pure-CPU stateless RPC. No I/O, no time reads inside the crate (caller
//! supplies `now_ms`), no logging on the hot path. Public entry point is
//! [`process_candidates`]; implementation modules are crate-private.
//!
//! # Cross-platform invariants
//!
//! Mirrors `ios/Sources/TaigiKeyboard/Lexicon/Utils/CandidateProcessor.swift`
//! and `android/.../ime/dictionary/CandidateProcessor.kt`. The Kotlin /
//! Swift sides retain platform copies of `removeDuplicates`,
//! `removeDisplayDuplicates`, `calculateScore`, `sortByScore`,
//! `romanToBase`, `inputToBase` so JVM unit tests in `src/test/` can
//! exercise the math without loading `librust_taigi.so`. Production paths
//! route through this crate.
//!
//! Score formula constants (`USER_FREQ_CAP=100`, `USER_FREQ_WEIGHT=100`,
//! `RECENCY_WINDOW_MS=3_600_000`, `RECENCY_BONUS=200`, `EXACT_BONUS=100`,
//! `COMPLETION_PENALTY=-1000`, `CLOSENESS_WEIGHT=500`,
//! `BASE_FREQ_DIVISOR=10`, `SOURCE_TIERS`, `TIER_DENOMINATOR=10`) are
//! pinned by `docs/engine/ranking-slice-audit.md` § 1.3 and verified
//! byte-identical across platforms at audit time.

mod dedup;
mod nfd;
mod process;
mod score;
mod sort;

pub use process::process_candidates;
