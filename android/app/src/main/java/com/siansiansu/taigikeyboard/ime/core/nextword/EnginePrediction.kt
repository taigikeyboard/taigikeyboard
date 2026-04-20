package com.siansiansu.taigikeyboard.ime.core.nextword

// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

/**
 * Engine-layer prediction value produced by [NextWordEngine.filterPredictions]
 * and consumed by [com.siansiansu.taigikeyboard.ime.text.smartbar.NextWordHandler],
 * which is the only place that knows how to turn this into a platform
 * `TaigiWord` for the candidate bar.
 *
 * Mirrors iOS `NextWord/EnginePrediction.swift`. Android carries one
 * additional field — [score] — so the wrapper can preserve today's
 * `TaigiWord.lengthScore = prediction.score.toInt()` mapping; iOS drops
 * the score because `contextUpdater.setNextWordPredictions` consumes the
 * list in array order without per-item weighting. Intentional divergence
 * documented in `nextword-engine-boundary.md` §13.10.
 */
data class EnginePrediction(
    /** Primary display text (converted roman, or hanzi fallback when roman empty). */
    val text: String,
    /** Optional secondary display (hanzi beneath roman, etc.). Null when roman is empty. */
    val subtitle: String?,
    /** Han-jī form, if any. */
    val hanzi: String,
    /** Tâi-lô raw form (the bigram source's TL string). */
    val tl: String,
    /** Opaque ordering weight — carried through from [RawNextWordPrediction.score]. */
    val score: Double,
)
