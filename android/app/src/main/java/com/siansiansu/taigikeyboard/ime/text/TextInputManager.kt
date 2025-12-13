
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
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels
import kotlinx.coroutines.*
import java.util.*

class TextInputManager private constructor() : CoroutineScope by MainScope(),
    TaigiKeyboard.EventListener {

    private val taigikeyboard = TaigiKeyboard.getInstance()

    private var activeKeyboardMode: KeyboardMode? = null
    private val keyboardViews = EnumMap<KeyboardMode, KeyboardView>(KeyboardMode::class.java)
    // CLIPBOARD 功能暫時移除
    // private var clipboardView: ClipboardView? = null
    private val osHandler = Handler(Looper.getMainLooper())
    private var textViewFlipper: ViewFlipper? = null
    var textViewGroup: android.view.ViewGroup? = null

    var keyVariation: KeyVariation = KeyVariation.NORMAL
    private val layoutManager: LayoutManager by lazy { LayoutManager(taigikeyboard, taigikeyboard.prefs) }
    lateinit var smartbarManager: SmartbarManager

    // 台語組字管理器
    private var composingManager: ComposingManager? = null

    /**
     * 取得台語組字管理器（供 SmartbarManager 使用）
     */
    fun getComposingManager(): ComposingManager? = composingManager

    // Caps/Space related properties
    var caps: Boolean = false
        private set
    var capsLock: Boolean = false
        private set
    private var cursorCapsMode: CapsMode = CapsMode.NONE
    private var editorCapsMode: CapsMode = CapsMode.NONE
    private var hasCapsRecentlyChanged: Boolean = false
    private var hasSpaceRecentlyPressed: Boolean = false

    // Composing related properties (舊英文輸入法的遺留變數，保留以免破壞其他功能)
    private var isComposingEnabled: Boolean = false
    private var isTextSelected: Boolean = false

    companion object {
        private const val TAG = "TextInputManager"
        private var instance: TextInputManager? = null

        @Synchronized
        fun getInstance(): TextInputManager {
            if (instance == null) {
                instance = TextInputManager()
            }
            return instance!!
        }
    }

    /**
     * Non-UI-related setup.
     */
    override fun onCreate() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onCreate()")

        smartbarManager = SmartbarManager.getInstance()
    }

    private suspend fun addKeyboardView(mode: KeyboardMode) {
        // CLIPBOARD mode uses a different view, handled separately
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

    /**
     * Sets up the newly registered input view.
     */
    override fun onRegisterInputView(inputView: InputView) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onRegisterInputView(inputView)")

        launch(Dispatchers.Default) {
            textViewGroup = inputView.findViewById(R.id.text_input)
            textViewFlipper = inputView.findViewById(R.id.text_input_view_flipper)

            // Register CandidateOverlayView
            withContext(Dispatchers.Main) {
                val overlayView = inputView.findViewById<com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateOverlayView>(
                    R.id.candidate_overlay
                )
                smartbarManager.registerCandidateOverlayView(overlayView)

                // 測量鍵盤高度並通知 SmartbarManager
                // 使用 post 確保在 layout 完成後測量
                textViewGroup?.post {
                    measureAndUpdateKeyboardHeight()
                }
            }

            // CLIPBOARD 功能暫時移除
            // withContext(Dispatchers.Main) {
            //     clipboardView = ClipboardView(taigikeyboard.context).apply {
            //         onPasteListener = { text ->
            //             pasteText(text)
            //             setActiveKeyboardMode(KeyboardMode.CHARACTERS)
            //         }
            //         onBackClickListener = {
            //             setActiveKeyboardMode(KeyboardMode.CHARACTERS)
            //         }
            //     }
            //     textViewFlipper?.addView(clipboardView)
            // }

            val activeKeyboardMode = getActiveKeyboardMode()
            addKeyboardView(activeKeyboardMode)
            withContext(Dispatchers.Main) {
                setActiveKeyboardMode(activeKeyboardMode)
            }
            for (mode in KeyboardMode.values()) {
                if (mode != activeKeyboardMode) {
                    addKeyboardView(mode)
                }
            }
        }
    }

    /**
     * Cancels all coroutines and cleans up.
     */
    override fun onDestroy() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onDestroy()")

        cancel()
        osHandler.removeCallbacksAndMessages(null)
        smartbarManager.onDestroy()
        instance = null
    }

    /**
     * Evaluates the [activeKeyboardMode], [keyVariation] and [isComposingEnabled] property values
     * when starting to interact with a input editor. Also resets the composing texts and sets the
     * initial caps mode accordingly.
     */
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

        // 初始化台語組字管理器
        if (isComposingEnabled && keyboardMode == KeyboardMode.CHARACTERS) {
            val inputMode = taigikeyboard.prefs.inputMode.let {
                when (it) {
                    "poj" -> ToneConverterModels.InputMode.POJ
                    "tl" -> ToneConverterModels.InputMode.TL
                    else -> ToneConverterModels.InputMode.POJ
                }
            }
            composingManager = ComposingManager(inputMode)
        } else {
            composingManager = null
        }

        updateCapsState()
        resetComposingText()
        setActiveKeyboardMode(keyboardMode)
        smartbarManager.onStartInputView(keyboardMode, isComposingEnabled)
    }

    /**
     * Handle stuff when finishing to interact with a input editor.
     */
    override fun onFinishInputView(finishingInput: Boolean) {
        smartbarManager.onFinishInputView()
    }

    override fun onWindowShown() {
        keyboardViews[KeyboardMode.CHARACTERS]?.updateVisibility()
    }

    /**
     * Gets [activeKeyboardMode].
     *
     * @return If null [KeyboardMode.CHARACTERS], else [activeKeyboardMode].
     */
    fun getActiveKeyboardMode(): KeyboardMode {
        return activeKeyboardMode ?: KeyboardMode.CHARACTERS
    }

    /**
     * 通知當前鍵盤的所有按鍵重繪
     */
    fun invalidateAllKeys() {
        keyboardViews[activeKeyboardMode]?.invalidateAllKeys()
    }

    /**
     * 只重繪指定 keyCode 的按鍵，避免不必要的全鍵盤刷新
     */
    fun invalidateKeysByCode(vararg keyCodes: Int) {
        keyboardViews[activeKeyboardMode]?.invalidateKeysByCode(*keyCodes)
    }

    /**
     * 測量並更新鍵盤總高度（smartbar + keyboard）
     * 通知 SmartbarManager 以便 overlay 使用正確高度
     */
    private fun measureAndUpdateKeyboardHeight() {
        val viewGroup = textViewGroup ?: return

        // 取得鍵盤內容區域的測量高度（不包含 overlay）
        val contentView = viewGroup.findViewById<android.view.View>(R.id.text_input_content)
        if (contentView?.measuredHeight ?: 0 > 0) {
            val height = contentView.measuredHeight
            smartbarManager.setKeyboardHeight(height)

            // if (BuildConfig.DEBUG) {
            //     Log.d(this::class.simpleName, "[HEIGHT] Measured keyboard height: $height")
            // }
        }
    }

    /**
     * Sets [activeKeyboardMode] and updates the [SmartbarManager.activeContainerId].
     */
    private fun setActiveKeyboardMode(mode: KeyboardMode) {
        // CLIPBOARD 功能暫時移除，切換到 CHARACTERS 模式
        val actualMode = if (mode == KeyboardMode.CLIPBOARD) {
            KeyboardMode.CHARACTERS
        } else {
            mode
        }

        textViewFlipper?.displayedChild =
            textViewFlipper?.indexOfChild(keyboardViews[actualMode]) ?: 0
        keyboardViews[actualMode]?.updateVisibility()
        keyboardViews[actualMode]?.requestLayout()
        keyboardViews[actualMode]?.requestLayoutAllKeys()

        // 鍵盤切換後重新測量高度（使用 post 確保 layout 完成）
        textViewGroup?.post {
            measureAndUpdateKeyboardHeight()
        }

        activeKeyboardMode = actualMode
        smartbarManager.activeContainerId = smartbarManager.getPreferredContainerId()
    }

    /**
     * Pastes text to the current input connection.
     */
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

        launch {
            val keyboardView = keyboardViews[KeyboardMode.CHARACTERS]
            keyboardView?.computedLayout = withContext(Dispatchers.IO) {
                layoutManager.fetchComputedLayout(KeyboardMode.CHARACTERS, taigikeyboard.activeSubtype)
            }
            keyboardView?.updateVisibility()
        }
    }

    override fun onPhahTaigiLayoutChanged(enabled: Boolean) {
        Log.i(this::class.simpleName, "onPhahTaigiLayoutChanged($enabled) - Reloading CHARACTERS layout")

        launch {
            // 等待 keyboardViews 初始化
            var retryCount = 0
            while (keyboardViews[KeyboardMode.CHARACTERS] == null && retryCount < 10) {
                Log.d(this::class.simpleName, "Waiting for keyboardView initialization... retry=$retryCount")
                kotlinx.coroutines.delay(100)
                retryCount++
            }

            val keyboardView = keyboardViews[KeyboardMode.CHARACTERS]
            Log.d(this::class.simpleName, "keyboardView for CHARACTERS: $keyboardView")

            if (keyboardView == null) {
                Log.e(this::class.simpleName, "ERROR: keyboardView is still null after retries! Cannot reload layout")
                return@launch
            }

            val newLayout = withContext(Dispatchers.IO) {
                layoutManager.fetchComputedLayout(KeyboardMode.CHARACTERS, taigikeyboard.activeSubtype)
            }
            Log.d(this::class.simpleName, "Fetched new layout: ${newLayout.name}")

            keyboardView.computedLayout = newLayout
            keyboardView.updateVisibility()
            Log.i(this::class.simpleName, "Layout reloaded after phahTaigiLayoutChanged")
        }
    }

    /**
     * 重新載入當前鍵盤模式的佈局
     * 用於切換全形/半形標點時更新 UI
     * 只更新當前顯示的鍵盤，其他模式會在切換到時自動載入
     */
    fun reloadCurrentLayout() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "reloadCurrentLayout()")

        val currentMode = activeKeyboardMode ?: return
        val keyboardView = keyboardViews[currentMode] ?: return

        launch {
            // 使用 SmartbarManager 的快取值，避免 DataStore 非同步讀取問題
            val isTranslateSwapped = smartbarManager.getCachedIsTranslateSwapped()

            // 只重新載入當前顯示的鍵盤
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

    /**
     * 重新載入所有鍵盤模式的佈局
     * 背景更新非當前模式，確保下次切換時使用正確的標點
     */
    fun reloadAllLayoutsInBackground() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "reloadAllLayoutsInBackground()")

        launch {
            // 使用 SmartbarManager 的快取值，避免 DataStore 非同步讀取問題
            val isTranslateSwapped = smartbarManager.getCachedIsTranslateSwapped()

            // 背景更新其他鍵盤模式
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

    /**
     * 處理游標更新事件
     *
     * 注意：台語組字由 ComposingManager 獨立管理，此處只更新 Caps 狀態
     */
    override fun onUpdateCursorAnchorInfo(cursorAnchorInfo: CursorAnchorInfo?) {
        cursorAnchorInfo ?: return

        // 更新文字選取狀態
        isTextSelected = cursorAnchorInfo.selectionEnd - cursorAnchorInfo.selectionStart != 0

        // 更新 Caps 狀態
        updateCapsState()
    }

    /**
     * 重置英文輸入法的組字狀態（舊程式碼遺留，用於非台語輸入模式）
     */
    private fun resetComposingText(notifyInputConnection: Boolean = true) {
        if (notifyInputConnection) {
            val ic = taigikeyboard.currentInputConnection
            ic?.finishComposingText()
        }
        // 舊的 composingText/composingTextStart 變數已移除
    }


    /**
     * Parses the [CapsMode] out of the given [flags].
     *
     * @param flags The input flags.
     * @return A [CapsMode] value.
     */
    private fun parseCapsModeFromFlags(flags: Int): CapsMode {
        return when {
            flags and InputType.TYPE_TEXT_FLAG_CAP_CHARACTERS > 0 -> {
                CapsMode.ALL
            }
            flags and InputType.TYPE_TEXT_FLAG_CAP_SENTENCES > 0 -> {
                CapsMode.SENTENCES
            }
            flags and InputType.TYPE_TEXT_FLAG_CAP_WORDS > 0 -> {
                CapsMode.WORDS
            }
            else -> {
                CapsMode.NONE
            }
        }
    }

    /**
     * Fetches the current cursor caps mode from the current input connection.
     *
     * @return The [CapsMode] according to the returned flags by the current input connection.
     */
    private fun fetchCurrentCursorCapsMode(): CapsMode {
        val ic = taigikeyboard.currentInputConnection
        val info = taigikeyboard.currentInputEditorInfo
        val capsFlags = ic?.getCursorCapsMode(info.inputType) ?: 0
        return parseCapsModeFromFlags(capsFlags)
    }

    /**
     * Updates the current caps state according to the [cursorCapsMode], while respecting
     * [capsLock] property and [autoCapitalizationEnabled] setting.
     */
    private fun updateCapsState() {
        cursorCapsMode = fetchCurrentCursorCapsMode()
        editorCapsMode = parseCapsModeFromFlags(taigikeyboard.currentInputEditorInfo.inputType)
        if (!capsLock) {
            // 檢查自動大寫設定
            caps = if (taigikeyboard.prefs.autoCapitalizationEnabled) {
                cursorCapsMode != CapsMode.NONE
            } else {
                // 關閉自動大寫時，強制小寫
                false
            }
            keyboardViews[activeKeyboardMode]?.invalidateAllKeys()
        }
    }

    /**
     * Handles a [KeyCode.DELETE] event.
     */
    private fun handleDelete() {
        val ic = taigikeyboard.currentInputConnection ?: return

        // 優先檢查台語組字管理器
        if (composingManager?.deleteBackward(ic) == true) {
            if (BuildConfig.DEBUG) {
                val rawInput = composingManager?.getRawInput()
                val composingText = composingManager?.getComposingText()
                Log.d(TAG, "[DELETE] deleteBackward=true, rawInput='$rawInput', composingText='$composingText'")
            }
            // 更新候選詞
            launch {
                updateTaigiCandidates()
            }
            return
        }
        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[DELETE] deleteBackward=false or composingManager=null")
        }

        // 一般退格處理
        ic.beginBatchEdit()
        resetComposingText()
        ic.sendKeyEvent(
            KeyEvent(
                KeyEvent.ACTION_DOWN,
                KeyEvent.KEYCODE_DEL
            )
        )
        ic.endBatchEdit()
    }

    /**
     * Handles a [KeyCode.ENTER] event.
     */
    private fun handleEnter() {
        val ic = taigikeyboard.currentInputConnection ?: return

        // 如果正在台語組字，確認組字
        if (composingManager?.isComposing() == true) {
            // 在確認之前先取得組字文字（確認後會清空）
            val committedText = composingManager?.getComposingText() ?: ""
            composingManager?.commitComposition(ic)

            // 羅馬字模式：確認候選詞後自動加空白（字尾非連字符時）
            if (taigikeyboard.prefs.autoSpaceEnabled && !taigikeyboard.prefs.isTranslateSwapped) {
                // 檢查字尾是否為連字符
                if (!committedText.endsWith("-")) {
                    ic.commitText(" ", 1)
                }
            }

            smartbarManager.clearCandidates()
            return
        }

        ic.beginBatchEdit()
        resetComposingText()
        val action = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0
        if (action and EditorInfo.IME_FLAG_NO_ENTER_ACTION > 0) {
            ic.sendKeyEvent(
                KeyEvent(
                    KeyEvent.ACTION_DOWN,
                    KeyEvent.KEYCODE_ENTER
                )
            )
        } else {
            when (action and EditorInfo.IME_MASK_ACTION) {
                EditorInfo.IME_ACTION_DONE,
                EditorInfo.IME_ACTION_GO,
                EditorInfo.IME_ACTION_NEXT,
                EditorInfo.IME_ACTION_PREVIOUS,
                EditorInfo.IME_ACTION_SEARCH,
                EditorInfo.IME_ACTION_SEND -> {
                    ic.performEditorAction(action)
                }
                else -> {
                    ic.sendKeyEvent(
                        KeyEvent(
                            KeyEvent.ACTION_DOWN,
                            KeyEvent.KEYCODE_ENTER
                        )
                    )
                }
            }
        }
        ic.endBatchEdit()
    }

    /**
     * Handles a [KeyCode.SHIFT] event.
     */
    private fun handleShift() {
        if (hasCapsRecentlyChanged) {
            osHandler.removeCallbacksAndMessages(null)
            caps = true
            capsLock = true
            hasCapsRecentlyChanged = false
        } else {
            caps = !caps
            capsLock = false
            hasCapsRecentlyChanged = true
            osHandler.postDelayed({
                hasCapsRecentlyChanged = false
            }, 300)
        }
        // 只重繪字母鍵和 Shift 鍵，不刷新整個鍵盤（效能優化）
        keyboardViews[activeKeyboardMode]?.invalidateCharacterKeys()
    }

    /**
     * Handles a [KeyCode.SPACE] event. Also handles the auto-correction of two space taps if
     * enabled by the user.
     */
    private fun handleSpace() {
        val ic = taigikeyboard.currentInputConnection ?: return

        // 如果正在台語組字，確認組字 + 插入空白
        if (composingManager?.isComposing() == true) {
            composingManager?.commitComposition(ic)
            ic.commitText(" ", 1)
            smartbarManager.clearCandidates()
            return
        }

        if (taigikeyboard.prefs.doubleSpacePeriod) {
            if (hasSpaceRecentlyPressed) {
                osHandler.removeCallbacksAndMessages(null)
                val text = ic.getTextBeforeCursor(2, 0) ?: ""
                if (text.length == 2 && !text.matches("""[.!?‽\s][\s]""".toRegex())) {
                    ic.deleteSurroundingText(1, 0)
                    ic.commitText(".", 1)
                }
                hasSpaceRecentlyPressed = false
            } else {
                hasSpaceRecentlyPressed = true
                osHandler.postDelayed({
                    hasSpaceRecentlyPressed = false
                }, 300)
            }
        }
        ic.commitText(KeyCode.SPACE.toChar().toString(), 1)
    }

    /**
     * Main logic point for sending a key press. Different actions may occur depending on the given
     * [KeyData]. This method handles all key press send events, which are text based. For media
     * input send events see MediaInputManager.
     *
     * @param keyData The [KeyData] object which should be sent.
     */
    fun sendKeyPress(keyData: KeyData) {
        val ic = taigikeyboard.currentInputConnection

        when (keyData.code) {
            KeyCode.DELETE -> handleDelete()
            KeyCode.ENTER -> handleEnter()
            KeyCode.LANGUAGE_SWITCH -> {
                // 短按：切換到下一個輸入法
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
            KeyCode.SWITCH_TO_CLIPBOARD_CONTEXT -> taigikeyboard.setActiveInput(R.id.clipboard_input)
            KeyCode.VIEW_CHARACTERS -> setActiveKeyboardMode(KeyboardMode.CHARACTERS)
            KeyCode.VIEW_NUMERIC -> setActiveKeyboardMode(KeyboardMode.NUMERIC)
            KeyCode.VIEW_NUMERIC_ADVANCED -> {
                // 在 symbol 鍵盤中，根據 isTranslateSwapped 狀態決定行為
                if (taigikeyboard.prefs.isTranslateSwapped && activeKeyboardMode == KeyboardMode.SYMBOLS) {
                    // 輸入「、」符號
                    ic?.beginBatchEdit()
                    ic?.commitText("、", 1)
                    ic?.endBatchEdit()
                } else {
                    // 預設行為：切換到數字鍵盤
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
                                // 台語組字由 handleSpace() 內部處理，不需要 resetComposingText()
                                handleSpace()
                            }
                            KeyCode.URI_COMPONENT_TLD -> {
                                // TLD 輸入前需確認台語組字
                                if (composingManager?.isComposing() == true) {
                                    composingManager?.commitComposition(ic)
                                }
                                val tld = when (caps) {
                                    true -> keyData.label.uppercase(Locale.getDefault())
                                    false -> keyData.label.lowercase(Locale.getDefault())
                                }
                                ic?.commitText(tld, 1)
                            }
                            else -> {
                                // 處理台語輸入（字母、數字、連字符）
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
     * 處理台語字元輸入
     */
    private fun handleTaigiInput(keyData: KeyData) {
        val ic = taigikeyboard.currentInputConnection ?: return
        val manager = composingManager ?: return

        val baseText = if (keyData.label.isNotEmpty() &&
            keyData.label != keyData.code.toChar().toString()) {
            keyData.label
        } else {
            keyData.code.toChar().toString()
        }

        // 決定字元大小寫：尊重當前 caps 狀態（包含自動大寫與手動 shift）
        val char = if (caps) {
            baseText.uppercase(Locale.getDefault())
        } else {
            baseText.lowercase(Locale.getDefault())
        }

        // 檢查是否為標點符號（除了連字符）
        if (isPunctuationExceptHyphen(char)) {
            if (manager.isComposing()) {
                manager.commitComposition(ic)
            }
            ic.commitText(char, 1)
            smartbarManager.clearCandidates()
            return
        }

        // 台語組字輸入（字母、數字、連字符）
        if (manager.isComposing()) {
            if (char == "-") {
                manager.appendHyphen(ic)
            } else {
                manager.appendCharacter(char, ic)
            }
        } else {
            manager.startComposing(char, ic)
        }

        // 更新候選詞
        launch {
            updateTaigiCandidates()
        }
    }

    /**
     * 更新台語候選詞
     */
    private suspend fun updateTaigiCandidates() {
        val manager = composingManager ?: run {
            if (BuildConfig.DEBUG) Log.d(TAG, "[CANDIDATES] composingManager=null, skip")
            return
        }

        // 取得原始輸入（用於 Trie 搜尋）和顯示文字（用於 UI）
        val rawInput = manager.getRawInput() ?: run {
            if (BuildConfig.DEBUG) Log.d(TAG, "[CANDIDATES] rawInput=null, clearCandidates")
            smartbarManager.clearCandidates()
            return
        }
        val displayText = manager.getComposingText() ?: rawInput

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[CANDIDATES] rawInput='$rawInput', displayText='$displayText'")
        }

        // 使用 TaigiAutocompleteService 搜尋候選詞
        val inputMode = taigikeyboard.prefs.inputMode.let {
            when (it) {
                "poj" -> ToneConverterModels.InputMode.POJ
                "tl" -> ToneConverterModels.InputMode.TL
                else -> ToneConverterModels.InputMode.POJ
            }
        }

        val autocompleteService = com.siansiansu.taigikeyboard.ime.text.composing.TaigiAutocompleteService(
            taigikeyboard.context,
            inputMode
        )

        val suggestions = autocompleteService.getSuggestions(rawInput, displayText)

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[CANDIDATES] found ${suggestions.size} suggestions")
        }

        // 更新 SmartbarManager
        withContext(Dispatchers.Main) {
            smartbarManager.updateCandidates(suggestions)
        }
    }

    /**
     * 檢查字元是否為標點符號（除了連字符）
     * 包含半形符號、全形符號及特殊符號
     */
    private fun isPunctuationExceptHyphen(char: String): Boolean {
        // 半形 ASCII 符號
        val halfwidthSymbols = ".,!?;:()[]{}\"'`~@#\$%^&*+=<>/\\|_"

        // 中文標點符號
        val chinesePunctuation = "、。，！？；：（）「」『』《》【】〈〉…"

        // 全形符號
        val fullwidthSymbols = "＠＃＄＿＆－＋／＊～｀｜＾＝｛｝＼％［］"

        // 特殊符號（數學、貨幣、商標等）
        val specialSymbols = "•√π÷×¶∆£¢€¥°©®™✓"

        val allSymbols = halfwidthSymbols + chinesePunctuation + fullwidthSymbols + specialSymbols
        return allSymbols.contains(char)
    }

    enum class CapsMode {
        ALL,
        NONE,
        SENTENCES,
        WORDS;
    }
}
