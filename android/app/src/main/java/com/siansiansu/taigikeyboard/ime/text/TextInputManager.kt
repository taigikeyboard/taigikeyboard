
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
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.InputView
import com.siansiansu.taigikeyboard.ime.core.Subtype
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyType
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardView
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutManager
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarManager
import com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager
import com.siansiansu.taigikeyboard.ime.dictionary.TPSConverter
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels
import com.siansiansu.taigikeyboard.ime.dictionary.ToneUtilities
import kotlinx.coroutines.*
import java.util.*

class TextInputManager private constructor() : CoroutineScope by MainScope(),
    TaigiKeyboard.EventListener {

    private val taigikeyboard = TaigiKeyboard.getInstance()

    private var activeKeyboardMode: KeyboardMode? = null
    private val keyboardViews = EnumMap<KeyboardMode, KeyboardView>(KeyboardMode::class.java)
    private val osHandler = Handler(Looper.getMainLooper())
    private var textViewFlipper: ViewFlipper? = null
    var textViewGroup: android.view.ViewGroup? = null

    var keyVariation: KeyVariation = KeyVariation.NORMAL
    private val layoutManager: LayoutManager by lazy { LayoutManager(taigikeyboard, taigikeyboard.prefs) }
    lateinit var smartbarManager: SmartbarManager

    // Composing manager (synchronized access via composingLock)
    private val composingLock = Any()
    private var composingManager: ComposingManager? = null

    // Composing related properties
    private var isComposingEnabled: Boolean = false
    private var isTextSelected: Boolean = false

    // --- Delegated handlers ---

    private val capsStateManager = CapsStateManager(
        taigikeyboard = taigikeyboard,
        onInvalidateAllKeys = { keyboardViews[activeKeyboardMode]?.invalidateAllKeys() },
        onInvalidateCharacterKeys = { keyboardViews[activeKeyboardMode]?.invalidateCharacterKeys() }
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
        private var instance: TextInputManager? = null

        @Synchronized
        fun getInstance(): TextInputManager {
            if (instance == null) {
                instance = TextInputManager()
            }
            return instance!!
        }
    }

    override fun onCreate() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onCreate()")

        smartbarManager = SmartbarManager.getInstance()
        candidateCoordinator = CandidateUpdateCoordinator(
            scope = this,
            taigikeyboard = taigikeyboard,
            getComposingManager = { composingManager },
            smartbarManager = smartbarManager
        )
    }

    private suspend fun addKeyboardView(mode: KeyboardMode) {
        if (mode == KeyboardMode.CLIPBOARD) {
            return
        }

        val keyboardView = KeyboardView(taigikeyboard.context)
        keyboardView.taigikeyboard = taigikeyboard
        keyboardView.prefs = taigikeyboard.prefs
        keyboardView.computedLayout = withContext(Dispatchers.IO) {
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
                val overlayView = inputView.findViewById<com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateOverlayView>(
                    R.id.candidate_overlay
                )
                smartbarManager.registerCandidateOverlayView(overlayView)

                val layoutOverlay = inputView.findViewById<com.siansiansu.taigikeyboard.ime.text.smartbar.LayoutSelectionOverlayView>(
                    R.id.layout_selection_overlay
                )
                smartbarManager.registerLayoutSelectionOverlayView(layoutOverlay)

                val symbolOverlay = inputView.findViewById<com.siansiansu.taigikeyboard.ime.text.smartbar.SymbolSelectionOverlayView>(
                    R.id.symbol_selection_overlay
                )
                smartbarManager.registerSymbolSelectionOverlayView(symbolOverlay)

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

        instance = null
    }

    override fun onStartInputView(info: EditorInfo?, restarting: Boolean) {
        val keyboardMode = when (info) {
            null -> KeyboardMode.CHARACTERS
            else -> when (info.inputType and InputType.TYPE_MASK_CLASS) {
                InputType.TYPE_CLASS_NUMBER -> {
                    keyVariation = KeyVariation.NORMAL
                    KeyboardMode.NUMERIC
                }
                InputType.TYPE_CLASS_PHONE -> {
                    keyVariation = KeyVariation.NORMAL
                    KeyboardMode.PHONE
                }
                InputType.TYPE_CLASS_TEXT -> {
                    keyVariation = when (info.inputType and InputType.TYPE_MASK_VARIATION) {
                        InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS,
                        InputType.TYPE_TEXT_VARIATION_WEB_EMAIL_ADDRESS -> {
                            KeyVariation.EMAIL_ADDRESS
                        }
                        InputType.TYPE_TEXT_VARIATION_PASSWORD,
                        InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD,
                        InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD -> {
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
        isComposingEnabled = when (keyboardMode) {
            KeyboardMode.NUMERIC,
            KeyboardMode.PHONE,
            KeyboardMode.PHONE2 -> false
            else -> keyVariation != KeyVariation.PASSWORD
        }

        synchronized(composingLock) {
            if (isComposingEnabled && keyboardMode == KeyboardMode.CHARACTERS) {
                val inputMode = taigikeyboard.prefs.inputMode.let {
                    when (it) {
                        "poj" -> ToneConverterModels.InputMode.POJ
                        "tl", "tps" -> ToneConverterModels.InputMode.TL
                        else -> ToneConverterModels.InputMode.POJ
                    }
                }
                composingManager = ComposingManager(
                    inputMode = inputMode,
                    enableDoubleTapOO = taigikeyboard.prefs.enableDoubleTapOO,
                    enableDoubleTapNN = taigikeyboard.prefs.enableDoubleTapNN,
                )
            } else {
                composingManager = null
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

    fun getActiveKeyboardMode(): KeyboardMode {
        return activeKeyboardMode ?: KeyboardMode.CHARACTERS
    }

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
        val actualMode = if (mode == KeyboardMode.CLIPBOARD) {
            KeyboardMode.CHARACTERS
        } else {
            mode
        }

        if (keyboardViews.containsKey(actualMode)) {
            switchToKeyboardView(actualMode)
        } else {
            activeKeyboardMode = actualMode
            launch(Dispatchers.Default) {
                addKeyboardView(actualMode)
                withContext(Dispatchers.Main) {
                    switchToKeyboardView(actualMode)
                }
            }
        }
    }

    private fun switchToKeyboardView(mode: KeyboardMode) {
        textViewFlipper?.displayedChild =
            textViewFlipper?.indexOfChild(keyboardViews[mode]) ?: 0
        keyboardViews[mode]?.updateVisibility()
        keyboardViews[mode]?.requestLayout()
        keyboardViews[mode]?.requestLayoutAllKeys()

        textViewGroup?.post {
            measureAndUpdateKeyboardHeight()
        }

        activeKeyboardMode = mode
        smartbarManager.activeContainerId = smartbarManager.getPreferredContainerId()
    }

    private fun pasteText(text: String) {
        val ic = taigikeyboard.currentInputConnection
        ic?.commitText(text, 1)
    }

    override fun onSubtypeChanged(newSubtype: Subtype) {
        launch {
            val keyboardView = keyboardViews[KeyboardMode.CHARACTERS]
            keyboardView?.computedLayout = withContext(Dispatchers.IO) {
                layoutManager.fetchComputedLayout(KeyboardMode.CHARACTERS, newSubtype)
            }
            keyboardView?.updateVisibility()
        }
    }

    override fun onInputModeChanged(newInputMode: String) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onInputModeChanged($newInputMode)")

        val newMode = when (newInputMode) {
            "poj" -> ToneConverterModels.InputMode.POJ
            "tl", "tps" -> ToneConverterModels.InputMode.TL
            else -> ToneConverterModels.InputMode.POJ
        }
        synchronized(composingLock) {
            composingManager?.let { manager ->
                manager.inputMode = newMode
                manager.enableDoubleTapOO = taigikeyboard.prefs.enableDoubleTapOO
                manager.enableDoubleTapNN = taigikeyboard.prefs.enableDoubleTapNN
            }
        }

        launch {
            val keyboardView = keyboardViews[KeyboardMode.CHARACTERS]
            keyboardView?.computedLayout = withContext(Dispatchers.IO) {
                layoutManager.fetchComputedLayout(KeyboardMode.CHARACTERS, taigikeyboard.activeSubtype)
            }
            keyboardView?.updateVisibility()
        }
    }

    override fun onKeyboardLayoutTypeChanged(newLayoutType: String) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onKeyboardLayoutTypeChanged($newLayoutType)")

        launch {
            val keyboardView = keyboardViews[KeyboardMode.CHARACTERS] ?: return@launch
            keyboardView.computedLayout = withContext(Dispatchers.IO) {
                layoutManager.fetchComputedLayout(KeyboardMode.CHARACTERS, taigikeyboard.activeSubtype)
            }
            keyboardView.updateVisibility()
        }
    }

    fun reloadCurrentLayout() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "reloadCurrentLayout()")

        val currentMode = activeKeyboardMode ?: return
        val keyboardView = keyboardViews[currentMode] ?: return

        launch {
            val isTranslateSwapped = smartbarManager.getCachedIsTranslateSwapped()

            val newLayout = withContext(Dispatchers.IO) {
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
                    val newLayout = withContext(Dispatchers.IO) {
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
        isTextSelected = cursorAnchorInfo.selectionEnd - cursorAnchorInfo.selectionStart != 0
        capsStateManager.updateCapsState()
    }

    private fun resetComposingText(notifyInputConnection: Boolean = true) {
        if (notifyInputConnection) {
            val ic = taigikeyboard.currentInputConnection
            ic?.finishComposingText()
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
                KeyEvent.KEYCODE_DEL
            )
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
                maskedAction in listOf(
                    EditorInfo.IME_ACTION_DONE,
                    EditorInfo.IME_ACTION_GO,
                    EditorInfo.IME_ACTION_NEXT,
                    EditorInfo.IME_ACTION_PREVIOUS,
                    EditorInfo.IME_ACTION_SEARCH,
                    EditorInfo.IME_ACTION_SEND
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
                    rawInput = capturedRawInput
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
                EditorInfo.IME_ACTION_SEND -> {
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
                    rawInput = capturedRawInput
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
            KeyCode.DELETE -> handleDelete()
            KeyCode.ENTER -> handleEnter()
            KeyCode.LANGUAGE_SWITCH -> {
                taigikeyboard.switchToNextInputMethod()
            }
            KeyCode.SETTINGS -> taigikeyboard.launchSettings()
            KeyCode.SHIFT -> handleShift()
            KeyCode.SHOW_INPUT_METHOD_PICKER -> {
                val im =
                    taigikeyboard.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
                im.showInputMethodPicker()
            }
            KeyCode.SWITCH_TO_MEDIA_CONTEXT -> taigikeyboard.setActiveInput(R.id.media_input)
            KeyCode.SWITCH_TO_TEXT_CONTEXT -> taigikeyboard.setActiveInput(R.id.text_input)
            KeyCode.VIEW_CHARACTERS -> setActiveKeyboardMode(KeyboardMode.CHARACTERS)
            KeyCode.VIEW_NUMERIC -> setActiveKeyboardMode(KeyboardMode.NUMERIC)
            KeyCode.VIEW_NUMERIC_ADVANCED -> {
                if (taigikeyboard.prefs.isTranslateSwapped && activeKeyboardMode == KeyboardMode.SYMBOLS) {
                    ic?.beginBatchEdit()
                    ic?.commitText("、", 1)
                    ic?.endBatchEdit()
                } else {
                    setActiveKeyboardMode(KeyboardMode.NUMERIC_ADVANCED)
                }
            }
            KeyCode.VIEW_PHONE -> setActiveKeyboardMode(KeyboardMode.PHONE)
            KeyCode.VIEW_PHONE2 -> setActiveKeyboardMode(KeyboardMode.PHONE2)
            KeyCode.VIEW_SYMBOLS -> setActiveKeyboardMode(KeyboardMode.SYMBOLS)
            KeyCode.VIEW_SYMBOLS2 -> setActiveKeyboardMode(KeyboardMode.SYMBOLS2)
            KeyCode.VIEW_CLIPBOARD -> setActiveKeyboardMode(KeyboardMode.CLIPBOARD)
            KeyCode.TRANSLATE -> smartbarManager.toggleTranslateSwapped()
            else -> {
                ic?.beginBatchEdit()
                when (activeKeyboardMode) {
                    KeyboardMode.NUMERIC,
                    KeyboardMode.NUMERIC_ADVANCED,
                    KeyboardMode.PHONE,
                    KeyboardMode.PHONE2 -> {
                        resetComposingText()
                        when (keyData.type) {
                            KeyType.CHARACTER,
                            KeyType.NUMERIC -> {
                                val text = keyData.code.toChar().toString()
                                ic?.commitText(text, 1)
                            }
                            else -> when (keyData.code) {
                                KeyCode.PHONE_PAUSE,
                                KeyCode.PHONE_WAIT -> {
                                    val text = keyData.code.toChar().toString()
                                    ic?.commitText(text, 1)
                                }
                            }
                        }
                    }
                    else -> when (keyData.type) {
                        KeyType.CHARACTER -> when (keyData.code) {
                            KeyCode.SPACE -> {
                                handleSpace()
                            }
                            KeyCode.URI_COMPONENT_TLD -> {
                                if (composingManager?.isComposing() == true) {
                                    composingManager?.commitComposition(ic)
                                    smartbarManager.clearCandidates()
                                }
                                val tld = when (caps) {
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
                        else -> {
                            Log.e(
                                this::class.simpleName,
                                "sendKeyPress(keyData): Received unknown key: $keyData"
                            )
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

        val baseText = if (keyData.label.isNotEmpty() &&
            keyData.label != keyData.code.toChar().toString()) {
            keyData.label
        } else {
            keyData.code.toChar().toString()
        }

        val inputMode = when (taigikeyboard.prefs.inputMode) {
            "poj" -> ToneConverterModels.InputMode.POJ
            "tl", "tps" -> ToneConverterModels.InputMode.TL
            else -> ToneConverterModels.InputMode.POJ
        }
        var char = when {
            capsLock -> ToneUtilities.fullUppercaseToneLetter(baseText, inputMode)
            caps -> ToneUtilities.uppercaseToneLetter(baseText, inputMode)
            else -> ToneUtilities.lowercaseToneLetter(baseText, inputMode)
        }

        // TPS layout: auto-select ㄇ/ㆬ and ㄫ/ㆭ/ㄥ based on composing context
        if (taigikeyboard.prefs.keyboardLayoutType == "tps") {
            char = TPSConverter.adjustTPSInitialKey(char, composingManager?.getRawInput() ?: "")
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

        // Punctuation check (except hyphen)
        if (isPunctuationExceptHyphen(char)) {
            if (manager.isComposing()) {
                manager.commitComposition(ic)
            }
            ic.commitText(char, 1)
            smartbarManager.clearCandidates()
            return
        }

        // Standalone digit: commit directly without entering composing mode.
        // Digits only enter composing as tone markers appended to existing romanization.
        if (!manager.isComposing() && char.length == 1 && char[0].isDigit()) {
            ic.commitText(char, 1)
            if (smartbarManager.isShowingNextWordCandidates()) {
                smartbarManager.clearCandidates()
            }
            return
        }

        // Taigi composing input
        if (manager.isComposing()) {
            if (char == "-") {
                manager.appendHyphen(ic)
            } else {
                manager.appendCharacter(char, ic)
            }
            smartbarManager.collapseToolbarIfOpen()
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
                smartbarManager.collapseToolbarIfOpen()
                candidateCoordinator.scheduleDisplayDerivation()
                if (BuildConfig.DEBUG) Log.d("PERF", "[1] handleTaigiInput newComposing: ${System.currentTimeMillis() - inputStart}ms")
                candidateCoordinator.updateTaigiCandidatesDebounced()
            }
        }
    }

    /**
     * Check if character is punctuation (except hyphen).
     */
    private fun isPunctuationExceptHyphen(char: String): Boolean {
        val halfwidthSymbols = ".,!?;:()[]{}\"'`~@#\$%^&*+=<>/\\|_"
        val chinesePunctuation = "、。，！？；：（）「」『』《》【】〈〉〔〕｛｝…⋯"
        val curlyQuotes = "\u201C\u201D\u2018\u2019"
        val specialSymbols = "—«»※"
        val currencySymbols = "€£¥¢$"
        val otherSymbols = "•·°©®™℃"
        val mathSymbols = "±×÷≠≈∞√"
        val fullwidthSymbols = "＠＃＄＿＆－＋／＊～｀｜＾＝｛｝＼％［］"

        val allSymbols = halfwidthSymbols + chinesePunctuation + curlyQuotes +
                         specialSymbols + currencySymbols + otherSymbols +
                         mathSymbols + fullwidthSymbols
        return allSymbols.contains(char)
    }
}
