// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.core.nextword

import kotlin.math.exp
import kotlin.math.max

/**
 * Pure NextWord scoring math extracted from `NextWordService` so the math
 * can be pinned by invariant tests without touching SQLite or Android
 * context. Mirrors iOS `NextWord/NextWordScorer.swift`.
 *
 * CROSS-PLATFORM INVARIANT — the constants below MUST mirror iOS
 * `NextWordScorer.swift`. Drift causes silent ranking divergence between
 * platforms. Modifying any constant requires iOS + Android + the matching
 * entry in `docs/architecture/behavioral-invariants.md` §7 / §8 in the
 * same commit.
 */
object NextWordScorer {
    /** Source weight applied to user-learned bigrams. */
    const val USER_WEIGHT: Int = 50

    /** Source weight applied to dictionary bigrams. */
    const val DICT_WEIGHT: Int = 1

    /** RIME-style exponential decay half-life (1 week). */
    const val DECAY_HALF_LIFE_HOURS: Double = 168.0

    /** Additive bonus ensuring user entries outrank dict entries. */
    const val LEARNING_BONUS: Double = 300.0

    /** Minimum decay retained for frequently-used entries (count >= threshold). */
    const val HIGH_USAGE_DECAY_FLOOR: Double = 0.95

    /** Minimum decay retained for infrequent entries. */
    const val LOW_USAGE_DECAY_FLOOR: Double = 0.3

    /** Usage count above which the high-usage floor kicks in. */
    const val HIGH_USAGE_THRESHOLD: Int = 3

    /**
     * Hard-coded `ln(2)` approximation (precision drift vs `ln(2.0)` is
     * ~0.03% at one week). Constant must match iOS `NextWordScorer.ln2`.
     */
    const val LN_2: Double = 0.693

    /**
     * Dictionary-layer score: raw count times [DICT_WEIGHT]. Kept as a
     * named function so call-sites declare intent even though the math is
     * trivial.
     */
    fun scoreDict(count: Int): Double = count.toDouble() * DICT_WEIGHT

    /**
     * RIME-style exponential decay factor. `decay = exp(-ageHours / halfLifeHours * ln(2))`.
     * Recent usage ≈ 1.0; one week out ≈ 0.5; one month out ≈ 0.06.
     * Negative age (clock rewind) yields a factor > 1 — callers must not
     * rely on `decay <= 1`.
     */
    fun calculateDecay(
        lastUsedMs: Long,
        nowMs: Long,
    ): Double {
        val ageHours = (nowMs - lastUsedMs) / 3_600_000.0
        return exp(-ageHours / DECAY_HALF_LIFE_HOURS * LN_2)
    }

    /**
     * User-layer score with decay floor + learning bonus. Guarantees any
     * user entry outranks an equal-count dict entry by at least
     * [LEARNING_BONUS].
     */
    fun calculateUserScore(
        count: Int,
        lastUsedMs: Long,
        nowMs: Long,
    ): Double {
        val decay = calculateDecay(lastUsedMs, nowMs)
        val rawScore = count.toDouble() * USER_WEIGHT
        val decayFloor =
            if (count >= HIGH_USAGE_THRESHOLD) HIGH_USAGE_DECAY_FLOOR else LOW_USAGE_DECAY_FLOOR
        val effectiveDecay = max(decayFloor, decay)
        return rawScore * effectiveDecay + LEARNING_BONUS
    }
}
