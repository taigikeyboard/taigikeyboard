// Per-keystroke dispatch + composing/handler logic extracted from
// TextInputManager. Owns hot-path key-event reactions; collaborators
// stay un-synchronized on bare composingManager reads (write-side
// synchronization is `TextInputManager.composingLock`, hot path
// deliberately lock-free). See B5 PR-3 round notes.

package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.content.Context
import android.os.Handler
import android.view.KeyEvent
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.engine.CaseTransformBridge
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.logging.tdebug
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.ime.text.CandidateUpdateCoordinator
import com.siansiansu.taigikeyboard.ime.text.CapsStateManager
import com.siansiansu.taigikeyboard.ime.text.CharacterInputPipeline
import com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager
import com.siansiansu.taigikeyboard.ime.text.composing.clearHostComposingRegion
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyType
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarManager
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
            candidateCoordinator.scheduleDisplayDerivation()
            candidateCoordinator.updateTaigiCandidatesDebounced()
            return
        }
        logger.debug(TAG) { "[DELETE] deleteBackward=false or composingManager=null" }

        ic.beginBatchEdit()
        resetComposingText()
        ic.sendKeyEvent(
            KeyEvent(
                KeyEvent.ACTION_DOWN,
                KeyEvent.KEYCODE_DEL,
            ),
        )
        ic.endBatchEdit()

        // English mode: update candidates after backspace
        if (prefs.inputMode == "english") {
            candidateCoordinator.updateEnglishCandidatesDebounced()
            return
        }

        // NextWord: re-predict from remaining text
        val textBeforeCursor = ic.getTextBeforeCursor(100, 0)?.toString() ?: ""
        smartbarManager.handleBackspaceForNextWord(textBeforeCursor)
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
            // 中文: Model B — clearCandidates 移到 commit 之前,讓引擎終端 NextWord
            // 中文: 預測存活(Enter 保留預測);引擎 effect 為關聯唯一來源。
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

            if (prefs.isAutoSpaceEnabled && !prefs.isTranslateSwapped) {
                if (!committedText.endsWith("-")) {
                    ic.commitText(" ", 1)
                }
            }
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
                candidateCoordinator.scheduleDisplayDerivation()
                candidateCoordinator.updateTaigiCandidatesDebounced()
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
            // 中文: Model B — 引擎 commit 已記關聯(唯一來源);Space 維持 commit 後
            // 中文: clearCandidates 抑制下詞顯示(舊手動呼叫會雙記並重新顯示)。
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
        var char = CaseTransformBridge.transformInputCase(
            text = baseText,
            letterCase = CaseTransformBridge.LetterCase.from(caps = caps, capsLock = capsLock),
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
            candidateCoordinator.updateEnglishCandidatesDebounced()
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

        // 組字字元（字母、TPS 符號、連字符號、˙）→ 進入組字
        if (isComposingCharacter(char)) {
            if (manager.isComposing()) {
                if (char == "-") {
                    manager.appendHyphen(ic)
                } else {
                    manager.appendCharacter(char, ic)
                }
                if (prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
                candidateCoordinator.scheduleDisplayDerivation()
                logger.debug("PERF") {
                    "[1] handleTaigiInput composing: ${System.currentTimeMillis() - inputStart}ms"
                }
                candidateCoordinator.updateTaigiCandidatesDebounced()
            } else {
                if (char == "-" && smartbarManager.isShowingNextWordCandidates()) {
                    ic.commitText("-", 1)
                    logger.debug(TAG) { "[INPUT] '-' committed in NextWord mode, keeping suggestions" }
                } else {
                    manager.startComposing(char, ic)
                    if (prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
                    candidateCoordinator.scheduleDisplayDerivation()
                    logger.debug("PERF") {
                        "[1] handleTaigiInput newComposing: ${System.currentTimeMillis() - inputStart}ms"
                    }
                    candidateCoordinator.updateTaigiCandidatesDebounced()
                }
            }
        } else if (manager.isComposing() && char.length == 1 && char[0].isDigit()) {
            // 組字中輸入數字 → 作為聲調標記追加
            manager.appendCharacter(char, ic)
            if (prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
            candidateCoordinator.scheduleDisplayDerivation()
            logger.debug("PERF") {
                "[1] handleTaigiInput composing digit: ${System.currentTimeMillis() - inputStart}ms"
            }
            candidateCoordinator.updateTaigiCandidatesDebounced()
        } else {
            // 非組字字元（標點、符號、箭頭等）→ 確認組字後直接輸出
            if (manager.isComposing()) {
                manager.commitComposition(ic)
            }
            ic.commitText(char, 1)
            smartbarManager.clearCandidates()
            return
        }
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
