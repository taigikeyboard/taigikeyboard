package com.siansiansu.taigikeyboard.ime.core.nextword

// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

/**
 * Engine-side NextWord prediction row at the service boundary.
 *
 * Mirrors iOS `NextWord/RawNextWordPrediction.swift`. Replaces the
 * platform-bound `NextWordService.Prediction` so pure engine code
 * (`NextWordEngine`) can consume prediction results without depending on
 * the service type. `score` is an opaque ordering weight — scoring lives
 * inside `NextWordService` (Phase IV-B re-hoists it into shared-core per
 * `nextword-engine-boundary.md` §2.4 impl-revision note).
 */
data class RawNextWordPrediction(
    val hanzi: String,
    val tl: String,
    val score: Double,
)
