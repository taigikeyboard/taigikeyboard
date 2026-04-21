
package com.siansiansu.taigikeyboard.ime.text

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.text.InputType
import android.util.Log
import android.view.KeyEvent
import android.view.inputmethod.CursorAnchorInfo
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.ExtractedTextRequest
import android.view.inputmethod.InputMethodManager
import android.widget.LinearLayout
import android.widget.ViewFlipper
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.InputView
import com.siansiansu.taigikeyboard.ime.core.Subtype
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.dictionary.TPSConverter
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels
import com.siansiansu.taigikeyboard.ime.dictionary.ToneUtilities
import com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager
import com.siansiansu.taigikeyboard.ime.text.composing.clearHostComposingRegion
import com.siansiansu.taigikeyboard.ime.text.composing.hostReportsNoComposingRegion
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyType
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardView
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutManager
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarManager
import kotlinx.coroutines.*
import java.util.*

class TextInputManager(
    private val taigikeyboard: TaigiKeyboard,
    private val prefs: com.siansiansu.taigikeyboard.ime.core.PrefHelper,
) : CoroutineScope by MainScope(),
    TaigiKeyboard.EventListener {
    private var activeKeyboardMode: KeyboardMode? = null
    private val keyboardViews = EnumMap<KeyboardMode, KeyboardView>(KeyboardMode::class.java)
    private val osHandler = Handler(Looper.getMainLooper())
    private var textViewFlipper: ViewFlipper? = null
    var textViewGroup: android.view.ViewGroup? = null

    var keyVariation: KeyVariation = KeyVariation.NORMAL
    private val layoutManager: LayoutManager by lazy { LayoutManager(taigikeyboard, prefs) }

    // Assigned by TaigiKeyboard.onCreate immediately after the SmartbarManager
    // is constructed — A7 reverses the pre-existing lazy-lookup cycle.
    lateinit var smartbarManager: SmartbarManager

    // Cancels previous layout reload to prevent race conditions on rapid mode switches
    private var layoutReloadJob: Job? = null

    // Composing manager (synchronized access via composingLock)
    private val composingLock = Any()
    private var composingManager: ComposingManager? = null

    // Composing related properties
    private var isComposingEnabled: Boolean = false

    // --- Delegated handlers ---

    private val capsStateManager =
        CapsStateManager(
            taigikeyboard = taigikeyboard,
            onInvalidateAllKeys = { keyboardViews[activeKeyboardMode]?.invalidateAllKeys() },
            onInvalidateCharacterKeys = { keyboardViews[activeKeyboardMode]?.invalidateCharacterKeys() },
        )

    private lateinit var candidateCoordinator: CandidateUpdateCoordinator

    // --- Public delegation API (preserves original interface) ---

    val caps: Boolean get() = capsStateManager.caps
    val capsLock: Boolean get() = capsStateManager.capsLock

    fun getCapsState(): Pair<Boolean, Boolean> = capsStateManager.getCapsState()

    fun getComposingManager(): ComposingManager? = synchronized(composingLock) { composingManager }

    companion object {
        private const val TAG = "TextInputManager"
        private val DOUBLE_SPACE_PERIOD_REGEX = """[.!?‽\s][\s]""".toRegex()
    }

    override fun onCreate() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onCreate()")

        candidateCoordinator =
            CandidateUpdateCoordinator(
                scope = this,
                taigikeyboard = taigikeyboard,
                getComposingManager = { composingManager },
                smartbarManager = smartbarManager,
            )
    }

    private suspend fun addKeyboardView(mode: KeyboardMode) {
        if (mode == KeyboardMode.CLIPBOARD) {
            return
        }

        val keyboardView = KeyboardView(taigikeyboard.context)
        keyboardView.taigikeyboard = taigikeyboard
        keyboardView.smartbarManager = smartbarManager
        keyboardView.prefs = prefs
        keyboardView.computedLayout =
            withContext(Dispatchers.IO) {
                layoutManager.fetchComputedLayout(mode, taigikeyboard.activeSubtype)
            }
        keyboardViews[mode] = keyboardView
        withContext(Dispatchers.Main) {
            textViewFlipper?.addView(keyboardView)
        }
    }

    override fun onRegisterInputView(inputView: InputView) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onRegisterInputView(inputView)")

        launch(Dispatchers.Default) {
            textViewGroup = inputView.findViewById(R.id.text_input)
            textViewFlipper = inputView.findViewById(R.id.text_input_view_flipper)

            withContext(Dispatchers.Main) {
                val overlayView =
                    inputView.findViewById<com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateOverlayView>(
                        R.id.candidate_overlay,
                    )
                smartbarManager.registerCandidateOverlayView(overlayView)

                val layoutOverlay =
                    inputView.findViewById<com.siansiansu.taigikeyboard.ime.text.smartbar.LayoutSelectionOverlayView>(
                        R.id.layout_selection_overlay,
                    )
                smartbarManager.registerLayoutSelectionOverlayView(layoutOverlay)

                val symbolOverlay =
                    inputView.findViewById<com.siansiansu.taigikeyboard.ime.text.smartbar.SymbolSelectionOverlayView>(
                        R.id.symbol_selection_overlay,
                    )
                smartbarManager.registerSymbolSelectionOverlayView(symbolOverlay)

                val settingsOverlay =
                    inputView.findViewById<com.siansiansu.taigikeyboard.ime.text.smartbar.SettingsSelectionOverlayView>(
                        R.id.settings_selection_overlay,
                    )
                smartbarManager.registerSettingsSelectionOverlayView(settingsOverlay)

                textViewGroup?.post {
                    measureAndUpdateKeyboardHeight()
                }
            }

            val activeKeyboardMode = getActiveKeyboardMode()
            addKeyboardView(activeKeyboardMode)
            withContext(Dispatchers.Main) {
                switchToKeyboardView(activeKeyboardMode)
            }
        }
    }

    override fun onDestroy() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onDestroy()")

        candidateCoordinator.destroy()

        cancel()
        osHandler.removeCallbacksAndMessages(null)
        smartbarManager.onDestroy()
    }

    override fun onStartInputView(
        info: EditorInfo?,
        restarting: Boolean,
    ) {
        val keyboardMode =
            when (info) {
                null -> {
                    KeyboardMode.CHARACTERS
                }

                else -> {
                    when (info.inputType and InputType.TYPE_MASK_CLASS) {
                        InputType.TYPE_CLASS_NUMBER -> {
                            keyVariation = KeyVariation.NORMAL
                            KeyboardMode.NUMERIC
                        }

                        InputType.TYPE_CLASS_PHONE -> {
                            keyVariation = KeyVariation.NORMAL
                            KeyboardMode.PHONE
                        }

                        InputType.TYPE_CLASS_TEXT -> {
                            keyVariation =
                                when (info.inputType and InputType.TYPE_MASK_VARIATION) {
                                    InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS,
                                    InputType.TYPE_TEXT_VARIATION_WEB_EMAIL_ADDRESS,
                                    -> {
                                        KeyVariation.EMAIL_ADDRESS
                                    }

                                    InputType.TYPE_TEXT_VARIATION_PASSWORD,
                                    InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD,
                                    InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD,
                                    -> {
                                        KeyVariation.PASSWORD
                                    }

                                    InputType.TYPE_TEXT_VARIATION_URI -> {
                                        KeyVariation.URI
                                    }

                                    else -> {
                                        KeyVariation.NORMAL
                                    }
                                }
                            KeyboardMode.CHARACTERS
                        }

                        else -> {
                            keyVariation = KeyVariation.NORMAL
                            KeyboardMode.CHARACTERS
                        }
                    }
                }
            }
        isComposingEnabled =
            when (keyboardMode) {
                KeyboardMode.NUMERIC,
                KeyboardMode.PHONE,
                KeyboardMode.PHONE2,
                -> false

                else -> keyVariation != KeyVariation.PASSWORD
            }

        synchronized(composingLock) {
            composingManager =
                if (isComposingEnabled && keyboardMode == KeyboardMode.CHARACTERS) {
                    ComposingManager(settingsProvider = taigikeyboard.prefs)
                } else {
                    null
                }
        }

        capsStateManager.updateCapsState()
        resetComposingText()
        setActiveKeyboardMode(keyboardMode)
        smartbarManager.onStartInputView(keyboardMode, isComposingEnabled)
    }

    override fun onFinishInputView(finishingInput: Boolean) {
        candidateCoordinator.cancelAll()
        smartbarManager.onFinishInputView()
    }

    override fun onWindowShown() {
        keyboardViews[KeyboardMode.CHARACTERS]?.updateVisibility()
    }

    fun getActiveKeyboardMode(): KeyboardMode = activeKeyboardMode ?: KeyboardMode.CHARACTERS

    fun invalidateAllKeys() {
        keyboardViews[activeKeyboardMode]?.invalidateAllKeys()
    }

    fun invalidateKeysByCode(vararg keyCodes: Int) {
        keyboardViews[activeKeyboardMode]?.invalidateKeysByCode(*keyCodes)
    }

    private fun measureAndUpdateKeyboardHeight() {
        val viewGroup = textViewGroup ?: return
        val contentView = viewGroup.findViewById<android.view.View>(R.id.text_input_content)
        if (contentView?.measuredHeight ?: 0 > 0) {
            val height = contentView.measuredHeight
            smartbarManager.setKeyboardHeight(height)
        }
    }

    private fun setActiveKeyboardMode(mode: KeyboardMode) {
        val actualMode =
            if (mode == KeyboardMode.CLIPBOARD) {
                KeyboardMode.CHARACTERS
            } else {
                mode
            }

        if (keyboardViews.containsKey(actualMode)) {
            switchToKeyboardView(actualMode)
        } else {
            launch(Dispatchers.Default) {
                addKeyboardView(actualMode)
                withContext(Dispatchers.Main) {
                    switchToKeyboardView(actualMode)
                }
            }
        }
    }

    private fun switchToKeyboardView(mode: KeyboardMode) {
        val keyboardView = keyboardViews[mode] ?: return
        textViewFlipper?.displayedChild =
            textViewFlipper?.indexOfChild(keyboardView) ?: 0
        keyboardView.updateVisibility()
        keyboardView.requestLayout()
        keyboardView.requestLayoutAllKeys()

        textViewGroup?.post {
            measureAndUpdateKeyboardHeight()
        }

        activeKeyboardMode = mode
        smartbarManager.activeContainerId = smartbarManager.getPreferredContainerId()
    }

    override fun onSubtypeChanged(newSubtype: Subtype) {
        layoutReloadJob?.cancel()
        layoutReloadJob =
            launch {
                val keyboardView = keyboardViews[KeyboardMode.CHARACTERS]
                keyboardView?.computedLayout =
                    withContext(Dispatchers.IO) {
                        layoutManager.fetchComputedLayout(KeyboardMode.CHARACTERS, newSubtype)
                    }
                keyboardView?.updateVisibility()
            }
    }

    override fun onInputModeChanged(newInputMode: String) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onInputModeChanged($newInputMode)")

        // ComposingManager reads inputMode + toneToggles per dispatch
        // via EngineSettingsProvider.current (live read) — no direct
        // field mutation needed.

        layoutReloadJob?.cancel()
        layoutReloadJob =
            launch {
                val keyboardView = keyboardViews[KeyboardMode.CHARACTERS]
                keyboardView?.computedLayout =
                    withContext(Dispatchers.IO) {
                        layoutManager.fetchComputedLayout(
                            KeyboardMode.CHARACTERS,
                            taigikeyboard.activeSubtype,
                            overrideInputMode = newInputMode,
                        )
                    }
                keyboardView?.updateVisibility()
            }
    }

    override fun onKeyboardLayoutTypeChanged(newLayoutType: String) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onKeyboardLayoutTypeChanged($newLayoutType)")

        layoutReloadJob?.cancel()
        layoutReloadJob =
            launch {
                val keyboardView = keyboardViews[KeyboardMode.CHARACTERS] ?: return@launch
                keyboardView.computedLayout =
                    withContext(Dispatchers.IO) {
                        layoutManager.fetchComputedLayout(KeyboardMode.CHARACTERS, taigikeyboard.activeSubtype)
                    }
                keyboardView.updateVisibility()
            }
    }

    fun reloadCurrentLayout() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "reloadCurrentLayout()")

        val currentMode = activeKeyboardMode ?: return
        val keyboardView = keyboardViews[currentMode] ?: return

        layoutReloadJob?.cancel()
        layoutReloadJob =
            launch {
                val isTranslateSwapped = smartbarManager.getCachedIsTranslateSwapped()

                val newLayout =
                    withContext(Dispatchers.IO) {
                        layoutManager.fetchComputedLayout(currentMode, taigikeyboard.activeSubtype, isTranslateSwapped)
                    }

                withContext(Dispatchers.Main) {
                    keyboardView.computedLayout = newLayout
                    keyboardView.updateVisibility()
                    keyboardView.requestLayout()
                    keyboardView.requestLayoutAllKeys()
                }
            }
    }

    fun reloadAllLayoutsInBackground() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "reloadAllLayoutsInBackground()")

        launch {
            val isTranslateSwapped = smartbarManager.getCachedIsTranslateSwapped()

            for ((mode, keyboardView) in keyboardViews) {
                if (mode != activeKeyboardMode) {
                    val newLayout =
                        withContext(Dispatchers.IO) {
                            layoutManager.fetchComputedLayout(mode, taigikeyboard.activeSubtype, isTranslateSwapped)
                        }
                    withContext(Dispatchers.Main) {
                        keyboardView.computedLayout = newLayout
                    }
                }
            }
        }
    }

    override fun onUpdateCursorAnchorInfo(cursorAnchorInfo: CursorAnchorInfo?) {
        cursorAnchorInfo ?: return
        capsStateManager.updateCapsState()
    }

    override fun onUpdateSelection(
        oldSelStart: Int,
        oldSelEnd: Int,
        newSelStart: Int,
        newSelEnd: Int,
        candidatesStart: Int,
        candidatesEnd: Int,
    ) {
        // When the host editor reports no composing region (both
        // candidate offsets == -1, e.g. user taps to move the cursor or
        // changes the selection), sync internal ComposingManager state so
        // a later commit/reset does not re-insert stale preedit at the
        // new cursor. Closes the root cause of the commitComposition
        // fast/slow split in `composing-state-boundary.md` §11.10
        // divergence #3; pinned by
        // `INVARIANT_composing_external_region_clear_discards_state`.
        if (hostReportsNoComposingRegion(candidatesStart, candidatesEnd)) {
            composingManager?.onExternalComposingRegionCleared()
        }
    }

    // Clears any residual host composing region without committing its
    // content. Called at session-start + non-composing fallback paths
    // (DELETE / ENTER / NUMERIC-PHONE key) where a stale region could
    // otherwise be silently committed by a bare `finishComposingText()`.
    // Pinned by `INVARIANT_composing_clear_preedit_does_not_commit`
    // (`behavioral-invariants.md` §13).
    private fun resetComposingText(notifyInputConnection: Boolean = true) {
        if (notifyInputConnection) {
            clearHostComposingRegion(taigikeyboard.currentInputConnection)
        }
    }

    /**
     * Handles a [KeyCode.DELETE] event.
     */
    private fun handleDelete() {
        val ic = taigikeyboard.currentInputConnection ?: return

        if (composingManager?.deleteBackward(ic) == true) {
            if (BuildConfig.DEBUG) {
                val rawInput = composingManager?.getRawInput()
                val composingText = composingManager?.getComposingText()
                Log.d(TAG, "[DELETE] deleteBackward=true, rawInput='$rawInput', composingText='$composingText'")
            }
            candidateCoordinator.scheduleDisplayDerivation()
            candidateCoordinator.updateTaigiCandidatesDebounced()
            return
        }
        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[DELETE] deleteBackward=false or composingManager=null")
        }

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
        if (taigikeyboard.prefs.inputMode == "english") {
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
            val capturedRawInput = composingManager?.getRawInput() ?: ""
            composingManager?.commitComposition(ic)
            smartbarManager.clearCandidates()

            val imeOptions = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0
            val maskedAction = imeOptions and EditorInfo.IME_MASK_ACTION

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[ENTER] composing mode, imeOptions=$imeOptions, maskedAction=$maskedAction")
            }

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
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[ENTER] performing action: $maskedAction")
                }
                ic.performEditorAction(maskedAction)
                return
            }

            if (taigikeyboard.prefs.isAutoSpaceEnabled && !taigikeyboard.prefs.isTranslateSwapped) {
                if (!committedText.endsWith("-")) {
                    ic.commitText(" ", 1)
                }
            }

            if (!taigikeyboard.prefs.isTranslateSwapped && committedText.isNotEmpty()) {
                smartbarManager.handleNextWordPrediction(
                    displayText = committedText,
                    committedText = committedText,
                    roman = committedText,
                    rawInput = capturedRawInput,
                )
            }
            return
        }

        resetComposingText()
        val imeOptions = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0
        val maskedAction = imeOptions and EditorInfo.IME_MASK_ACTION

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[ENTER] non-composing mode, imeOptions=$imeOptions, maskedAction=$maskedAction")
        }

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
                    if (BuildConfig.DEBUG) {
                        Log.d(TAG, "[ENTER] performing action: $maskedAction")
                    }
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
        if (taigikeyboard.prefs.inputMode == "english") {
            ic.commitText(" ", 1)
            smartbarManager.clearCandidates()
            return
        }

        // TPS mode: space as tone 1/4 syllable boundary marker.
        // If the current syllable has no explicit tone mark, space adds a syllable
        // boundary and stays in composing mode (like Microsoft Zhuyin's space for tone 1).
        // If the syllable already has a tone mark or ends with space, fall through to commit.
        if (taigikeyboard.prefs.keyboardLayoutType == "tps" && composingManager?.isComposing() == true) {
            val rawInput = composingManager?.getRawInput() ?: ""
            val lastChar = rawInput.lastOrNull()
            if (lastChar != null && !TPSConverter.isTPSToneMark(lastChar) && lastChar != ' ') {
                composingManager?.appendCharacter(" ", ic)
                candidateCoordinator.scheduleDisplayDerivation()
                candidateCoordinator.updateTaigiCandidatesDebounced()
                return
            }
        }

        if (composingManager?.isComposing() == true) {
            val committedText = composingManager?.getComposingText() ?: ""
            val capturedRawInput = composingManager?.getRawInput() ?: ""
            composingManager?.commitComposition(ic)
            ic.commitText(" ", 1)
            smartbarManager.clearCandidates()

            if (committedText.isNotEmpty()) {
                smartbarManager.handleNextWordPrediction(
                    displayText = committedText,
                    committedText = committedText,
                    roman = committedText,
                    rawInput = capturedRawInput,
                )
            }
            return
        }

        if (taigikeyboard.prefs.doubleSpacePeriod) {
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
     * Main logic point for sending a key press.
     */
    fun sendKeyPress(keyData: KeyData) {
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
                setActiveKeyboardMode(KeyboardMode.CHARACTERS)
            }

            KeyCode.VIEW_NUMERIC -> {
                setActiveKeyboardMode(KeyboardMode.NUMERIC)
            }

            KeyCode.VIEW_NUMERIC_ADVANCED -> {
                if (taigikeyboard.prefs.isTranslateSwapped && activeKeyboardMode == KeyboardMode.SYMBOLS) {
                    ic?.beginBatchEdit()
                    ic?.commitText("、", 1)
                    ic?.endBatchEdit()
                } else {
                    setActiveKeyboardMode(KeyboardMode.NUMERIC_ADVANCED)
                }
            }

            KeyCode.VIEW_PHONE -> {
                setActiveKeyboardMode(KeyboardMode.PHONE)
            }

            KeyCode.VIEW_PHONE2 -> {
                setActiveKeyboardMode(KeyboardMode.PHONE2)
            }

            KeyCode.VIEW_SYMBOLS -> {
                setActiveKeyboardMode(KeyboardMode.SYMBOLS)
            }

            KeyCode.VIEW_SYMBOLS2 -> {
                setActiveKeyboardMode(KeyboardMode.SYMBOLS2)
            }

            KeyCode.VIEW_CLIPBOARD -> {
                setActiveKeyboardMode(KeyboardMode.CLIPBOARD)
            }

            KeyCode.TRANSLATE -> {
                smartbarManager.toggleTranslateSwapped()
            }

            else -> {
                ic?.beginBatchEdit()
                when (activeKeyboardMode) {
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
                                if (BuildConfig.DEBUG) {
                                    Log.e(
                                        this::class.simpleName,
                                        "sendKeyPress(keyData): Received unknown key: $keyData",
                                    )
                                }
                            }
                        }
                    }
                }
                ic?.endBatchEdit()
            }
        }
    }

    /**
     * Handle Taigi character input.
     */
    private fun handleTaigiInput(keyData: KeyData) {
        val inputStart = System.currentTimeMillis()
        val ic = taigikeyboard.currentInputConnection ?: return

        val baseText =
            if (keyData.label.isNotEmpty() &&
                keyData.label != keyData.code.toChar().toString()
            ) {
                keyData.label
            } else {
                keyData.code.toChar().toString()
            }

        val inputMode =
            when (taigikeyboard.prefs.inputMode) {
                "poj" -> ToneConverterModels.InputMode.POJ
                "tl", "tps" -> ToneConverterModels.InputMode.TL
                else -> ToneConverterModels.InputMode.POJ
            }
        var char =
            when {
                capsLock -> ToneUtilities.fullUppercaseToneLetter(baseText, inputMode)
                caps -> ToneUtilities.uppercaseToneLetter(baseText, inputMode)
                else -> ToneUtilities.lowercaseToneLetter(baseText, inputMode)
            }

        // TPS layout: context-aware character adjustments
        if (taigikeyboard.prefs.keyboardLayoutType == "tps") {
            val rawInput = composingManager?.getRawInput() ?: ""
            char = TPSConverter.adjustTPSInitialKey(char, rawInput)
            char = TPSConverter.adjustTPSNasalizedVowelKey(char, rawInput)
            // Syllabic nasal auto-correct: ㄇ+tone → ㆬ, ㄫ+tone → ㆭ
            val nasalReplacement = TPSConverter.syllabicNasalReplacement(char, rawInput.lastOrNull())
            if (nasalReplacement != null) {
                composingManager?.replaceLastCharacter(nasalReplacement, ic)
            }
            // Palatalization auto-correct: ㄗ/ㄘ/ㄙ/ㆡ + ㄧ/ㆪ → ㄐ/ㄑ/ㄒ/ㆢ
            val replacement = TPSConverter.palatalizationReplacement(char, rawInput.lastOrNull())
            if (replacement != null) {
                composingManager?.replaceLastCharacter(replacement, ic)
            }
        }

        // English mode: commit directly
        if (taigikeyboard.prefs.inputMode == "english") {
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
                if (taigikeyboard.prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
                candidateCoordinator.scheduleDisplayDerivation()
                if (BuildConfig.DEBUG) Log.d("PERF", "[1] handleTaigiInput composing: ${System.currentTimeMillis() - inputStart}ms")
                candidateCoordinator.updateTaigiCandidatesDebounced()
            } else {
                if (char == "-" && smartbarManager.isShowingNextWordCandidates()) {
                    ic.commitText("-", 1)
                    if (BuildConfig.DEBUG) {
                        Log.d(TAG, "[INPUT] '-' committed in NextWord mode, keeping suggestions")
                    }
                } else {
                    manager.startComposing(char, ic)
                    if (taigikeyboard.prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
                    candidateCoordinator.scheduleDisplayDerivation()
                    if (BuildConfig.DEBUG) {
                        Log.d(
                            "PERF",
                            "[1] handleTaigiInput newComposing: ${System.currentTimeMillis() - inputStart}ms",
                        )
                    }
                    candidateCoordinator.updateTaigiCandidatesDebounced()
                }
            }
        } else if (manager.isComposing() && char.length == 1 && char[0].isDigit()) {
            // 組字中輸入數字 → 作為聲調標記追加
            manager.appendCharacter(char, ic)
            if (taigikeyboard.prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
            candidateCoordinator.scheduleDisplayDerivation()
            if (BuildConfig.DEBUG) Log.d("PERF", "[1] handleTaigiInput composing digit: ${System.currentTimeMillis() - inputStart}ms")
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

    /**
     * Check if the character should enter composition mode (allowlist).
     *
     * Only romanization letters, TPS bopomofo, TPS tone marks,
     * hyphen (syllable boundary), and ˙ (U+02D9, TPS tone 8) enter composing.
     * Everything else (punctuation, symbols, arrows, emoji, etc.) commits directly.
     */
    private fun isComposingCharacter(char: String): Boolean {
        val first = char.firstOrNull() ?: return false
        // isLetter() covers: a-z, A-Z (Lu/Ll), TPS bopomofo ㄅ-ㆷ (Lo),
        // TPS tone marks ˋ ˊ ˇ ˆ (Lm).
        // Three TPS tone marks are Sk (Symbol, modifier), not caught by isLetter:
        //   ˪ (U+02EA, tone 3), ˫ (U+02EB, tone 7), ˙ (U+02D9, tone 8)
        return first.isLetter() || first == '-' ||
            first == '\u02EA' || first == '\u02EB' || first == '\u02D9'
    }
}
