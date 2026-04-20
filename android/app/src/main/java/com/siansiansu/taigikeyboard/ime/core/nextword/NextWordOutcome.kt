package com.siansiansu.taigikeyboard.ime.core.nextword

// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

/**
 * Platform-neutral lifecycle events the executor lowers into engine intents.
 *
 * Mirrors iOS `NextWord/NextWordOutcome.swift` `NextWordIntent`.
 */
sealed class NextWordIntent {
    /**
     * User selected a candidate or committed composing text.
     * [requireRomanMode] gates Enter-commits-raw-romanization paths;
     * [triggerPrediction] is `false` when Space confirms the composing
     * text without asking for the next-word bar.
     */
    data class WordSelected(
        val text: String,
        val roman: String,
        val requireRomanMode: Boolean,
        val triggerPrediction: Boolean,
    ) : NextWordIntent()

    /** Backspace after a word selection — re-predict, never record an association. */
    data class Backspace(
        val lastChar: String,
    ) : NextWordIntent()

    /**
     * Executor's context-timeout coroutine fired. Engine clears state and
     * bumps generation so any in-flight prediction query is invalidated.
     */
    object ContextTimeoutFired : NextWordIntent()

    /**
     * User began composing a new syllable / digit — hide suggestions but
     * keep association state intact (matches iOS `clearDisplay`).
     */
    object ClearForNewComposing : NextWordIntent()

    /** Full reset (sentence-end punctuation outside `WordSelected`, or empty document). */
    object ResetFull : NextWordIntent()
}

/**
 * Foundation-only settings snapshot passed into the engine. Captures only
 * the fields the engine actually reads; the executor builds this value
 * from `EngineSettingsProvider.current` once per intent.
 */
data class NextWordEngineSettings(
    /** Current input mode — `"poj"`, `"tl"`, `"tps"`, or `"english"`. */
    val inputMode: String,
    val isTranslateSwapped: Boolean,
    val isAssociationRecordingEnabled: Boolean,
)

/**
 * Long-lived state the engine mutates. Separate from [NextWordDecisionInput]
 * so the executor cannot accidentally persist a stale `nowMs` or settings.
 *
 * [currentGeneration] is the monotonic counter the executor tags in-flight
 * prediction queries with at dispatch time; a mismatch at resolution means
 * the intent that launched the query has been superseded, so the result
 * is dropped. Wraps at `Long.MAX_VALUE` (≈ 2^63 ms) which is astronomical
 * — Kotlin idiom for iOS `UInt64 &+=` per `nextword-engine-boundary.md`
 * §13.6.
 */
data class NextWordPersistedState(
    val lastSelectedWord: String? = null,
    val lastSelectedRoman: String? = null,
    val lastSelectionTimeMs: Long = 0,
    val isShowing: Boolean = false,
    val currentGeneration: Long = 0,
) {
    companion object {
        val initial = NextWordPersistedState()
    }
}

/** Per-intent snapshot supplied by the executor. Engine never reads a clock or provider itself. */
data class NextWordDecisionInput(
    val nowMs: Long,
    val settings: NextWordEngineSettings,
)

/**
 * Named DTO for a bigram association pair. Preferred over a tuple so
 * `NextWordOutcome.Effect.RecordAssociation` serializes cleanly and the
 * shape ports directly to Rust structs in Phase IV-A.
 */
data class NextWordAssociationPair(
    val prev: String,
    val prevTl: String,
    val next: String,
    val nextTl: String,
)

/**
 * Engine decision output. [effects] executes in order on the platform side.
 *
 * Binding contract in `nextword-engine-boundary.md` §§2–3 (cross-platform)
 * + §13 (Android addendum).
 */
data class NextWordOutcome(
    val newState: NextWordPersistedState,
    val effects: List<Effect>,
) {
    sealed class Effect {
        /**
         * Schedule (or reschedule) the context-timeout coroutine.
         * Android uses milliseconds to match Kotlin `delay(Long)` idiom;
         * iOS uses `TimeInterval` seconds. Same 30-second invariant on
         * both platforms (`NextWordEngine.CONTEXT_TIMEOUT_MS` = `30_000`).
         */
        data class RescheduleContextTimeout(
            val afterMs: Long,
        ) : Effect()

        object CancelContextTimeout : Effect()

        data class RecordAssociation(
            val pair: NextWordAssociationPair,
        ) : Effect()

        data class RecordCompoundAssociations(
            val pairs: List<NextWordAssociationPair>,
        ) : Effect()

        /**
         * [nowMs] is the decision-time clock read — the executor MUST
         * reuse it when calling `NextWordService.predict(nowMs = nowMs)`
         * so the association-window check (inside `decide`) and the
         * user-row decay scoring (inside `predict`) see ONE consistent
         * "now" per intent. Reading `System.currentTimeMillis()` a second
         * time at dispatch breaks the clock-injection contract documented
         * in `nextword-engine-boundary.md` §13.3.
         */
        data class QueryPredictions(
            val word: String,
            val roman: String,
            val generation: Long,
            val nowMs: Long,
        ) : Effect()

        /**
         * [generation] is informational — the engine bumps generation on
         * the same intent that emits `ClearPredictionsUI`, so the effect is
         * current-generation by construction and the executor does not
         * need to re-check before clearing.
         */
        data class ClearPredictionsUI(
            val generation: Long,
        ) : Effect()
    }
}
