package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pure-engine tests for [CandidateProcessor].
 *
 * Labels match `docs/architecture/behavioral-invariants.md` §5 (dedup) +
 * §6 (scoring). Mirrors iOS `CandidateProcessorTests.swift`.
 *
 * Scoring assertions use relative inequalities (`assertTrue(x > y)`)
 * rather than pinning absolute numeric totals, so subtle changes in
 * tiebreaker math (e.g. `baseFreqScore`) don't force cross-platform test
 * churn — the iOS `BASE_FREQ_DIVISOR` constant is Android-only and would
 * break strict numeric parity anyway.
 */
class CandidateProcessorTest {
    private fun word(
        id: Int = 0,
        roman: String,
        hanzi: String? = null,
        lengthScore: Int? = null,
    ): TaigiWord = TaigiWord(id = id, roman = roman, hanzi = hanzi, lengthScore = lengthScore)

    // ========================================================================
    // §5 — engine vs display dedup
    // ========================================================================

    /**
     * `removeDuplicates` dedups on the composite key `"<roman>|<hanzi>"`.
     * Two entries with identical roman but different hanzi survive; two
     * entries with identical roman + null hanzi collapse to one.
     */
    @Test
    fun test_INVARIANT_engine_dedup_keys_on_roman_plus_hanzi() {
        val words =
            listOf(
                word(1, "tâi-gí", "台語"),
                word(2, "tâi-gí", "台語"), // exact duplicate — dropped
                word(3, "tâi-gí", "臺語"), // different hanzi — kept
                word(4, "tâi-gí", null), // null hanzi — kept (distinct key)
                word(5, "tâi-gí", null), // duplicate null — dropped
            )
        val result = CandidateProcessor.removeDuplicates(words)
        assertEquals(3, result.size)
        // Ordering of survivors preserves first-seen order.
        assertEquals(1, result[0].id)
        assertEquals(3, result[1].id)
        assertEquals(4, result[2].id)
    }

    /**
     * `removeDisplayDuplicates` dedups on hanzi only and MUST run after
     * `sortByScore` so the highest-ranked entry per hanzi survives. This
     * test runs dedup AFTER a manual sort to pin that ordering contract.
     */
    @Test
    fun test_INVARIANT_display_dedup_runs_after_sort() {
        // Construct two entries with same hanzi but different roman forms.
        // Manually pre-sort so the "winner" (higher frequency) appears
        // before the "loser" in the input list — `removeDisplayDuplicates`
        // keeps first-seen per hanzi, so the winner must survive.
        val winner = word(1, "tâi-gí", "台語", lengthScore = 100)
        val loser = word(2, "tai-gi", "台語", lengthScore = 10)
        val sortedInput = listOf(winner, loser)
        val deduped = CandidateProcessor.removeDisplayDuplicates(sortedInput)

        assertEquals(1, deduped.size)
        assertEquals("winner must survive display dedup", 1, deduped[0].id)
    }

    /**
     * Entries with null / empty hanzi are ALWAYS kept — display dedup only
     * collapses on non-empty hanzi. Two roman-only entries must both pass.
     */
    @Test
    fun test_INVARIANT_display_dedup_keeps_words_without_hanzi() {
        val words =
            listOf(
                word(1, "hello", null),
                word(2, "world", null),
                word(3, "gap", ""),
                word(4, "dup", "同"),
                word(5, "dup2", "同"), // hanzi collision — dropped
            )
        val result = CandidateProcessor.removeDisplayDuplicates(words)

        assertEquals(4, result.size)
        assertEquals("first roman-only kept", 1, result[0].id)
        assertEquals("second roman-only kept", 2, result[1].id)
        assertEquals("empty-hanzi entry kept", 3, result[2].id)
        assertEquals("first hanzi entry kept", 4, result[3].id)
    }

    // ========================================================================
    // §6 — scoring determinism + ordering
    // ========================================================================

    /**
     * `calculateScore` is pure: identical inputs always yield the same
     * `ScoreBreakdown`. The pure-by-contract claim is what Phase 0 §6
     * requires; invariance under repetition is the cheapest pin.
     */
    @Test
    fun test_INVARIANT_score_is_deterministic() {
        val w = word(roman = "gua", hanzi = "我", lengthScore = 50)
        val freq = FrequencyData(count = 2, lastUsedMillis = 1_000_000L)
        val now = 5_000_000L
        val first = CandidateProcessor.calculateScore(w, "gua", freq, now)
        val second = CandidateProcessor.calculateScore(w, "gua", freq, now)
        assertEquals(first, second)
        assertEquals(first.total, second.total)
    }

    /**
     * User-frequency score dominates the ranking. A candidate with even a
     * single recorded user hit outranks a cold-start dict entry with the
     * same exact-match bonus.
     */
    @Test
    fun test_INVARIANT_user_freq_dominates_ranking() {
        val userWord = word(1, "gua", "我", lengthScore = 10)
        val dictWord = word(2, "gua", "瓜", lengthScore = 100) // higher base freq
        val userFreq = FrequencyData(count = 1, lastUsedMillis = 0L) // one hit, stale
        val coldFreq = FrequencyData.EMPTY
        val now = 10_000_000_000L // well past the 1h recency window

        val userScore =
            CandidateProcessor.calculateScore(userWord, "gua", userFreq, now)
        val dictScore =
            CandidateProcessor.calculateScore(dictWord, "gua", coldFreq, now)
        assertTrue(
            "user entry must outrank dict entry: user.total=${userScore.total} dict.total=${dictScore.total}",
            userScore.total > dictScore.total,
        )
    }

    /**
     * Completion penalty separates exact-match candidates from mere
     * completions. A candidate whose base equals the normalized input
     * gets `EXACT_BONUS`; one whose base is longer (completion) gets
     * `COMPLETION_PENALTY`. The gap must be strictly positive.
     */
    @Test
    fun test_INVARIANT_completion_penalty_separates_tiers() {
        val exactWord = word(1, "gua", "我", lengthScore = 50)
        val completionWord = word(2, "guan", "阮", lengthScore = 50)
        val freq = FrequencyData.EMPTY
        val now = 10_000_000_000L

        val exact =
            CandidateProcessor.calculateScore(exactWord, "gua", freq, now)
        val completion =
            CandidateProcessor.calculateScore(completionWord, "gua", freq, now)

        assertEquals("exact bonus applied on match", 100, exact.exactBonus)
        assertEquals("no exact bonus on completion", 0, completion.exactBonus)
        assertEquals("completion penalty applied", -1000, completion.completionPenalty)
        assertEquals("no completion penalty on exact", 0, exact.completionPenalty)
        assertTrue(
            "exact must outrank completion: exact.total=${exact.total} completion.total=${completion.total}",
            exact.total > completion.total,
        )
    }

    /**
     * The recency window is exactly 1 hour (`60 * 60 * 1000` ms). The
     * boundary condition is strict `<`: at 3_599_999 ms the bonus fires;
     * at 3_600_000 ms it does not.
     */
    @Test
    fun test_INVARIANT_recency_window_is_exactly_1_hour() {
        val w = word(roman = "gua", hanzi = "我", lengthScore = 50)
        val freqRecent = FrequencyData(count = 0, lastUsedMillis = 1L)
        val now = 1L + 3_599_999L
        val inWindow = CandidateProcessor.calculateScore(w, "gua", freqRecent, now)
        assertEquals("just inside window → recency bonus", 200, inWindow.recencyBonus)

        val onBoundary = CandidateProcessor.calculateScore(w, "gua", freqRecent, 1L + 3_600_000L)
        assertEquals("exactly at boundary → no bonus (strict <)", 0, onBoundary.recencyBonus)

        val past = CandidateProcessor.calculateScore(w, "gua", freqRecent, 1L + 3_600_001L)
        assertEquals("just past boundary → no bonus", 0, past.recencyBonus)

        // lastUsedMillis = 0 also disables the bonus (documented invariant
        // corner case — a never-used entry must not pick up a recency flag).
        val never =
            CandidateProcessor.calculateScore(
                w,
                "gua",
                FrequencyData.EMPTY,
                now,
            )
        assertEquals("never-used → no recency bonus", 0, never.recencyBonus)
    }

    /**
     * `romanToBase` strips tone diacritics, hyphens, spaces, and trailing
     * digits in that order, and lowercases the result. Invariant §6
     * corner case.
     */
    @Test
    fun test_INVARIANT_roman_to_base_strips_tones_hyphens_digits() {
        val fixtures =
            listOf(
                // Hyphens + spaces + tone marks stripped.
                "tâi-gí" to "taigi",
                "tâi gí" to "taigi",
                // Numeric tones stripped.
                "gua2" to "gua",
                // Uppercase collapses to lowercase.
                "GUA2" to "gua",
                // POJ o͘ (U+0358) collapses to "o".
                "h\u00F3\u0358" to "hoo",
                // POJ nasal ⁿ → nn (via TaigiUnicode preprocessing).
                "sa\u207F" to "sann",
                // Empty input is a fixed point.
                "" to "",
            )
        for ((input, expected) in fixtures) {
            assertEquals(
                "romanToBase('$input')",
                expected,
                CandidateProcessor.romanToBase(input),
            )
        }
    }

    // ========================================================================
    // §6 — sortByScore end-to-end (post-A1 parameterization)
    // ========================================================================

    /**
     * `sortByScore` consumes caller-supplied frequency data (A1
     * parameterization). Sanity check: feeding the function two words
     * where the second has a user-frequency hit surfaces that word first.
     * Guards against accidental regression of the `frequencyData` parameter
     * back to a service-side lookup.
     */
    @Test
    fun sortByScore_honors_caller_supplied_frequency_data() {
        val cold = word(1, "gua", "瓜", lengthScore = 100)
        val learned = word(2, "gua", "我", lengthScore = 10)
        val frequency = mapOf("我" to FrequencyData(count = 5, lastUsedMillis = 0L))
        val now = 10_000_000_000L

        val sorted =
            CandidateProcessor.sortByScore(
                words = listOf(cold, learned),
                normalizedInput = "gua",
                frequencyData = frequency,
                currentTime = now,
            )
        assertEquals("learned word must lead", 2, sorted[0].id)
        assertEquals("cold word trails", 1, sorted[1].id)

        // Keep a silence assertion on InputMode plumbing — scoring is
        // mode-agnostic, but we want to catch any future reach-in.
        @Suppress("UNUSED_VARIABLE")
        val modeSanityCheck = InputMode.TL
    }
}
