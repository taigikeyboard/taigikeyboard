package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.logging.TraceContext
import com.siansiansu.taigikeyboard.ime.core.logging.TraceId
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

/**
 * Handles candidate click events extracted from SmartbarManager.
 *
 * Processes Taigi candidate selection, English suggestion replacement,
 * and overlay suggestion selection with proper text output formatting.
 */
class CandidateClickHandler(
    private val scope: CoroutineScope,
    private val prefs: PrefHelper,
    private val taigikeyboard: TaigiKeyboard,
    private val userFreq: UserFrequencyService,
    private val getCurrentSuggestions: () -> List<TaigiWord>,
    private val getIsTranslateSwapped: () -> Boolean,
    private val getOutputBothScripts: () -> Boolean,
    private val getComposingManager: () -> com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager?,
    private val onClearCandidates: () -> Unit,
    private val onNextWordPrediction: (displayText: String, committedText: String, roman: String, hanzi: String?, rawInput: String) -> Unit,
) {
    /**
     * Handle candidate click from RecyclerView.
     */
    fun handleCandidateClick(
        selectedWord: TaigiWord,
        index: Int,
    ) {
        TraceContext.withTrace(TraceId.next()) {
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[INPUT] fn=handleCandidateClick gesture=candidate-tap")
            }

            if (BuildConfig.DEBUG) {
                val isNextWord = getCurrentSuggestions().firstOrNull()?.id?.let { it < 0 } ?: false
                Log.d(TAG, "[CLICK-ENTRY] onClick triggered, isNextWordMode=$isNextWord, suggestionsCount=${getCurrentSuggestions().size}")
                Log.d(TAG, "[CLICK] index=$index, suggestionsSize=${getCurrentSuggestions().size}")
            }

            val ic = taigikeyboard.currentInputConnection ?: return

            val composingManager = getComposingManager()
            val capturedRawInput = composingManager?.getRawInput() ?: ""

            val isEnglishSuggestion = selectedWord.id <= -100
            val isNextWordPrediction = selectedWord.id < 0 && !isEnglishSuggestion

            val cachedIsTranslateSwapped = getIsTranslateSwapped()
            val cachedOutputBothScripts = getOutputBothScripts()
            val isTPSLayout = prefs.keyboardLayoutType == "tps" || prefs.inputMode == "tps"
            val effectiveSwapped = isTPSLayout || cachedIsTranslateSwapped

            // Convert roman to TPS for bracket annotation when in TPS mode
            val bracketRoman =
                if (isTPSLayout) {
                    RustEngineBridge.tlDisplayToTps(selectedWord.roman, prefs.tpsOrMapsToER)
                } else {
                    selectedWord.roman
                }

            val textToCommit =
                when {
                    isEnglishSuggestion -> {
                        selectedWord.roman
                    }

                    cachedOutputBothScripts && !selectedWord.hanzi.isNullOrEmpty() -> {
                        if (effectiveSwapped) {
                            "${selectedWord.hanzi} ($bracketRoman)"
                        } else {
                            "$bracketRoman (${selectedWord.hanzi})"
                        }
                    }

                    effectiveSwapped && !selectedWord.hanzi.isNullOrEmpty() -> {
                        selectedWord.hanzi
                    }

                    else -> {
                        selectedWord.roman
                    }
                }

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[CLICK] id=${selectedWord.id}, roman='${selectedWord.roman}', hanzi='${selectedWord.hanzi}'")
                Log.d(
                    TAG,
                    "[CLICK] isTranslateSwapped=$cachedIsTranslateSwapped, effectiveSwapped=$effectiveSwapped, outputBothScripts=$cachedOutputBothScripts",
                )
                Log.d(TAG, "[CLICK] textToCommit='$textToCommit', isNextWord=$isNextWordPrediction, isEnglish=$isEnglishSuggestion")
            }

            if (isEnglishSuggestion) {
                val textBeforeCursor = ic.getTextBeforeCursor(100, 0)?.toString() ?: ""
                val currentWord = NextWordHandler.extractCurrentWord(textBeforeCursor)
                if (currentWord.isNotEmpty()) {
                    ic.deleteSurroundingText(currentWord.length, 0)
                }
                ic.commitText(textToCommit, 1)
                onClearCandidates()
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[ENGLISH-CLICK] Replaced '$currentWord' with '$textToCommit'")
                }
            } else if (isNextWordPrediction) {
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[NEXTWORD-CLICK] BEFORE commitText: text='$textToCommit', ic=$ic")
                }
                val result = ic.commitText(textToCommit, 1)
                composingManager?.reset(ic)
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[NEXTWORD-CLICK] AFTER commitText: result=$result")
                }
            } else {
                composingManager?.selectSuggestion(textToCommit, ic)
            }

            // Auto-space (disabled for TPS via effectiveSwapped)
            if (prefs.isAutoSpaceEnabled && (!effectiveSwapped || cachedOutputBothScripts)) {
                if (!textToCommit.endsWith("-")) {
                    ic.commitText(" ", 1)
                }
            }

            // Record usage frequency
            if (prefs.frequencyRecordingEnabled) {
                scope.launch {
                    userFreq.recordUsage(selectedWord.displayText)
                }
            }

            // NextWord prediction
            onNextWordPrediction(
                selectedWord.displayText,
                textToCommit,
                selectedWord.roman,
                selectedWord.hanzi,
                capturedRawInput,
            )
        }
    }

    /**
     * Handle English candidate click (3-column layout).
     */
    fun handleEnglishCandidateClick(index: Int) {
        TraceContext.withTrace(TraceId.next()) {
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[INPUT] fn=handleEnglishCandidateClick gesture=candidate-tap")
            }

            val currentSuggestions = getCurrentSuggestions()
            if (index >= currentSuggestions.size) return

            val selectedWord = currentSuggestions[index]
            val ic = taigikeyboard.currentInputConnection ?: return

            val textBeforeCursor = ic.getTextBeforeCursor(100, 0)?.toString() ?: ""
            val currentWord = NextWordHandler.extractCurrentWord(textBeforeCursor)

            if (currentWord.isNotEmpty()) {
                ic.deleteSurroundingText(currentWord.length, 0)
            }
            ic.commitText(selectedWord.roman, 1)
            onClearCandidates()

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[ENGLISH-CLICK] Replaced '$currentWord' with '${selectedWord.roman}'")
            }
        }
    }

    /**
     * Handle overlay suggestion selection.
     */
    fun handleOverlaySuggestionSelected(
        word: TaigiWord,
        index: Int,
    ) {
        val ic = taigikeyboard.currentInputConnection ?: return
        val composingManager = getComposingManager()

        val isNextWordPred = word.id < 0
        val cachedIsTranslateSwapped = getIsTranslateSwapped()
        val cachedOutputBothScripts = getOutputBothScripts()
        val isTPSLayout = prefs.keyboardLayoutType == "tps" || prefs.inputMode == "tps"
        val effectiveSwapped = isTPSLayout || cachedIsTranslateSwapped

        // Convert roman to TPS for bracket annotation when in TPS mode
        val bracketRoman =
            if (isTPSLayout) {
                RustEngineBridge.tlDisplayToTps(word.roman, prefs.tpsOrMapsToER)
            } else {
                word.roman
            }

        val textToCommit =
            when {
                cachedOutputBothScripts && !word.hanzi.isNullOrEmpty() -> {
                    if (effectiveSwapped) {
                        "${word.hanzi} ($bracketRoman)"
                    } else {
                        "$bracketRoman (${word.hanzi})"
                    }
                }

                effectiveSwapped && !word.hanzi.isNullOrEmpty() -> {
                    word.hanzi
                }

                else -> {
                    word.roman
                }
            }

        if (isNextWordPred) {
            ic.commitText(textToCommit, 1)
            composingManager?.reset(ic)
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[OVERLAY] NextWord commitText: '$textToCommit'")
            }
        } else {
            composingManager?.selectSuggestion(textToCommit, ic)
        }

        // Auto-space (disabled for TPS via effectiveSwapped)
        if (prefs.isAutoSpaceEnabled && (!effectiveSwapped || cachedOutputBothScripts)) {
            if (!textToCommit.endsWith("-")) {
                ic.commitText(" ", 1)
            }
        }

        // Record usage frequency
        if (prefs.frequencyRecordingEnabled) {
            scope.launch {
                userFreq.recordUsage(word.displayText)
            }
        }

        // NextWord prediction
        onNextWordPrediction(
            word.displayText,
            textToCommit,
            word.roman,
            word.hanzi,
            "",
        )

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[OVERLAY] Selected suggestion: ${word.displayText} at index $index")
        }
    }

    companion object {
        private const val TAG = "CandidateClickHandler"
    }
}
