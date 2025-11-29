package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.graphics.Color
import android.text.Spannable
import android.text.SpannableStringBuilder
import android.text.style.ForegroundColorSpan
import android.text.style.RelativeSizeSpan
import android.util.Log
import android.view.View
import android.widget.Button
import android.widget.ImageButton
import android.widget.LinearLayout
import androidx.core.view.children
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.text.TextInputManager
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

/**
 * Smartbar 管理器
 *
 * 負責管理 Smartbar 的狀態與候選詞顯示
 * 支援動態生成候選詞按鈕，最多顯示 100 個候選詞
 * 候選詞數量由 LexiconService 控制（預設 limit = 100）
 */
class SmartbarManager private constructor() :
    TaigiKeyboard.EventListener {

    private val taigikeyboard: TaigiKeyboard = TaigiKeyboard.getInstance()
    private var isComposingEnabled: Boolean = false
    private val textInputManager: TextInputManager = TextInputManager.getInstance()
    private val prefs: PrefHelper by lazy { PrefHelper(taigikeyboard.context) }
    var smartbarView: SmartbarView? = null
        private set
    var candidateOverlayView: CandidateOverlayView? = null
        private set

    var activeContainerId: Int = R.id.candidates
        set(value) { field = value; updateActiveContainerVisibility() }

    // 用於記錄使用者頻率的 Coroutine Scope
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    // 儲存當前候選詞列表（用於頻率記錄）
    private var currentSuggestions: List<TaigiWord> = emptyList()

    // 展開收合狀態
    private var isExpanded: Boolean = false
    private var hasCandidates: Boolean = false

    // 鍵盤總高度（smartbar + keyboard）
    private var keyboardHeight: Int = 0

    // isTranslateSwapped 本地快取（避免 DataStore 非同步寫入導致的時序問題）
    private var cachedIsTranslateSwapped: Boolean = false
    private var cachedOutputBothScripts: Boolean = false

    private val candidateViewOnClickListener = View.OnClickListener { v ->
        val button = v as Button
        val candidatesContainer = smartbarView?.candidatesView ?: return@OnClickListener
        val buttonIndex = candidatesContainer.indexOfChild(button)

        if (buttonIndex >= 0 && buttonIndex < currentSuggestions.size) {
            val selectedWord = currentSuggestions[buttonIndex]
            val ic = taigikeyboard.currentInputConnection ?: return@OnClickListener

            // 取得組字管理器
            val composingManager = taigikeyboard.textInputManager.getComposingManager()

            // 根據 isTranslateSwapped 和 outputBothScripts 決定要輸出的文字
            // showHanjiMode 固定為 true
            val textToCommit = when {
                // 漢羅攏出模式
                cachedOutputBothScripts && !selectedWord.hanzi.isNullOrEmpty() -> {
                    if (cachedIsTranslateSwapped) {
                        "${selectedWord.hanzi} (${selectedWord.roman})"
                    } else {
                        "${selectedWord.roman} (${selectedWord.hanzi})"
                    }
                }
                // 翻譯交換模式：顯示漢字
                cachedIsTranslateSwapped && !selectedWord.hanzi.isNullOrEmpty() -> selectedWord.hanzi
                // 預設顯示羅馬字
                else -> selectedWord.roman
            }

            // 選擇候選詞（使用 ComposingManager 處理狀態清除）
            composingManager?.selectSuggestion(textToCommit, ic)

            // 羅馬字模式或漢羅攏出模式：選擇候選詞後自動加空白
            if (prefs.autoSpaceEnabled && (!cachedIsTranslateSwapped || cachedOutputBothScripts)) {
                ic.commitText(" ", 1)
            }

            // 記錄使用頻率（非同步）
            scope.launch {
                UserFrequencyService.recordUsage(selectedWord.displayText)
            }

            // 清除候選詞顯示
            clearCandidates()
        }
    }
    private val candidateViewOnLongClickListener = View.OnLongClickListener { v ->
        true
    }
    private val numberRowButtonOnClickListener = View.OnClickListener { v ->
        val keyData = when (v.id) {
            R.id.number_row_0 -> KeyData(48, "0")
            R.id.number_row_1 -> KeyData(49, "1")
            R.id.number_row_2 -> KeyData(50, "2")
            R.id.number_row_3 -> KeyData(51, "3")
            R.id.number_row_4 -> KeyData(52, "4")
            R.id.number_row_5 -> KeyData(53, "5")
            R.id.number_row_6 -> KeyData(54, "6")
            R.id.number_row_7 -> KeyData(55, "7")
            R.id.number_row_8 -> KeyData(56, "8")
            R.id.number_row_9 -> KeyData(57, "9")
            else -> KeyData(0)
        }
        taigikeyboard.textInputManager.sendKeyPress(keyData)
    }
    private val quickActionOnClickListener = View.OnClickListener { v ->
        when (v.id) {
            R.id.quick_action_switch_to_media_context -> {
                activeContainerId = getPreferredContainerId()
                taigikeyboard.setActiveInput(R.id.media_input)
            }
            R.id.quick_action_switch_to_clipboard -> {
                activeContainerId = getPreferredContainerId()
                taigikeyboard.setActiveInput(R.id.clipboard_input)
            }
            R.id.quick_action_open_settings -> {
                // 開啟鍵盤設定頁面
                taigikeyboard.requestHideSelf(0)
                val intent = android.content.Intent(taigikeyboard.context, com.siansiansu.taigikeyboard.settings.KeyboardSettingsActivity::class.java)
                intent.flags = android.content.Intent.FLAG_ACTIVITY_NEW_TASK or
                              android.content.Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED or
                              android.content.Intent.FLAG_ACTIVITY_CLEAR_TOP
                taigikeyboard.context.startActivity(intent)
            }
            else -> return@OnClickListener
        }
    }
    // TODO: 暫時停用展開收合功能
    // private val quickActionToggleOnClickListener = View.OnClickListener {
    //     activeContainerId = when (activeContainerId) {
    //         R.id.quick_actions -> getPreferredContainerId()
    //         else -> R.id.quick_actions
    //     }
    // }

    companion object {
        private const val TAG = "SmartbarManager"
        private var instance: SmartbarManager? = null

        @Synchronized
        fun getInstance(): SmartbarManager {
            if (instance == null) {
                instance = SmartbarManager()
            }
            return instance!!
        }
    }

    fun registerSmartbarView(smartbarView: SmartbarView) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "registerSmartbarView(smartbarView)")

        this.smartbarView = smartbarView

        // TODO: 暫時停用展開收合功能
        // smartbarView.quickActionToggle?.setOnClickListener(quickActionToggleOnClickListener)
        val quickActions = smartbarView.findViewById<LinearLayout>(R.id.quick_actions)
        for (quickAction in quickActions.children) {
            if (quickAction is ImageButton) {
                quickAction.setOnClickListener(quickActionOnClickListener)
            }
        }
        val numberRow = smartbarView.findViewById<LinearLayout>(R.id.number_row)
        for (numberRowButton in numberRow.children) {
            if (numberRowButton is Button) {
                numberRowButton.setOnClickListener(numberRowButtonOnClickListener)
            }
        }
        // 候選詞按鈕事件監聽器將在動態建立按鈕時註冊

        // 展開收合按鈕點擊事件
        smartbarView.expandToggleButton?.setOnClickListener {
            toggleExpandState()
        }
    }

    /**
     * 註冊候選詞 Overlay 視圖
     */
    fun registerCandidateOverlayView(overlayView: CandidateOverlayView) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "registerCandidateOverlayView(overlayView)")

        this.candidateOverlayView = overlayView

        // 設定 overlay 回調
        overlayView.onCollapse = {
            collapseCandidateView()
        }

        overlayView.onSuggestionSelected = { word, index ->
            handleOverlaySuggestionSelected(word, index)
        }

        overlayView.onTranslateToggle = {
            toggleTranslateSwapped()
        }
    }

    override fun onDestroy() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onDestroy()")

        smartbarView = null
        instance = null
    }

    fun onStartInputView(keyboardMode: KeyboardMode, isComposingEnabled: Boolean) {
        this.isComposingEnabled = isComposingEnabled

        // 初始化快取
        cachedIsTranslateSwapped = prefs.isTranslateSwapped
        cachedOutputBothScripts = prefs.outputBothScripts

        when {
            keyboardMode == KeyboardMode.NUMERIC ||
            keyboardMode == KeyboardMode.PHONE ||
            keyboardMode == KeyboardMode.PHONE2 -> {
                smartbarView?.visibility = View.GONE
            }
            else -> {
                smartbarView?.visibility = View.VISIBLE
                // 初始狀態：顯示 quick actions（預設狀態）
                // 候選詞會在組字時由 updateCandidates() 自動切換顯示
                activeContainerId = when {
                    isComposingEnabled && hasCandidates -> R.id.candidates_container
                    else -> R.id.quick_actions  // 預設顯示 quick actions
                }
            }
        }
    }

    fun onFinishInputView() {
        clearCandidates()
    }

    /**
     * 更新候選詞顯示
     * 每次清空並重新建立按鈕，確保樣式一致
     */
    fun updateCandidates(suggestions: List<TaigiWord>) {
        val view = smartbarView ?: return
        val candidatesContainer = view.candidatesView ?: return

        if (suggestions.isEmpty()) {
            clearCandidates()
            return
        }

        // 儲存當前候選詞
        currentSuggestions = suggestions
        hasCandidates = true

        // 切換到候選詞視圖
        if (activeContainerId != R.id.candidates_container) {
            activeContainerId = R.id.candidates_container
        }

        // 重置候選詞列滑動位置（回到起點）
        view.resetCandidateScrollPosition()

        // 清空所有舊按鈕
        candidatesContainer.removeAllViews()

        // 計算共用數值
        val margin = taigikeyboard.context.resources.getDimensionPixelSize(
            R.dimen.smartbar_button_margin
        )
        val padding = taigikeyboard.context.resources.getDimensionPixelSize(
            R.dimen.smartbar_button_padding
        )
        val reducedVerticalPadding = padding / 2  // 減少上下 padding 至 50%

        // 為每個候選詞建立新按鈕
        suggestions.forEachIndexed { i, word ->
            val button = Button(taigikeyboard.context).apply {
                // 套用 SmartbarCandidate 樣式（文字樣式）
                setTextAppearance(R.style.SmartbarCandidate)
                // 套用背景與動畫
                // 第 0 個候選詞（當前組字）使用不同的背景，預設狀態有淡灰色提示
                setBackgroundResource(
                    if (i == 0) R.drawable.candidate_composing_background
                    else R.drawable.candidate_button_background
                )
                stateListAnimator = android.animation.AnimatorInflater.loadStateListAnimator(
                    taigikeyboard.context,
                    R.animator.candidate_button_scale
                )
                // 確保不全部大寫
                isAllCaps = false

                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.MATCH_PARENT
                ).apply {
                    // 增加水平間距，垂直間距增加以避免 button 接觸候選詞列上下邊緣
                    val horizontalSpacing = margin * 5  // 5dp（從 3dp 增加，讓候選詞不擠在一起）
                    val verticalSpacing = margin * 6  // 6dp（從 4dp 增加到 6dp）
                    setMargins(horizontalSpacing, verticalSpacing, horizontalSpacing, verticalSpacing)
                }
                // 設定按鈕尺寸限制
                // minWidth: 64dp（維持預設值，確保按鈕易於點擊）
                // minHeight: 48dp（符合 Android 觸控目標最小尺寸）
                val density = taigikeyboard.context.resources.displayMetrics.density
                val minButtonWidth = (64 * density).toInt()
                val minButtonHeight = (48 * density).toInt()

                minWidth = minButtonWidth
                minimumWidth = minButtonWidth
                minHeight = minButtonHeight
                minimumHeight = minButtonHeight
                setPadding(padding, reducedVerticalPadding, padding, reducedVerticalPadding)
                setOnClickListener(candidateViewOnClickListener)
                setOnLongClickListener(candidateViewOnLongClickListener)
            }

            // 顯示格式：根據 isTranslateSwapped 決定顯示內容
            // showHanjiMode 固定為 true
            // 規則：
            // 1. isTranslateSwapped = false: title = 羅馬字, subtitle = 漢字
            // 2. isTranslateSwapped = true: title = 漢字, subtitle = 羅馬字
            val displayText: CharSequence = when {
                // 沒有漢字：只顯示羅馬字
                word.hanzi.isNullOrEmpty() -> word.roman

                // 翻譯交換模式：漢字為主 (title)，羅馬字為副 (subtitle)
                cachedIsTranslateSwapped -> {
                    SpannableStringBuilder().apply {
                        // 漢字（title - 主要文字）
                        append(word.hanzi)
                        append(" ")

                        // 羅馬字（subtitle - 副標題，縮小至 70% 且使用較淡的顏色）
                        val subtitleStart = length
                        append(word.roman)

                        // 設定字體大小為主文字的 70%
                        setSpan(
                            RelativeSizeSpan(0.70f),
                            subtitleStart,
                            length,
                            Spannable.SPAN_EXCLUSIVE_EXCLUSIVE
                        )

                        // 從主題取得 subtitle 顏色
                        val subtitleColor = com.siansiansu.taigikeyboard.util.getColorFromAttr(
                            taigikeyboard.context,
                            com.siansiansu.taigikeyboard.R.attr.smartbar_candidate_subtitle_fgColor
                        )
                        setSpan(
                            ForegroundColorSpan(subtitleColor),
                            subtitleStart,
                            length,
                            Spannable.SPAN_EXCLUSIVE_EXCLUSIVE
                        )
                    }
                }

                // 預設模式：羅馬字為主 (title)，漢字為副 (subtitle)
                else -> {
                    SpannableStringBuilder().apply {
                        // 羅馬字（title - 主要文字）
                        append(word.roman)
                        append(" ")

                        // 漢字（subtitle - 副標題，縮小至 70% 且使用較淡的顏色）
                        val subtitleStart = length
                        append(word.hanzi)

                        // 設定字體大小為主文字的 70%
                        setSpan(
                            RelativeSizeSpan(0.70f),
                            subtitleStart,
                            length,
                            Spannable.SPAN_EXCLUSIVE_EXCLUSIVE
                        )

                        // 從主題取得 subtitle 顏色
                        val subtitleColor = com.siansiansu.taigikeyboard.util.getColorFromAttr(
                            taigikeyboard.context,
                            com.siansiansu.taigikeyboard.R.attr.smartbar_candidate_subtitle_fgColor
                        )
                        setSpan(
                            ForegroundColorSpan(subtitleColor),
                            subtitleStart,
                            length,
                            Spannable.SPAN_EXCLUSIVE_EXCLUSIVE
                        )
                    }
                }
            }

            button.text = displayText

            // 設定字體：根據 customFontEnabled 決定
            button.typeface = com.siansiansu.taigikeyboard.util.FontUtils.getKeyFont(
                customFontEnabled = prefs.customFontEnabled,
                context = taigikeyboard.context
            )

            candidatesContainer.addView(button)
        }

        // 更新展開按鈕可見性
        updateExpandButtonVisibility()
    }

    /**
     * 取得當前的翻譯模式狀態
     * 提供給其他元件使用，確保所有元件讀取同一份快取
     */
    fun getCachedIsTranslateSwapped(): Boolean {
        return cachedIsTranslateSwapped
    }

    /**
     * 切換翻譯模式（交換候選詞中的漢字與羅馬字順序）
     */
    fun toggleTranslateSwapped() {
        // 切換本地快取（立即生效，避免 DataStore 非同步寫入的延遲）
        cachedIsTranslateSwapped = !cachedIsTranslateSwapped

        // 同時更新 DataStore 設定（非同步，持久化）
        prefs.isTranslateSwapped = cachedIsTranslateSwapped

        // 同步更新 outputBothScripts 快取（確保一致性）
        cachedOutputBothScripts = prefs.outputBothScripts

        // 重新渲染當前候選詞以套用新的顯示順序
        if (currentSuggestions.isNotEmpty()) {
            updateCandidates(currentSuggestions)

            // 如果 overlay 正在顯示，也要更新 overlay 中的候選詞
            // 展開視圖只顯示建議候選詞（跳過第 0 個組字文字候選詞）
            if (isExpanded) {
                val suggestionsForOverlay = currentSuggestions.drop(1)
                candidateOverlayView?.updateSuggestions(suggestionsForOverlay)
            }
        }

        // 立即重新載入當前鍵盤佈局以套用新的標點符號（全形/半形）
        textInputManager.reloadCurrentLayout()

        // 背景更新其他鍵盤模式，確保下次切換時使用正確的標點
        textInputManager.reloadAllLayoutsInBackground()

        // 只重繪 TRANSLATE 和 VIEW_NUMERIC_ADVANCED 按鍵，避免刷新整個鍵盤（效能優化）
        textInputManager.invalidateKeysByCode(
            com.siansiansu.taigikeyboard.ime.text.key.KeyCode.TRANSLATE,
            com.siansiansu.taigikeyboard.ime.text.key.KeyCode.VIEW_NUMERIC_ADVANCED
        )

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[TRANSLATE] isTranslateSwapped 切換為: $cachedIsTranslateSwapped")
        }
    }

    /**
     * 清除候選詞顯示
     */
    fun clearCandidates() {
        currentSuggestions = emptyList()
        hasCandidates = false

        // 重置候選詞列滑動位置
        smartbarView?.resetCandidateScrollPosition()

        // 清空候選詞容器
        smartbarView?.candidatesView?.removeAllViews()

        // 候選詞清空後，切換至 quick actions
        if (activeContainerId == R.id.candidates_container) {
            activeContainerId = R.id.quick_actions
        }

        // 隱藏展開按鈕
        updateExpandButtonVisibility()

        // 收合展開視圖（如果已展開）
        if (isExpanded) {
            collapseCandidateView()
        }
    }

    /**
     * 切換展開/收合狀態
     */
    private fun toggleExpandState() {
        isExpanded = !isExpanded
        smartbarView?.setExpandButtonState(isExpanded)

        if (isExpanded) {
            expandCandidateView()
        } else {
            collapseCandidateView()
        }

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[EXPAND] State toggled: isExpanded=$isExpanded")
        }
    }

    /**
     * 更新展開按鈕可見性
     * 只在有候選詞時顯示
     */
    private fun updateExpandButtonVisibility() {
        smartbarView?.setExpandButtonVisible(hasCandidates)
    }

    /**
     * 設定鍵盤總高度
     * @param height 鍵盤總高度（smartbar + keyboard）
     */
    fun setKeyboardHeight(height: Int) {
        keyboardHeight = height
        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[HEIGHT] Keyboard height set to: $height")
        }
    }

    /**
     * 展開候選詞視圖
     */
    private fun expandCandidateView() {
        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[EXPAND] Expanding candidate view")
        }

        val overlay = candidateOverlayView
        if (overlay == null) {
            if (BuildConfig.DEBUG) {
                Log.w(TAG, "[EXPAND] Overlay view not registered")
            }
            return
        }

        // 展開視圖只顯示建議候選詞（跳過第 0 個組字文字候選詞）
        // 第 0 個位置是當前組字文字，從第 1 個位置開始才是建議候選詞
        val suggestionsForOverlay = if (currentSuggestions.isNotEmpty()) {
            currentSuggestions.drop(1)
        } else {
            currentSuggestions
        }

        // 顯示 overlay，傳入鍵盤高度
        overlay.show(suggestionsForOverlay, keyboardHeight)
    }

    /**
     * 收合候選詞視圖
     */
    private fun collapseCandidateView() {
        isExpanded = false
        smartbarView?.setExpandButtonState(false)

        // 隱藏 overlay
        candidateOverlayView?.hide()

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[EXPAND] Collapsing candidate view")
        }
    }

    /**
     * 處理 overlay 中選擇候選詞
     */
    private fun handleOverlaySuggestionSelected(word: TaigiWord, index: Int) {
        val ic = taigikeyboard.currentInputConnection ?: return

        // 取得組字管理器
        val composingManager = taigikeyboard.textInputManager.getComposingManager()

        // 根據 isTranslateSwapped 和 outputBothScripts 決定要輸出的文字
        // showHanjiMode 固定為 true
        val textToCommit = when {
            // 漢羅攏出模式
            cachedOutputBothScripts && !word.hanzi.isNullOrEmpty() -> {
                if (cachedIsTranslateSwapped) {
                    "${word.hanzi} (${word.roman})"
                } else {
                    "${word.roman} (${word.hanzi})"
                }
            }
            // 翻譯交換模式：顯示漢字
            cachedIsTranslateSwapped && !word.hanzi.isNullOrEmpty() -> word.hanzi
            // 預設顯示羅馬字
            else -> word.roman
        }

        // 選擇候選詞（使用 ComposingManager 處理狀態清除）
        composingManager?.selectSuggestion(textToCommit, ic)

        // 羅馬字模式或漢羅攏出模式：選擇候選詞後自動加空白
        if (prefs.autoSpaceEnabled && (!cachedIsTranslateSwapped || cachedOutputBothScripts)) {
            ic.commitText(" ", 1)
        }

        // 記錄使用頻率（非同步）
        scope.launch {
            UserFrequencyService.recordUsage(word.displayText)
        }

        // 清除候選詞顯示
        clearCandidates()

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[OVERLAY] Selected suggestion: ${word.displayText} at index $index")
        }
    }


    fun getPreferredContainerId(): Int {
        return when {
            hasCandidates -> R.id.candidates_container
            else -> R.id.quick_actions
        }
    }

    private fun updateActiveContainerVisibility() {
        val smartbarView = smartbarView ?: return

        when (activeContainerId) {
            R.id.quick_actions -> {
                smartbarView.candidatesContainer?.visibility = View.GONE
                smartbarView.numberRowView?.visibility = View.GONE
                smartbarView.quickActionsView?.visibility = View.VISIBLE
            }
            R.id.number_row -> {
                smartbarView.candidatesContainer?.visibility = View.GONE
                smartbarView.numberRowView?.visibility = View.VISIBLE
                smartbarView.quickActionsView?.visibility = View.GONE
            }
            R.id.candidates_container -> {
                smartbarView.candidatesContainer?.visibility = View.VISIBLE
                smartbarView.numberRowView?.visibility = View.GONE
                smartbarView.quickActionsView?.visibility = View.GONE
            }
            else -> {
                smartbarView.candidatesContainer?.visibility = View.GONE
                smartbarView.numberRowView?.visibility = View.GONE
                smartbarView.quickActionsView?.visibility = View.GONE
            }
        }
    }
}
