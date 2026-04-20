package com.siansiansu.taigikeyboard.ime.text.smartbar

import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.nextword.NextWordDecisionInput
import com.siansiansu.taigikeyboard.ime.core.nextword.NextWordEngine
import com.siansiansu.taigikeyboard.ime.core.nextword.NextWordEngineSettings
import com.siansiansu.taigikeyboard.ime.core.nextword.NextWordIntent
import com.siansiansu.taigikeyboard.ime.core.nextword.NextWordOutcome
import com.siansiansu.taigikeyboard.ime.core.nextword.NextWordPersistedState
import com.siansiansu.taigikeyboard.ime.core.nextword.RawNextWordPrediction
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettingsProvider
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiPhonetics
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Platform executor for NextWord prediction on Android.
 *
 * Thin wrapper around [NextWordEngine] (A5-impl): the engine owns pure
 * decision / filtering logic; this executor owns
 * - the coroutine-scheduled context-timeout job,
 * - `EngineSettingsProvider` reads,
 * - `NextWordService` I/O (recordAssociation / predict),
 * - UI callbacks (`onUpdateCandidates` / `onClearCandidates`),
 * - the generation counter check on async prediction results.
 *
 * Public API is preserved from pre-A5 shape so callers in
 * [SmartbarManager] / [com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateClickHandler]
 * do not change.
 *
 * Mirrors iOS `NextWord/NextWordController.swift` post-G5-impl shape.
 * Android-side binding contract in `docs/architecture/nextword-engine-boundary.md`
 * §13 Android Addendum.
 */
class NextWordHandler(
    private val scope: CoroutineScope,
    private val settingsProvider: EngineSettingsProvider,
    private val nextWord: NextWordService,
    private val logger: LoggerBackend,
    private val onUpdateCandidates: (List<TaigiWord>) -> Unit,
    private val onClearCandidates: () -> Unit,
) {
    private var state: NextWordPersistedState = NextWordPersistedState.initial

    /** Active context-timeout job. `null` when no timeout is scheduled. */
    private var contextTimeoutJob: Job? = null

    fun getLastSelectedWord(): String? = state.lastSelectedWord

    fun isShowingNextWordCandidates(): Boolean = state.isShowing

    /**
     * Zero association state + cancel any pending timeout, without clearing
     * the UI. Called on `onStartInputView` when switching input fields.
     *
     * Deliberately NOT routed through [NextWordEngine.decide] — the engine's
     * [NextWordIntent.ResetFull] emits `ClearPredictionsUI` when `isShowing`,
     * which would call back into [onClearCandidates]. This call-site is a
     * plain state sync, so the wrapper mutates directly.
     */
    fun resetContext() {
        cancelContextTimeoutJob()
        state = NextWordPersistedState.initial.copy(currentGeneration = state.currentGeneration + 1)
    }

    /**
     * Mark `isShowing = false` + bump generation to drop any in-flight
     * prediction result that might land after this point.
     *
     * Deliberately NOT routed through [NextWordEngine.decide] — this is
     * called from [SmartbarManager.clearCandidates] AFTER the UI is already
     * being cleared; re-emitting [onClearCandidates] would recurse.
     */
    fun clearNextWordState() {
        state = state.copy(isShowing = false, currentGeneration = state.currentGeneration + 1)
    }

    /**
     * Direct `isShowing` setter used by [SmartbarManager.updateCandidates]
     * after the candidate bar has been rendered with NextWord suggestions.
     * Does NOT bump generation — the prediction round that landed these
     * suggestions is still the current round.
     */
    fun setShowingNextWord(showing: Boolean) {
        state = state.copy(isShowing = showing)
    }

    /**
     * Handle a word selection (candidate tap / Enter commit).
     *
     * [committedText] is the full committed-text tail from the host editor;
     * if its last char is sentence-end punctuation we route [NextWordIntent.ResetFull]
     * instead of [NextWordIntent.WordSelected]. This mirrors the pre-A5
     * wrapper-side sentence-end reset. [displayText] is the actual word
     * selected (passed through [NextWordEngine.isNoiseText] / noise-guard
     * inside the engine).
     *
     * [rawInput] stays on the signature for pre-A5 call-site parity; the
     * engine does not consume it.
     */
    fun handleNextWordPrediction(
        displayText: String,
        committedText: String,
        roman: String,
        @Suppress("UNUSED_PARAMETER") hanzi: String? = null,
        @Suppress("UNUSED_PARAMETER") rawInput: String = "",
    ) {
        // Pre-A5 parity: when the committed-text tail ends with sentence-end
        // punctuation, zero the association context BEFORE proceeding with
        // the word-selection. The subsequent `WordSelected` dispatch sees
        // `lastSelectedWord == null`, so `shouldRecordAssociation` is false
        // (no bigram recorded at the sentence boundary) and the new word
        // starts a fresh context. Deliberately NOT routed through
        // `NextWordIntent.ResetFull` — that would drop the prediction query
        // this call-site still expects.
        val lastCommittedChar = committedText.lastOrNull()
        if (lastCommittedChar != null && lastCommittedChar in NextWordEngine.sentenceEndPunctuation) {
            state = state.copy(lastSelectedWord = null, lastSelectedRoman = null)
        }
        apply(
            NextWordIntent.WordSelected(
                text = displayText,
                roman = roman,
                requireRomanMode = false,
                triggerPrediction = true,
            ),
        )
    }

    /**
     * Update `lastSelectedWord` without triggering a prediction query —
     * invoked when Space confirms composing text.
     *
     * Deliberately NOT routed through [NextWordEngine.decide]. Pre-A5
     * Android semantics on this path differ from iOS `wordSelected`:
     * - Android records **only** compound associations inside the word
     *   (no `prev→this` bigram), even if a prior `lastSelectedWord` sits
     *   inside the 10 s association window.
     * - Android does NOT reschedule the context timeout.
     * - Android does NOT bump the generation counter.
     *
     * Refactor-freeze preserves these semantics verbatim. Routing through
     * `WordSelected(triggerPrediction=false)` would have added the
     * `prev→this` record + the active-timer reschedule as new behavior on
     * Space; that convergence toward iOS is a future parity correction,
     * not in scope for A5-impl.
     */
    fun updateLastSelectedWord(
        word: String,
        roman: String? = null,
    ) {
        if (word.isEmpty()) return
        val romanTl = TaigiPhonetics.pojDisplayToTLDisplay(roman ?: word)
        if (currentEngineSettings().isAssociationRecordingEnabled) {
            val compound =
                NextWordEngine.compoundAssociationPairs(displayText = word, roman = romanTl)
            if (compound.isNotEmpty()) {
                recordCompoundAssociationsAsync(compound.map { it.toRecordArgs() })
            }
        }
        val nowMs = System.currentTimeMillis()
        state =
            if (NextWordEngine.isNoiseText(word)) {
                state.copy(lastSelectionTimeMs = nowMs)
            } else {
                state.copy(
                    lastSelectedWord = word,
                    lastSelectedRoman = romanTl,
                    lastSelectionTimeMs = nowMs,
                )
            }
    }

    /**
     * Backspace handler: re-predict from the trailing character of the
     * remaining text, or reset + clear UI unconditionally if text is empty.
     *
     * Empty-path preserves pre-A5 behavior (unconditional `onClearCandidates()`
     * regardless of prior `isShowing` state) by skipping the engine's
     * `ResetFull` — whose `ClearPredictionsUI` effect gates on `isShowing`
     * — and clearing directly. Active-timer cancel + generation bump are
     * applied so any in-flight prediction is invalidated.
     */
    fun handleBackspaceForNextWord(textBeforeCursor: String) {
        val trimmed = textBeforeCursor.trimEnd()
        if (trimmed.isEmpty()) {
            cancelContextTimeoutJob()
            state = NextWordPersistedState.initial.copy(currentGeneration = state.currentGeneration + 1)
            onClearCandidates()
            return
        }
        apply(NextWordIntent.Backspace(lastChar = trimmed.last().toString()))
    }

    // region Intent dispatch

    private fun apply(intent: NextWordIntent) {
        val input = makeDecisionInput()
        val outcome = NextWordEngine.decide(intent = intent, state = state, input = input)
        state = outcome.newState
        for (effect in outcome.effects) {
            execute(effect)
        }
    }

    private fun makeDecisionInput(): NextWordDecisionInput =
        NextWordDecisionInput(
            nowMs = System.currentTimeMillis(),
            settings = currentEngineSettings(),
        )

    private fun currentEngineSettings(): NextWordEngineSettings = settingsProvider.current.toNextWordSettings()

    // endregion

    // region Effect interpreter

    private fun execute(effect: NextWordOutcome.Effect) {
        when (effect) {
            is NextWordOutcome.Effect.RescheduleContextTimeout -> {
                scheduleContextTimeout(afterMs = effect.afterMs)
            }

            NextWordOutcome.Effect.CancelContextTimeout -> {
                cancelContextTimeoutJob()
            }

            is NextWordOutcome.Effect.RecordAssociation -> {
                recordAssociationAsync(effect.pair.toRecordArgs())
            }

            is NextWordOutcome.Effect.RecordCompoundAssociations -> {
                recordCompoundAssociationsAsync(effect.pairs.map { it.toRecordArgs() })
            }

            is NextWordOutcome.Effect.QueryPredictions -> {
                dispatchPredictionQuery(
                    word = effect.word,
                    roman = effect.roman,
                    generation = effect.generation,
                    nowMs = effect.nowMs,
                )
            }

            is NextWordOutcome.Effect.ClearPredictionsUI -> {
                clearPredictionsUI()
            }
        }
    }

    private fun recordAssociationAsync(args: RecordArgs) {
        scope.launch {
            if (!currentEngineSettings().isAssociationRecordingEnabled) return@launch
            nextWord.recordAssociation(
                prev = args.prev,
                prevTl = args.prevTl,
                nextHanzi = args.next,
                nextTl = args.nextTl,
            )
            logger.debug(TAG) { "[NEXTWORD] Record: '${args.prev}(${args.prevTl})' → '${args.next}'" }
        }
    }

    /**
     * Loop compound associations sequentially inside one coroutine to avoid
     * races on the SQLite UNIQUE constraint `(prev_word, next_word, next_tl)`.
     */
    private fun recordCompoundAssociationsAsync(pairs: List<RecordArgs>) {
        if (pairs.isEmpty()) return
        scope.launch {
            if (!currentEngineSettings().isAssociationRecordingEnabled) return@launch
            for (args in pairs) {
                nextWord.recordAssociation(
                    prev = args.prev,
                    prevTl = args.prevTl,
                    nextHanzi = args.next,
                    nextTl = args.nextTl,
                )
                logger.debug(TAG) { "[NEXTWORD] Record compound: '${args.prev}' → '${args.next}'" }
            }
        }
    }

    /**
     * Dispatch the async prediction query.
     *
     * [nowMs] is the decision-time clock snapshot propagated from
     * `Effect.QueryPredictions` — reused verbatim on the `predict` call so
     * the association-window check (inside `NextWordEngine.decide`) and
     * the user-row decay scoring (inside `NextWordService.predict`) see
     * ONE consistent "now" per intent. Re-reading `currentTimeMillis()`
     * here would drift the two clocks by however long the coroutine
     * dispatch takes, violating `nextword-engine-boundary.md` §13.3.
     */
    private fun dispatchPredictionQuery(
        word: String,
        roman: String,
        generation: Long,
        nowMs: Long,
    ) {
        logger.debug(TAG) { "[NEXTWORD] Query gen=$generation word='$word' roman='$roman'" }
        scope.launch {
            val raw =
                nextWord.predict(
                    word = word,
                    roman = roman,
                    settings = settingsProvider.current,
                    nowMs = nowMs,
                )
            withContext(Dispatchers.Main) {
                handleQueryResult(raw = raw, generation = generation)
            }
        }
    }

    /**
     * Handle an async prediction result. Drops late results when the
     * generation tagged at dispatch time no longer matches the current
     * state generation — the intent that launched the query has been
     * superseded (context timeout, backspace-to-empty, new word selected,
     * etc.), so rendering the result would be stale.
     *
     * On non-empty results the executor reschedules the context-timeout
     * job and sets `isShowing = true` — mirrors iOS `handleQueryResult`
     * (`NextWordController.swift:180`).
     */
    private fun handleQueryResult(
        raw: List<RawNextWordPrediction>,
        generation: Long,
    ) {
        if (generation != state.currentGeneration) {
            logger.debug(TAG) {
                "[NEXTWORD] Drop stale result gen=$generation current=${state.currentGeneration}"
            }
            return
        }

        val freshSettings = currentEngineSettings()
        val filtered = NextWordEngine.filterPredictions(raw, freshSettings)

        if (filtered.isEmpty()) {
            state = state.copy(isShowing = false)
            onClearCandidates()
            return
        }

        val words =
            filtered.mapIndexed { index, prediction ->
                TaigiWord(
                    id = -index - 1,
                    roman = if (prediction.subtitle != null) prediction.text else "",
                    hanzi = prediction.hanzi,
                    lengthScore = prediction.score.toInt(),
                )
            }
        onUpdateCandidates(words)
        state = state.copy(isShowing = true)
        scheduleContextTimeout(NextWordEngine.CONTEXT_TIMEOUT_MS)
    }

    private fun clearPredictionsUI() {
        onClearCandidates()
    }

    // endregion

    // region Context-timeout scheduling

    /**
     * Schedule (or reschedule) the active context-timeout job. Cancel
     * the old job before launching the new one so the executor keeps a
     * single owned [Job] — parallel jobs would race on the fire path.
     */
    private fun scheduleContextTimeout(afterMs: Long) {
        cancelContextTimeoutJob()
        contextTimeoutJob =
            scope.launch {
                delay(afterMs)
                onContextTimeoutFired()
            }
    }

    private fun cancelContextTimeoutJob() {
        contextTimeoutJob?.cancel()
        contextTimeoutJob = null
    }

    private fun onContextTimeoutFired() {
        logger.debug(TAG) { "[NEXTWORD] Context timeout fired" }
        contextTimeoutJob = null
        apply(NextWordIntent.ContextTimeoutFired)
    }

    // endregion

    private data class RecordArgs(
        val prev: String,
        val prevTl: String,
        val next: String,
        val nextTl: String,
    )

    private fun com.siansiansu.taigikeyboard.ime.core.nextword.NextWordAssociationPair.toRecordArgs(): RecordArgs =
        RecordArgs(prev, prevTl, next, nextTl)

    companion object {
        private const val TAG = "NextWordHandler"

        /**
         * Noise classifier preserved for [SmartbarManager] / test parity.
         * [NextWordEngine.isNoiseText] is the canonical decision point
         * inside the engine; this wrapper-level helper exists only because
         * the old module-level `isNoise` function was exposed on
         * `NextWordHandler.Companion` and outside callers depended on it.
         */
        fun isNoise(word: String): Boolean = NextWordEngine.isNoiseText(word)

        /**
         * Compound splitter preserved for outside callers; delegates to
         * [NextWordEngine.splitCompound] which preserves the Android
         * `"-"` + whitespace split semantics (intentional divergence from
         * iOS documented in `nextword-engine-boundary.md` §13).
         */
        fun splitCompoundWord(word: String): List<String> = NextWordEngine.splitCompound(word)

        /**
         * Extract the trailing "word" from [text] using whitespace and
         * ASCII punctuation as separators. Unchanged from pre-A5 shape.
         */
        fun extractCurrentWord(text: String): String {
            val trimmed = text.trimEnd()
            if (trimmed.isEmpty()) return ""
            val lastSeparatorIndex = trimmed.indexOfLast { it.isWhitespace() || it in ".,!?;:" }
            return if (lastSeparatorIndex >= 0) {
                trimmed.substring(lastSeparatorIndex + 1)
            } else {
                trimmed
            }
        }
    }
}

private fun EngineSettings.toNextWordSettings(): NextWordEngineSettings =
    NextWordEngineSettings(
        inputMode = inputMode,
        isTranslateSwapped = isTranslateSwapped,
        isAssociationRecordingEnabled = isAssociationRecordingEnabled,
    )
