//! Continuous-input ranking primitives — source rank, user-frequency
//! boost + decay, `(display_text, canonical_tl)` frequency map, the
//! dictionary-derived score, and the candidate sort key.
//!
//! Pure-CPU. No I/O, no time reads inside the crate (caller supplies
//! `now_ms`), no logging on the hot path. Consumed by `lexicon::continuous`
//! and `composing`; the implementation modules are crate-private.
//!
//! [`set_dev_sort_order`] and [`set_dev_rank_one_word_path`] are the two
//! pieces of state: a thread-local comparison order, and whether a one-word
//! best path joins that comparison. The web developer page sets them.
//! Unset, every comparison is the field order of [`CandidateSortKey`] and a
//! one-word path stays ahead of it. Official input methods never set either.
//!
//! Constants (`BOOST_ALPHA`, `MAX_BOOST`, `USER_WEIGHT_DECAY_TAU_MS`,
//! `CONTINUOUS_DEFAULT_SOURCE_RANK`) are the cross-platform single source
//! of truth — platforms never redefine them.

mod context;
mod score;
mod sort_key;

pub use context::{ContextRanks, CONTEXT_RANK_BUNDLED, CONTEXT_RANK_NONE, CONTEXT_RANK_USER};
pub use score::{
    calculate_continuous_score, decayed_user_weight_delta, source_tier_rank, user_freq_boost,
    FrequencyData, FrequencyMap, BOOST_ALPHA, CONTINUOUS_DEFAULT_SOURCE_RANK, MAX_BOOST,
    USER_WEIGHT_DECAY_TAU_MS,
};
pub use sort_key::{
    candidate_lead, dev_rank_one_word_path, reset_dev_sort_order, set_dev_rank_one_word_path,
    set_dev_sort_order, sort_by_candidate_key, CandidateRankFacts, CandidateSortKey,
    COVERAGE_KIND_ABBREV, COVERAGE_KIND_FULL, COVERAGE_KIND_PARTIAL_PREFIX, WALKER_COST_UNPRICED,
};
