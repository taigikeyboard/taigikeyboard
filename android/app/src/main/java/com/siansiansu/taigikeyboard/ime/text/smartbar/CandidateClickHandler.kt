// Candidate click handler, extracted from SmartbarManager: handles Taigi candidate selection,
// English suggestion replacement, and overlay candidate selection, and formats the output text.

package com.siansiansu.taigikeyboard.ime.text.smartbar

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.tlDisplayToTps
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
            taigikeyboard.beginInputEvent()

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
            val isTPSLayout = prefs.isTpsLayout
            val effectiveSwapped = isTPSLayout || cachedIsTranslateSwapped

            val resolved =
                if (isEnglishSuggestion) {
                    // An English word IS Latin text, so it takes the spacing.
                    ResolvedCommit(selectedWord.roman, wroteRomanization = true)
                } else {
                    resolveTaigiCommit(selectedWord, isTPSLayout, effectiveSwapped, cachedOutputBothScripts)
                }
            val textToCommit = resolved.text

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

            appendAutoSpaceIfEarned(taigikeyboard, ic, textToCommit, resolved.wroteRomanization)

            // Record usage frequency. R5 pair-key (#7): the candidate's
            // canonical-TL reading from the metadata sidechannel keeps
            // 一字多音 in separate buckets; "" only on wire skew / TPS-OOV.
            if (prefs.frequencyRecordingEnabled) {
                val canonicalTl = selectedWord.additionalInfo[TaigiWord.MetadataKeys.CANONICAL_TL] ?: ""
                scope.launch {
                    userFreq.recordUsage(selectedWord.displayText, canonicalTl)
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
     * Document text + auto-space verdict for one Taigi candidate tap — every
     * tap path (strip, expanded overlay, Continuous) resolves through here.
     * A §42 漢羅濫 [TaigiWord.MetadataKeys.CELL_SCRIPT]-marked cell commits
     * the script its marker names ([resolveMarkedCellCommit]); an unmarked
     * cell follows the mode ([resolveUnmarkedCommit]), with the 括號標註 roman
     * TPS-rendered under the TPS layout.
     */
    private fun resolveTaigiCommit(
        word: TaigiWord,
        isTPSLayout: Boolean,
        effectiveSwapped: Boolean,
        outputBothScripts: Boolean,
    ): ResolvedCommit {
        word.additionalInfo[TaigiWord.MetadataKeys.CELL_SCRIPT]?.let { cellScript ->
            resolveMarkedCellCommit(
                cellScript = cellScript,
                roman = word.roman,
                hanzi = word.hanzi,
                outputBothScripts = outputBothScripts,
            )
        }?.let { return it }
        val bracketRoman =
            if (isTPSLayout) RustEngineBridge.tlDisplayToTps(word.roman, prefs.tpsOrMapsToER) else word.roman
        return resolveUnmarkedCommit(
            roman = word.roman,
            bracketRoman = bracketRoman,
            hanzi = word.hanzi,
            effectiveSwapped = effectiveSwapped,
            outputBothScripts = outputBothScripts,
        )
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
        taigikeyboard.beginInputEvent()
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
        val isTPSLayout = prefs.isTpsLayout
        val effectiveSwapped = isTPSLayout || cachedIsTranslateSwapped

        val resolved = resolveTaigiCommit(word, isTPSLayout, effectiveSwapped, cachedOutputBothScripts)
        val textToCommit = resolved.text

        if (isNextWordPred) {
            ic.commitText(textToCommit, 1)
            composingManager?.reset(ic)
            logger.debug(TAG) { "[OVERLAY] NextWord commitText: '$textToCommit'" }
        } else {
            composingManager?.selectSuggestion(textToCommit, ic)
        }

        appendAutoSpaceIfEarned(taigikeyboard, ic, textToCommit, resolved.wroteRomanization)

        // Record usage frequency. R5 pair-key (#7): canonical-TL reading
        // from the metadata sidechannel; "" only on wire skew / TPS-OOV.
        if (prefs.frequencyRecordingEnabled) {
            val canonicalTl = word.additionalInfo[TaigiWord.MetadataKeys.CANONICAL_TL] ?: ""
            scope.launch {
                userFreq.recordUsage(word.displayText, canonicalTl)
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
     *
     * §42 漢羅濫 split cells: a [TaigiWord.MetadataKeys.CELL_SCRIPT]-marked
     * cell resolves its document string via [resolveMarkedCellCommit]
     * (the marker is authoritative; the mode-derived when-expr is bypassed)
     * and its auto-space verdict rides [ResolvedCommit.wroteRomanization].
     * Identity (`commitContinuous` canonicalText/associationTl, 詞頻
     * pair-key) is marker-independent — both cells commit the same
     * candidate.
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
        //
        // §42 漢羅濫 split cells: a CELL_SCRIPT-marked cell resolves the
        // document text DIRECTLY from the marker (hanji cell → 漢字 or
        // `漢字 (羅馬字)` under 括號標註; roman cell → the BARE roman,
        // brackets ignored — desktop `.alternate` parity), bypassing the
        // mode-derived when-expr. Marked cells never exist under TPS (the
        // builder split is gated off there), so no TPS re-render applies.
        val cachedIsTranslateSwapped = getIsTranslateSwapped()
        val cachedOutputBothScripts = getOutputBothScripts()
        val isTPSLayout = prefs.isTpsLayout
        val effectiveSwapped = isTPSLayout || cachedIsTranslateSwapped
        val resolved = resolveTaigiCommit(selectedWord, isTPSLayout, effectiveSwapped, cachedOutputBothScripts)
        val textToCommit = resolved.text

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
        // R5 pair-key (#7): reuse the `associationTl` canonical-TL sidechannel
        // already resolved above (same reading NextWord learns).
        if (result.didCommit && prefs.frequencyRecordingEnabled) {
            scope.launch {
                userFreq.recordUsage(displayText, associationTl)
            }
        }

        // Mid-commit: engine stays in Continuous with a fresh pending span,
        // but `PerformAutocomplete` is a delegate no-op so the strip would
        // keep stale `consumedBytes` metadata until the next keypress. Trigger
        // the standard refresh. Final-commit deliberately skipped: it emits
        // NextWordWordSelected which drives async NextWord predict; a Taigi
        // refresh would see `rawInput=null` and call `clearCandidates()`,
        // racing with / wiping the fresh predictions.
        if (result.didCommit && !result.didFinalCommit) {
            onRequestCandidateRefresh()
        }

        // Auto-space only on final-commit (engine returned to Idle this call).
        // Mid-commits leave the buffer non-empty so a stray space would split
        // the word mid-syllable. Suffix check runs on the actual committed
        // document string (`textToCommit`), not the canonical key (Codex
        // post-impl: auto-space suffix check must use the document string).
        if (result.didFinalCommit) {
            appendAutoSpaceIfEarned(taigikeyboard, ic, textToCommit, resolved.wroteRomanization)
        }
    }

    companion object {
        private const val TAG = "CandidateClickHandler"
    }
}

/**
 * 括號標註 / both-scripts commit form when 漢字 leads — the romanization
 * rides in trailing brackets. Single spelling for the four hanji-first
 * commit sites; the roman-first inverse (`roman (hanzi)`) stays inline.
 */
private fun bracketedCommit(
    hanzi: String,
    roman: String,
): String = "$hanzi ($roman)"

/**
 * Document text + auto-space verdict for an UNMARKED commit — the three
 * sites that build the string from the candidate's own `(roman, hanzi)`
 * pair rather than from a §42 cell marker.
 *
 * The verdict is resolved by the SAME arm that picks the string, never from
 * the output mode afterwards. Auto-space is a property of ROMANIZATION
 * (`guá beh khì` needs the gaps, 我欲去 does not), and a candidate with no
 * Hanji — the 字面羅馬字 candidate (§34), an out-of-vocabulary name, a
 * romanization-only custom entry — falls to the last arm and writes its
 * romanization whatever the mode leads with.
 *
 * `bracketRoman` is the 括號標註 rendering of `roman` (TPS-converted in a
 * TPS layout); the bare `roman` is what a roman-led commit writes.
 */
// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Actions/ActionHandler+Suggestions.swift
// `formatOutputText` and macos/.../CandidateDocumentText.swift `resolved`. Drift causes
// silent divergence (a missing or stray auto-space after a swapped-mode commit).
internal fun resolveUnmarkedCommit(
    roman: String,
    bracketRoman: String,
    hanzi: String?,
    effectiveSwapped: Boolean,
    outputBothScripts: Boolean,
): ResolvedCommit =
    when {
        // 括號標註 writes the pair, so the romanization IS in the document
        // whichever half leads.
        outputBothScripts && !hanzi.isNullOrEmpty() ->
            ResolvedCommit(
                text =
                    if (effectiveSwapped) {
                        bracketedCommit(hanzi, bracketRoman)
                    } else {
                        "$bracketRoman ($hanzi)"
                    },
                wroteRomanization = true,
            )

        effectiveSwapped && !hanzi.isNullOrEmpty() ->
            ResolvedCommit(text = hanzi, wroteRomanization = false)

        else -> ResolvedCommit(text = roman, wroteRomanization = true)
    }

/**
 * Insert a single trailing space when auto-space is enabled, the commit wrote
 * romanization, and the committed text does not already end in a hyphen
 * continuation — and arm the punctuation swap on it.
 *
 * The ONE place a space and the knowledge that it is ours are set together, so
 * they can never come apart: a commit that earns nothing arms nothing, and the
 * event already consumed the previous arm ([TaigiKeyboard.beginInputEvent]).
 * Callers resolve [wroteRomanization] from the arm that picked the document
 * string — [resolveUnmarkedCommit], [resolveMarkedCellCommit], or
 * [rawPreeditWritesRomanization] — never from the output mode.
 */
internal fun appendAutoSpaceIfEarned(
    taigikeyboard: TaigiKeyboard,
    ic: android.view.inputmethod.InputConnection,
    committedText: String,
    wroteRomanization: Boolean,
) {
    if (!shouldAppendAutoSpace(taigikeyboard.prefs.isAutoSpaceEnabled, wroteRomanization, committedText)) return
    ic.commitText(" ", 1)
    taigikeyboard.armAutoSpaceSwap()
}

/**
 * Whether the layout in use composes romanization — TL and POJ do, TPS
 * composes Bopomofo, which takes no word spacing. The verdict for every commit
 * that writes the composition AS TYPED (Enter on the raw input), which does
 * not go through a candidate's rendering.
 */
// CROSS-PLATFORM INVARIANT — one name on all four platforms: ios
// `ActionHandler.rawPreeditWritesRomanization`, macOS/Windows
// `AutoSpacePolicy.rawPreeditWritesRomanization(inputMode:)` /
// `policies::raw_preedit_writes_romanization`.
internal fun rawPreeditWritesRomanization(isTPSLayout: Boolean): Boolean = !isTPSLayout

/**
 * Whether to insert the trailing auto-space: the setting is on, the commit
 * wrote romanization, and the committed DOCUMENT string does not end in a
 * hyphen continuation (a mid-word 連字 keeps composing). The Continuous
 * caller still owns the final-commit gate.
 */
// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Actions/ActionHandler+Suggestions.swift
// `shouldAppendAutoSpace`. Drift causes silent divergence (one platform spacing after a
// hyphen continuation).
internal fun shouldAppendAutoSpace(
    isAutoSpaceEnabled: Boolean,
    wroteRomanization: Boolean,
    committedText: String,
): Boolean = isAutoSpaceEnabled && wroteRomanization && !committedText.endsWith("-")

/**
 * Document text + auto-space verdict for a 漢羅濫
 * [TaigiWord.MetadataKeys.CELL_SCRIPT]-marked cell (§42 second exception).
 * Hanji cell → the 漢字, or `漢字 (羅馬字)` when 括號標註 is on (only then
 * did the commit write romanization); roman cell → the BARE roman, brackets
 * IGNORED (desktop `.alternate` parity), always romanization. Returns
 * `null` for an unknown marker or a marker whose payload is missing (wire
 * defect) — the caller falls back to the unmarked mode-derived path.
 * Top-level pure function so the contract is unit-testable without
 * collaborators.
 */
// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Keyboard/ActionHandler+Suggestions.swift marked-cell commit resolve
// and the desktop `.alternate` commit rule. Drift causes silent divergence (a bracketed roman-cell commit, or a missing auto-space).
internal fun resolveMarkedCellCommit(
    cellScript: String,
    roman: String,
    hanzi: String?,
    outputBothScripts: Boolean,
): ResolvedCommit? =
    when {
        cellScript == TaigiWord.MetadataKeys.CELL_SCRIPT_ROMAN && roman.isNotEmpty() ->
            ResolvedCommit(text = roman, wroteRomanization = true)

        cellScript == TaigiWord.MetadataKeys.CELL_SCRIPT_HANJI && !hanzi.isNullOrEmpty() ->
            if (outputBothScripts && roman.isNotEmpty()) {
                ResolvedCommit(text = bracketedCommit(hanzi, roman), wroteRomanization = true)
            } else {
                // No roman to bracket → the bare 漢字, never empty brackets.
                ResolvedCommit(text = hanzi, wroteRomanization = false)
            }

        else -> null
    }

/**
 * What one commit writes into the document, and whether that string carries
 * romanization — the single input the auto-space gate reads.
 */
// CROSS-PLATFORM INVARIANT — mirrors ios `ActionHandler.ResolvedCommit`,
// macos `CandidateDocumentText.ResolvedCommit`, windows
// `composing::document_text::ResolvedCommit`.
internal data class ResolvedCommit(
    val text: String,
    val wroteRomanization: Boolean,
)
