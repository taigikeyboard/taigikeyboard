package com.siansiansu.taigikeyboard.ime.text.smartbar

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettingsProvider
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Platform executor for NextWord prediction on Android — post-v3.5.5 Rust slice.
 *
 * Decision logic + persisted state moved into `engine/nextword/` (Rust); this
 * handler is the Android-side platform executor:
 * - serializes intents through `RustEngineBridge.nextword*`,
 * - interprets the returned `NextWordDecideResult.Effect` list against
 *   coroutine-scheduled timeout, `NextWordService` I/O, UI callbacks,
 * - caches `lastSelectedWord` / `isShowing` echoed back from the engine so
 *   `SmartbarManager` / `CandidateClickHandler` reads stay synchronous,
 * - pushes UI visibility back into the engine via `nextwordSetIsShowing`
 *   after async predict() results render so downstream clear/reset paths
 *   gate `clearPredictionsUI` correctly (commit 7 bridge gap fix).
 *
 * Public API preserved from pre-Rust shape so callers in [SmartbarManager] /
 * [CandidateClickHandler] do not change. Mirrors iOS
 * `NextWord/NextWordController.swift`.
 */
class NextWordHandler(
    private val scope: CoroutineScope,
    private val settingsProvider: EngineSettingsProvider,
    private val nextWord: NextWordService,
    private val logger: LoggerBackend,
    private val onUpdateCandidates: (List<TaigiWord>) -> Unit,
    private val onClearCandidates: () -> Unit,
) {
    // / Cached state echoed from every decide call. Synchronous read for
    // / SmartbarManager / CandidateClickHandler.
    private var cachedLastSelectedWord: String? = null
    private var cachedIsShowing: Boolean = false

    // / Active context-timeout job. `null` when no timeout scheduled.
    private var contextTimeoutJob: Job? = null

    // / Per-IME-session envelope generation. Engine `EngineHandle` resets state
    // / on mismatch BEFORE applying the request. Bumped on real input-context
    // / changes via [resetContext] (called from `onStartInputView`).
    private var envelopeGen: Long = 1L

    fun getLastSelectedWord(): String? = cachedLastSelectedWord

    fun isShowingNextWordCandidates(): Boolean = cachedIsShowing

    /**
     * Envelope generation owned by this executor; exposed so other shared
     * collaborators (e.g. `TaigiAutocompleteService.applyContextBoost`) can
     * pass the same value to bridge calls and avoid spurious state resets.
     */
    fun nextwordEnvelopeGeneration(): Long = envelopeGen

    /**
     * Zero association state + cancel any pending timeout. Called on
     * `onStartInputView` when switching input fields.
     *
     * Bumps `envelopeGen` so the engine handle's mismatch reset wipes Rust
     * state to default BEFORE the subsequent [RustEngineBridge.nextwordResetFull]
     * call processes — which then bumps `current_generation`, emits
     * `CancelContextTimeout` (already cancelled above; idempotent), and
     * skips `ClearPredictionsUI` because `is_showing` is now false post-reset.
     * Net effect matches the pre-Rust `resetContext()` behavior (zero
     * association state, cancel timer, no UI emit).
     */
    fun resetContext() {
        cancelContextTimeoutJob()
        envelopeGen += 1L
        val settings = settingsProvider.current
        val result = RustEngineBridge.nextwordResetFull(
            nowMs = System.currentTimeMillis(),
            mode = settings.inputMode.toEngineInputMode(),
            translateSwapped = settings.isTranslateSwapped,
            associationRecordingEnabled = settings.isAssociationRecordingEnabled,
            generation = envelopeGen,
        )
        applyDecideResult(result)
    }

    /**
     * Mark `isShowing = false` + bump generation to drop any in-flight
     * prediction result that might land after this point. Called from
     * [SmartbarManager.clearCandidates] AFTER the UI is already cleared, so
     * we explicitly close the visibility gate (`SetIsShowing(false)`) before
     * dispatching `ClearForNewComposing` to avoid recursive `onClearCandidates`.
     */
    fun clearNextWordState() {
        val settings = settingsProvider.current
        val cfgMode = settings.inputMode.toEngineInputMode()
        val swapped = settings.isTranslateSwapped
        val recording = settings.isAssociationRecordingEnabled
        applyDecideResult(
            RustEngineBridge.nextwordSetIsShowing(false, cfgMode, swapped, recording, envelopeGen),
        )
        applyDecideResult(
            RustEngineBridge.nextwordClearForNewComposing(
                System.currentTimeMillis(),
                cfgMode,
                swapped,
                recording,
                envelopeGen,
            ),
        )
    }

    /**
     * Direct `is_showing` setter — invoked by [SmartbarManager.updateCandidates]
     * after the candidate bar has been rendered with NextWord suggestions.
     * Routes through [RustEngineBridge.nextwordSetIsShowing]; no generation
     * bump per the bridge contract (the prediction round that produced
     * these suggestions is still current).
     */
    fun setShowingNextWord(showing: Boolean) {
        val settings = settingsProvider.current
        applyDecideResult(
            RustEngineBridge.nextwordSetIsShowing(
                showing,
                mode = settings.inputMode.toEngineInputMode(),
                translateSwapped = settings.isTranslateSwapped,
                associationRecordingEnabled = settings.isAssociationRecordingEnabled,
                generation = envelopeGen,
            ),
        )
    }

    /**
     * Handle a word selection (candidate tap / Enter commit).
     *
     * [committedText] is the full committed-text tail; if its last char is
     * sentence-end punctuation we route a `ResetFull` first to zero the
     * association context — pre-Rust parity (the subsequent `WordSelected`
     * sees a clean state, so no bigram is recorded across the sentence
     * boundary). The slight strengthening over the pre-Rust shadow-mutation
     * (also bumps generation + cancels timer) is acceptable: this path is
     * rare and the timer/gen state is about to be replaced by the
     * `WordSelected` intent anyway.
     *
     * [rawInput] / [hanzi] stay on the signature for pre-Rust call-site
     * parity; the engine does not consume them.
     */
    fun handleNextWordPrediction(
        displayText: String,
        committedText: String,
        roman: String,
        @Suppress("UNUSED_PARAMETER") hanzi: String? = null,
        @Suppress("UNUSED_PARAMETER") rawInput: String = "",
    ) {
        val settings = settingsProvider.current
        val nowMs = System.currentTimeMillis()

        val lastCommittedChar = committedText.lastOrNull()
        if (lastCommittedChar != null && lastCommittedChar in SENTENCE_END_PUNCTUATION) {
            applyDecideResult(
                RustEngineBridge.nextwordResetFull(
                    nowMs = nowMs,
                    mode = settings.inputMode.toEngineInputMode(),
                    translateSwapped = settings.isTranslateSwapped,
                    associationRecordingEnabled = settings.isAssociationRecordingEnabled,
                    generation = envelopeGen,
                ),
            )
        }

        applyDecideResult(
            RustEngineBridge.nextwordWordSelected(
                text = displayText,
                roman = roman,
                requireRomanMode = false,
                triggerPrediction = true,
                nowMs = nowMs,
                mode = settings.inputMode.toEngineInputMode(),
                translateSwapped = settings.isTranslateSwapped,
                associationRecordingEnabled = settings.isAssociationRecordingEnabled,
                generation = envelopeGen,
            ),
        )
    }

    /**
     * Update `lastSelectedWord` without triggering a prediction query —
     * invoked when Space confirms composing text. Routes through the
     * Android-only [RustEngineBridge.nextwordUpdateLastSelectedWord] intent
     * (audit §5 #5 / Codex v1 P1) so pre-Rust Android-specific semantics
     * are preserved verbatim:
     * - records ONLY compound associations inside the word (no `prev→this`),
     * - does NOT reschedule the context timeout,
     * - does NOT bump `current_generation`.
     */
    fun updateLastSelectedWord(
        word: String,
        roman: String? = null,
    ) {
        if (word.isEmpty()) return
        val settings = settingsProvider.current
        applyDecideResult(
            RustEngineBridge.nextwordUpdateLastSelectedWord(
                text = word,
                roman = roman ?: word,
                nowMs = System.currentTimeMillis(),
                mode = settings.inputMode.toEngineInputMode(),
                translateSwapped = settings.isTranslateSwapped,
                associationRecordingEnabled = settings.isAssociationRecordingEnabled,
                generation = envelopeGen,
            ),
        )
    }

    /**
     * Backspace handler: re-predict from the trailing character of the
     * remaining text, or unconditionally clear UI when text is empty.
     *
     * Empty path bumps `envelopeGen` (forces engine state drop) + dispatches
     * `ResetFull` to cancel timer + bump `current_generation`, then forces
     * `onClearCandidates()` regardless of the engine's `clearPredictionsUI`
     * gate. Preserves the pre-Rust unconditional-clear behavior (the engine
     * gate is closed because envelope-mismatch reset wiped `is_showing` to
     * false before `ResetFull` processed).
     */
    fun handleBackspaceForNextWord(textBeforeCursor: String) {
        val trimmed = textBeforeCursor.trimEnd()
        val settings = settingsProvider.current
        val nowMs = System.currentTimeMillis()
        if (trimmed.isEmpty()) {
            cancelContextTimeoutJob()
            envelopeGen += 1L
            applyDecideResult(
                RustEngineBridge.nextwordResetFull(
                    nowMs = nowMs,
                    mode = settings.inputMode.toEngineInputMode(),
                    translateSwapped = settings.isTranslateSwapped,
                    associationRecordingEnabled = settings.isAssociationRecordingEnabled,
                    generation = envelopeGen,
                ),
            )
            onClearCandidates()
            cachedIsShowing = false
            return
        }
        applyDecideResult(
            RustEngineBridge.nextwordBackspace(
                lastChar = trimmed.last().toString(),
                nowMs = nowMs,
                mode = settings.inputMode.toEngineInputMode(),
                translateSwapped = settings.isTranslateSwapped,
                associationRecordingEnabled = settings.isAssociationRecordingEnabled,
                generation = envelopeGen,
            ),
        )
    }

    // region Effect interpretation

    // / Mirror engine state echo, then run effects in the order the engine
    // / emitted. Runs on the IME main thread (caller invariant).
    private fun applyDecideResult(result: RustEngineBridge.NextWordDecideResult) {
        cachedLastSelectedWord = result.lastSelectedWord
        cachedIsShowing = result.isShowing
        for (effect in result.effects) {
            execute(effect)
        }
    }

    private fun execute(effect: RustEngineBridge.NextWordDecideResult.Effect) {
        when (effect) {
            is RustEngineBridge.NextWordDecideResult.Effect.RescheduleContextTimeout -> {
                scheduleContextTimeout(afterMs = effect.afterMs)
            }

            RustEngineBridge.NextWordDecideResult.Effect.CancelContextTimeout -> {
                cancelContextTimeoutJob()
            }

            is RustEngineBridge.NextWordDecideResult.Effect.RecordAssociation -> {
                recordAssociationAsync(effect.pair)
            }

            is RustEngineBridge.NextWordDecideResult.Effect.RecordCompoundAssociations -> {
                recordCompoundAssociationsAsync(effect.pairs)
            }

            is RustEngineBridge.NextWordDecideResult.Effect.QueryPredictions -> {
                dispatchPredictionQuery(
                    word = effect.word,
                    roman = effect.roman,
                    queryGeneration = effect.generation,
                    nowMs = effect.nowMs,
                )
            }

            is RustEngineBridge.NextWordDecideResult.Effect.ClearPredictionsUI -> {
                onClearCandidates()
                cachedIsShowing = false
            }
        }
    }

    private fun recordAssociationAsync(pair: RustEngineBridge.NextWordAssociationPair) {
        scope.launch {
            if (!settingsProvider.current.isAssociationRecordingEnabled) return@launch
            nextWord.recordAssociation(
                prev = pair.prev,
                prevTl = pair.prevTl,
                nextHanzi = pair.next,
                nextTl = pair.nextTl,
            )
            logger.debug(TAG) { "[NEXTWORD] Record: '${pair.prev}(${pair.prevTl})' → '${pair.next}'" }
        }
    }

    /**
     * Loop sequentially in one coroutine to avoid races on the SQLite
     * UNIQUE constraint `(prev_word, next_word, next_tl)`.
     */
    private fun recordCompoundAssociationsAsync(pairs: List<RustEngineBridge.NextWordAssociationPair>) {
        if (pairs.isEmpty()) return
        scope.launch {
            if (!settingsProvider.current.isAssociationRecordingEnabled) return@launch
            for (pair in pairs) {
                nextWord.recordAssociation(
                    prev = pair.prev,
                    prevTl = pair.prevTl,
                    nextHanzi = pair.next,
                    nextTl = pair.nextTl,
                )
                logger.debug(TAG) { "[NEXTWORD] Record compound: '${pair.prev}' → '${pair.next}'" }
            }
        }
    }

    /**
     * Dispatch the async prediction query. [nowMs] from the effect is reused
     * verbatim on the predict() call so the association-window check (Rust
     * side) and user-row decay scoring (filter step) see ONE consistent
     * "now" per intent — `nextword-engine-boundary.md` §13.3.
     */
    private fun dispatchPredictionQuery(
        word: String,
        roman: String,
        queryGeneration: Long,
        nowMs: Long,
    ) {
        logger.debug(TAG) { "[NEXTWORD] Query gen=$queryGeneration word='$word' roman='$roman'" }
        scope.launch {
            val raw = nextWord.predict(
                word = word,
                roman = roman,
                settings = settingsProvider.current,
                nowMs = nowMs,
            )
            withContext(Dispatchers.Main) {
                handleQueryResult(raw = raw, queryGeneration = queryGeneration, nowMs = nowMs)
            }
        }
    }

    /**
     * Resolve an async prediction query. Pushes raw rows back through
     * [RustEngineBridge.nextwordFilter] for score+merge+sort+limit + stale
     * generation drop. Renders the result, then pushes the new visibility
     * back to engine state via `nextwordSetIsShowing` so downstream
     * clear/reset paths can emit `ClearPredictionsUI` correctly.
     */
    private fun handleQueryResult(
        raw: List<RustEngineBridge.NextWordRawRow>,
        queryGeneration: Long,
        nowMs: Long,
    ) {
        val settings = settingsProvider.current
        val filterResult = RustEngineBridge.nextwordFilter(
            raw = raw,
            queryGeneration = queryGeneration,
            nowMs = nowMs,
            limit = 30,
            mode = settings.inputMode.toEngineInputMode(),
            translateSwapped = settings.isTranslateSwapped,
            associationRecordingEnabled = settings.isAssociationRecordingEnabled,
            generation = envelopeGen,
        )
        if (filterResult.wasStale) {
            logger.debug(TAG) { "[NEXTWORD] Drop stale result gen=$queryGeneration" }
            return
        }

        val nowShowing = filterResult.predictions.isNotEmpty()
        if (nowShowing) {
            val words = filterResult.predictions.mapIndexed { index, p ->
                TaigiWord(
                    id = -index - 1,
                    // Pre-Rust convention: emit roman in `roman` only when
                    // subtitle is present (== prediction is mode-shaped to
                    // roman-display). Hanji-only predictions land with
                    // subtitle=null and roman="" so SmartbarManager renders
                    // hanzi without the roman line.
                    roman = if (p.subtitle != null) p.text else "",
                    hanzi = p.hanzi,
                    lengthScore = p.score.toInt(),
                )
            }
            onUpdateCandidates(words)
            scheduleContextTimeout(CONTEXT_TIMEOUT_MS)
        } else {
            onClearCandidates()
        }

        applyDecideResult(
            RustEngineBridge.nextwordSetIsShowing(
                nowShowing,
                mode = settings.inputMode.toEngineInputMode(),
                translateSwapped = settings.isTranslateSwapped,
                associationRecordingEnabled = settings.isAssociationRecordingEnabled,
                generation = envelopeGen,
            ),
        )
    }

    // endregion

    // region Context-timeout scheduling

    private fun scheduleContextTimeout(afterMs: Long) {
        cancelContextTimeoutJob()
        contextTimeoutJob = scope.launch {
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
        val settings = settingsProvider.current
        applyDecideResult(
            RustEngineBridge.nextwordContextTimeoutFired(
                nowMs = System.currentTimeMillis(),
                mode = settings.inputMode.toEngineInputMode(),
                translateSwapped = settings.isTranslateSwapped,
                associationRecordingEnabled = settings.isAssociationRecordingEnabled,
                generation = envelopeGen,
            ),
        )
    }

    // endregion

    companion object {
        private const val TAG = "NextWordHandler"

        /**
         * Mirrors `engine/nextword/src/decide.rs` `CONTEXT_TIMEOUT_MS = 30_000`.
         * CROSS-PLATFORM INVARIANT: changing this value requires a paired
         * update in the Rust crate + an `INVARIANT_*` parity-test mirror.
         */
        const val CONTEXT_TIMEOUT_MS: Long = 30_000L

        /**
         * Sentence-end pre-check set used by [handleNextWordPrediction].
         * Mirrors `engine/nextword/src/decide.rs` `SENTENCE_END_PUNCTUATION`.
         * CROSS-PLATFORM INVARIANT — keep in sync with the Rust constant.
         */
        private val SENTENCE_END_PUNCTUATION: Set<Char> =
            setOf('。', '！', '？', '.', '!', '?')

        /**
         * Extract the trailing "word" from [text] using whitespace and ASCII
         * punctuation as separators. Pure platform helper (UI-side,
         * unrelated to the engine state machine); preserved across the
         * Rust swap because `CandidateClickHandler` depends on it.
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

/**
 * Map the platform's `EngineSettings.inputMode: String` to the
 * `RustEngineBridge` `InputMode` enum. Mirrors the conversion in
 * `CandidateUpdateCoordinator`. `"tps"` is a layout, not an engine
 * mode — falls back to TL because the NextWord engine has no TPS arm.
 */
private fun String.toEngineInputMode(): InputMode = when (this) {
    "poj" -> InputMode.POJ
    "tl", "tps" -> InputMode.TL
    "english" -> InputMode.ENGLISH
    else -> InputMode.POJ
}
