package com.siansiansu.taigikeyboard.ime.core.nextword

import com.siansiansu.taigikeyboard.ime.dictionary.TaigiPhonetics

// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

/**
 * Pure decision + filtering layer for NextWord prediction.
 *
 * Mirrors iOS `NextWord/NextWordEngine.swift`. The executor
 * ([com.siansiansu.taigikeyboard.ime.text.smartbar.NextWordHandler]) owns
 * the coroutine-scheduled context-timeout job, `EngineSettingsProvider`
 * reads, and UI callbacks; this engine owns validation, state transitions,
 * association-window math, compound-word splitting, and prediction
 * filtering.
 *
 * Invariants (see `docs/architecture/nextword-engine-boundary.md`):
 * - No clock reads. Callers supply `nowMs` via [NextWordDecisionInput].
 * - No settings provider. Callers pass a [NextWordEngineSettings] value.
 * - No coroutines / Dispatchers / Job. Scheduling is described as effects
 *   the executor interprets.
 * - Any state-invalidating intent bumps [NextWordPersistedState.currentGeneration]
 *   so late async prediction results can be discarded by the executor.
 */
object NextWordEngine {
    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/NextWord/NextWordEngine.swift:27.
    // Drift causes silent divergence. A8-sweep tightens the marker wording across constants.
    /** Strict-less-than association window. 10_000 ms gap does NOT record; negative deltas also skipped. */
    const val ASSOCIATION_TIMEOUT_MS: Long = 10_000L

    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/NextWord/NextWordEngine.swift:30.
    // Drift causes silent divergence. Android uses milliseconds (Kotlin `delay(Long)` idiom);
    // iOS uses `TimeInterval` seconds. Same 30-second wait.

    /** Context-timeout window. Scheduled via `Effect.RescheduleContextTimeout(afterMs)`. */
    const val CONTEXT_TIMEOUT_MS: Long = 30_000L

    /** Punctuation that ends a sentence and resets NextWord state. */
    val sentenceEndPunctuation: Set<Char> =
        setOf('。', '！', '？', '.', '!', '?')

    /**
     * Superset of sentence-end: any char in this set aborts `WordSelected`
     * without recording or predicting. Android preserves its pre-A5-impl
     * superset (includes Chinese punctuation, quote variants, digits via
     * [isNoiseText]'s all-ASCII-digits check). iOS limits the set to the
     * punctuation characters only. Intentional divergence — documented in
     * `nextword-engine-boundary.md` §13 Android addendum.
     */
    private val noisePunctuation: Set<Char> =
        setOf(
            '。',
            '！',
            '？',
            '.',
            '!',
            '?',
            '，',
            ',',
            '、',
            '；',
            ';',
            '：',
            ':',
            '「',
            '」',
            '『',
            '』',
            '"',
            '"',
            '\'',
            '（',
            '）',
            '(',
            ')',
            '【',
            '】',
            '[',
            ']',
            '{',
            '}',
            '—',
            '–',
            '-',
            '～',
            '~',
            '…',
            '·',
            ' ',
            '　',
        )

    /**
     * Decide what the platform executor should do in response to a single
     * lifecycle event. Pure function: same inputs → same outputs.
     */
    fun decide(
        intent: NextWordIntent,
        state: NextWordPersistedState,
        input: NextWordDecisionInput,
    ): NextWordOutcome =
        when (intent) {
            is NextWordIntent.WordSelected -> {
                decideWordSelected(
                    text = intent.text,
                    roman = intent.roman,
                    requireRomanMode = intent.requireRomanMode,
                    triggerPrediction = intent.triggerPrediction,
                    state = state,
                    input = input,
                )
            }

            is NextWordIntent.Backspace -> {
                decideBackspace(lastChar = intent.lastChar, state = state, input = input)
            }

            NextWordIntent.ContextTimeoutFired -> {
                resetAndClearPredictions(state)
            }

            NextWordIntent.ClearForNewComposing -> {
                decideClearForNewComposing(state)
            }

            NextWordIntent.ResetFull -> {
                resetAndClearPredictions(state)
            }
        }

    /**
     * Filter + shape raw service predictions into UI-ready values. Pure
     * function mirroring today's wrapper-side `updateCandidatesWithPredictions`
     * logic but typed on the shared DTO so it can move to shared-core
     * without depending on platform types.
     *
     * Divergence from iOS filter: Android checks `inputMode == "tl"` to
     * decide whether to convert roman; iOS checks `inputMode == .poj`. Same
     * outcome (convert when mode is POJ), different dispatch. Kept
     * consistent with pre-A5 Android behavior.
     */
    fun filterPredictions(
        raw: List<RawNextWordPrediction>,
        settings: NextWordEngineSettings,
    ): List<EnginePrediction> {
        val useTl = settings.inputMode == "tl"
        return raw.mapNotNull { prediction ->
            val roman =
                if (useTl) {
                    prediction.tl
                } else {
                    TaigiPhonetics.tlDisplayToPOJDisplay(prediction.tl)
                }

            if (!settings.isTranslateSwapped && roman.isEmpty()) return@mapNotNull null

            val text = if (roman.isEmpty()) prediction.hanzi else roman
            val subtitle: String? = if (roman.isEmpty()) null else prediction.hanzi

            EnginePrediction(
                text = text,
                subtitle = subtitle,
                hanzi = prediction.hanzi,
                tl = prediction.tl,
                score = prediction.score,
            )
        }
    }

    // region Pure helpers (exposed for testing)

    /**
     * Association window check. Strict `<` at 10s; negative deltas (clock
     * skew / wrapped) return false — a parity correction vs the pre-A5
     * code, which would have returned true for a future-relative state.
     */
    fun shouldRecordAssociation(
        state: NextWordPersistedState,
        nowMs: Long,
    ): Boolean {
        if (state.lastSelectedWord == null) return false
        val delta = nowMs - state.lastSelectionTimeMs
        return delta in 0 until ASSOCIATION_TIMEOUT_MS
    }

    /**
     * Split a compound word on `-` OR whitespace. Android preserves the
     * pre-A5 wrapper-side `split("-", " ")` semantics; iOS splits on `-`
     * only. Intentional divergence per `nextword-engine-boundary.md` §13.
     */
    fun splitCompound(word: String): List<String> {
        if (word.isEmpty()) return emptyList()
        return word.split("-", " ").filter { it.isNotEmpty() }
    }

    /**
     * Build sequential bigram pairs from a compound word. Order preserved
     * so the executor can feed them to `NextWordService.recordAssociation`
     * sequentially — parallel writes would race on the SQLite UNIQUE
     * constraint on `(prev_word, next_word, next_tl)`.
     */
    fun compoundAssociationPairs(
        displayText: String,
        roman: String,
    ): List<NextWordAssociationPair> {
        val parts = splitCompound(displayText)
        val romanParts = splitCompound(roman)
        if (parts.size <= 1) return emptyList()

        val pairs = ArrayList<NextWordAssociationPair>(parts.size - 1)
        for (i in 0 until parts.size - 1) {
            pairs.add(
                NextWordAssociationPair(
                    prev = parts[i],
                    prevTl = romanParts.getOrNull(i) ?: "",
                    next = parts[i + 1],
                    nextTl = romanParts.getOrNull(i + 1) ?: "",
                ),
            )
        }
        return pairs
    }

    /**
     * Punctuation / ASCII space / full-width space / pure-digit text never
     * triggers NextWord. Android superset per §13 addendum — scope limited
     * to the exact char set pre-A5 `NextWordHandler.NOISE_CHARS` contained:
     * [noisePunctuation] already bundles both space variants, so this
     * function does NOT fall back to `Char.isWhitespace()` (which would
     * widen the set to tab / newline / vertical tab — a behavior change
     * vs pre-A5).
     */
    fun isNoiseText(text: String): Boolean {
        if (text.isEmpty()) return true
        return text.all { it in noisePunctuation || (it.code in '0'.code..'9'.code) }
    }

    fun isSentenceEndPunctuation(text: String): Boolean {
        val firstChar = text.firstOrNull() ?: return false
        return firstChar in sentenceEndPunctuation
    }

    // endregion

    // region Intent handlers

    private fun decideWordSelected(
        text: String,
        roman: String,
        requireRomanMode: Boolean,
        triggerPrediction: Boolean,
        state: NextWordPersistedState,
        input: NextWordDecisionInput,
    ): NextWordOutcome {
        // Enter commits raw romanization only; skip entirely in Hanji mode.
        if (requireRomanMode && input.settings.isTranslateSwapped) {
            return NextWordOutcome(newState = state, effects = emptyList())
        }

        // Noise text (punctuation / whitespace / digits) never records or
        // predicts. Sentence-end is a subset of noise: branch on it first
        // so the reset path fires instead of the no-op path.
        if (text.isEmpty() || isNoiseText(text)) {
            return if (isSentenceEndPunctuation(text)) {
                resetAndClearPredictions(state)
            } else {
                NextWordOutcome(newState = state, effects = emptyList())
            }
        }

        // pojDisplayToTLDisplay is idempotent on TL input — safe for POJ and TPS alike.
        val textTl = TaigiPhonetics.pojDisplayToTLDisplay(roman)
        val prevTl = TaigiPhonetics.pojDisplayToTLDisplay(state.lastSelectedRoman ?: "")

        val effects = mutableListOf<NextWordOutcome.Effect>()

        if (input.settings.isAssociationRecordingEnabled) {
            val prevWord = state.lastSelectedWord
            if (prevWord != null && shouldRecordAssociation(state = state, nowMs = input.nowMs)) {
                effects.add(
                    NextWordOutcome.Effect.RecordAssociation(
                        NextWordAssociationPair(
                            prev = prevWord,
                            prevTl = prevTl,
                            next = text,
                            nextTl = textTl,
                        ),
                    ),
                )
            }
            val compound = compoundAssociationPairs(displayText = text, roman = textTl)
            if (compound.isNotEmpty()) {
                effects.add(NextWordOutcome.Effect.RecordCompoundAssociations(compound))
            }
        }

        effects.add(NextWordOutcome.Effect.RescheduleContextTimeout(afterMs = CONTEXT_TIMEOUT_MS))

        val newState =
            state.copy(
                lastSelectedWord = text,
                lastSelectedRoman = textTl,
                lastSelectionTimeMs = input.nowMs,
                currentGeneration = state.currentGeneration + 1,
            )

        if (triggerPrediction) {
            effects.add(
                NextWordOutcome.Effect.QueryPredictions(
                    word = text,
                    roman = textTl,
                    generation = newState.currentGeneration,
                    nowMs = input.nowMs,
                ),
            )
        }

        return NextWordOutcome(newState = newState, effects = effects)
    }

    private fun decideBackspace(
        lastChar: String,
        state: NextWordPersistedState,
        input: NextWordDecisionInput,
    ): NextWordOutcome {
        val newState =
            state.copy(
                lastSelectedWord = lastChar,
                lastSelectedRoman = null,
                lastSelectionTimeMs = input.nowMs,
                currentGeneration = state.currentGeneration + 1,
            )
        return NextWordOutcome(
            newState = newState,
            effects =
                listOf(
                    NextWordOutcome.Effect.QueryPredictions(
                        word = lastChar,
                        roman = "",
                        generation = newState.currentGeneration,
                        nowMs = input.nowMs,
                    ),
                ),
        )
    }

    private fun decideClearForNewComposing(state: NextWordPersistedState): NextWordOutcome {
        val newState =
            state.copy(
                isShowing = false,
                currentGeneration = state.currentGeneration + 1,
            )
        val effects: List<NextWordOutcome.Effect> =
            if (state.isShowing) {
                listOf(NextWordOutcome.Effect.ClearPredictionsUI(generation = newState.currentGeneration))
            } else {
                emptyList()
            }
        return NextWordOutcome(newState = newState, effects = effects)
    }

    /**
     * Shared reset path used by sentence-end punctuation, context timeout,
     * and `ResetFull` intents. Cancels the timeout, optionally clears the
     * UI, zeroes state, bumps generation.
     */
    private fun resetAndClearPredictions(state: NextWordPersistedState): NextWordOutcome {
        val newGeneration = state.currentGeneration + 1
        val newState = NextWordPersistedState.initial.copy(currentGeneration = newGeneration)

        val effects = mutableListOf<NextWordOutcome.Effect>(NextWordOutcome.Effect.CancelContextTimeout)
        if (state.isShowing) {
            effects.add(NextWordOutcome.Effect.ClearPredictionsUI(generation = newGeneration))
        }
        return NextWordOutcome(newState = newState, effects = effects)
    }

    // endregion
}
