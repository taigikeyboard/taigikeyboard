
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

    // 台語組字管理器
    private var composingManager: ComposingManager? = null

    // 英文自動補全服務
    private var englishAutocompleteService: com.siansiansu.taigikeyboard.ime.text.composing.EnglishAutocompleteService? = null

    // Cached TaigiAutocompleteService — reused across keystrokes, recreated only when inputMode changes
    private var taigiAutocompleteService: com.siansiansu.taigikeyboard.ime.text.composing.TaigiAutocompleteService? = null
    private var cachedInputMode: ToneConverterModels.InputMode? = null

    /**
     * 取得台語組字管理器（供 SmartbarManager 使用）
     */
    fun getComposingManager(): ComposingManager? = composingManager

    // Caps/Space related properties
    // 這些狀態需要被 SmartbarManager 讀取，用於候選詞大小寫轉換
    var caps: Boolean = false
        private set
    var capsLock: Boolean = false
        private set

    /**
     * 取得當前大小寫狀態（供 SmartbarManager 使用）
     * @return Pair(caps, capsLock)
     */
    fun getCapsState(): Pair<Boolean, Boolean> = Pair(caps, capsLock)
    private var cursorCapsMode: CapsMode = CapsMode.NONE
    private var editorCapsMode: CapsMode = CapsMode.NONE
    private var hasCapsRecentlyChanged: Boolean = false
    private var hasSpaceRecentlyPressed: Boolean = false

    // Composing related properties (舊英文輸入法的遺留變數，保留以免破壞其他功能)
    private var isComposingEnabled: Boolean = false
    private var isTextSelected: Boolean = false

    // 候選詞更新 Job（用於取消機制）
    private var candidateUpdateJob: Job? = null
    private var englishCandidateUpdateJob: Job? = null
    // Display derivation Job (runs deriveDisplay off main thread)
    private var displayDerivationJob: Job? = null

    companion object {
        private const val TAG = "TextInputManager"
        private const val CANDIDATE_DEBOUNCE_MS = 50L  // Debounce 延遲時間（毫秒）
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
     * Only builds the active keyboard mode; other modes are built on-demand.
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

                // Register LayoutSelectionOverlayView
                val layoutOverlay = inputView.findViewById<com.siansiansu.taigikeyboard.ime.text.smartbar.LayoutSelectionOverlayView>(
                    R.id.layout_selection_overlay
                )
                smartbarManager.registerLayoutSelectionOverlayView(layoutOverlay)

                // 測量鍵盤高度並通知 SmartbarManager
                // 使用 post 確保在 layout 完成後測量
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

    /**
     * Cancels all coroutines and cleans up.
     */
    override fun onDestroy() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onDestroy()")

        // 取消候選詞更新 Job
        candidateUpdateJob?.cancel()
        candidateUpdateJob = null
        englishCandidateUpdateJob?.cancel()
        englishCandidateUpdateJob = null
        displayDerivationJob?.cancel()
        displayDerivationJob = null

        cancel()
        osHandler.removeCallbacksAndMessages(null)
        smartbarManager.onDestroy()

        // 關閉英文自動補全服務
        englishAutocompleteService?.close()
        englishAutocompleteService = null

        // Clear cached TaigiAutocompleteService
        taigiAutocompleteService = null
        cachedInputMode = null

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
            composingManager = ComposingManager(
                inputMode = inputMode,
                enableDoubleTapOO = taigikeyboard.prefs.enableDoubleTapOO,
                enableDoubleTapNN = taigikeyboard.prefs.enableDoubleTapNN,
            )
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
        // 取消進行中的候選詞更新
        candidateUpdateJob?.cancel()
        candidateUpdateJob = null
        englishCandidateUpdateJob?.cancel()
        englishCandidateUpdateJob = null
        displayDerivationJob?.cancel()
        displayDerivationJob = null

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
     * Builds the KeyboardView on-demand if it hasn't been created yet.
     */
    private fun setActiveKeyboardMode(mode: KeyboardMode) {
        // CLIPBOARD 功能暫時移除，切換到 CHARACTERS 模式
        val actualMode = if (mode == KeyboardMode.CLIPBOARD) {
            KeyboardMode.CHARACTERS
        } else {
            mode
        }

        if (keyboardViews.containsKey(actualMode)) {
            switchToKeyboardView(actualMode)
        } else {
            // Build on-demand: keep current view visible while loading
            activeKeyboardMode = actualMode
            launch(Dispatchers.Default) {
                addKeyboardView(actualMode)
                withContext(Dispatchers.Main) {
                    switchToKeyboardView(actualMode)
                }
            }
        }
    }

    /**
     * Switches the ViewFlipper to show the specified keyboard mode.
     */
    private fun switchToKeyboardView(mode: KeyboardMode) {
        textViewFlipper?.displayedChild =
            textViewFlipper?.indexOfChild(keyboardViews[mode]) ?: 0
        keyboardViews[mode]?.updateVisibility()
        keyboardViews[mode]?.requestLayout()
        keyboardViews[mode]?.requestLayoutAllKeys()

        // 鍵盤切換後重新測量高度（使用 post 確保 layout 完成）
        textViewGroup?.post {
            measureAndUpdateKeyboardHeight()
        }

        activeKeyboardMode = mode
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

        // Sync composingManager.inputMode so segmentation uses the correct trie
        val newMode = when (newInputMode) {
            "poj" -> ToneConverterModels.InputMode.POJ
            "tl" -> ToneConverterModels.InputMode.TL
            else -> ToneConverterModels.InputMode.POJ
        }
        composingManager?.let { manager ->
            manager.inputMode = newMode
            manager.enableDoubleTapOO = taigikeyboard.prefs.enableDoubleTapOO
            manager.enableDoubleTapNN = taigikeyboard.prefs.enableDoubleTapNN
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
            scheduleDisplayDerivation()
            // 更新候選詞（使用 debounce 機制）
            updateTaigiCandidatesDebounced()
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

        // English mode: 退格後更新英文候選詞
        if (taigikeyboard.prefs.inputMode == "english") {
            updateEnglishCandidatesDebounced()
            return
        }

        // NextWord: 退格後根據剩餘文字重新預測
        val textBeforeCursor = ic.getTextBeforeCursor(100, 0)?.toString() ?: ""
        smartbarManager.handleBackspaceForNextWord(textBeforeCursor)
    }

    /**
     * Handles a [KeyCode.ENTER] event.
     */
    private fun handleEnter() {
        val ic = taigikeyboard.currentInputConnection ?: return

        // 如果正在台語組字，先確認組字
        if (composingManager?.isComposing() == true) {
            // Capture rawInput and composingText before commitComposition clears them
            val committedText = composingManager?.getComposingText() ?: ""
            val capturedRawInput = composingManager?.getRawInput() ?: ""
            composingManager?.commitComposition(ic)

            // 清除候選詞
            smartbarManager.clearCandidates()

            // 檢查是否有 IME action（如搜尋、傳送等）
            val imeOptions = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0
            val maskedAction = imeOptions and EditorInfo.IME_MASK_ACTION

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[ENTER] composing mode, imeOptions=$imeOptions, maskedAction=$maskedAction")
            }

            // 如果有特定的 IME action，執行該 action（不加空白）
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
                // 執行 IME action（如搜尋）- 傳入 maskedAction 而非完整 imeOptions
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[ENTER] performing action: $maskedAction")
                }
                ic.performEditorAction(maskedAction)
                return
            }

            // 沒有特定 IME action 時：羅馬字模式加空白
            if (taigikeyboard.prefs.isAutoSpaceEnabled && !taigikeyboard.prefs.isTranslateSwapped) {
                // 檢查字尾是否為連字符
                if (!committedText.endsWith("-")) {
                    ic.commitText(" ", 1)
                }
            }

            // 羅馬字模式（isTranslateSwapped=false）：記錄羅馬字到 NextWord
            // 漢字模式（isTranslateSwapped=true）：不記錄，因為組字不會產生漢字
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

        // 參考 FlorisBoard: 不使用 beginBatchEdit/endBatchEdit
        resetComposingText()
        val imeOptions = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0
        val maskedAction = imeOptions and EditorInfo.IME_MASK_ACTION

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[ENTER] non-composing mode, imeOptions=$imeOptions, maskedAction=$maskedAction")
        }

        // 參考 FlorisBoard handleEnter() 邏輯
        if (imeOptions and EditorInfo.IME_FLAG_NO_ENTER_ACTION > 0) {
            // flagNoEnterAction: 發送換行
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
                    // 直接執行 IME action，不發送 KeyEvent
                    ic.performEditorAction(maskedAction)
                }
                else -> {
                    // 其他情況：發送換行
                    ic.commitText("\n", 1)
                }
            }
        }
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

        // English mode: 直接輸出空白，清除候選詞
        if (taigikeyboard.prefs.inputMode == "english") {
            ic.commitText(" ", 1)
            smartbarManager.clearCandidates()
            return
        }

        // 如果正在台語組字，確認組字 + 插入空白
        if (composingManager?.isComposing() == true) {
            // Capture rawInput and composingText before commitComposition clears them
            val committedText = composingManager?.getComposingText() ?: ""
            val capturedRawInput = composingManager?.getRawInput() ?: ""
            composingManager?.commitComposition(ic)
            ic.commitText(" ", 1)
            smartbarManager.clearCandidates()

            // Feed phrase buffer with captured rawInput, then update NextWord context
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
            if (hasSpaceRecentlyPressed) {
                osHandler.removeCallbacksAndMessages(null)
                val text = ic.getTextBeforeCursor(2, 0) ?: ""
                if (text.length == 2 && !text.matches(DOUBLE_SPACE_PERIOD_REGEX)) {
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
                                    smartbarManager.clearCandidates()
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
        val inputStart = System.currentTimeMillis()
        val ic = taigikeyboard.currentInputConnection ?: return

        val baseText = if (keyData.label.isNotEmpty() &&
            keyData.label != keyData.code.toChar().toString()) {
            keyData.label
        } else {
            keyData.code.toChar().toString()
        }

        // 決定字元大小寫：尊重當前 caps 狀態（包含自動大寫與手動 shift）
        // 使用對照表正確轉換聲調字母（如 á → Á）
        val inputMode = when (taigikeyboard.prefs.inputMode) {
            "poj" -> ToneConverterModels.InputMode.POJ
            "tl" -> ToneConverterModels.InputMode.TL
            else -> ToneConverterModels.InputMode.POJ
        }
        val char = when {
            capsLock -> ToneUtilities.fullUppercaseToneLetter(baseText, inputMode)
            caps -> ToneUtilities.uppercaseToneLetter(baseText, inputMode)
            else -> ToneUtilities.lowercaseToneLetter(baseText, inputMode)
        }

        // English mode：直接輸出字元，不進入組字邏輯
        if (taigikeyboard.prefs.inputMode == "english") {
            ic.commitText(char, 1)
            // 處理單次 Shift 復位（Caps Lock 除外）
            if (caps && !capsLock) {
                caps = false
                updateCapsState()
            }
            // 更新英文候選詞（使用 debounce 機制）
            updateEnglishCandidatesDebounced()
            return
        }

        val manager = composingManager ?: return

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
            smartbarManager.collapseToolbarIfOpen()
            scheduleDisplayDerivation()
            // 更新候選詞（使用 debounce 機制）
            if (BuildConfig.DEBUG) Log.d("PERF", "[1] handleTaigiInput composing: ${System.currentTimeMillis() - inputStart}ms")
            updateTaigiCandidatesDebounced()
        } else {
            // 非組字模式：檢查是否正在顯示 NextWord 候選詞
            if (char == "-" && smartbarManager.isShowingNextWordCandidates()) {
                // NextWord 模式下輸入 "-"：直接輸出，保留 NextWord 候選詞
                // 用戶可以繼續點選 NextWord，或輸入其他字開始組字
                ic.commitText("-", 1)
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[INPUT] '-' committed in NextWord mode, keeping suggestions")
                }
            } else {
                // 開始新組字（包含 "-" 開頭的組字）
                manager.startComposing(char, ic)
                smartbarManager.collapseToolbarIfOpen()
                scheduleDisplayDerivation()
                // 更新候選詞（使用 debounce 機制）
                if (BuildConfig.DEBUG) Log.d("PERF", "[1] handleTaigiInput newComposing: ${System.currentTimeMillis() - inputStart}ms")
                updateTaigiCandidatesDebounced()
            }
        }
    }

    /**
     * 帶 debounce 和取消機制的台語候選詞更新
     * - 取消前一個未完成的更新任務
     * - 等待 debounce 時間後才執行搜尋
     * - 避免快速連續輸入時的 Race Condition
     */
    private fun updateTaigiCandidatesDebounced() {
        candidateUpdateJob?.cancel()

        candidateUpdateJob = launch {
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
     * Cancels any pending derivation. The derived display is applied on the main thread
     * once computation completes. A stale-input check prevents applying outdated results.
     */
    private fun scheduleDisplayDerivation() {
        displayDerivationJob?.cancel()
        val manager = composingManager ?: return
        val raw = manager.getRawInput() ?: return

        displayDerivationJob = launch {
            val derived = withContext(Dispatchers.Default) {
                manager.deriveDisplay(raw)
            }
            // Back on Main thread — apply only if input hasn't changed
            val ic = taigikeyboard.currentInputConnection ?: return@launch
            if (manager.isComposing() && manager.getRawInput() == raw) {
                manager.applyDerivedDisplay(derived, ic)
            }
        }
    }

    /**
     * 帶 debounce 和取消機制的英文候選詞更新
     */
    private fun updateEnglishCandidatesDebounced() {
        if (BuildConfig.DEBUG) {
            Log.d("ENSPELL", "[1] updateEnglishCandidatesDebounced() called")
        }

        englishCandidateUpdateJob?.cancel()

        englishCandidateUpdateJob = launch {
            delay(CANDIDATE_DEBOUNCE_MS)

            if (!isActive) return@launch

            if (BuildConfig.DEBUG) {
                Log.d("ENSPELL", "[2] After debounce, calling updateEnglishCandidates()")
            }
            updateEnglishCandidates()
        }
    }

    /**
     * 更新台語候選詞
     */
    private suspend fun updateTaigiCandidates() {
        // DEBUG: 追蹤調用來源
        if (BuildConfig.DEBUG) {
            val stackTrace = Thread.currentThread().stackTrace
            val caller = stackTrace.getOrNull(3)?.methodName ?: "unknown"
            Log.d(TAG, "[DEBUG] updateTaigiCandidates() called from: $caller")
        }

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

        // Reuse TaigiAutocompleteService — only recreate when inputMode changes
        val inputMode = taigikeyboard.prefs.inputMode.let {
            when (it) {
                "poj" -> ToneConverterModels.InputMode.POJ
                "tl" -> ToneConverterModels.InputMode.TL
                else -> ToneConverterModels.InputMode.POJ
            }
        }

        if (taigiAutocompleteService == null || cachedInputMode != inputMode) {
            cachedInputMode = inputMode
            taigiAutocompleteService = com.siansiansu.taigikeyboard.ime.text.composing.TaigiAutocompleteService(
                taigikeyboard.context,
                inputMode,
                prefs = taigikeyboard.prefs
            )
        }

        val searchStart = System.currentTimeMillis()
        val suggestions = taigiAutocompleteService!!.autocomplete(rawInput, displayText, smartbarManager.getLastSelectedWord())
        if (BuildConfig.DEBUG) Log.d("PERF", "[3] autocomplete (${suggestions.size} results): ${System.currentTimeMillis() - searchStart}ms")

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[CANDIDATES] found ${suggestions.size} suggestions")
        }

        // 更新 SmartbarManager
        val uiStart = System.currentTimeMillis()
        withContext(Dispatchers.Main) {
            smartbarManager.updateCandidates(suggestions)
            if (BuildConfig.DEBUG) Log.d("PERF", "[4] updateCandidates UI: ${System.currentTimeMillis() - uiStart}ms")
        }
    }

    /**
     * 更新英文候選詞
     *
     * 使用 EnglishAutocompleteService 取得拼字建議
     * 只在 English mode 下呼叫
     */
    private suspend fun updateEnglishCandidates() {
        if (BuildConfig.DEBUG) {
            Log.d("ENSPELL", "[3] updateEnglishCandidates() called")
        }

        val ic = taigikeyboard.currentInputConnection ?: run {
            if (BuildConfig.DEBUG) Log.d("ENSPELL", "[3] inputConnection is null")
            return
        }

        // 取得游標前的文字
        val textBeforeCursor = ic.getTextBeforeCursor(100, 0)?.toString() ?: ""

        if (textBeforeCursor.isEmpty()) {
            if (BuildConfig.DEBUG) Log.d("ENSPELL", "[3] textBeforeCursor is empty")
            smartbarManager.clearCandidates()
            return
        }

        if (BuildConfig.DEBUG) {
            Log.d("ENSPELL", "[3] textBeforeCursor='$textBeforeCursor'")
        }

        // 初始化服務（懶載入）
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

            // 轉換為 TaigiWord 格式（複用現有 SmartbarManager）
            val words = suggestions.mapIndexed { index, suggestion ->
                com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord(
                    id = -100 - index,  // 負數 ID 表示英文建議
                    roman = suggestion.text,
                    hanzi = null,
                    lengthScore = null
                )
            }

            withContext(Dispatchers.Main) {
                if (words.isNotEmpty()) {
                    // 使用英文三欄式候選詞佈局
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
     * 檢查字元是否為標點符號（除了連字符）
     * 包含半形符號、全形符號及特殊符號
     * 與 iOS isPunctuationExceptHyphen() 同步
     */
    private fun isPunctuationExceptHyphen(char: String): Boolean {
        // 半形 ASCII 符號
        val halfwidthSymbols = ".,!?;:()[]{}\"'`~@#\$%^&*+=<>/\\|_"

        // 中文標點符號
        val chinesePunctuation = "、。，！？；：（）「」『』《》【】〈〉〔〕｛｝…⋯"

        // Curly quotes（與 iOS 同步）
        val curlyQuotes = "\u201C\u201D\u2018\u2019"  // " " ' '

        // 特殊符號
        val specialSymbols = "—«»※"

        // 貨幣符號
        val currencySymbols = "€£¥¢$"

        // 其他符號
        val otherSymbols = "•·°©®™℃"

        // 數學符號
        val mathSymbols = "±×÷≠≈∞√"

        // 全形符號
        val fullwidthSymbols = "＠＃＄＿＆－＋／＊～｀｜＾＝｛｝＼％［］"

        val allSymbols = halfwidthSymbols + chinesePunctuation + curlyQuotes +
                         specialSymbols + currencySymbols + otherSymbols +
                         mathSymbols + fullwidthSymbols
        return allSymbols.contains(char)
    }

    enum class CapsMode {
        ALL,
        NONE,
        SENTENCES,
        WORDS;
    }
}
