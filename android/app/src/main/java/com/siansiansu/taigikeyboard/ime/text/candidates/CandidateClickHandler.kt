// Candidate click handler, extracted from SmartbarManager: handles Taigi candidate selection,
// English suggestion replacement, and overlay candidate selection; the engine resolves what a
// Taigi tap writes.

package com.siansiansu.taigikeyboard.ime.text.candidates

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.proto.CommitScript
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.TraceContext
import com.siansiansu.taigikeyboard.ime.core.logging.TraceId
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.text.composing.Usage
import com.siansiansu.taigikeyboard.ime.text.composing.UsageRecorder
import com.siansiansu.taigikeyboard.ime.text.nextword.NextWordController

/**
 * Handles candidate click events extracted from SmartbarManager.
 *
 * Processes Taigi candidate selection, English suggestion replacement,
 * and overlay suggestion selection.
 */
class CandidateClickHandler(
    private val taigikeyboard: TaigiKeyboard,
    private val usage: UsageRecorder,
    private val getCurrentSuggestions: () -> List<TaigiWord>,
    private val getComposingManager: () -> com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager?,
    private val onClearCandidates: () -> Unit,
    private val onNextWordPrediction: (displayText: String, roman: String) -> Unit,
    /**
     * Schedule a Taigi candidate recompute. Called only after a Continuous
     * mid-commit where the engine's `RefreshCandidates` effect alone is
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

            // Continuous-input branch routes BEFORE the sentinel-id branches.
            // The engine emits NextWordWordSelected on final commits which the
            // ComposingManager NextWordEffectRouter routes — onNextWordPrediction
            // is intentionally NOT called from the continuous branch to avoid
            // double-firing predictions.
            if (selectedWord.additionalInfo[TaigiWord.MetadataKeys.IS_CONTINUOUS] == "true") {
                // No composing manager (field became composing-ineligible under a
                // stale strip) → drop the tap, as iOS drops a pick it cannot route.
                val manager = composingManager ?: return@withTrace
                handleContinuousCandidateClick(selectedWord, ic, manager)
                return@withTrace
            }

            if (selectedWord.id <= -100) {
                // An English word IS Latin text, so it takes the spacing.
                val textToCommit = selectedWord.roman
                logger.debug(TAG) { "[ENGLISH-CLICK] id=${selectedWord.id}, text='$textToCommit'" }
                val textBeforeCursor = ic.getTextBeforeCursor(100, 0)?.toString() ?: ""
                val currentWord = NextWordController.extractCurrentWord(textBeforeCursor)
                if (currentWord.isNotEmpty()) {
                    ic.deleteSurroundingText(currentWord.length, 0)
                }
                ic.commitText(textToCommit, 1)
                onClearCandidates()
                logger.debug(TAG) { "[ENGLISH-CLICK] Replaced '$currentWord' with '$textToCommit'" }
                appendAutoSpaceIfEarned(taigikeyboard, ic, textToCommit, true)
                recordCommittedWord(selectedWord)
                return@withTrace
            }

            // A NextWord prediction (`id < 0`): the only other word the strip
            // carries (`TaigiAutocompleteService` produces Continuous cells,
            // `NextWordController` the predictions).
            commitNextWordPrediction(selectedWord, ic, composingManager)
        }
    }

    /**
     * Handle English candidate click (3-column layout).
     */
    fun handleEnglishCandidateClick(index: Int) {
        TraceContext.withTrace(TraceId.next()) {
            logger.debug(TAG) { "[INPUT] fn=handleEnglishCandidateClick gesture=candidate-tap" }
            taigikeyboard.beginInputEvent()

            val currentSuggestions = getCurrentSuggestions()
            if (index >= currentSuggestions.size) return

            val selectedWord = currentSuggestions[index]
            val ic = taigikeyboard.currentInputConnection ?: return

            val textBeforeCursor = ic.getTextBeforeCursor(100, 0)?.toString() ?: ""
            val currentWord = NextWordController.extractCurrentWord(textBeforeCursor)

            if (currentWord.isNotEmpty()) {
                ic.deleteSurroundingText(currentWord.length, 0)
            }
            ic.commitText(selectedWord.roman, 1)
            onClearCandidates()

            logger.debug(TAG) { "[ENGLISH-CLICK] Replaced '$currentWord' with '${selectedWord.roman}'" }
            // CROSS-PLATFORM INVARIANT — mirrors iOS ActionHandler.handle(_ suggestion:)
            // English arm: an English word takes the Auto-Space spacing and arms the swap.
            appendAutoSpaceIfEarned(taigikeyboard, ic, selectedWord.roman, wroteRomanization = true)
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

        // Continuous-input branch routes BEFORE the NextWord path. Overlay taps
        // on Continuous candidates must go through commitContinuous with the
        // consumedBytes / syllableCount sidechannel; a plain commitText would
        // write displayText only and mis-align the engine pending buffer.
        if (word.additionalInfo[TaigiWord.MetadataKeys.IS_CONTINUOUS] == "true") {
            val manager = composingManager ?: return
            handleContinuousCandidateClick(word, ic, manager)
            return
        }

        // A NextWord prediction (`id < 0`): the only non-Continuous word the
        // overlay carries.
        commitNextWordPrediction(word, ic, composingManager)

        logger.debug(TAG) { "[OVERLAY] Selected suggestion: ${word.displayText} at index $index" }
    }

    /**
     * One NextWord prediction tap, strip or expanded overlay (D1): the engine
     * resolves what it writes from the prediction's own scripts under the
     * live settings and counts the pick; the composition is then reset,
     * auto-space judged and the pair learned. No composing manager (a stale
     * strip in a composing-ineligible field) drops the tap, as a Continuous
     * tap does.
     */
    private fun commitNextWordPrediction(
        word: TaigiWord,
        ic: android.view.inputmethod.InputConnection,
        composingManager: com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager?,
    ) {
        val manager = composingManager ?: return
        val pick = predictionPick(word)
        val commit = manager.commitPrediction(pick)
        if (commit == null) {
            logger.debug(TAG) { "[NEXTWORD-CLICK] ignored by the engine id=${word.id} script=${pick.script}" }
            return
        }
        logger.debug(TAG) { "[NEXTWORD-CLICK] id=${word.id}, roman='${word.roman}', hanji='${word.hanji}', text='${commit.text}'" }
        ic.commitText(commit.text, 1)
        manager.reset(ic)
        appendAutoSpace(taigikeyboard, ic, taigikeyboard.prefs.isAutoSpaceEnabled && commit.earnsAutoSpace)
        learnCommittedWord(word, pick.canonicalTl)
    }

    /** Usage count + NextWord learning for a committed English word. */
    private fun recordCommittedWord(word: TaigiWord) {
        // R5 pair-key (#7): the canonical-TL reading from the metadata sidechannel
        // keeps multi-reading Hanji in separate buckets; "" on English rows.
        val canonicalTl = word.additionalInfo[TaigiWord.MetadataKeys.CANONICAL_TL] ?: ""
        usage.record(Usage(word.displayText, canonicalTl))
        learnCommittedWord(word, canonicalTl)
    }

    private fun learnCommittedWord(
        word: TaigiWord,
        canonicalTl: String,
    ) {
        // NextWord learns the canonical reading, not the rendered `roman` (the
        // Syllable Separator rewrites its hyphens, §49); iOS learns from the
        // same sidechannel.
        onNextWordPrediction(word.displayText, canonicalTl.ifEmpty { word.roman })
    }

    /**
     * Continuous-input candidate tap (slot 0 and slot N, identical contract),
     * strip and expanded overlay alike.
     *
     * R5: the engine resolves what the pick writes from the candidate's own
     * scripts under the live settings, counts the pick, and answers what it
     * did — so Continuous commits match iOS / macOS / Windows / Linux by
     * construction. The pick is read off the word's metadata ([continuousPick]);
     * a §42 split cell commits the script its marker names ([commitScript]).
     *
     * `DISPLAY_TEXT`, `CONSUMED_BYTES`, and `SYLLABLE_COUNT` are
     * strict-required (Item 4 fork F2=A); missing or unparseable → drop the
     * tap. No fallback to a plain commit of the text — would lose
     * `consumedBytes` and corrupt `Phase::Continuous { raw }` byte alignment.
     */
    private fun handleContinuousCandidateClick(
        selectedWord: TaigiWord,
        ic: android.view.inputmethod.InputConnection,
        composingManager: com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager,
    ) {
        val pick = continuousPick(selectedWord)
        if (pick == null) {
            val info = selectedWord.additionalInfo
            logger.w(
                TAG,
                "[CONTINUOUS] decode failed displayText=${info[TaigiWord.MetadataKeys.DISPLAY_TEXT]} consumedBytes=${info[TaigiWord.MetadataKeys.CONSUMED_BYTES]} syllableCount=${info[TaigiWord.MetadataKeys.SYLLABLE_COUNT]}",
            )
            return
        }
        val outcome = composingManager.commitContinuous(pick, ic)
        logger.debug(TAG) { "[CONTINUOUS] commit script=${pick.script} outcome=$outcome" }

        when (outcome) {
            // Mid-commit: engine stays in Continuous with a fresh pending span,
            // but `RefreshCandidates` is a delegate no-op so the strip would
            // keep stale `consumedBytes` metadata until the next keypress.
            // Trigger the standard refresh. Final-commit deliberately skipped:
            // it emits NextWordWordSelected which drives async NextWord
            // predict; a Taigi refresh would see `rawInput=null` and call
            // `clearCandidates()`, racing with / wiping the fresh predictions.
            RustEngineBridge.ContinuousCommitOutcome.Nailed -> onRequestCandidateRefresh()
            // Auto-space only on the final commit, as the engine judged what
            // the pick wrote (romanization, no trailing `-`, §23).
            is RustEngineBridge.ContinuousCommitOutcome.Finalized ->
                appendAutoSpace(taigikeyboard, ic, taigikeyboard.prefs.isAutoSpaceEnabled && outcome.earnsAutoSpace)
            // A stale tap (the engine had silently reset to Idle) or a
            // rejected pick wrote nothing.
            RustEngineBridge.ContinuousCommitOutcome.Ignored -> Unit
        }
    }

    companion object {
        private const val TAG = "CandidateClickHandler"
    }
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
 * string (an English word, or [rawPreeditWritesRomanization]) — never from
 * the output mode.
 */
internal fun appendAutoSpaceIfEarned(
    taigikeyboard: TaigiKeyboard,
    ic: android.view.inputmethod.InputConnection,
    committedText: String,
    wroteRomanization: Boolean,
) = appendAutoSpace(
    taigikeyboard,
    ic,
    shouldAppendAutoSpace(taigikeyboard.prefs.isAutoSpaceEnabled, wroteRomanization, committedText),
)

/**
 * The one insertion site of the auto space: [isEarned] is the whole verdict,
 * the live Auto-Space setting included ([appendAutoSpaceIfEarned], or the
 * engine's `earns_auto_space` ANDed with it on a Continuous or prediction commit).
 */
private fun appendAutoSpace(
    taigikeyboard: TaigiKeyboard,
    ic: android.view.inputmethod.InputConnection,
    isEarned: Boolean,
) {
    if (!isEarned) return
    ic.commitText(" ", 1)
    taigikeyboard.armAutoSpaceSwap()
}

/**
 * The engine request for a Continuous candidate tap, read off the word's
 * metadata (`TaigiAutocompleteService.kt` `continuousSidechannels`) and its own
 * `roman` / `hanji` (the candidate's, never rewritten on Android). `null` when
 * a strict key is missing. Mirrors iOS `ActionHandler.continuousPick(for:)`.
 */
internal fun continuousPick(word: TaigiWord): RustEngineBridge.ContinuousPick? {
    val info = word.additionalInfo
    val canonicalText = info[TaigiWord.MetadataKeys.DISPLAY_TEXT] ?: return null
    val consumedBytes = info[TaigiWord.MetadataKeys.CONSUMED_BYTES]?.toIntOrNull() ?: return null
    val syllableCount = info[TaigiWord.MetadataKeys.SYLLABLE_COUNT]?.toIntOrNull() ?: return null
    return RustEngineBridge.ContinuousPick(
        script = commitScript(word),
        roman = word.roman,
        canonicalText = canonicalText,
        // Absent only when the pick carries no reading (TPS-OOV) → "" → the
        // engine uses the raw committed slice for NextWord.
        associationTl = info[TaigiWord.MetadataKeys.CANONICAL_TL] ?: "",
        // §50 — the pick's hanji (identity, not the committed script): the
        // engine learns a composition only when every segment carried one.
        hanji = word.hanji?.takeIf { it.isNotEmpty() },
        consumedBytes = consumedBytes,
        syllableCount = syllableCount,
    )
}

// CROSS-PLATFORM INVARIANT — mirrors ios ActionHandler+Suggestions.swift `commitScript(for:)`.

/**
 * Which script a Continuous tap commits: a §42 split cell the one its
 * [TaigiWord.MetadataKeys.CELL_SCRIPT] marker names, every other cell what the
 * output settings lead with. An unknown marker commits the lead; a marker
 * whose script the pick lacks the engine resolves to the lead too.
 */
internal fun commitScript(word: TaigiWord): CommitScript =
    when (word.additionalInfo[TaigiWord.MetadataKeys.CELL_SCRIPT]) {
        TaigiWord.MetadataKeys.CELL_SCRIPT_HANJI -> CommitScript.COMMIT_SCRIPT_HANJI
        TaigiWord.MetadataKeys.CELL_SCRIPT_ROMAN -> CommitScript.COMMIT_SCRIPT_ROMAN
        else -> CommitScript.COMMIT_SCRIPT_LEAD
    }

/**
 * The engine request for a NextWord prediction tap, read off the word's own
 * `roman` / `hanji` (`NextWordController.kt` `predictionWord`: `roman` is
 * empty for a Hanji-only prediction) and its canonical-TL sidechannel. A §42
 * split cell commits the script its marker names ([commitScript]); a split
 * cell whose own script is missing commits the lead, as a Continuous pick
 * does. Mirrors iOS `ActionHandler.predictionPick(for:)`.
 */
internal fun predictionPick(word: TaigiWord): RustEngineBridge.PredictionPick =
    RustEngineBridge.PredictionPick(
        script = commitScript(word),
        roman = word.roman,
        hanji = word.hanji.orEmpty(),
        canonicalTl = word.additionalInfo[TaigiWord.MetadataKeys.CANONICAL_TL] ?: "",
    )

// CROSS-PLATFORM INVARIANT — one name on all five platforms: ios
// `ActionHandler.rawPreeditWritesRomanization`, desktop-core (macOS / Windows / Linux)
// `policies::raw_preedit_writes_romanization`.

/**
 * Whether the layout in use composes romanization — TL and POJ do, TPS
 * composes Bopomofo, which takes no word spacing. The verdict for every commit
 * that writes the composition AS TYPED (Enter on the raw input), which does
 * not go through a candidate's rendering.
 */
internal fun rawPreeditWritesRomanization(isTPSLayout: Boolean): Boolean = !isTPSLayout

// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Actions/ActionHandler+Suggestions.swift
// `shouldAppendAutoSpace`. Drift causes silent divergence (one platform spacing after a
// hyphen continuation).

/**
 * Whether to insert the trailing auto-space: the setting is on, the commit
 * wrote romanization, and the committed DOCUMENT string does not end in a
 * hyphen continuation (a mid-word hyphen keeps composing). The Continuous
 * caller still owns the final-commit gate.
 */
internal fun shouldAppendAutoSpace(
    isAutoSpaceEnabled: Boolean,
    wroteRomanization: Boolean,
    committedText: String,
): Boolean = isAutoSpaceEnabled && wroteRomanization && !committedText.endsWith("-")
