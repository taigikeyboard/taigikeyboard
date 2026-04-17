package com.siansiansu.taigikeyboard.ime.dictionary

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
