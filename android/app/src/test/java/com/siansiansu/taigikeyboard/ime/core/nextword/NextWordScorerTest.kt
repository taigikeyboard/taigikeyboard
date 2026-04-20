package com.siansiansu.taigikeyboard.ime.core.nextword

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlin.math.abs
import kotlin.math.exp

/**
 * Pure tests for [NextWordScorer]. Mirrors iOS `NextWordScorerTests.swift`.
 *
 * Labels in this file match `docs/architecture/behavioral-invariants.md`
 * §7 (decay) + §8 (weighting). Scoring math is pure Kotlin stdlib so the
 * assertions run without SQLite, `Context`, or coroutines on the
 * classpath.
 */
class NextWordScorerTest {
    private val hourMs = 3_600_000L
    private val oneWeekMs = 168L * hourMs

    /**
     * `calculateDecay(lastUsedMs, nowMs)` implements the RIME half-life
     * formula `exp(-ageHours / 168 * ln(2))`. One-week-old samples land
     * at ≈0.5; fresh samples at ≈1.0.
     */
    @Test
    fun test_INVARIANT_decay_half_life_is_168_hours() {
        // age = 0 → decay = 1
        val fresh = NextWordScorer.calculateDecay(lastUsedMs = 0L, nowMs = 0L)
        assertEquals(1.0, fresh, 0.0001)

        // age = 168h → decay = 0.5 (within floating-point precision, plus
        // the hard-coded 0.693 vs ln(2) ≈ 0.03% drift documented at §7).
        val oneWeek = NextWordScorer.calculateDecay(lastUsedMs = 0L, nowMs = oneWeekMs)
        assertEquals(0.5, oneWeek, 0.001)

        // age = 336h (two weeks) → decay = 0.25.
        val twoWeeks = NextWordScorer.calculateDecay(lastUsedMs = 0L, nowMs = 2L * oneWeekMs)
        assertEquals(0.25, twoWeeks, 0.001)

        // Formula sanity check: matches `exp(-age / half-life * ln(2))`
        // for an arbitrary age (3 days) against the direct computation.
        val threeDaysMs = 72L * hourMs
        val threeDaysDecay = NextWordScorer.calculateDecay(0L, threeDaysMs)
        val expected = exp(-72.0 / 168.0 * NextWordScorer.LN_2)
        assertTrue(
            "three-day decay must match the exp(-age/168 * ln2) formula",
            abs(threeDaysDecay - expected) < 1e-9,
        )
    }

    /**
     * `calculateUserScore` applies `HIGH_USAGE_DECAY_FLOOR = 0.95` once
     * the count reaches `HIGH_USAGE_THRESHOLD = 3`. Frequently-used
     * entries retain ≥ 95% of their raw score no matter how stale.
     */
    @Test
    fun test_INVARIANT_high_usage_decay_floor_95() {
        // count = 3, age = 1 year → decay collapses; floor keeps it at 0.95.
        val ancientMs = 365L * 24L * hourMs
        val score = NextWordScorer.calculateUserScore(count = 3, lastUsedMs = 0L, nowMs = ancientMs)
        // rawScore = 3 * 50 = 150; floor = 0.95 → 142.5 + 300 bonus = 442.5.
        val expected = 150.0 * NextWordScorer.HIGH_USAGE_DECAY_FLOOR + NextWordScorer.LEARNING_BONUS
        assertEquals(expected, score, 0.0001)

        // count = HIGH_USAGE_THRESHOLD exactly — boundary still uses high floor.
        assertEquals(
            expected,
            NextWordScorer.calculateUserScore(
                count = NextWordScorer.HIGH_USAGE_THRESHOLD,
                lastUsedMs = 0L,
                nowMs = ancientMs,
            ),
            0.0001,
        )
    }

    /**
     * Counts below `HIGH_USAGE_THRESHOLD` fall under the low floor (0.3).
     * Recent usage still dominates; only stale low-count entries hit the
     * floor.
     */
    @Test
    fun test_INVARIANT_low_usage_decay_floor_30() {
        val ancientMs = 365L * 24L * hourMs
        // count = 1, age = 1 year → raw = 50, floor = 0.3 → 15 + 300 = 315.
        val score = NextWordScorer.calculateUserScore(count = 1, lastUsedMs = 0L, nowMs = ancientMs)
        val expected = 50.0 * NextWordScorer.LOW_USAGE_DECAY_FLOOR + NextWordScorer.LEARNING_BONUS
        assertEquals(expected, score, 0.0001)

        // count = 2 (still below threshold) uses the low floor.
        assertEquals(
            100.0 * NextWordScorer.LOW_USAGE_DECAY_FLOOR + NextWordScorer.LEARNING_BONUS,
            NextWordScorer.calculateUserScore(count = 2, lastUsedMs = 0L, nowMs = ancientMs),
            0.0001,
        )

        // Recent low-usage entries get the full decay, not the floor —
        // the floor is only a lower bound.
        val fresh = NextWordScorer.calculateUserScore(count = 1, lastUsedMs = 0L, nowMs = 0L)
        assertEquals(50.0 + NextWordScorer.LEARNING_BONUS, fresh, 0.0001)
    }

    /**
     * Pin test: the seven published scoring constants must match the iOS
     * `NextWordScorer.swift` values exactly. Any modification goes through
     * iOS + Android + `behavioral-invariants.md` in the same PR per
     * `rules/cross-platform-alignment.md` §3a.
     */
    @Test
    fun test_INVARIANT_scorer_constants_match_android() {
        assertEquals(50, NextWordScorer.USER_WEIGHT)
        assertEquals(1, NextWordScorer.DICT_WEIGHT)
        assertEquals(168.0, NextWordScorer.DECAY_HALF_LIFE_HOURS, 0.0)
        assertEquals(300.0, NextWordScorer.LEARNING_BONUS, 0.0)
        assertEquals(0.95, NextWordScorer.HIGH_USAGE_DECAY_FLOOR, 0.0)
        assertEquals(0.3, NextWordScorer.LOW_USAGE_DECAY_FLOOR, 0.0)
        assertEquals(3, NextWordScorer.HIGH_USAGE_THRESHOLD)
        // `LN_2` is hard-coded; the 0.03% drift vs kotlin.math.ln(2.0) is
        // acceptable and documented at Phase 0 §7.
        assertEquals(0.693, NextWordScorer.LN_2, 0.0)
    }

    /**
     * User weight is 50× dictionary weight. Given identical counts and
     * ignoring decay / bonus, the user-layer raw contribution outranks
     * the dict-layer raw contribution by a factor of 50.
     */
    @Test
    fun test_INVARIANT_user_weight_is_50() {
        assertEquals(50, NextWordScorer.USER_WEIGHT)
        // 1 user hit's raw (pre-decay) portion = 50 * 1.
        // Compared to scoreDict(50): user wins by the learning bonus alone.
        val dictScore = NextWordScorer.scoreDict(50)
        assertEquals(50.0, dictScore, 0.0001)
    }

    /**
     * Dictionary score is the raw count, no weighting. `scoreDict(n) == n`.
     */
    @Test
    fun test_INVARIANT_dict_weight_is_1() {
        assertEquals(1, NextWordScorer.DICT_WEIGHT)
        assertEquals(1.0, NextWordScorer.scoreDict(1), 0.0)
        assertEquals(42.0, NextWordScorer.scoreDict(42), 0.0)
        assertEquals(0.0, NextWordScorer.scoreDict(0), 0.0)
    }

    /**
     * Every user score carries a `LEARNING_BONUS = 300.0` additive
     * constant. Dictionary scores never include it. The floor behavior
     * still applies below.
     */
    @Test
    fun test_INVARIANT_learning_bonus_is_300() {
        assertEquals(300.0, NextWordScorer.LEARNING_BONUS, 0.0)

        // A freshly-used count=1 user entry scores exactly `raw + bonus`.
        val userScore = NextWordScorer.calculateUserScore(count = 1, lastUsedMs = 0L, nowMs = 0L)
        assertEquals(50.0 + 300.0, userScore, 0.0001)

        // Dictionary equivalent has no bonus.
        val dictScore = NextWordScorer.scoreDict(1)
        assertTrue(
            "learning bonus must separate user from dict",
            userScore - dictScore >= NextWordScorer.LEARNING_BONUS,
        )
    }

    /**
     * A user entry always outranks a dict entry of equivalent count, even
     * when the user entry has aged enough to hit the low-usage floor.
     * This is the essential UX contract — user learning never gets buried
     * by cold-start dict weight.
     */
    @Test
    fun test_INVARIANT_user_entry_outranks_dict_entry() {
        val ancientMs = 365L * 24L * hourMs
        // Worst case for user: count=1, ancient.
        val worstUser = NextWordScorer.calculateUserScore(1, 0L, ancientMs)
        // Best plausible dict: high count, single digit range for hot entries.
        val bestDict = NextWordScorer.scoreDict(100)
        assertTrue(
            "user entry (worst case) must outrank dict entry (count=100): user=$worstUser dict=$bestDict",
            worstUser > bestDict,
        )

        // Equal counts: user with full decay collapse still wins.
        val equalCount = 5
        val userFaded =
            NextWordScorer.calculateUserScore(equalCount, 0L, ancientMs)
        val dictSame = NextWordScorer.scoreDict(equalCount)
        assertTrue(
            "user entry (count=$equalCount, ancient) must outrank dict entry (count=$equalCount): user=$userFaded dict=$dictSame",
            userFaded > dictSame,
        )
    }
}
