// 中文: 候選點擊處理器 — 從 SmartbarManager 抽出。
// 中文: 處理 Taigi 候選選擇、英文建議替換、overlay 候選選擇,並做 outputText 格式化。

package com.siansiansu.taigikeyboard.ime.text.smartbar

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.TraceContext
import com.siansiansu.taigikeyboard.ime.core.logging.TraceId
import com.siansiansu.taigikeyboard.ime.core.logging.debug
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
    private val onNextWordPrediction: (
        displayText: String,
        committedText: String,
        roman: String,
        hanzi: String?,
        rawInput: String,
    ) -> Unit,
    /**
     * Schedule a Taigi candidate recompute. Called only after a Continuous
     * mid-commit where the engine's `PerformAutocomplete` effect alone is
     * not enough to refresh the strip with the post-commit pending span.
     */
    private val onRequestCandidateRefresh: () -> Unit = {},
) {
    private val logger get() = taigikeyboard.compositionRoot.logger

    /**
     * Handle candidate click from RecyclerView.
     */
    fun handleCandidateClick(
        selectedWord: TaigiWord,
        index: Int,
    ) {
        TraceContext.withTrace(TraceId.next()) {
            logger.debug(TAG) { "[INPUT] fn=handleCandidateClick gesture=candidate-tap" }

            if (logger.isDebugEnabled) {
                val isNextWord = getCurrentSuggestions().firstOrNull()?.id?.let { it < 0 } ?: false
                logger.d(
                    TAG,
                    "[CLICK-ENTRY] onClick triggered, isNextWordMode=$isNextWord, suggestionsCount=${getCurrentSuggestions().size}",
                )
                logger.d(TAG, "[CLICK] index=$index, suggestionsSize=${getCurrentSuggestions().size}")
            }

            val ic = taigikeyboard.currentInputConnection ?: return

            val composingManager = getComposingManager()
            val capturedRawInput = composingManager?.getRawInput() ?: ""

            // Continuous-input branch routes BEFORE the sentinel-id branches.
            // The engine emits NextWordWordSelected on final commits which the
            // ComposingManager NextWordEffectRouter routes — onNextWordPrediction
            // is intentionally NOT called from the continuous branch to avoid
            // double-firing predictions.
            if (selectedWord.additionalInfo[TaigiWord.MetadataKeys.IS_CONTINUOUS] == "true" && composingManager != null) {
                handleContinuousCandidateClick(selectedWord, ic, composingManager)
                return@withTrace
            }

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

            if (logger.isDebugEnabled) {
                logger.d(
                    TAG,
                    "[CLICK] id=${selectedWord.id}, roman='${selectedWord.roman}', hanzi='${selectedWord.hanzi}'",
                )
                logger.d(
                    TAG,
                    "[CLICK] isTranslateSwapped=$cachedIsTranslateSwapped, effectiveSwapped=$effectiveSwapped, outputBothScripts=$cachedOutputBothScripts",
                )
                logger.d(
                    TAG,
                    "[CLICK] textToCommit='$textToCommit', isNextWord=$isNextWordPrediction, isEnglish=$isEnglishSuggestion",
                )
            }

            if (isEnglishSuggestion) {
                val textBeforeCursor = ic.getTextBeforeCursor(100, 0)?.toString() ?: ""
                val currentWord = NextWordHandler.extractCurrentWord(textBeforeCursor)
                if (currentWord.isNotEmpty()) {
                    ic.deleteSurroundingText(currentWord.length, 0)
                }
                ic.commitText(textToCommit, 1)
                onClearCandidates()
                logger.debug(TAG) { "[ENGLISH-CLICK] Replaced '$currentWord' with '$textToCommit'" }
            } else if (isNextWordPrediction) {
                logger.debug(TAG) { "[NEXTWORD-CLICK] BEFORE commitText: text='$textToCommit', ic=$ic" }
                val result = ic.commitText(textToCommit, 1)
                composingManager?.reset(ic)
                logger.debug(TAG) { "[NEXTWORD-CLICK] AFTER commitText: result=$result" }
            } else {
                composingManager?.selectSuggestion(textToCommit, ic)
            }

            appendAutoSpaceIfApplicable(ic, textToCommit, effectiveSwapped, cachedOutputBothScripts)

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
            logger.debug(TAG) { "[INPUT] fn=handleEnglishCandidateClick gesture=candidate-tap" }

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

            logger.debug(TAG) { "[ENGLISH-CLICK] Replaced '$currentWord' with '${selectedWord.roman}'" }
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

        // Continuous-input branch routes BEFORE the sentinel-id branches.
        // Overlay taps on Continuous candidates must go through commitContinuous
        // with the consumedBytes / syllableCount sidechannel; the default
        // selectSuggestion path would commit displayText only and mis-align
        // the engine pending buffer.
        if (word.additionalInfo[TaigiWord.MetadataKeys.IS_CONTINUOUS] == "true" && composingManager != null) {
            handleContinuousCandidateClick(word, ic, composingManager)
            return
        }

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
            logger.debug(TAG) { "[OVERLAY] NextWord commitText: '$textToCommit'" }
        } else {
            composingManager?.selectSuggestion(textToCommit, ic)
        }

        appendAutoSpaceIfApplicable(ic, textToCommit, effectiveSwapped, cachedOutputBothScripts)

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

        logger.debug(TAG) { "[OVERLAY] Selected suggestion: ${word.displayText} at index $index" }
    }

    /**
     * Continuous-input candidate tap (slot 0 and slot N, identical contract).
     *
     * Per `docs/engine/continuous-input-ranking.md` §10.3 clarification γ
     * (REVISED, Bug 1): commits the **swap/TPS/both-scripts-formatted
     * document string** built from the raw [TaigiWord.roman] / [TaigiWord.hanzi]
     * via the same `bracketRoman` + when-expr the legacy lexicon path uses —
     * so Continuous and lexicon commits match for the same candidate under
     * the same settings. The `DISPLAY_TEXT` sidechannel (engine canonical
     * `hanji.unwrap_or(roman)`) is NOT the document string; it is forwarded
     * as `commitContinuous(canonicalText = …)` so `user_frequency.db` +
     * NextWord key on the canonical token regardless of display mode
     * (decision b).
     *
     * Decodes the [TaigiWord.additionalInfo] sidechannel, dispatches
     * `commitContinuous`, and gates per-segment frequency learning +
     * final-commit auto-space on the effect-backed
     * [RustEngineBridge.CommitContinuousResult]. Stale taps where the engine
     * has already left Continuous collapse to `(false, false)` so neither
     * side-effect fires.
     *
     * `DISPLAY_TEXT`, `CONSUMED_BYTES`, and `SYLLABLE_COUNT` are all
     * strict-required (Item 4 fork F2=A); missing or unparseable → drop the
     * tap. The document string IS derived from [TaigiWord.roman] /
     * [TaigiWord.hanzi] (that is the legacy-parity contract); only the
     * canonical key rides the sidechannel. No fallback to
     * `selectSuggestion(text)` — would lose `consumedBytes` and corrupt
     * `Phase::Continuous { raw }` byte alignment.
     */
    private fun handleContinuousCandidateClick(
        selectedWord: TaigiWord,
        ic: android.view.inputmethod.InputConnection,
        composingManager: com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager,
    ) {
        val info = selectedWord.additionalInfo
        val displayText = info[TaigiWord.MetadataKeys.DISPLAY_TEXT]
        val consumedBytes = info[TaigiWord.MetadataKeys.CONSUMED_BYTES]?.toIntOrNull()
        val syllableCount = info[TaigiWord.MetadataKeys.SYLLABLE_COUNT]?.toIntOrNull()
        if (displayText == null || consumedBytes == null || syllableCount == null) {
            logger.w(
                TAG,
                "[CONTINUOUS] decode failed displayText=${info[TaigiWord.MetadataKeys.DISPLAY_TEXT]} consumedBytes=${info[TaigiWord.MetadataKeys.CONSUMED_BYTES]} syllableCount=${info[TaigiWord.MetadataKeys.SYLLABLE_COUNT]}",
            )
            return
        }

        // v3.5.8 Phase 9 Bug 1 (Option A): commit the SAME swap/TPS/both-
        // scripts-formatted string the legacy lexicon path commits (mirror
        // of handleCandidateClick's bracketRoman + when-expr, minus the
        // english arm — continuous candidates are never english). Built from
        // the RAW TaigiWord.roman/.hanzi (Android does not view-rewrite the
        // word before click handling, unlike iOS Suggestion). The canonical
        // DISPLAY_TEXT sidechannel is forwarded as canonicalText so
        // user_frequency.db + NextWord keys stay mode-independent (decision b).
        val cachedIsTranslateSwapped = getIsTranslateSwapped()
        val cachedOutputBothScripts = getOutputBothScripts()
        val isTPSLayout = prefs.keyboardLayoutType == "tps" || prefs.inputMode == "tps"
        val effectiveSwapped = isTPSLayout || cachedIsTranslateSwapped
        val bracketRoman =
            if (isTPSLayout) {
                RustEngineBridge.tlDisplayToTps(selectedWord.roman, prefs.tpsOrMapsToER)
            } else {
                selectedWord.roman
            }
        val textToCommit =
            when {
                cachedOutputBothScripts && !selectedWord.hanzi.isNullOrEmpty() -> {
                    if (effectiveSwapped) {
                        "${selectedWord.hanzi} ($bracketRoman)"
                    } else {
                        "$bracketRoman (${selectedWord.hanzi})"
                    }
                }

                effectiveSwapped && !selectedWord.hanzi.isNullOrEmpty() -> {
                    selectedWord.hanzi!!
                }

                else -> {
                    selectedWord.roman
                }
            }

        // R2: canonical TL identity sidechannel — forwarded as
        // `associationTl` so NextWord learns the same next_tl/prev_tl a
        // normal candidate commit records. Absent (wire skew / older
        // suggestion) → "" → engine falls back to the raw committed slice.
        val associationTl = info[TaigiWord.MetadataKeys.CANONICAL_TL] ?: ""
        val result = composingManager.commitContinuous(
            displayText = textToCommit,
            canonicalText = displayText,
            associationTl = associationTl,
            consumedBytes = consumedBytes,
            syllableCount = syllableCount,
            ic = ic,
        )

        logger.debug(TAG) {
            "[CONTINUOUS] commit displayText='$displayText' didCommit=${result.didCommit} didFinalCommit=${result.didFinalCommit}"
        }

        // Per-segment frequency on every successful commit (mid OR final).
        // Stale taps (didCommit=false) skip — engine had silently reset to Idle
        // so we'd be polluting UserFrequencyService with non-events.
        if (result.didCommit && prefs.frequencyRecordingEnabled) {
            scope.launch {
                userFreq.recordUsage(displayText)
            }
        }

        // Mid-commit: engine stays in Continuous with a fresh pending span,
        // but `PerformAutocomplete` is a delegate no-op so the strip would
        // keep stale `consumedBytes` metadata until the next keypress. Trigger
        // the standard debounced refresh. Final-commit deliberately skipped:
        // it emits NextWordWordSelected which drives async NextWord predict;
        // a debounced Taigi refresh would later see `rawInput=null` and call
        // `clearCandidates()`, racing with / wiping the fresh predictions.
        if (result.didCommit && !result.didFinalCommit) {
            onRequestCandidateRefresh()
        }

        // Auto-space only on final-commit (engine returned to Idle this call).
        // Mid-commits leave the buffer non-empty so a stray space would split
        // the word mid-syllable.
        if (result.didFinalCommit) {
            // Reuse the hoisted swap/output flags. Suffix check runs on the
            // actual committed document string (`textToCommit`), not the
            // canonical key (Codex post-impl: auto-space suffix check must use
            // the document string).
            appendAutoSpaceIfApplicable(ic, textToCommit, effectiveSwapped, cachedOutputBothScripts)
        }
    }

    /**
     * Insert a single trailing space when auto-space is enabled, the layout is
     * not effectively swapped (or both scripts are being output), and the
     * committed text doesn't already end in a hyphen continuation. Shared
     * by [handleCandidateClick], [handleOverlaySuggestionSelected], and
     * [handleContinuousCandidateClick] so the four-clause predicate stays
     * single-sourced.
     */
    private fun appendAutoSpaceIfApplicable(
        ic: android.view.inputmethod.InputConnection,
        committedText: String,
        effectiveSwapped: Boolean,
        outputBothScripts: Boolean,
    ) {
        if (!prefs.isAutoSpaceEnabled) return
        if (effectiveSwapped && !outputBothScripts) return
        if (committedText.endsWith("-")) return
        ic.commitText(" ", 1)
    }

    companion object {
        private const val TAG = "CandidateClickHandler"
    }
}
