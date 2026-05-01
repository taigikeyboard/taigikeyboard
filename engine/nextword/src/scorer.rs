//! Pure NextWord scoring math. Mirrors iOS `NextWordScorer.swift` /
//! Android `ime/core/nextword/NextWordScorer.kt`. Constants pinned by
//! `behavioral-invariants.md` §§7, 8 — modifying any value here requires
//! invariant-doc + parity-test updates in the same PR.

/// Source weight applied to user-learned bigrams.
pub(crate) const USER_WEIGHT: f64 = 50.0;

/// Source weight applied to dictionary bigrams.
pub(crate) const DICT_WEIGHT: f64 = 1.0;

/// RIME-style exponential decay half-life (1 week).
pub(crate) const DECAY_HALF_LIFE_HOURS: f64 = 168.0;

/// Additive bonus ensuring user entries outrank dict entries.
pub(crate) const LEARNING_BONUS: f64 = 300.0;

/// Minimum decay retained for frequently-used entries (count >= threshold).
pub(crate) const HIGH_USAGE_DECAY_FLOOR: f64 = 0.95;

/// Minimum decay retained for infrequent entries.
pub(crate) const LOW_USAGE_DECAY_FLOOR: f64 = 0.30;

/// Usage count above which the high-usage floor kicks in.
pub(crate) const HIGH_USAGE_THRESHOLD: i64 = 3;

/// Hard-coded `ln(2)` literal (3-digit). Constant must match iOS inline
/// `0.693` literal at `NextWordScorer.swift:62` and Android
/// `NextWordScorer.kt:46` `LN_2 = 0.693`. Drift over a 1-week window is
/// ~0.03% — acceptable per the Android scorer doc comment.
pub(crate) const LN_2: f64 = 0.693;

/// Dictionary-layer score: raw count × `DICT_WEIGHT`.
pub(crate) fn score_dict(count: i64) -> f64 {
    count as f64 * DICT_WEIGHT
}

/// Exponential time decay: `decay = exp(-ageHours / halfLifeHours * ln(2))`.
/// Recent usage ≈ 1.0; one week out ≈ 0.5; one month out ≈ 0.06. Negative
/// age (clock rewind) yields a factor > 1 — callers must not rely on
/// `decay <= 1`.
pub(crate) fn calculate_decay(last_used_ms: i64, now_ms: i64) -> f64 {
    let age_hours = (now_ms - last_used_ms) as f64 / 3_600_000.0;
    (-age_hours / DECAY_HALF_LIFE_HOURS * LN_2).exp()
}

/// User-layer score with decay floor + learning bonus. Guarantees a user
/// entry always outranks an equal-count dict entry by at least
/// `LEARNING_BONUS`.
pub(crate) fn calculate_user_score(count: i64, last_used_ms: i64, now_ms: i64) -> f64 {
    let decay = calculate_decay(last_used_ms, now_ms);
    let raw_score = count as f64 * USER_WEIGHT;
    let floor = if count >= HIGH_USAGE_THRESHOLD {
        HIGH_USAGE_DECAY_FLOOR
    } else {
        LOW_USAGE_DECAY_FLOOR
    };
    raw_score * decay.max(floor) + LEARNING_BONUS
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-9;

    #[test]
    fn score_dict_count_zero_is_zero() {
        assert_eq!(score_dict(0), 0.0);
    }

    #[test]
    fn score_dict_scales_linearly() {
        assert_eq!(score_dict(7), 7.0 * DICT_WEIGHT);
        assert_eq!(score_dict(100), 100.0 * DICT_WEIGHT);
    }

    #[test]
    fn user_outranks_dict_at_equal_count() {
        let now = 1_000_000_000_i64;
        let user = calculate_user_score(1, now, now);
        let dict = score_dict(100);
        assert!(user > dict, "user={} dict={}", user, dict);
    }

    #[test]
    fn fresh_user_score_includes_learning_bonus() {
        let now = 0;
        let score = calculate_user_score(1, 0, now);
        let expected = 1.0 * USER_WEIGHT + LEARNING_BONUS;
        assert!((score - expected).abs() < TOLERANCE);
    }

    #[test]
    fn high_and_low_floors_diverge_at_threshold() {
        let now = 1_000_000_000_i64;
        let very_old = now - (DECAY_HALF_LIFE_HOURS * 3_600_000.0 * 365.0) as i64;
        let high = calculate_user_score(HIGH_USAGE_THRESHOLD, very_old, now);
        let low = calculate_user_score(HIGH_USAGE_THRESHOLD - 1, very_old, now);
        let high_expected = (HIGH_USAGE_THRESHOLD as f64) * USER_WEIGHT * HIGH_USAGE_DECAY_FLOOR
            + LEARNING_BONUS;
        let low_expected = ((HIGH_USAGE_THRESHOLD - 1) as f64) * USER_WEIGHT * LOW_USAGE_DECAY_FLOOR
            + LEARNING_BONUS;
        assert!((high - high_expected).abs() < TOLERANCE);
        assert!((low - low_expected).abs() < TOLERANCE);
    }

    #[test]
    fn decay_at_zero_age_is_one() {
        assert!((calculate_decay(0, 0) - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn decay_at_one_half_life_is_half() {
        let half_life_ms = (DECAY_HALF_LIFE_HOURS * 3_600_000.0) as i64;
        let decay = calculate_decay(0, half_life_ms);
        // LN_2 = 0.693 (3-digit), not full precision — accept ~0.001 drift.
        assert!((decay - 0.5).abs() < 1e-3, "decay={} expected ≈ 0.5", decay);
    }

    #[test]
    fn decay_at_two_half_lives_is_quarter() {
        let two_half_lives_ms = (DECAY_HALF_LIFE_HOURS * 3_600_000.0 * 2.0) as i64;
        let decay = calculate_decay(0, two_half_lives_ms);
        assert!((decay - 0.25).abs() < 1e-3, "decay={} expected ≈ 0.25", decay);
    }

    #[test]
    fn decay_is_monotonically_decreasing() {
        let now: i64 = 10_000_000_000;
        let mut previous = calculate_decay(now, now);
        for hours_ago in 1..=72 {
            let last_used = now - hours_ago * 3_600_000;
            let decay = calculate_decay(last_used, now);
            assert!(
                decay <= previous + 1e-12,
                "decay should not increase: prev={} curr={} hours_ago={}",
                previous,
                decay,
                hours_ago
            );
            previous = decay;
        }
    }
}
