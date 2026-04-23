// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug

/**
 * Candidate-word scoring, dedup, and ordering.
 *
 * Owns the ranking math extracted from `LexiconService`. Stateless — no
 * service reach-ins, no DB lookups. Callers batch-fetch user-frequency
 * data and pass it in, mirroring iOS `CandidateProcessor.swift`.
 *
 * CROSS-PLATFORM INVARIANT — mirrors
 * ios/Sources/TaigiKeyboard/Lexicon/Utils/CandidateProcessor.swift
 * `calculateScore`. The iOS-synced scoring constants MUST stay in
 * lock-step: `USER_FREQ_CAP`, `USER_FREQ_WEIGHT`, `RECENCY_WINDOW_MS`,
 * `RECENCY_BONUS`, `EXACT_BONUS`, `COMPLETION_PENALTY`, `CLOSENESS_WEIGHT`,
 * `SOURCE_TIERS`, `TIER_DENOMINATOR`. Drift causes silent ranking divergence.
 * `BASE_FREQ_DIVISOR` is Android-only (dictionary-frequency normalisation)
 * and is NOT part of the invariant set.
 */
object CandidateProcessor {
    private const val TAG = "CandidateProcessor"

    private const val USER_FREQ_CAP = 100
    private const val USER_FREQ_WEIGHT = 100
    private const val RECENCY_WINDOW_MS = 60L * 60L * 1000L
    private const val RECENCY_BONUS = 200
    private const val EXACT_BONUS = 100
    private const val COMPLETION_PENALTY = -1000
    private const val CLOSENESS_WEIGHT = 500
    private const val BASE_FREQ_DIVISOR = 10

    /**
     * Maps a dictionary-source bit to a `baseFreqScore` multiplier numerator.
     * CROSS-PLATFORM INVARIANT — mirrors `dictionary/common/source_bits.py`
     * (SOURCE_TIERS + TIER_DENOMINATOR) and
     * `ios/.../CandidateProcessor.swift` SOURCE_TIERS. First-match-wins on
     * overlapping bits. Drift causes silent ranking divergence.
     */
    private data class SourceTier(
        val bit: Int,
        val numerator: Int,
    )

    private val SOURCE_TIERS: List<SourceTier> =
        listOf(
            SourceTier(bit = 0, numerator = 15), // kautian
            SourceTier(bit = 1, numerator = 13), // taigitv
            SourceTier(bit = 7, numerator = 12), // stti
            SourceTier(bit = 6, numerator = 11), // kungge
        )
    private const val DEFAULT_TIER_NUMERATOR = 10
    private const val TIER_DENOMINATOR = 10

    private fun tierNumerator(bitmask: Int?): Int {
        if (bitmask == null) return DEFAULT_TIER_NUMERATOR
        for (tier in SOURCE_TIERS) {
            if ((bitmask and (1 shl tier.bit)) != 0) return tier.numerator
        }
        return DEFAULT_TIER_NUMERATOR
    }

    /**
     * Breakdown of candidate score components — single source of truth for
     * both sorting and debug logging. Mirrors iOS
     * `CandidateProcessor.ScoreBreakdown`.
     */
    data class ScoreBreakdown(
        val userFreqScore: Int,
        val recencyBonus: Int,
        val exactBonus: Int,
        val completionPenalty: Int,
        val closenessBonus: Int,
        val baseFreqScore: Int,
    ) {
        val total: Int
            get() = userFreqScore + recencyBonus + exactBonus + completionPenalty + closenessBonus + baseFreqScore
    }

    fun removeDuplicates(words: List<TaigiWord>): List<TaigiWord> {
        val seen = mutableSetOf<String>()
        val result = mutableListOf<TaigiWord>()

        for (word in words) {
            val key = "${word.roman}|${word.hanzi ?: ""}"
            if (key in seen) continue
            seen.add(key)
            result.add(word)
        }

        return result
    }

    fun removeDisplayDuplicates(words: List<TaigiWord>): List<TaigiWord> {
        val seenHanzi = mutableSetOf<String>()
        return words.filter { word ->
            val hanzi = word.hanzi
            if (hanzi.isNullOrEmpty()) {
                true
            } else {
                seenHanzi.add(hanzi)
            }
        }
    }

    fun calculateScore(
        word: TaigiWord,
        normalizedInput: String,
        frequencyData: FrequencyData,
        currentTime: Long,
    ): ScoreBreakdown {
        val candidateBase = romanToBase(word.roman)
        val inputBase = inputToBase(normalizedInput)

        val cappedUserFreq = minOf(frequencyData.count, USER_FREQ_CAP)
        val userFreqScore = cappedUserFreq * USER_FREQ_WEIGHT

        val recencyBonus =
            if (frequencyData.lastUsedMillis > 0 &&
                (currentTime - frequencyData.lastUsedMillis) < RECENCY_WINDOW_MS
            ) {
                RECENCY_BONUS
            } else {
                0
            }

        val exactBonus = if (candidateBase == inputBase) EXACT_BONUS else 0
        val completionPenalty = if (candidateBase != inputBase) COMPLETION_PENALTY else 0

        val inputLen = maxOf(inputBase.length, 1)
        val candidateLen = maxOf(candidateBase.length, 1)
        val matchRatio = minOf(inputLen, candidateLen).toDouble() / maxOf(inputLen, candidateLen).toDouble()
        val closenessBonus = (matchRatio * CLOSENESS_WEIGHT).toInt()

        val rawBase = (word.lengthScore ?: 0) / BASE_FREQ_DIVISOR
        val baseFreqScore = rawBase * tierNumerator(word.sourceBitmask) / TIER_DENOMINATOR

        return ScoreBreakdown(
            userFreqScore = userFreqScore,
            recencyBonus = recencyBonus,
            exactBonus = exactBonus,
            completionPenalty = completionPenalty,
            closenessBonus = closenessBonus,
            baseFreqScore = baseFreqScore,
        )
    }

    /**
     * Sort [words] by descending score. User-frequency data is supplied by
     * the caller so this function stays free of service reach-ins —
     * `LexiconService.rankByFrequency` is the production caller; tests
     * may pin [frequencyData] and [currentTime] for deterministic ranking.
     */
    fun sortByScore(
        words: List<TaigiWord>,
        normalizedInput: String,
        frequencyData: Map<String, FrequencyData>,
        currentTime: Long,
        logger: LoggerBackend = NullLoggerBackend,
    ): List<TaigiWord> {
        val sorted =
            words
                .map { word ->
                    val freqData = frequencyData[word.displayText] ?: FrequencyData.EMPTY
                    word to calculateScore(word, normalizedInput, freqData, currentTime)
                }.sortedByDescending { it.second.total }

        logScoreDetails(sorted, normalizedInput, logger)

        return sorted.map { it.first }
    }

    // `internal` so `CandidateProcessorTest` can pin the Phase 0 §6
    // `INVARIANT_roman_to_base_strips_tones_hyphens_digits` label directly
    // against this helper. No external non-test caller outside the module.
    internal fun romanToBase(roman: String): String {
        val noHyphens = roman.replace("-", "").replace(" ", "")
        val withOo = TaigiUnicode.nfdPreprocessed(noHyphens)
        return withOo
            .filter {
                Character.getType(it) != Character.NON_SPACING_MARK.toInt()
            }.filter { !it.isDigit() }
            .lowercase()
    }

    internal fun inputToBase(normalizedInput: String): String = normalizedInput.filter { !it.isDigit() }.lowercase()

    private fun logScoreDetails(
        sorted: List<Pair<TaigiWord, ScoreBreakdown>>,
        normalizedInput: String,
        logger: LoggerBackend,
    ) {
        if (!logger.isDebugEnabled) return
        for ((word, b) in sorted) {
            val hanzi = word.hanzi ?: ""
            logger.debug(TAG) {
                "[SCORE] input='$normalizedInput' | ${word.roman} $hanzi: user=${b.userFreqScore} recency=${b.recencyBonus} exact=${b.exactBonus} close=${b.closenessBonus} base=${b.baseFreqScore} completion=${b.completionPenalty} total=${b.total}"
            }
        }
    }
}
