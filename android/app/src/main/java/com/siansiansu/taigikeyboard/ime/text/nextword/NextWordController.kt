// NextWord platform executor — Android wiring into the Rust nextword crate (since v3.5.5).
// Decisions/state live in Rust; this file handles intent serialization, Effect interpretation
// (timeout / UI callback), and caches lastSelectedWord/predictionsVisible from engine.

package com.siansiansu.taigikeyboard.ime.text.nextword

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.engineInputMode
import com.siansiansu.taigikeyboard.engine.nextwordBackspace
import com.siansiansu.taigikeyboard.engine.nextwordClearForNewComposing
import com.siansiansu.taigikeyboard.engine.nextwordContextTimeoutFired
import com.siansiansu.taigikeyboard.engine.nextwordPredictNext
import com.siansiansu.taigikeyboard.engine.nextwordResetAll
import com.siansiansu.taigikeyboard.engine.nextwordSetPredictionsVisible
import com.siansiansu.taigikeyboard.engine.nextwordUpdateLastSelectedWord
import com.siansiansu.taigikeyboard.engine.nextwordWordSelected
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.settings.EngineSettings
import com.siansiansu.taigikeyboard.ime.settings.EngineSettingsProvider
import com.siansiansu.taigikeyboard.ime.settings.InputMode
import com.siansiansu.taigikeyboard.ime.text.candidates.CandidateClickHandler
import com.siansiansu.taigikeyboard.ime.text.candidates.splitIntoSingleScriptCells
import com.siansiansu.taigikeyboard.ime.text.keyboard.lastGrapheme
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarManager
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
 *   coroutine-scheduled timeout and UI callbacks (the engine keeps the
 *   learned bigrams and reads them for predictions itself — roadmap P8b),
 * - caches `lastSelectedWord` / `predictionsVisible` echoed back from the engine so
 *   `SmartbarManager` / `CandidateClickHandler` reads stay synchronous,
 * - pushes UI visibility back into the engine via `nextwordSetPredictionsVisible`
 *   after async predict() results render so downstream clear/reset paths
 *   gate `clearPredictionsUI` correctly (commit 7 bridge gap fix).
 *
 * Public API preserved from pre-Rust shape so callers in [SmartbarManager] /
 * [CandidateClickHandler] do not change. Mirrors iOS
 * `NextWord/NextWordController.swift`.
 */
class NextWordController(
    private val scope: CoroutineScope,
    private val settingsProvider: EngineSettingsProvider,
    // Predictions wait for the bundled lexicon (`CompositionRoot.awaitLexiconReady`):
    // `association.bin` answers the same query. `false` = install failed; predict anyway.
    private val awaitLexiconReady: suspend () -> Boolean,
    private val logger: LoggerBackend,
    private val onUpdateCandidates: (List<TaigiWord>) -> Unit,
    private val onClearCandidates: () -> Unit,
    // Hanji with Romanization split (§42): live-read per render so a picker change applies to
    // the next prediction list. Same provider shape as CandidateUpdateCoordinator.
    private val splitCombinedCellsProvider: () -> Boolean,
) {
    // / Cached state echoed from every decide call. Synchronous read for
    // / SmartbarManager / CandidateClickHandler.
    private var cachedLastSelectedWord: String? = null
    private var cachedPredictionsVisible: Boolean = false

    // / Active context-timeout job. `null` when no timeout scheduled.
    private var contextTimeoutJob: Job? = null

    // / Per-IME-session envelope generation. Engine `EngineHandle` resets state
    // / on mismatch BEFORE applying the request. Bumped on real input-context
    // / changes via [resetContext] (called from `onStartInputView`).
    private var envelopeGen: Long = 1L

    fun getLastSelectedWord(): String? = cachedLastSelectedWord

    fun isShowingNextWordCandidates(): Boolean = cachedPredictionsVisible

    /**
     * Zero association state + cancel any pending timeout. Called on
     * `onStartInputView` when switching input fields.
     *
     * Bumps `envelopeGen` so the engine handle's mismatch reset wipes Rust
     * state to default BEFORE the subsequent [RustEngineBridge.nextwordResetAll]
     * call processes — which then bumps `current_generation`, emits
     * `CancelContextTimeout` (already cancelled above; idempotent), and
     * skips `ClearPredictionsUI` because `predictions_visible` is now false post-reset.
     * Net effect matches the pre-Rust `resetContext()` behavior (zero
     * association state, cancel timer, no UI emit).
     */
    fun resetContext() {
        cancelContextTimeoutJob()
        envelopeGen += 1L
        val settings = settingsProvider.current
        val result = RustEngineBridge.nextwordResetAll(
            nowMs = System.currentTimeMillis(),
            inputMode = settings.inputMode.toEngineInputMode(),
            hanjiFirst = settings.isHanjiFirst,
            generation = envelopeGen,
        )
        applyDecideResult(result)
    }

    /**
     * Mark predictions hidden + bump generation to drop any in-flight
     * prediction result that might land after this point. Called from
     * [SmartbarManager.clearCandidates] AFTER the UI is already cleared, so
     * the `ClearPredictionsUI` effect the engine emits while it was still
     * showing is applied to the mirror only — re-running [onClearCandidates]
     * would recurse into the caller.
     */
    fun clearNextWordState() {
        val settings = settingsProvider.current
        applyDecideResult(
            RustEngineBridge.nextwordClearForNewComposing(
                System.currentTimeMillis(),
                settings.inputMode.toEngineInputMode(),
                settings.isHanjiFirst,
                envelopeGen,
            ),
            isUiAlreadyCleared = true,
        )
    }

    /**
     * Direct `predictions_visible` setter — invoked by [SmartbarManager.updateCandidates]
     * after the candidate bar has been rendered with NextWord suggestions.
     * Routes through [RustEngineBridge.nextwordSetPredictionsVisible]; no generation
     * bump per the bridge contract (the prediction round that produced
     * these suggestions is still current).
     */
    fun setShowingNextWord(showing: Boolean) {
        val settings = settingsProvider.current
        applyDecideResult(
            RustEngineBridge.nextwordSetPredictionsVisible(
                showing,
                inputMode = settings.inputMode.toEngineInputMode(),
                hanjiFirst = settings.isHanjiFirst,
                generation = envelopeGen,
            ),
        )
    }

    /**
     * Handle a word selection from the candidate strip / overlay under
     * `Phase::Composing`. Every platform-side tap predicts, so
     * `triggerPrediction` is fixed at `true`.
     *
     * The engine alone decides what the selection teaches and whether it
     * ends the sentence context (`decide.rs`, behavioral-invariants §40) —
     * a word ending in sentence punctuation (`多謝！`) is learned like any
     * other, exactly as on iOS.
     */
    fun handleNextWordPrediction(
        displayText: String,
        roman: String,
    ) = handleEngineWordSelected(displayText, roman, triggerPrediction = true, preceding = emptyList())

    /**
     * Forward a `NextWordWordSelected` — the engine's `Phase::Continuous`
     * final-commit Effect with its `triggerPrediction` flag and `preceding`
     * segments preserved verbatim (the engine learns the whole composition,
     * §40); the effect path must not silently ignore a future `false` from
     * the engine.
     */
    fun handleEngineWordSelected(
        text: String,
        roman: String,
        triggerPrediction: Boolean,
        preceding: List<com.siansiansu.taigikeyboard.engine.proto.CommittedWord>,
    ) {
        val settings = settingsProvider.current
        applyDecideResult(
            RustEngineBridge.nextwordWordSelected(
                text = text,
                roman = roman,
                requireRomanMode = false,
                triggerPrediction = triggerPrediction,
                nowMs = System.currentTimeMillis(),
                inputMode = settings.inputMode.toEngineInputMode(),
                hanjiFirst = settings.isHanjiFirst,
                generation = envelopeGen,
                preceding = preceding,
            ),
        )
    }

    /**
     * A character the user typed straight into the document, outside any
     * composition (punctuation, a symbol). Forwarded as a commit with no
     * reading and no prediction so the engine can end the context on
     * sentence-end punctuation — what stops the last word of one sentence
     * being learned as the predecessor of the first word of the next
     * (`decide.rs` sentence-end rule). Whether the character does that, or is
     * noise that changes nothing, is the engine's call. Mirrors desktop-core
     * `ComposingManager::note_character_typed_outside_composition`.
     */
    fun noteCharacterTypedOutsideComposition(char: String) {
        val settings = settingsProvider.current
        applyDecideResult(
            RustEngineBridge.nextwordWordSelected(
                text = char,
                roman = "",
                requireRomanMode = false,
                triggerPrediction = false,
                nowMs = System.currentTimeMillis(),
                inputMode = settings.inputMode.toEngineInputMode(),
                hanjiFirst = settings.isHanjiFirst,
                generation = envelopeGen,
            ),
        )
    }

    /**
     * Continuous-input nail / unnail handshake (composing engine
     * `NextWordUpdateLastSelectedWord` effect). The engine learns nothing
     * from it and keeps the committed context — a nailed segment is not in
     * the document yet; the final commit's `preceding` carries it
     * (behavioral-invariants §40).
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
                inputMode = settings.inputMode.toEngineInputMode(),
                hanjiFirst = settings.isHanjiFirst,
                generation = envelopeGen,
            ),
        )
    }

    /**
     * Backspace handler: re-predict from the trailing character of the
     * remaining text, or unconditionally clear UI when text is empty.
     *
     * Empty path bumps `envelopeGen` (forces engine state drop) + dispatches
     * `ResetAll` to cancel timer + bump `current_generation`, then forces
     * `onClearCandidates()` regardless of the engine's `clearPredictionsUI`
     * gate. Preserves the pre-Rust unconditional-clear behavior (the engine
     * gate is closed because envelope-mismatch reset wiped `predictions_visible` to
     * false before `ResetAll` processed).
     */
    fun handleBackspaceForNextWord(textBeforeCursor: String) {
        val trimmed = textBeforeCursor.trimEnd()
        val settings = settingsProvider.current
        val nowMs = System.currentTimeMillis()
        if (trimmed.isEmpty()) {
            cancelContextTimeoutJob()
            envelopeGen += 1L
            applyDecideResult(
                RustEngineBridge.nextwordResetAll(
                    nowMs = nowMs,
                    inputMode = settings.inputMode.toEngineInputMode(),
                    hanjiFirst = settings.isHanjiFirst,
                    generation = envelopeGen,
                ),
            )
            onClearCandidates()
            cachedPredictionsVisible = false
            return
        }
        applyDecideResult(
            RustEngineBridge.nextwordBackspace(
                lastChar = lastGrapheme(trimmed),
                nowMs = nowMs,
                inputMode = settings.inputMode.toEngineInputMode(),
                hanjiFirst = settings.isHanjiFirst,
                generation = envelopeGen,
            ),
        )
    }

    // region Effect interpretation

    // / Mirror engine state echo, then run effects in the order the engine
    // / emitted. Runs on the IME main thread (caller invariant).
    private fun applyDecideResult(
        result: RustEngineBridge.NextWordDecideResult,
        isUiAlreadyCleared: Boolean = false,
    ) {
        cachedLastSelectedWord = result.lastSelectedWord
        cachedPredictionsVisible = result.predictionsVisible
        for (effect in result.effects) {
            execute(effect, isUiAlreadyCleared)
        }
    }

    private fun execute(
        effect: RustEngineBridge.NextWordDecideResult.Effect,
        isUiAlreadyCleared: Boolean,
    ) {
        when (effect) {
            is RustEngineBridge.NextWordDecideResult.Effect.RescheduleContextTimeout -> {
                scheduleContextTimeout(afterMs = effect.afterMs)
            }

            RustEngineBridge.NextWordDecideResult.Effect.CancelContextTimeout -> {
                cancelContextTimeoutJob()
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
                if (!isUiAlreadyCleared) onClearCandidates()
                cachedPredictionsVisible = false
            }
        }
    }

    /**
     * Dispatch the async prediction query. [nowMs] from the effect is reused
     * verbatim on the engine call so the association-window check (Rust
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
        // Dictionary toggles snapshot at query start, before the lexicon-ready
        // suspension — the bundled lookup answers for the settings the query began under.
        val toggles = RustEngineBridge.DictionaryToggles.from(settingsProvider.current)
        scope.launch {
            awaitLexiconReady()
            // Snapshotted on Main, where every other intent reads them.
            val settings = settingsProvider.current
            val generation = envelopeGen
            // Off Main: the engine reads the learned rows from its own
            // `user_association.db` inside this call (roadmap P8b).
            val filterResult =
                withContext(Dispatchers.IO) {
                    RustEngineBridge.nextwordPredictNext(
                        word = word,
                        roman = roman,
                        toggles = toggles,
                        queryGeneration = queryGeneration,
                        nowMs = nowMs,
                        limit = 30,
                        inputMode = settings.inputMode.toEngineInputMode(),
                        hanjiFirst = settings.isHanjiFirst,
                        generation = generation,
                        candidateDisplayMode = settings.candidateDisplayMode,
                        syllableSeparator = settings.syllableSeparator,
                    )
                }
            // A context change meanwhile makes this answer another context's.
            if (generation != envelopeGen) return@launch
            handleQueryResult(filterResult, queryGeneration, settings)
        }
    }

    /**
     * Render an async prediction query's answer — [RustEngineBridge.nextwordPredictNext]
     * read the learned rows and added the bundled rows, then score+merge+sort+limit +
     * stale generation drop — then push the new visibility back to engine state via
     * `nextwordSetPredictionsVisible` so downstream clear/reset paths can emit
     * `ClearPredictionsUI` correctly. On Main.
     */
    private fun handleQueryResult(
        filterResult: RustEngineBridge.NextWordFilterResult,
        queryGeneration: Long,
        settings: EngineSettings,
    ) {
        if (filterResult.wasStale) {
            logger.debug(TAG) { "[NEXTWORD] Drop stale result gen=$queryGeneration" }
            return
        }

        val nowShowing = filterResult.predictions.isNotEmpty()
        if (nowShowing) {
            onUpdateCandidates(
                buildPredictionWords(filterResult.predictions, splitCombinedCellsProvider()),
            )
            scheduleContextTimeout(CONTEXT_TIMEOUT_MS)
        } else {
            onClearCandidates()
        }

        applyDecideResult(
            RustEngineBridge.nextwordSetPredictionsVisible(
                nowShowing,
                inputMode = settings.inputMode.toEngineInputMode(),
                hanjiFirst = settings.isHanjiFirst,
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
                inputMode = settings.inputMode.toEngineInputMode(),
                hanjiFirst = settings.isHanjiFirst,
                generation = envelopeGen,
            ),
        )
    }

    // endregion

    companion object {
        private const val TAG = "NextWordController"

        /**
         * Mirrors `engine/nextword/src/decide.rs` `CONTEXT_TIMEOUT_MS = 30_000`.
         * CROSS-PLATFORM INVARIANT: changing this value requires a paired
         * update in the Rust crate + an `INVARIANT_*` parity-test mirror.
         */
        const val CONTEXT_TIMEOUT_MS: Long = 30_000L

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
 * The engine `input_mode` for a stored `inputMode` string. TPS goes out as `"tps"`, which the
 * engine renders predictions for as TL (`nextword/src/filter.rs`); an unknown value reads as POJ,
 * as [InputMode.fromPrefString] does.
 */
private fun String.toEngineInputMode(): String = engineInputMode(this, unknownAs = "poj")

/**
 * NextWord prediction cells. Ids run `-1..-n` — the NextWord sentinel range
 * (`-99..-1`; English is `<= -100`) read by [CandidateClickHandler]; 30
 * predictions split to at most 60 cells, still inside it.
 *
 * Hanji with Romanization ([splitCombinedCells], §42): [splitIntoSingleScriptCells] — a
 * prediction with romanization becomes a Hanji cell then a romanization cell sharing
 * the prediction's identity; a hanji-only prediction lists its Hanji cell
 * alone. Every other mode emits one dual-script word per prediction
 * (`roman = ""` when the engine shaped no subtitle, so the strip renders the
 * hanji alone).
 */
internal fun buildPredictionWords(
    predictions: List<RustEngineBridge.NextWordEnginePrediction>,
    splitCombinedCells: Boolean,
): List<TaigiWord> {
    if (!splitCombinedCells) {
        return predictions.mapIndexed { index, p ->
            predictionWord(id = -index - 1, prediction = p, cellScript = null)
        }
    }

    return splitIntoSingleScriptCells(
        items = predictions,
        hanjiOf = { it.hanji },
        romanOf = { if (it.subtitle != null) it.text else null },
    ) { prediction, cellScript, ordinal ->
        predictionWord(id = -ordinal - 1, prediction = prediction, cellScript = cellScript)
    }
}

private fun predictionWord(
    id: Int,
    prediction: RustEngineBridge.NextWordEnginePrediction,
    cellScript: String?,
): TaigiWord {
    // R5 pair-key (#7): canonical-TL reading for the user-frequency
    // `(displayText, canonicalTl)` write. `tl` is the engine-side canonical
    // TL (only text/subtitle are mode-shaped), matching the Continuous read
    // key. Without it the freq write would land in the legacy tl=="" bucket
    // and 重/tāng could inherit a count learned from 重/tîng.
    val identity = TaigiWord.MetadataKeys.CANONICAL_TL to prediction.tl
    return TaigiWord(
        id = id,
        roman = if (prediction.subtitle != null) prediction.text else "",
        hanji = prediction.hanji,
        additionalInfo =
            if (cellScript == null) mapOf(identity) else mapOf(identity, TaigiWord.MetadataKeys.CELL_SCRIPT to cellScript),
    )
}
