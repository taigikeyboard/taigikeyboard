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
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
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

    // NextWord 相關狀態
    private var lastSelectedWord: String? = null
    private var lastSelectionTime: Long = 0
    private var isShowingNextWord: Boolean = false

    /**
     * 檢查目前是否正在顯示 NextWord 候選詞
     */
    fun isShowingNextWordCandidates(): Boolean = isShowingNextWord

    private val candidateViewOnClickListener = View.OnClickListener { v ->
        // DEBUG: 追蹤點擊事件
        if (BuildConfig.DEBUG) {
            val isNextWord = currentSuggestions.firstOrNull()?.id?.let { it < 0 } ?: false
            Log.d(TAG, "[CLICK-ENTRY] onClick triggered, isNextWordMode=$isNextWord, suggestionsCount=${currentSuggestions.size}")
        }

        val button = v as Button
        val candidatesContainer = smartbarView?.candidatesView
        if (candidatesContainer == null) {
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[CLICK] candidatesContainer is null, returning")
            }
            return@OnClickListener
        }
        val buttonIndex = candidatesContainer.indexOfChild(button)

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[CLICK] buttonIndex=$buttonIndex, suggestionsSize=${currentSuggestions.size}")
        }

        if (buttonIndex >= 0 && buttonIndex < currentSuggestions.size) {
            val selectedWord = currentSuggestions[buttonIndex]
            val ic = taigikeyboard.currentInputConnection ?: return@OnClickListener

            // 取得組字管理器
            val composingManager = taigikeyboard.textInputManager.getComposingManager()

            // 判斷是否為 NextWord 候選詞（id < 0）
            val isNextWordPrediction = selectedWord.id < 0

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
                // 翻譯交換模式（漢字模式）：直接顯示漢字
                cachedIsTranslateSwapped && !selectedWord.hanzi.isNullOrEmpty() -> selectedWord.hanzi
                // 預設顯示羅馬字（一般候選詞和 NextWord 候選詞皆同）
                else -> selectedWord.roman
            }

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[CLICK] id=${selectedWord.id}, roman='${selectedWord.roman}', hanzi='${selectedWord.hanzi}'")
                Log.d(TAG, "[CLICK] isTranslateSwapped=$cachedIsTranslateSwapped, outputBothScripts=$cachedOutputBothScripts")
                Log.d(TAG, "[CLICK] textToCommit='$textToCommit', isNextWord=$isNextWordPrediction")
            }

            if (isNextWordPrediction) {
                // NextWord 候選詞：直接 commitText（此時沒有 composing text）
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[NEXTWORD-CLICK] BEFORE commitText: text='$textToCommit', ic=$ic")
                }
                val result = ic.commitText(textToCommit, 1)
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[NEXTWORD-CLICK] AFTER commitText: result=$result")
                }
            } else {
                // 一般候選詞：使用 ComposingManager 處理狀態清除
                composingManager?.selectSuggestion(textToCommit, ic)
            }

            // 依照 autoSpaceEnabled 設定加空白（一般候選詞和 NextWord 候選詞皆適用）
            if (prefs.autoSpaceEnabled && (!cachedIsTranslateSwapped || cachedOutputBothScripts)) {
                if (!textToCommit.endsWith("-")) {
                    ic.commitText(" ", 1)
                }
            }

            // 記錄使用頻率（非同步）
            scope.launch {
                UserFrequencyService.recordUsage(selectedWord.displayText)
            }

            // NextWord: 處理上下文和預測
            handleNextWordPrediction(
                displayText = selectedWord.displayText,
                committedText = textToCommit,
                roman = selectedWord.roman
            )
        }
    }

    /**
     * 處理 NextWord 預測
     *
     * @param displayText 選中詞的顯示文字（用於預測查詢）
     * @param committedText 實際提交的文字（用於判斷是否重置上下文）
     * @param roman 選中詞的羅馬字（TL 或 POJ，依 inputMode 決定）
     */
    fun handleNextWordPrediction(displayText: String, committedText: String, roman: String) {
        val currentTime = System.currentTimeMillis()

        // 檢查是否需要重置上下文
        val shouldReset = when {
            // 選中的文字以句末標點結尾
            committedText.lastOrNull() in SENTENCE_END_PUNCTUATION -> true
            // 超過 30 秒無操作
            lastSelectionTime > 0 && (currentTime - lastSelectionTime) > CONTEXT_TIMEOUT_MS -> true
            else -> false
        }

        if (shouldReset) {
            lastSelectedWord = null
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[NEXTWORD] Context reset")
            }
        }

        // 判斷是否應該記錄關聯（有前一詞且在間隔時間內）
        val shouldRecordAssociation = lastSelectedWord != null &&
            (currentTime - lastSelectionTime) < ASSOCIATION_TIMEOUT_MS

        // 拆分複合詞（如 tshit-niû → [tshit, niû]）
        val parts = splitCompoundWord(displayText)
        val romanParts = splitCompoundWord(roman)

        // 捕獲當前的 lastSelectedWord（避免在 coroutine 內被修改）
        val prevWord = lastSelectedWord

        scope.launch {
            val useTl = (prefs.inputMode == "tl")

            // 單層關聯記錄（前一詞 → 當前詞）
            // 範例：lastSelectedWord = 早安, currentWord = 你好
            // 記錄：早安 → 你好
            if (shouldRecordAssociation && prevWord != null) {
                // 跳過雜訊：如果當前詞是標點符號或數字，不記錄關聯
                if (!isNoise(displayText)) {
                    val nextTl = if (useTl) roman else ""
                    val nextPoj = if (!useTl) roman else ""

                    NextWordService.recordAssociation(
                        prev = prevWord,
                        nextHanzi = displayText,
                        nextTl = nextTl,
                        nextPoj = nextPoj,
                        context = taigikeyboard.context
                    )

                    if (BuildConfig.DEBUG) {
                        Log.d(TAG, "[NEXTWORD] Record: '$prevWord' → '$displayText'")
                    }
                } else if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[NEXTWORD] Skip noise: '$displayText'")
                }
            }

            // 記錄複合詞內部的關聯（如 tshit → niû）
            // 複合詞內部固定使用 "-" 分隔
            for (i in 0 until parts.size - 1) {
                val prevPart = parts[i]
                val nextPart = parts[i + 1]
                val nextRoman = romanParts.getOrNull(i + 1) ?: ""
                val nextTl = if (useTl) nextRoman else ""
                val nextPoj = if (!useTl) nextRoman else ""

                NextWordService.recordAssociation(
                    prev = prevPart,
                    nextHanzi = nextPart,
                    nextTl = nextTl,
                    nextPoj = nextPoj,
                    delimiter = "-",  // 複合詞內部固定用 "-"
                    context = taigikeyboard.context
                )

                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[NEXTWORD] Record compound: '$prevPart' → '$nextPart' (delimiter='-')")
                }
            }

            // 查詢下一詞預測（使用完整詞）
            // NextWordService 內部會：
            // - 字典查詢：用最後一字（字元層級）
            // - 使用者查詢：用完整詞（詞層級）
            val predictions = NextWordService.predict(
                word = displayText,
                context = taigikeyboard.context
            )

            // 更新候選詞顯示
            kotlinx.coroutines.withContext(Dispatchers.Main) {
                if (predictions.isNotEmpty()) {
                    updateCandidatesWithPredictions(predictions)
                } else {
                    clearCandidates()
                }
            }
        }

        // 更新上下文（雜訊不更新 lastSelectedWord）
        if (!isNoise(displayText)) {
            lastSelectedWord = displayText
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[NEXTWORD] lastSelectedWord updated: '$displayText'")
            }
        } else if (BuildConfig.DEBUG) {
            Log.d(TAG, "[NEXTWORD] Skip updating lastSelectedWord for noise: '$displayText'")
        }
        lastSelectionTime = currentTime
    }

    /**
     * 拆分複合詞
     *
     * 將包含 "-" 的複合詞拆分為多個部分
     * 例如：tshit-niû → [tshit, niû]
     *       tshit-niû-á → [tshit, niû, á]
     *       tshit → [tshit]（無 "-" 則返回原詞）
     *
     * @param word 要拆分的詞
     * @return 拆分後的部分列表
     */
    private fun splitCompoundWord(word: String): List<String> {
        if (word.isEmpty()) return emptyList()
        return word.split("-").filter { it.isNotEmpty() }
    }

    /**
     * 更新 lastSelectedWord（不觸發 NextWord 預測）
     *
     * 用於空白鍵確認組字時，記錄已輸出的文字，
     * 讓後續輸入可以建立關聯
     *
     * @param word 已輸出的文字
     */
    fun updateLastSelectedWord(word: String) {
        if (word.isEmpty()) return

        // 拆分複合詞
        val parts = splitCompoundWord(word)

        // 記錄複合詞內部的關聯（如 tshit → niû）
        // 複合詞內部固定使用 "-" 分隔
        if (parts.size > 1) {
            val useTl = (prefs.inputMode == "tl")

            scope.launch {
                for (i in 0 until parts.size - 1) {
                    val prevPart = parts[i]
                    val nextPart = parts[i + 1]
                    // 空白確認時沒有羅馬字資訊，只記錄漢字關聯
                    val nextTl = if (useTl) nextPart else ""
                    val nextPoj = if (!useTl) nextPart else ""

                    NextWordService.recordAssociation(
                        prev = prevPart,
                        nextHanzi = nextPart,
                        nextTl = nextTl,
                        nextPoj = nextPoj,
                        delimiter = "-",  // 複合詞內部固定用 "-"
                        context = taigikeyboard.context
                    )

                    if (BuildConfig.DEBUG) {
                        Log.d(TAG, "[NEXTWORD] Record compound (space): '$prevPart' → '$nextPart' (delimiter='-')")
                    }
                }
            }
        }

        // 更新上下文（雜訊不更新 lastSelectedWord）
        if (!isNoise(word)) {
            lastSelectedWord = word
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[NEXTWORD] updateLastSelectedWord: '$word'")
            }
        } else if (BuildConfig.DEBUG) {
            Log.d(TAG, "[NEXTWORD] updateLastSelectedWord: skip noise '$word'")
        }
        lastSelectionTime = System.currentTimeMillis()
    }

    /**
     * 使用預測結果更新候選詞顯示
     *
     * @param predictions 預測結果列表（包含漢字和羅馬字）
     */
    private fun updateCandidatesWithPredictions(predictions: List<NextWordService.Prediction>) {
        // 根據 inputMode 選擇羅馬字（TL 或 POJ）
        val useTl = (prefs.inputMode == "tl")

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[NEXTWORD] updateCandidatesWithPredictions: ${predictions.size} predictions, useTl=$useTl, isTranslateSwapped=$cachedIsTranslateSwapped")
        }

        // 將預測結果轉換為 TaigiWord
        // 若用戶選擇羅馬字輸出模式，過濾掉沒有羅馬字的候選詞
        val words = predictions.mapIndexedNotNull { index, prediction ->
            val roman = if (useTl) prediction.tl else prediction.poj

            // 羅馬字模式下，若無羅馬字則跳過
            if (!cachedIsTranslateSwapped && roman.isEmpty()) {
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[NEXTWORD] Filtered out '${prediction.hanzi}' (no roman, isTranslateSwapped=$cachedIsTranslateSwapped)")
                }
                return@mapIndexedNotNull null
            }

            TaigiWord(
                id = -index - 1,  // 負數 ID 表示預測結果
                roman = roman,
                hanzi = prediction.hanzi,
                lengthScore = prediction.score.toInt(),  // Double → Int（時間衰減後的分數）
                delimiter = prediction.delimiter  // 傳入分隔符
            )
        }

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[NEXTWORD] After filter: ${words.size} words")
            words.forEachIndexed { index, word ->
                Log.d(TAG, "[NEXTWORD] Word[$index]: hanzi='${word.hanzi}', roman='${word.roman}', score=${word.lengthScore}")
            }
        }

        if (words.isNotEmpty()) {
            updateCandidates(words)
        } else {
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
            R.id.quick_action_open_settings -> {
                // 開啟 APP 主畫面
                taigikeyboard.requestHideSelf(0)
                val intent = android.content.Intent(taigikeyboard.context, com.siansiansu.taigikeyboard.settings.SettingsMainActivity::class.java)
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

        // NextWord 常數
        private const val ASSOCIATION_TIMEOUT_MS = 10000L  // 連續選詞間隔閾值（10 秒）
        private const val CONTEXT_TIMEOUT_MS = 30_000L   // 上下文超時（30 秒）
        private val SENTENCE_END_PUNCTUATION = setOf('。', '！', '？', '.', '!', '?')  // 句末標點

        // 雜訊字元（不作為 context 記錄）
        // 包含：標點符號、空白、數字
        private val NOISE_CHARS = setOf(
            // 句末標點
            '。', '！', '？', '.', '!', '?',
            // 其他標點
            '，', ',', '、', '；', ';', '：', ':',
            '「', '」', '『', '』', '"', '"', '\'',
            '（', '）', '(', ')', '【', '】', '[', ']', '{', '}',
            '—', '–', '-', '～', '~', '…', '·',
            // 空白
            ' ', '　',
            // 數字
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'
        )

        /**
         * 判斷是否為雜訊（不應作為 context）
         * - 純標點符號
         * - 純數字
         * - 純空白
         */
        private fun isNoise(word: String): Boolean {
            if (word.isEmpty()) return true
            return word.all { it in NOISE_CHARS }
        }

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

        // 重置 NextWord 上下文（切換輸入框）
        lastSelectedWord = null
        lastSelectionTime = 0

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

        // DEBUG: 追蹤候選詞更新
        val isNextWord = suggestions.firstOrNull()?.id?.let { it < 0 } ?: false
        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[DEBUG] updateCandidates: count=${suggestions.size}, isNextWord=$isNextWord, first='${suggestions.firstOrNull()?.displayText}'")
        }

        // 儲存當前候選詞
        currentSuggestions = suggestions
        hasCandidates = true
        isShowingNextWord = isNextWord

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
                // NextWord 候選詞（id < 0）不需要組字背景，全部使用一般背景
                val isNextWordCandidate = word.id < 0
                setBackgroundResource(
                    if (i == 0 && !isNextWordCandidate) R.drawable.candidate_composing_background
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

            // 設定字體：根據 fontType 決定
            button.typeface = com.siansiansu.taigikeyboard.util.FontUtils.getTypefaceByType(
                fontType = prefs.fontType,
                context = taigikeyboard.context
            )

            candidatesContainer.addView(button)
        }

        // DEBUG: 確認按鈕建立完成
        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[DEBUG] updateCandidates completed: buttonCount=${candidatesContainer.childCount}, containerVisible=${view.candidatesContainer?.visibility == View.VISIBLE}")
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
        // DEBUG: 追蹤調用來源
        if (BuildConfig.DEBUG) {
            val stackTrace = Thread.currentThread().stackTrace
            val caller = stackTrace.getOrNull(3)?.methodName ?: "unknown"
            Log.d(TAG, "[DEBUG] clearCandidates() called from: $caller, hadCandidates=$hasCandidates")
        }

        currentSuggestions = emptyList()
        hasCandidates = false
        isShowingNextWord = false

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
     * 處理退格鍵的 NextWord 預測
     *
     * 退格刪除文字後，根據剩餘文字的最後一個字重新預測下一詞。
     * 若文字已清空，則清除 NextWord 候選詞。
     *
     * @param textBeforeCursor 游標前的文字（退格後）
     */
    fun handleBackspaceForNextWord(textBeforeCursor: String) {
        // 移除空白和標點符號，取得有效文字
        val trimmedText = textBeforeCursor.trimEnd()

        if (trimmedText.isEmpty()) {
            // 文字已清空，清除 NextWord 候選詞並重置上下文
            clearCandidates()
            lastSelectedWord = null
            lastSelectionTime = 0

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[NEXTWORD] Backspace: text empty, cleared predictions")
            }
            return
        }

        // 取得最後一個字進行預測
        val lastChar = trimmedText.last().toString()

        scope.launch {
            val predictions = NextWordService.predict(
                word = lastChar,
                context = taigikeyboard.context
            )

            kotlinx.coroutines.withContext(Dispatchers.Main) {
                if (predictions.isNotEmpty()) {
                    updateCandidatesWithPredictions(predictions)
                } else {
                    clearCandidates()
                }
            }
        }

        // 更新上下文（但不記錄關聯，因為是退格操作）
        lastSelectedWord = lastChar
        lastSelectionTime = System.currentTimeMillis()

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[NEXTWORD] Backspace: re-predict from '$lastChar'")
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

        // 判斷是否為 NextWord 候選詞（id < 0）
        val isNextWordPrediction = word.id < 0

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
            // 翻譯交換模式（漢字模式）：直接顯示漢字
            cachedIsTranslateSwapped && !word.hanzi.isNullOrEmpty() -> word.hanzi
            // 預設顯示羅馬字（一般候選詞和 NextWord 候選詞皆同）
            else -> word.roman
        }

        if (isNextWordPrediction) {
            // NextWord 候選詞：直接 commitText（此時沒有 composing text）
            ic.commitText(textToCommit, 1)
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[OVERLAY] NextWord commitText: '$textToCommit'")
            }
        } else {
            // 一般候選詞：使用 ComposingManager 處理狀態清除
            composingManager?.selectSuggestion(textToCommit, ic)
        }

        // 依照 autoSpaceEnabled 設定加空白（一般候選詞和 NextWord 候選詞皆適用）
        if (prefs.autoSpaceEnabled && (!cachedIsTranslateSwapped || cachedOutputBothScripts)) {
            if (!textToCommit.endsWith("-")) {
                ic.commitText(" ", 1)
            }
        }

        // 記錄使用頻率（非同步）
        scope.launch {
            UserFrequencyService.recordUsage(word.displayText)
        }

        // NextWord: 處理上下文和預測
        handleNextWordPrediction(
            displayText = word.displayText,
            committedText = textToCommit,
            roman = word.roman
        )

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

        // DEBUG: 追蹤容器可見性變化
        if (BuildConfig.DEBUG) {
            val containerName = when (activeContainerId) {
                R.id.quick_actions -> "quick_actions"
                R.id.number_row -> "number_row"
                R.id.candidates_container -> "candidates_container"
                else -> "unknown($activeContainerId)"
            }
            Log.d(TAG, "[DEBUG] updateActiveContainerVisibility: $containerName")
        }

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
