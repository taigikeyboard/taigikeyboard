// Candidate update coordinator, extracted from TextInputManager. Schedules Taigi candidate
// fetches off Main (English still debounced), owns display-derivation scheduling and lifecycle
// cancellation; caches TaigiAutocompleteService across keystrokes.

package com.siansiansu.taigikeyboard.ime.text

import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.TraceContext
import com.siansiansu.taigikeyboard.ime.core.logging.TraceId
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager
import com.siansiansu.taigikeyboard.ime.text.composing.EnglishAutocompleteService
import com.siansiansu.taigikeyboard.ime.text.composing.TaigiAutocompleteService
import com.siansiansu.taigikeyboard.ime.text.composing.shouldSplitCombinedCells
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarManager
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Coordinates candidate update jobs extracted from TextInputManager.
 *
 * Schedules Taigi candidate fetches (immediate, off Main) and debounced
 * English updates, plus display-derivation scheduling, with cancellation on
 * lifecycle events.
 */
class CandidateUpdateCoordinator(
    private val scope: CoroutineScope,
    private val taigikeyboard: TaigiKeyboard,
    private val getComposingManager: () -> ComposingManager?,
    private val smartbarManager: SmartbarManager,
) {
    private val logger get() = taigikeyboard.compositionRoot.logger

    private var candidateUpdateJob: Job? = null
    private var englishCandidateUpdateJob: Job? = null

    // Cached TaigiAutocompleteService — created once and reused across
    // keystrokes. Mode-agnostic after v3.5.8 Item 13 (the engine owns
    // input-mode handling), so no recreate-on-mode-change is needed.
    // Protected by serviceLock for thread-safe access.
    private val serviceLock = Any()
    private var taigiAutocompleteService: com.siansiansu.taigikeyboard.ime.text.composing.TaigiAutocompleteService? = null

    // English autocomplete service
    private var englishAutocompleteService: EnglishAutocompleteService? = null

    /**
     * Schedules a Taigi candidate update for the current composing state.
     * Runs on `Dispatchers.Default`: the engine's dictionary scan costs tens
     * of milliseconds for a one-letter prefix and must not block touch
     * handling. Only the strip publish hops back to Main (see
     * [publishIfCurrent]).
     *
     * No fixed delay: the 50 ms debounce that used to sit here (v3.4.5, when
     * autocomplete ran in Kotlin on Main) added a constant to every
     * keystroke's candidate latency once the fetch moved off Main; iOS
     * fetches per keystroke with no delay. A newer keystroke cancels the
     * older job; a JNI call already in flight cannot be interrupted, so the
     * job either stops at its next suspension point or reaches
     * [publishIfCurrent], which drops the stale result.
     *
     * The composing state is captured HERE, on Main, at schedule time — not
     * on the worker. A candidate tap that final-commits before the job runs
     * is not a cancellation, and a job that captured its state only on
     * waking would see `rawInput == null`, pass every guard, and clear the
     * NextWord predictions the commit just produced.
     */
    fun updateTaigiCandidates() {
        candidateUpdateJob?.cancel()
        val manager = getComposingManager() ?: return
        val fetchContext = FetchContext(manager, manager.stateToken())
        val trace = TraceContext.current
        val candidateStart = System.currentTimeMillis()

        candidateUpdateJob =
            scope.launch(Dispatchers.Default) {
                if (!isActive || !fetchContext.isCurrent(getComposingManager())) return@launch

                val traceId = trace ?: TraceId.untraced
                logger.debug(TAG) {
                    "[trace=$traceId] [CANDIDATE] fn=updateTaigiCandidates worker-start (untraced from here)"
                }

                fetchTaigiCandidates(fetchContext)

                logger.debug("PERF") {
                    "[TOTAL] updateTaigiCandidates: ${System.currentTimeMillis() - candidateStart}ms"
                }
            }
    }

    /**
     * English candidate update. Reads the editor text here, on Main, at
     * schedule time — the same shape as [updateTaigiCandidates]'s
     * [FetchContext] capture — then runs the matcher on a worker and
     * publishes on Main. No debounce: every keystroke refreshes the strip
     * (the Taigi path dropped its debounce in #50 for the same reason).
     * The publish is guarded by [SmartbarManager.clearEpoch] captured here,
     * so a strip cleared by any path while the matcher ran stays cleared.
     */
    fun updateEnglishCandidates() {
        englishCandidateUpdateJob?.cancel()
        val textBeforeCursor =
            taigikeyboard.currentInputConnection?.getTextBeforeCursor(100, 0)?.toString() ?: ""
        if (textBeforeCursor.isEmpty()) {
            smartbarManager.clearCandidates()
            return
        }
        val clearEpoch = smartbarManager.clearEpoch
        logger.debug(EN_TAG) { "[1] updateEnglishCandidates textBeforeCursor='$textBeforeCursor'" }
        englishCandidateUpdateJob =
            scope.launch(Dispatchers.Default) {
                val words = englishSuggestions(textBeforeCursor)
                withContext(Dispatchers.Main.immediate) {
                    if (smartbarManager.clearEpoch != clearEpoch) return@withContext
                    if (words.isNotEmpty()) {
                        smartbarManager.updateEnglishCandidates(words)
                    } else {
                        smartbarManager.clearCandidates()
                    }
                }
            }
    }

    private suspend fun fetchTaigiCandidates(fetchContext: FetchContext) {
        val rawInput = fetchContext.rawInput
        if (rawInput.isNullOrEmpty()) {
            logger.debug(TAG) { "[CANDIDATES] rawInput=${if (rawInput == null) "null" else "empty"}, clearCandidates" }
            publishIfCurrent(fetchContext) { smartbarManager.clearCandidates() }
            return
        }
        val displayText = fetchContext.manager.getComposingText() ?: rawInput

        logger.debug(TAG) { "[CANDIDATES] rawInput='$rawInput', displayText='$displayText'" }

        val service =
            synchronized(serviceLock) {
                taigiAutocompleteService ?: TaigiAutocompleteService(
                    logger = taigikeyboard.compositionRoot.logger,
                    // Runs on the worker: `fetchContinuousCandidates` is
                    // read-only (no InputConnection, no state mirror), so
                    // no Main hop is needed. The manager is re-resolved per
                    // fetch so a detach between scheduling and fetch
                    // collapses to empty.
                    continuousFetcher = {
                        getComposingManager()?.fetchContinuousCandidates() ?: emptyList()
                    },
                    // 漢羅濫 split cells (§42 second exception). Live-read per
                    // fetch — the cached service must see a settings change on
                    // the next keystroke. TPS ignores the picker, so the split
                    // never fires under the TPS layout.
                    splitCombinedCellsProvider = {
                        val prefs = taigikeyboard.prefs
                        shouldSplitCombinedCells(prefs.candidateDisplayMode, prefs.isTpsLayout)
                    },
                ).also { taigiAutocompleteService = it }
            }

        val searchStart = System.currentTimeMillis()
        val suggestions = service.autocomplete(
            rawInput = rawInput,
            displayText = displayText,
        )
        logger.debug("PERF") {
            "[3] autocomplete (${suggestions.size} results): ${System.currentTimeMillis() - searchStart}ms"
        }

        logger.debug(TAG) { "[CANDIDATES] found ${suggestions.size} suggestions" }

        val uiStart = System.currentTimeMillis()
        publishIfCurrent(fetchContext) {
            smartbarManager.updateCandidates(suggestions)
            logger.debug("PERF") { "[4] updateCandidates UI: ${System.currentTimeMillis() - uiStart}ms" }
        }
    }

    /**
     * Hop to Main and run [publish] against the candidate strip unless the
     * job was cancelled or [fetchContext] no longer matches the live
     * composing state (backspace cleared the buffer, a new keystroke landed,
     * a candidate tap committed, or the input context changed).
     */
    private suspend fun publishIfCurrent(
        fetchContext: FetchContext,
        publish: () -> Unit,
    ) {
        withContext(Dispatchers.Main.immediate) {
            if (!isActive) return@withContext
            val current = getComposingManager()
            if (!fetchContext.isCurrent(current)) {
                logger.debug(TAG) {
                    "[CANDIDATES] stale result dropped (was='${fetchContext.rawInput}', now='${current?.getRawInput()}')"
                }
                return@withContext
            }
            publish()
        }
    }

    /** Spell-check suggestions for the current word of [textBeforeCursor], as strip rows. */
    private suspend fun englishSuggestions(textBeforeCursor: String): List<TaigiWord> {
        // `destroy()` cancels this job before it nulls the field under the
        // same lock; a worker that reaches the lock afterwards must not
        // re-create a service nobody closes.
        val context = currentCoroutineContext()
        val service =
            synchronized(serviceLock) {
                context.ensureActive()
                englishAutocompleteService ?: EnglishAutocompleteService(taigikeyboard.context)
                    .also { englishAutocompleteService = it }
            }
        return try {
            val suggestions = service.getSuggestions(textBeforeCursor)
            logger.debug(EN_TAG) { "[2] getSuggestions → ${suggestions.map { it.text }}" }
            suggestions.mapIndexed { index, suggestion ->
                TaigiWord(
                    id = -100 - index,
                    roman = suggestion.text,
                    hanzi = null,
                    lengthScore = null,
                )
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            logger.e(EN_TAG, "[ERROR] Failed to get suggestions", e)
            emptyList()
        }
    }

    /**
     * Cancel all pending jobs (called on onFinishInputView).
     */
    fun cancelAll() {
        candidateUpdateJob?.cancel()
        candidateUpdateJob = null
        englishCandidateUpdateJob?.cancel()
        englishCandidateUpdateJob = null
    }

    /**
     * Destroy and clean up resources (called on onDestroy).
     */
    fun destroy() {
        cancelAll()
        synchronized(serviceLock) {
            englishAutocompleteService?.close()
            englishAutocompleteService = null
            taigiAutocompleteService = null
        }
    }

    companion object {
        private const val TAG = "CandidateCoordinator"
        private const val EN_TAG = "ENSPELL"
    }
}

/**
 * The composing state a worker-side candidate fetch was computed against. A
 * result is published only if the same manager still shows the same raw
 * buffer under the same input-context generation — otherwise the strip would
 * render candidates for text the user has already moved past.
 */
internal class FetchContext(
    val manager: ComposingManager,
    private val token: ComposingManager.StateToken,
) {
    val rawInput: String? get() = token.rawInput

    fun isCurrent(current: ComposingManager?): Boolean = current === manager && current.stateToken() == token
}
