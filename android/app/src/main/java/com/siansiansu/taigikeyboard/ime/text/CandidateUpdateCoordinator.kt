package com.siansiansu.taigikeyboard.ime.text

import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels
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
    private val smartbarManager: SmartbarManager
) {
    private var candidateUpdateJob: Job? = null
    private var englishCandidateUpdateJob: Job? = null
    private var displayDerivationJob: Job? = null

    // Cached TaigiAutocompleteService — reused across keystrokes, recreated only when inputMode changes
    // Protected by serviceLock for thread-safe access
    private val serviceLock = Any()
    private var taigiAutocompleteService: com.siansiansu.taigikeyboard.ime.text.composing.TaigiAutocompleteService? = null
    private var cachedInputMode: ToneConverterModels.InputMode? = null

    // English autocomplete service
    private var englishAutocompleteService: com.siansiansu.taigikeyboard.ime.text.composing.EnglishAutocompleteService? = null

    /**
     * Debounced Taigi candidate update.
     */
    fun updateTaigiCandidatesDebounced() {
        candidateUpdateJob?.cancel()

        candidateUpdateJob = scope.launch {
            delay(CANDIDATE_DEBOUNCE_MS)
            if (!isActive) return@launch

            val candidateStart = System.currentTimeMillis()
            updateTaigiCandidates()

            if (BuildConfig.DEBUG) {
                Log.d("PERF", "[TOTAL] updateTaigiCandidates: ${System.currentTimeMillis() - candidateStart}ms")
            }
        }
    }

    /**
     * Schedule display derivation (segmentation + tone conversion) off the main thread.
     */
    fun scheduleDisplayDerivation() {
        displayDerivationJob?.cancel()
        val manager = getComposingManager() ?: return
        val raw = manager.getRawInput() ?: return

        displayDerivationJob = scope.launch {
            val derived = withContext(Dispatchers.Default) {
                manager.deriveDisplay(raw)
            }
            val ic = taigikeyboard.currentInputConnection ?: return@launch
            if (manager.isComposing() && manager.getRawInput() == raw) {
                manager.applyDerivedDisplay(derived, ic)
            }
        }
    }

    /**
     * Debounced English candidate update.
     */
    fun updateEnglishCandidatesDebounced() {
        if (BuildConfig.DEBUG) {
            Log.d("ENSPELL", "[1] updateEnglishCandidatesDebounced() called")
        }

        englishCandidateUpdateJob?.cancel()

        englishCandidateUpdateJob = scope.launch {
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

        val manager = getComposingManager() ?: run {
            if (BuildConfig.DEBUG) Log.d(TAG, "[CANDIDATES] composingManager=null, skip")
            return
        }

        val rawInput = manager.getRawInput() ?: run {
            if (BuildConfig.DEBUG) Log.d(TAG, "[CANDIDATES] rawInput=null, clearCandidates")
            smartbarManager.clearCandidates()
            return
        }
        val displayText = manager.getComposingText() ?: rawInput

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[CANDIDATES] rawInput='$rawInput', displayText='$displayText'")
        }

        // Reuse TaigiAutocompleteService — only recreate when inputMode changes
        val inputMode = taigikeyboard.prefs.inputMode.let {
            when (it) {
                "poj" -> ToneConverterModels.InputMode.POJ
                "tl", "tps" -> ToneConverterModels.InputMode.TL
                else -> ToneConverterModels.InputMode.POJ
            }
        }

        val service = synchronized(serviceLock) {
            if (taigiAutocompleteService == null || cachedInputMode != inputMode) {
                cachedInputMode = inputMode
                taigiAutocompleteService = com.siansiansu.taigikeyboard.ime.text.composing.TaigiAutocompleteService(
                    taigikeyboard.context,
                    inputMode,
                    prefs = taigikeyboard.prefs
                )
            }
            taigiAutocompleteService!!
        }

        val searchStart = System.currentTimeMillis()
        val suggestions = service.autocomplete(rawInput, displayText, smartbarManager.getLastSelectedWord())
        if (BuildConfig.DEBUG) Log.d("PERF", "[3] autocomplete (${suggestions.size} results): ${System.currentTimeMillis() - searchStart}ms")

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[CANDIDATES] found ${suggestions.size} suggestions")
        }

        val uiStart = System.currentTimeMillis()
        withContext(Dispatchers.Main) {
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

        val ic = taigikeyboard.currentInputConnection ?: run {
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
            englishAutocompleteService = com.siansiansu.taigikeyboard.ime.text.composing.EnglishAutocompleteService(taigikeyboard.context)
        }

        val service = englishAutocompleteService ?: run {
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

            val words = suggestions.mapIndexed { index, suggestion ->
                com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord(
                    id = -100 - index,
                    roman = suggestion.text,
                    hanzi = null,
                    lengthScore = null
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
            cachedInputMode = null
        }
    }

    companion object {
        private const val TAG = "CandidateCoordinator"
        private const val CANDIDATE_DEBOUNCE_MS = 50L
    }
}
