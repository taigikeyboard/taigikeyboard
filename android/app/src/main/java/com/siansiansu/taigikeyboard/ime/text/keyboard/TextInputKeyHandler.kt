// Per-keystroke dispatch + composing/handler logic extracted from
// TextInputManager. Owns hot-path key-event reactions; collaborators
// stay un-synchronized on bare composingManager reads (write-side
// synchronization is `TextInputManager.composingLock`, hot path
// deliberately lock-free). See B5 PR-3 round notes.

package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.content.Context
import android.os.Handler
import android.text.InputType
import android.view.KeyEvent
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager
import com.siansiansu.taigikeyboard.engine.isTpsToneMark
import com.siansiansu.taigikeyboard.engine.transformInputCase
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.logging.tdebug
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.ime.text.AutoSpacePunctuation
import com.siansiansu.taigikeyboard.ime.text.CandidateUpdateCoordinator
import com.siansiansu.taigikeyboard.ime.text.CapsStateManager
import com.siansiansu.taigikeyboard.ime.text.CharacterInputPipeline
import com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager
import com.siansiansu.taigikeyboard.ime.text.composing.clearHostComposingRegion
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyType
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarManager
import com.siansiansu.taigikeyboard.ime.text.smartbar.appendAutoSpaceIfEarned
import com.siansiansu.taigikeyboard.ime.text.smartbar.rawPreeditWritesRomanization
import java.text.BreakIterator
import java.util.Locale

/**
 * Drives per-keystroke handling for the IME text path. Owns
 * [sendKeyPress] dispatch + DELETE / ENTER / SPACE / Taigi handlers.
 * Hot-path bare `composingManagerProvider()` reads stay lock-free
 * (write-side synchronization lives on [com.siansiansu.taigikeyboard.ime.text.TextInputManager]).
 */
internal class TextInputKeyHandler(
    private val taigikeyboard: TaigiKeyboard,
    private val prefs: PrefHelper,
    private val capsStateManager: CapsStateManager,
    private val uiCoordinator: KeyboardUiCoordinator,
    private val osHandler: Handler,
    private val composingManagerProvider: () -> ComposingManager?,
    private val candidateCoordinatorProvider: () -> CandidateUpdateCoordinator,
    private val smartbarManagerProvider: () -> SmartbarManager,
) {
    private val logger get() = taigikeyboard.compositionRoot.logger

    private val composingManager: ComposingManager? get() = composingManagerProvider()
    private val candidateCoordinator: CandidateUpdateCoordinator get() = candidateCoordinatorProvider()
    private val smartbarManager: SmartbarManager get() = smartbarManagerProvider()

    private val caps: Boolean get() = capsStateManager.caps
    private val capsLock: Boolean get() = capsStateManager.capsLock

    companion object {
        private const val TAG = "TextInputKeyHandler"
        private val DOUBLE_SPACE_PERIOD_REGEX = """[.!?‽\s][\s]""".toRegex()

        // UTF-16 look-behind window for last-grapheme deletion. A bounded
        // heuristic: 64 units covers any practical grapheme cluster (ZWJ family
        // emoji, flags, skin-tone modifiers) without scanning the whole field on
        // each backspace. Clusters have no hard length cap; this is pragmatic.
        private const val GRAPHEME_LOOKBEHIND = 64
    }

    // Clears any residual host composing region without committing its
    // content. Called at session-start + non-composing fallback paths
    // (DELETE / ENTER / NUMERIC-PHONE key) where a stale region could
    // otherwise be silently committed by a bare `finishComposingText()`.
    // Pinned by `INVARIANT_composing_clear_preedit_does_not_commit`
    // (`behavioral-invariants.md` §13).
    fun resetComposingText(notifyInputConnection: Boolean = true) {
        if (notifyInputConnection) {
            clearHostComposingRegion(taigikeyboard.currentInputConnection)
        }
    }

    /**
     * Main logic point for sending a key press.
     */
    fun sendKeyPress(keyData: KeyData) {
        taigikeyboard.compositionRoot.logger.tdebug(TAG) {
            "[SEND] fn=sendKeyPress code=${keyData.code} label='${keyData.label}' type=${keyData.type}"
        }
        // Every key gets exactly one chance at the punctuation swap: the arm
        // is consumed here, before any dispatch or early return, and only the
        // auto-space paths put it back.
        taigikeyboard.beginInputEvent()
        val ic = taigikeyboard.currentInputConnection

        when (keyData.code) {
            KeyCode.DELETE -> {
                handleDelete()
            }

            KeyCode.ENTER -> {
                handleEnter()
            }

            KeyCode.LANGUAGE_SWITCH -> {
                taigikeyboard.switchToNextInputMethod()
            }

            KeyCode.SETTINGS -> {
                taigikeyboard.launchSettings()
            }

            KeyCode.SHIFT -> {
                handleShift()
            }

            KeyCode.SHOW_INPUT_METHOD_PICKER -> {
                val im =
                    taigikeyboard.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
                im.showInputMethodPicker()
            }

            KeyCode.SWITCH_TO_MEDIA_CONTEXT -> {
                taigikeyboard.setActiveInput(R.id.media_input)
            }

            KeyCode.SWITCH_TO_TEXT_CONTEXT -> {
                taigikeyboard.setActiveInput(R.id.text_input)
            }

            KeyCode.VIEW_CHARACTERS -> {
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.CHARACTERS)
            }

            KeyCode.VIEW_NUMERIC -> {
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.NUMERIC)
            }

            KeyCode.VIEW_NUMERIC_ADVANCED -> {
                if (prefs.isTranslateSwapped && uiCoordinator.activeKeyboardMode == KeyboardMode.SYMBOLS) {
                    ic?.beginBatchEdit()
                    ic?.commitText("、", 1)
                    ic?.endBatchEdit()
                } else {
                    uiCoordinator.setActiveKeyboardMode(KeyboardMode.NUMERIC_ADVANCED)
                }
            }

            KeyCode.VIEW_PHONE -> {
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.PHONE)
            }

            KeyCode.VIEW_PHONE2 -> {
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.PHONE2)
            }

            KeyCode.VIEW_SYMBOLS -> {
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.SYMBOLS)
            }

            KeyCode.VIEW_SYMBOLS2 -> {
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.SYMBOLS2)
            }

            KeyCode.VIEW_CLIPBOARD -> {
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.CLIPBOARD)
            }

            KeyCode.TRANSLATE -> {
                smartbarManager.toggleTranslateSwapped()
            }

            else -> {
                ic?.beginBatchEdit()
                when (uiCoordinator.activeKeyboardMode) {
                    KeyboardMode.NUMERIC,
                    KeyboardMode.NUMERIC_ADVANCED,
                    KeyboardMode.PHONE,
                    KeyboardMode.PHONE2,
                    -> {
                        resetComposingText()
                        when (keyData.type) {
                            KeyType.CHARACTER,
                            KeyType.NUMERIC,
                            -> {
                                val text = keyData.code.toChar().toString()
                                ic?.commitText(text, 1)
                            }

                            else -> {
                                when (keyData.code) {
                                    KeyCode.PHONE_PAUSE,
                                    KeyCode.PHONE_WAIT,
                                    -> {
                                        val text = keyData.code.toChar().toString()
                                        ic?.commitText(text, 1)
                                    }
                                }
                            }
                        }
                    }

                    else -> {
                        when (keyData.type) {
                            KeyType.CHARACTER -> {
                                when (keyData.code) {
                                    KeyCode.SPACE -> {
                                        handleSpace()
                                    }

                                    KeyCode.URI_COMPONENT_TLD -> {
                                        if (composingManager?.isComposing() == true) {
                                            composingManager?.commitComposition(ic)
                                            smartbarManager.clearCandidates()
                                        }
                                        val tld =
                                            when (caps) {
                                                true -> keyData.label.uppercase(Locale.getDefault())
                                                false -> keyData.label.lowercase(Locale.getDefault())
                                            }
                                        ic?.commitText(tld, 1)
                                    }

                                    else -> {
                                        handleTaigiInput(keyData)
                                        ic?.endBatchEdit()
                                        return
                                    }
                                }
                            }

                            else -> {
                                logger.e(TAG, "sendKeyPress(keyData): Received unknown key: $keyData")
                            }
                        }
                    }
                }
                ic?.endBatchEdit()
            }
        }
    }

    /**
     * Handles a [KeyCode.DELETE] event.
     */
    private fun handleDelete() {
        val ic = taigikeyboard.currentInputConnection ?: return

        if (composingManager?.deleteBackward(ic) == true) {
            logger.debug(TAG) {
                val rawInput = composingManager?.getRawInput()
                val composingText = composingManager?.getComposingText()
                "[DELETE] deleteBackward=true, rawInput='$rawInput', composingText='$composingText'"
            }
            candidateCoordinator.updateTaigiCandidates()
            return
        }
        logger.debug(TAG) { "[DELETE] deleteBackward=false or composingManager=null" }

        deleteCommittedTextBackward(ic)

        // English mode: update candidates after backspace
        if (prefs.inputMode == "english") {
            candidateCoordinator.updateEnglishCandidates()
            return
        }

        // NextWord: re-predict from remaining text
        val textBeforeCursor = ic.getTextBeforeCursor(100, 0)?.toString() ?: ""
        smartbarManager.handleBackspaceForNextWord(textBeforeCursor)
    }

    // Deletes one unit of committed text backward, dispatching by editor
    // capability. Android contract: raw key events are intended for TYPE_NULL
    // editors; rich editors delete via InputConnection editing APIs. Rich web
    // editors (e.g. Gmail Google Chat, inputType TYPE_CLASS_TEXT) silently drop
    // a synthetic KEYCODE_DEL, so committed-text deletion must use the semantic
    // path. Mirrors florisboard AbstractEditorInstance, aiongtaigi-sushi, and
    // MOE Taigi. The composing-delete path is separate (ComposingManager).
    //
    // Any stale host composing region is dropped without being committed
    // (INVARIANT_composing_clear_preedit_does_not_commit): the selection branch's
    // commitText clears it atomically, the no-selection branch calls
    // resetComposingText (zero-then-finish) first. The clear is deliberately
    // skipped when a selection is present — there setComposingText("") would
    // itself delete the selection and the following delete would double-delete.
    private fun deleteCommittedTextBackward(ic: InputConnection) {
        val inputType = taigikeyboard.currentInputEditorInfo?.inputType ?: InputType.TYPE_NULL
        val isRawEditor = (inputType and InputType.TYPE_MASK_CLASS) == InputType.TYPE_NULL
        val hasSelection = !isRawEditor && !ic.getSelectedText(0).isNullOrEmpty()

        // Only the no-selection rich path reads cursor context + defensively
        // clears a stale composing region; TYPE_NULL and selection need neither.
        val readsContext = !isRawEditor && !hasSelection
        if (readsContext) resetComposingText()
        val textBefore =
            if (readsContext) ic.getTextBeforeCursor(GRAPHEME_LOOKBEHIND, 0)?.toString() else null

        when (val deletion = resolveBackspaceDeletion(inputType, hasSelection, textBefore)) {
            BackspaceDeletion.RawKeyEvent -> taigikeyboard.sendDownUpKeyEvents(KeyEvent.KEYCODE_DEL)
            BackspaceDeletion.Selection -> ic.commitText("", 1)
            BackspaceDeletion.NoOp -> Unit
            // Editor refused context (some password / custom editors). Code-point
            // delete keeps surrogate pairs whole; some editors don't implement it
            // (returns false pre-Android-13), so fall back to UTF-16 delete.
            BackspaceDeletion.CodePoint ->
                if (!ic.deleteSurroundingTextInCodePoints(1, 0)) ic.deleteSurroundingText(1, 0)
            is BackspaceDeletion.Grapheme -> ic.deleteSurroundingText(deletion.length, 0)
        }
    }

    /**
     * Handles a [KeyCode.ENTER] event.
     */
    private fun handleEnter() {
        val ic = taigikeyboard.currentInputConnection ?: return

        if (composingManager?.isComposing() == true) {
            val committedText = composingManager?.getComposingText() ?: ""
            // Model B §10.3: clear the candidate strip + NextWord state
            // BEFORE the commit so the engine's terminal NextWordWordSelected
            // (fired by commitComposition→CommitRaw) SURVIVES. clearCandidates()
            // bumps the NextWord generation; running it AFTER the commit (the
            // old order) stale-drops the engine's in-flight prediction query
            // (= Enter regression). The engine effect is the SOLE
            // association/prediction source — the old manual
            // handleNextWordPrediction double-recorded the association and
            // wasted the engine's prediction. Enter KEEPS the engine
            // prediction (nothing clears between this commit and the async
            // render). clearCandidates() only touches the smartbar strip +
            // NextWord state, never the IC composing region (owned by
            // ComposingManager), so running it pre-commit is safe.
            smartbarManager.clearCandidates()
            composingManager?.commitComposition(ic)

            val imeOptions = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0
            val maskedAction = imeOptions and EditorInfo.IME_MASK_ACTION

            logger.debug(TAG) { "[ENTER] composing mode, imeOptions=$imeOptions, maskedAction=$maskedAction" }

            if (imeOptions and EditorInfo.IME_FLAG_NO_ENTER_ACTION == 0 &&
                maskedAction in
                listOf(
                    EditorInfo.IME_ACTION_DONE,
                    EditorInfo.IME_ACTION_GO,
                    EditorInfo.IME_ACTION_NEXT,
                    EditorInfo.IME_ACTION_PREVIOUS,
                    EditorInfo.IME_ACTION_SEARCH,
                    EditorInfo.IME_ACTION_SEND,
                )
            ) {
                logger.debug(TAG) { "[ENTER] performing action: $maskedAction" }
                ic.performEditorAction(maskedAction)
                return
            }

            // Auto-space follows what the commit WROTE, not the output mode:
            // Enter writes the composition as typed, so the layout is the
            // whole question — TL and POJ compose romanization, TPS composes
            // Bopomofo, which takes no word spacing. Recorded either way, so
            // the punctuation swap below knows what the space (if any) was
            // written for.
            appendAutoSpaceIfEarned(
                taigikeyboard,
                ic,
                committedText,
                rawPreeditWritesRomanization(prefs.isTpsLayout),
            )
            // Model B §10.3: NO manual handleNextWordPrediction. The engine's
            // terminal NextWordWordSelected (from commitComposition→CommitRaw
            // above, dispatched via dispatchComposingNextWordEffect) is the
            // SOLE association/prediction source; clearCandidates() was moved
            // before the commit so that engine prediction survives → Enter
            // keeps showing next-word predictions.
            return
        }

        resetComposingText()
        val imeOptions = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0
        val maskedAction = imeOptions and EditorInfo.IME_MASK_ACTION

        logger.debug(TAG) { "[ENTER] non-composing mode, imeOptions=$imeOptions, maskedAction=$maskedAction" }

        if (imeOptions and EditorInfo.IME_FLAG_NO_ENTER_ACTION > 0) {
            ic.commitText("\n", 1)
        } else {
            when (maskedAction) {
                EditorInfo.IME_ACTION_DONE,
                EditorInfo.IME_ACTION_GO,
                EditorInfo.IME_ACTION_NEXT,
                EditorInfo.IME_ACTION_PREVIOUS,
                EditorInfo.IME_ACTION_SEARCH,
                EditorInfo.IME_ACTION_SEND,
                -> {
                    logger.debug(TAG) { "[ENTER] performing action: $maskedAction" }
                    ic.performEditorAction(maskedAction)
                }

                else -> {
                    ic.commitText("\n", 1)
                }
            }
        }
    }

    /**
     * Handles a [KeyCode.SHIFT] event.
     */
    private fun handleShift() {
        capsStateManager.handleShift()
    }

    /**
     * Handles a [KeyCode.SPACE] event.
     */
    private fun handleSpace() {
        val ic = taigikeyboard.currentInputConnection ?: return

        // English mode: commit space, clear candidates
        if (prefs.inputMode == "english") {
            ic.commitText(" ", 1)
            smartbarManager.clearCandidates()
            return
        }

        // TPS mode: space as tone 1/4 syllable boundary marker.
        // If the current syllable has no explicit tone mark, space adds a syllable
        // boundary and stays in composing mode (like Microsoft Zhuyin's space for tone 1).
        // If the syllable already has a tone mark or ends with space, fall through to commit.
        if (prefs.keyboardLayoutType == "tps" && composingManager?.isComposing() == true) {
            val rawInput = composingManager?.getRawInput() ?: ""
            val lastChar = rawInput.lastOrNull()
            if (lastChar != null && !RustEngineBridge.isTpsToneMark(lastChar) && lastChar != ' ') {
                composingManager?.appendCharacter(" ", ic)
                candidateCoordinator.updateTaigiCandidates()
                return
            }
        }

        if (composingManager?.isComposing() == true) {
            // Model B §10.3: the engine commit (commitComposition→CommitRaw)
            // fires the terminal NextWordWordSelected → records the
            // association (the SOLE source; the old manual
            // handleNextWordPrediction double-recorded it AND re-showed the
            // prediction — the Model-B Space regression). Space SUPPRESSES the
            // next-word *display*: clearCandidates() runs AFTER the commit
            // (kept order) → bumps the NextWord generation so the engine's
            // in-flight prediction query is dropped stale.
            composingManager?.commitComposition(ic)
            ic.commitText(" ", 1)
            smartbarManager.clearCandidates()
            return
        }

        if (prefs.doubleSpacePeriod) {
            if (capsStateManager.hasSpaceRecentlyPressed) {
                osHandler.removeCallbacksAndMessages(null)
                val text = ic.getTextBeforeCursor(2, 0) ?: ""
                if (text.length == 2 && !text.matches(DOUBLE_SPACE_PERIOD_REGEX)) {
                    ic.deleteSurroundingText(1, 0)
                    ic.commitText(".", 1)
                }
                capsStateManager.hasSpaceRecentlyPressed = false
            } else {
                capsStateManager.hasSpaceRecentlyPressed = true
                osHandler.postDelayed({
                    capsStateManager.hasSpaceRecentlyPressed = false
                }, 300)
            }
        }
        ic.commitText(KeyCode.SPACE.toChar().toString(), 1)
    }

    /**
     * Handle Taigi character input.
     */
    private fun handleTaigiInput(keyData: KeyData) {
        val inputStart = System.currentTimeMillis()
        val ic = taigikeyboard.currentInputConnection ?: return
        taigikeyboard.compositionRoot.logger.tdebug(TAG) {
            "[TAIGI] fn=handleTaigiInput code=${keyData.code} label='${keyData.label}'"
        }

        val baseText =
            if (keyData.label.isNotEmpty() &&
                keyData.label != keyData.code.toChar().toString()
            ) {
                keyData.label
            } else {
                keyData.code.toChar().toString()
            }

        val inputMode = InputMode.fromPrefString(prefs.inputMode)
        // Per-keystroke (not per-frame) — no cache needed; direct bridge call.
        var char = RustEngineBridge.transformInputCase(
            text = baseText,
            letterCase = RustEngineBridge.LetterCase.from(caps = caps, capsLock = capsLock),
            mode = inputMode,
        )

        // TPS layout: context-aware character adjustments via CharacterInputPipeline
        // (mirrors iOS CharacterInputPipeline.adjust collapsed entry point).
        if (prefs.keyboardLayoutType == "tps") {
            val rawInput = composingManager?.getRawInput() ?: ""
            val adjustment = CharacterInputPipeline.adjust(char, rawInput)
            char = adjustment.char
            adjustment.replaceLast?.let { composingManager?.replaceLastCharacter(it, ic) }
        }

        // English mode: commit directly
        if (prefs.inputMode == "english") {
            ic.commitText(char, 1)
            if (caps && !capsLock) {
                capsStateManager.resetSingleShift()
            }
            candidateCoordinator.updateEnglishCandidates()
            return
        }

        val manager = composingManager ?: return

        // Standalone digit: commit directly without entering composing mode.
        // Digits only enter composing as tone markers appended to existing romanization.
        if (!manager.isComposing() && char.length == 1 && char[0].isDigit()) {
            ic.commitText(char, 1)
            if (smartbarManager.isShowingNextWordCandidates()) {
                smartbarManager.clearCandidates()
            }
            return
        }

        if (isComposingCharacter(char)) {
            if (manager.isComposing()) {
                if (char == "-") {
                    manager.appendHyphen(ic)
                } else {
                    manager.appendCharacter(char, ic)
                }
                if (prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
                logger.debug("PERF") {
                    "[1] handleTaigiInput composing: ${System.currentTimeMillis() - inputStart}ms"
                }
                candidateCoordinator.updateTaigiCandidates()
            } else {
                if (char == "-" && smartbarManager.isShowingNextWordCandidates()) {
                    ic.commitText("-", 1)
                    logger.debug(TAG) { "[INPUT] '-' committed in NextWord mode, keeping suggestions" }
                } else {
                    manager.startComposing(char, ic)
                    if (prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
                    logger.debug("PERF") {
                        "[1] handleTaigiInput newComposing: ${System.currentTimeMillis() - inputStart}ms"
                    }
                    candidateCoordinator.updateTaigiCandidates()
                }
            }
        } else if (manager.isComposing() && char.length == 1 && char[0].isDigit()) {
            manager.appendCharacter(char, ic)
            if (prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
            logger.debug("PERF") {
                "[1] handleTaigiInput composing digit: ${System.currentTimeMillis() - inputStart}ms"
            }
            candidateCoordinator.updateTaigiCandidates()
        } else {
            if (manager.isComposing()) {
                manager.commitComposition(ic)
            }
            commitNonComposingCharacter(ic, char)
            smartbarManager.clearCandidates()
            return
        }
    }

    /**
     * Commit a non-composing character (punctuation / symbol), applying the
     * auto-space "smart punctuation" swap: when auto-space is active and the
     * char before the cursor is the auto-inserted trailing space, attaching
     * punctuation deletes that space and re-inserts it AFTER the punctuation
     * (`guá ` + `?` → `guá? `, never `guá ?`).
     */
    // CROSS-PLATFORM INVARIANT — mirrors iOS
    // ActionHandler.insertNonComposingCharacter. Drift causes silent divergence.
    private fun commitNonComposingCharacter(ic: InputConnection, char: String) {
        // No-selection guard: with an active selection the preceding space is
        // text before the selection, not an auto-space; the punctuation must
        // replace the selection normally (Codex P2).
        if (taigikeyboard.isAutoSpaceSwapArmed &&
            AutoSpacePunctuation.isAttaching(char) &&
            ic.getSelectedText(0).isNullOrEmpty() &&
            ic.getTextBeforeCursor(1, 0)?.toString() == " "
        ) {
            ic.deleteSurroundingText(1, 0)
            ic.commitText("$char ", 1)
            // Re-armed on the space the swap just wrote, so `?!` chains keep
            // swapping (`guá? ` + `!` → `guá?! `).
            taigikeyboard.armAutoSpaceSwap()
            return
        }
        ic.commitText(char, 1)
    }

}

/**
 * Pure-function classifier: returns `true` when [char] enters composing
 * mode (romanization letters, TPS bopomofo, TPS tone marks, hyphen
 * boundary, U+02D9 ˙). Used by [TextInputKeyHandler.handleTaigiInput];
 * exposed at file scope so pure-JVM tests can cover the table.
 */
internal fun isComposingCharacter(char: String): Boolean {
    val first = char.firstOrNull() ?: return false
    // isLetter() covers: a-z, A-Z (Lu/Ll), TPS bopomofo ㄅ-ㆷ (Lo),
    // TPS tone marks ˋ ˊ ˇ ˆ (Lm).
    // Three TPS tone marks are Sk (Symbol, modifier), not caught by isLetter:
    //   ˪ (U+02EA, tone 3), ˫ (U+02EB, tone 7), ˙ (U+02D9, tone 8)
    return first.isLetter() ||
        first == '-' ||
        first == '˪' ||
        first == '˫' ||
        first == '˙'
}

/**
 * Backspace deletion action chosen from editor capability + cursor context.
 * Resolved by [resolveBackspaceDeletion], applied by [TextInputKeyHandler] —
 * split out so the dispatch table is unit-testable without an InputConnection
 * mock.
 */
internal sealed interface BackspaceDeletion {
    // TYPE_NULL editor (terminal / some games): raw DOWN+UP key event.
    object RawKeyEvent : BackspaceDeletion

    // Active selection: replace it with empty text.
    object Selection : BackspaceDeletion

    // Cursor at field start: nothing to delete.
    object NoOp : BackspaceDeletion

    // Editor refused to expose context: degraded single code-point delete.
    object CodePoint : BackspaceDeletion

    // Rich editor: delete the last grapheme cluster ([length] UTF-16 units).
    data class Grapheme(val length: Int) : BackspaceDeletion
}

/**
 * Pure backspace dispatch: maps editor capability + cursor context to a
 * [BackspaceDeletion]. TYPE_NULL wins first (raw key only), then an active
 * selection, then the cursor-context cases. Exposed at file scope so pure-JVM
 * tests lock the table; the InputConnection calls that apply each action stay
 * dogfood-verified.
 */
internal fun resolveBackspaceDeletion(
    inputType: Int,
    hasSelection: Boolean,
    textBefore: String?,
): BackspaceDeletion =
    when {
        (inputType and InputType.TYPE_MASK_CLASS) == InputType.TYPE_NULL -> BackspaceDeletion.RawKeyEvent
        hasSelection -> BackspaceDeletion.Selection
        textBefore == null -> BackspaceDeletion.CodePoint
        textBefore.isEmpty() -> BackspaceDeletion.NoOp
        else -> BackspaceDeletion.Grapheme(lastGraphemeLength(textBefore))
    }

/**
 * UTF-16 length of the last grapheme cluster in [text], for backspace
 * deletion. [BreakIterator] keeps emoji / ZWJ sequences / combining marks /
 * flags as one user-perceived character on-device (ICU-backed); code-point
 * counting would split them. A fresh iterator per call: the type is not
 * thread-safe.
 */
internal fun lastGraphemeLength(text: String): Int {
    if (text.isEmpty()) return 0
    val iterator = BreakIterator.getCharacterInstance()
    iterator.setText(text)
    return iterator.last() - iterator.previous()
}
