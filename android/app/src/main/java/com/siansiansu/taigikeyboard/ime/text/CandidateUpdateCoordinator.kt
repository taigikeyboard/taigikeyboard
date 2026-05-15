// 中文: 候選詞更新作業協調器 — 從 TextInputManager 抽出。
// 中文: 管理 Taigi / English 候選 debounce 流程、display 衍生排程、生命週期取消;
// 中文: 並對 TaigiAutocompleteService 做 inputMode-aware 快取以省重建成本。

package com.siansiansu.taigikeyboard.ime.text

import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.TraceContext
import com.siansiansu.taigikeyboard.ime.core.logging.TraceId
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Coordinates candidate update jobs extracted from TextInputManager.
 *
 * Manages debounced Taigi/English candidate updates and display derivation
 * scheduling with proper cancellation on lifecycle events.
 */
class CandidateUpdateCoordinator(
    private val scope: CoroutineScope,
    private val taigikeyboard: TaigiKeyboard,
    private val getComposingManager: () -> ComposingManager?,
    private val smartbarManager: SmartbarManager,
) {
    private var candidateUpdateJob: Job? = null
    private var englishCandidateUpdateJob: Job? = null
    private var displayDerivationJob: Job? = null

    // Cached TaigiAutocompleteService — created once and reused across
    // keystrokes. Mode-agnostic after v3.5.8 Item 13 (the engine owns
    // input-mode handling), so no recreate-on-mode-change is needed.
    // Protected by serviceLock for thread-safe access.
    private val serviceLock = Any()
    private var taigiAutocompleteService: com.siansiansu.taigikeyboard.ime.text.composing.TaigiAutocompleteService? = null

    // English autocomplete service
    private var englishAutocompleteService: com.siansiansu.taigikeyboard.ime.text.composing.EnglishAutocompleteService? = null

    /**
     * Debounced Taigi candidate update.
     */
    fun updateTaigiCandidatesDebounced() {
        candidateUpdateJob?.cancel()
        val trace = TraceContext.current

        candidateUpdateJob =
            scope.launch {
                delay(CANDIDATE_DEBOUNCE_MS)
                if (!isActive) return@launch

                val traceId = trace ?: TraceId.untraced
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[trace=$traceId] [CANDIDATE] fn=updateTaigiCandidatesDebounced debounce-fire (untraced from here)")
                }

                val candidateStart = System.currentTimeMillis()
                updateTaigiCandidates()

                if (BuildConfig.DEBUG) {
                    Log.d("PERF", "[TOTAL] updateTaigiCandidates: ${System.currentTimeMillis() - candidateStart}ms")
                }
            }
    }

    /**
     * Obsolete after v3.5.4 (Composing → Rust shared core). The Rust
     * dispatch returns the display form synchronously inside the
     * bridge-emitted `UpdatePreedit` effect, which the
     * [DefaultComposingDelegate] applies via `ic.setComposingText` during
     * the dispatch itself. The async refresh path existed only to absorb
     * Kotlin tone-converter latency; Rust dispatch is microsecond-scale
     * so no async refresh is needed.
     *
     * Kept as a no-op shim so existing call sites (TextInputManager) do
     * not need to be edited in this commit.
     */
    fun scheduleDisplayDerivation() {
        displayDerivationJob?.cancel()
    }

    /**
     * Debounced English candidate update.
     */
    fun updateEnglishCandidatesDebounced() {
        if (BuildConfig.DEBUG) {
            Log.d("ENSPELL", "[1] updateEnglishCandidatesDebounced() called")
        }

        englishCandidateUpdateJob?.cancel()

        englishCandidateUpdateJob =
            scope.launch {
                delay(CANDIDATE_DEBOUNCE_MS)
                if (!isActive) return@launch

                if (BuildConfig.DEBUG) {
                    Log.d("ENSPELL", "[2] After debounce, calling updateEnglishCandidates()")
                }
                updateEnglishCandidates()
            }
    }

    /**
     * Update Taigi candidates using autocomplete service.
     */
    private suspend fun updateTaigiCandidates() {
        if (BuildConfig.DEBUG) {
            val stackTrace = Thread.currentThread().stackTrace
            val caller = stackTrace.getOrNull(3)?.methodName ?: "unknown"
            Log.d(TAG, "[DEBUG] updateTaigiCandidates() called from: $caller")
        }

        val manager =
            getComposingManager() ?: run {
                if (BuildConfig.DEBUG) Log.d(TAG, "[CANDIDATES] composingManager=null, skip")
                return
            }

        val rawInput =
            manager.getRawInput() ?: run {
                if (BuildConfig.DEBUG) Log.d(TAG, "[CANDIDATES] rawInput=null, clearCandidates")
                smartbarManager.clearCandidates()
                return
            }
        if (rawInput.isEmpty()) {
            if (BuildConfig.DEBUG) Log.d(TAG, "[CANDIDATES] rawInput=empty, clearCandidates")
            smartbarManager.clearCandidates()
            return
        }
        val displayText = manager.getComposingText() ?: rawInput

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[CANDIDATES] rawInput='$rawInput', displayText='$displayText'")
        }

        val service =
            synchronized(serviceLock) {
                taigiAutocompleteService ?: com.siansiansu.taigikeyboard.ime.text.composing.TaigiAutocompleteService(
                    logger = taigikeyboard.compositionRoot.logger,
                    // Continuous-input fetcher hops back to the IME main
                    // thread before invoking `fetchContinuousCandidates`;
                    // the underlying `applyTransition` writes to
                    // InputConnection so the off-main coroutine context the
                    // autocomplete job runs in is unsafe. BOTH the manager
                    // and IC are re-resolved INSIDE the Main block so a
                    // detach/editor swap between the dispatch hop and the
                    // fetch returns null and collapses to empty.
                    continuousFetcher = {
                        withContext(Dispatchers.Main) {
                            val mgr = getComposingManager()
                            val ic = taigikeyboard.currentInputConnection
                            if (mgr != null && ic != null) {
                                mgr.fetchContinuousCandidates(ic)
                            } else {
                                emptyList()
                            }
                        }
                    },
                ).also { taigiAutocompleteService = it }
            }

        val searchStart = System.currentTimeMillis()
        val suggestions = service.autocomplete(
            rawInput = rawInput,
            displayText = displayText,
        )
        if (BuildConfig.DEBUG) {
            Log.d(
                "PERF",
                "[3] autocomplete (${suggestions.size} results): ${System.currentTimeMillis() - searchStart}ms",
            )
        }

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[CANDIDATES] found ${suggestions.size} suggestions")
        }

        val uiStart = System.currentTimeMillis()
        withContext(Dispatchers.Main) {
            // Stale-result guard: composing state may have changed during the
            // search await (backspace cleared the buffer, or a new keystroke
            // arrived). Drop this result rather than overwrite the now-current
            // candidate list with stale data.
            if (!isActive) return@withContext
            val currentRaw = getComposingManager()?.getRawInput()
            if (currentRaw != rawInput) {
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[CANDIDATES] stale result dropped (was='$rawInput', now='$currentRaw')")
                }
                return@withContext
            }
            smartbarManager.updateCandidates(suggestions)
            if (BuildConfig.DEBUG) Log.d("PERF", "[4] updateCandidates UI: ${System.currentTimeMillis() - uiStart}ms")
        }
    }

    /**
     * Update English candidates using spell-check service.
     */
    private suspend fun updateEnglishCandidates() {
        if (BuildConfig.DEBUG) {
            Log.d("ENSPELL", "[3] updateEnglishCandidates() called")
        }

        val ic =
            taigikeyboard.currentInputConnection ?: run {
                if (BuildConfig.DEBUG) Log.d("ENSPELL", "[3] inputConnection is null")
                return
            }

        val textBeforeCursor = ic.getTextBeforeCursor(100, 0)?.toString() ?: ""

        if (textBeforeCursor.isEmpty()) {
            if (BuildConfig.DEBUG) Log.d("ENSPELL", "[3] textBeforeCursor is empty")
            smartbarManager.clearCandidates()
            return
        }

        if (BuildConfig.DEBUG) {
            Log.d("ENSPELL", "[3] textBeforeCursor='$textBeforeCursor'")
        }

        if (englishAutocompleteService == null) {
            if (BuildConfig.DEBUG) Log.d("ENSPELL", "[4] Creating EnglishAutocompleteService...")
            englishAutocompleteService =
                com.siansiansu.taigikeyboard.ime.text.composing
                    .EnglishAutocompleteService(taigikeyboard.context)
        }

        val service =
            englishAutocompleteService ?: run {
                if (BuildConfig.DEBUG) Log.d("ENSPELL", "[4] service is null after creation")
                return
            }

        try {
            if (BuildConfig.DEBUG) Log.d("ENSPELL", "[5] Calling getSuggestions()...")
            val startTime = System.currentTimeMillis()
            val suggestions = service.getSuggestions(textBeforeCursor)
            val elapsed = System.currentTimeMillis() - startTime

            if (BuildConfig.DEBUG) {
                Log.d("ENSPELL", "[6] getSuggestions() returned ${suggestions.size} suggestions in ${elapsed}ms")
                suggestions.forEachIndexed { i, s -> Log.d("ENSPELL", "[6]   [$i] ${s.text}") }
            }

            val words =
                suggestions.mapIndexed { index, suggestion ->
                    com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord(
                        id = -100 - index,
                        roman = suggestion.text,
                        hanzi = null,
                        lengthScore = null,
                    )
                }

            withContext(Dispatchers.Main) {
                if (words.isNotEmpty()) {
                    smartbarManager.updateEnglishCandidates(words)
                } else {
                    smartbarManager.clearCandidates()
                }
            }
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e("ENSPELL", "[ERROR] Failed to get suggestions", e)
            }
            withContext(Dispatchers.Main) {
                smartbarManager.clearCandidates()
            }
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
        displayDerivationJob?.cancel()
        displayDerivationJob = null
    }

    /**
     * Destroy and clean up resources (called on onDestroy).
     */
    fun destroy() {
        cancelAll()
        englishAutocompleteService?.close()
        englishAutocompleteService = null
        synchronized(serviceLock) {
            taigiAutocompleteService = null
        }
    }

    companion object {
        private const val TAG = "CandidateCoordinator"
        private const val CANDIDATE_DEBOUNCE_MS = 50L
    }
}
