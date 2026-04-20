package com.siansiansu.taigikeyboard.ime.core.nextword

import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings

/**
 * Narrow seam over the NextWord I/O layer that [NextWordHandler] depends on.
 * Exists so the executor can be exercised against a fake in pure-JVM unit
 * tests (see `INVARIANT_nextword_late_prediction_is_discarded` +
 * `INVARIANT_nextword_rescheduling_leaks_no_timer`); production code wires
 * `NextWordService` as the concrete binding.
 *
 * NOTE: Not shared-core — this is a platform seam. `RawNextWordPrediction`
 * and `EngineSettings` remain shared-core; the I/O surface is not.
 */
interface NextWordPredictor {
    /**
     * Predict the next character given the last-committed [word]. Merges
     * dictionary bigrams with user-learned entries. [nowMs] is the single
     * decision-time clock snapshot propagated from the executor per
     * `nextword-engine-boundary.md` §13.3.
     */
    suspend fun predict(
        word: String,
        roman: String = "",
        limit: Int = DEFAULT_LIMIT,
        settings: EngineSettings,
        nowMs: Long,
    ): List<RawNextWordPrediction>

    /** Persist a user-learned `prev → next` bigram. */
    suspend fun recordAssociation(
        prev: String,
        prevTl: String = "",
        nextHanzi: String,
        nextTl: String = "",
    )

    companion object {
        /** Default candidate limit when the executor does not specify one. */
        const val DEFAULT_LIMIT: Int = 30
    }
}
