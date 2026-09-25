//! Continuous-input ranking primitives — source rank, user-frequency
//! boost + decay, `(display_text, canonical_tl)` frequency map, and the
//! dictionary-derived score.
//!
//! Pure-CPU, stateless. No I/O, no time reads inside the crate (caller
//! supplies `now_ms`), no logging on the hot path. Consumed by
//! `lexicon::continuous` and `composing`; the implementation module is
//! crate-private.
//!
//! Constants (`BOOST_ALPHA`, `MAX_BOOST`, `USER_WEIGHT_DECAY_TAU_MS`,
//! `CONTINUOUS_DEFAULT_SOURCE_RANK`) are the cross-platform single source
//! of truth — platforms never redefine them.

mod score;

pub use score::{
    build_frequency_map, calculate_continuous_score, decayed_user_weight_delta, source_tier_rank,
    user_freq_boost, FrequencyData, FrequencyMap, BOOST_ALPHA, CONTINUOUS_DEFAULT_SOURCE_RANK,
    MAX_BOOST, USER_WEIGHT_DECAY_TAU_MS,
};
