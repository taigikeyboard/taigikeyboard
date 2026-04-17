package com.siansiansu.taigikeyboard.ime.dictionary

import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService

/**
 * Candidate-word scoring, dedup, and ordering.
 *
 * Extracted from LexiconService so that the service owns orchestration
 * (trie lookup, binary-reader IO, custom-dict merge) while this object
 * owns the ranking math. Mirrors iOS `CandidateProcessor.swift`.
 */
object CandidateProcessor {
    private const val TAG = "CandidateProcessor"

    /**
     * Breakdown of candidate score components (single source of truth).
     * Used by both sorting and debug logging — no recalculation needed.
     * Aligned with iOS CandidateProcessor.ScoreBreakdown.
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
        frequencyData: UserFrequencyService.FrequencyData,
        currentTime: Long,
    ): ScoreBreakdown {
        val candidateBase = romanToBase(word.roman)
        val inputBase = inputToBase(normalizedInput)

        val cappedUserFreq = minOf(frequencyData.count, 100)
        val userFreqScore = cappedUserFreq * 100

        val oneHourMillis = 60 * 60 * 1000L
        val recencyBonus =
            if (frequencyData.lastUsedMillis > 0 &&
                (currentTime - frequencyData.lastUsedMillis) < oneHourMillis
            ) {
                200
            } else {
                0
            }

        val exactBonus = if (candidateBase == inputBase) 100 else 0
        val completionPenalty = if (candidateBase != inputBase) -1000 else 0

        val inputLen = maxOf(inputBase.length, 1)
        val candidateLen = maxOf(candidateBase.length, 1)
        val matchRatio = minOf(inputLen, candidateLen).toDouble() / maxOf(inputLen, candidateLen).toDouble()
        val closenessBonus = (matchRatio * 500).toInt()

        val baseFreqScore = (word.lengthScore ?: 0) / 10

        return ScoreBreakdown(
            userFreqScore = userFreqScore,
            recencyBonus = recencyBonus,
            exactBonus = exactBonus,
            completionPenalty = completionPenalty,
            closenessBonus = closenessBonus,
            baseFreqScore = baseFreqScore,
        )
    }

    suspend fun sortByScore(
        words: List<TaigiWord>,
        normalizedInput: String,
    ): List<TaigiWord> {
        val currentTime = System.currentTimeMillis()
        val wordTexts = words.map { it.displayText }.distinct()
        val frequencyDataMap = UserFrequencyService.frequencyDataBatch(wordTexts)

        val sorted =
            words
                .map { word ->
                    val freqData =
                        frequencyDataMap[word.displayText]
                            ?: UserFrequencyService.FrequencyData(0, 0)
                    word to calculateScore(word, normalizedInput, freqData, currentTime)
                }.sortedByDescending { it.second.total }

        if (BuildConfig.DEBUG) {
            logScoreDetails(sorted, normalizedInput)
        }

        return sorted.map { it.first }
    }

    private fun romanToBase(roman: String): String {
        val noHyphens = roman.replace("-", "").replace(" ", "")
        val withOo = TaigiUnicode.nfdPreprocessed(noHyphens)
        return withOo
            .filter {
                Character.getType(it) != Character.NON_SPACING_MARK.toInt()
            }.filter { !it.isDigit() }
            .lowercase()
    }

    private fun inputToBase(normalizedInput: String): String = normalizedInput.filter { !it.isDigit() }.lowercase()

    private fun logScoreDetails(
        sorted: List<Pair<TaigiWord, ScoreBreakdown>>,
        normalizedInput: String,
    ) {
        for ((word, b) in sorted) {
            val hanzi = word.hanzi ?: ""
            Log.d(
                TAG,
                "[SCORE] input='$normalizedInput' | ${word.roman} $hanzi: user=${b.userFreqScore} recency=${b.recencyBonus} exact=${b.exactBonus} close=${b.closenessBonus} base=${b.baseFreqScore} completion=${b.completionPenalty} total=${b.total}",
            )
        }
    }
}
