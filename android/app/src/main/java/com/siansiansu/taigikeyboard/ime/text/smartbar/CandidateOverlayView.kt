package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.util.AttributeSet
import android.util.Log
import android.view.LayoutInflater
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord

/**
 * 候選詞展開 Overlay 視圖
 *
 * 覆蓋在鍵盤上方，以網格形式顯示候選詞
 * 提供分頁瀏覽、翻譯切換等功能
 */
class CandidateOverlayView : FrameLayout {

    companion object {
        private const val TAG = "CandidateOverlayView"
        private const val ITEMS_PER_PAGE = 20
        private const val MAX_CHARS_PER_ROW = 25
        private const val MIN_ITEMS_PER_ROW = 3
        private const val MAX_ITEMS_PER_ROW = 4
        private const val LONG_WORD_THRESHOLD = 12
    }

    // UI 元件
    private var gridContainer: LinearLayout? = null
    private var scrollView: ScrollView? = null
    private var controlsContainer: LinearLayout? = null
    private var collapseButton: ImageButton? = null
    private var pageUpButton: ImageButton? = null
    private var pageDownButton: ImageButton? = null
    private var translateButton: ImageButton? = null

    // 狀態
    private var isVisible: Boolean = false
    private var currentPage: Int = 0
    private var suggestions: List<TaigiWord> = emptyList()
    private val prefs: PrefHelper by lazy { PrefHelper(TaigiKeyboard.getInstance().context) }

    // 點擊保護：防止展開按鈕的點擊事件傳播到 cell
    private var isClickEnabled: Boolean = true

    // 回調
    var onCollapse: (() -> Unit)? = null
    var onSuggestionSelected: ((TaigiWord, Int) -> Unit)? = null
    var onTranslateToggle: (() -> Unit)? = null

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)

    init {
        // 初始化視圖
        LayoutInflater.from(context).inflate(R.layout.candidate_overlay, this, true)
        visibility = GONE
    }

    override fun onFinishInflate() {
        super.onFinishInflate()

        // 綁定 UI 元件
        gridContainer = findViewById(R.id.overlay_grid_container)
        scrollView = findViewById(R.id.overlay_scroll_view)
        controlsContainer = findViewById(R.id.overlay_controls)
        collapseButton = findViewById(R.id.overlay_collapse_button)
        pageUpButton = findViewById(R.id.overlay_page_up_button)
        pageDownButton = findViewById(R.id.overlay_page_down_button)
        translateButton = findViewById(R.id.overlay_translate_button)

        // 設定點擊監聽器
        collapseButton?.setOnClickListener {
            hide()
            onCollapse?.invoke()
        }

        pageUpButton?.setOnClickListener { scrollToPreviousPage() }
        pageDownButton?.setOnClickListener { scrollToNextPage() }
        translateButton?.setOnClickListener {
            onTranslateToggle?.invoke()
            // 立即更新按鍵狀態
            updateTranslateButtonState()
        }

        // 初始化 translate 按鍵狀態
        updateTranslateButtonState()
    }

    /**
     * 顯示 overlay
     * @param suggestions 候選詞列表
     * @param keyboardHeight 鍵盤總高度（用於限制 overlay 高度）
     */
    fun show(suggestions: List<TaigiWord>, keyboardHeight: Int) {
        // 點擊保護：立刻禁用，防止展開按鈕的點擊事件傳播到 cell
        isClickEnabled = false

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[SHOW] show() called, isVisible=$isVisible, suggestions=${suggestions.size}")
        }
        if (isVisible) return

        this.suggestions = suggestions
        this.currentPage = 0

        // 設定 overlay 高度為鍵盤高度，確保只覆蓋鍵盤區域
        if (keyboardHeight > 0) {
            layoutParams = (layoutParams as? FrameLayout.LayoutParams)?.apply {
                height = keyboardHeight
            } ?: FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                keyboardHeight
            ).apply {
                gravity = android.view.Gravity.TOP
            }
        }

        // 重置候選詞網格滑動位置（回到頂部）
        resetScrollPosition()

        // 建立候選詞網格
        buildCandidateGrid()

        // 更新 translate 按鍵狀態
        updateTranslateButtonState()

        // 直接顯示，不使用動畫
        visibility = VISIBLE
        isVisible = true

        // 點擊保護：150ms 後解除
        postDelayed({ isClickEnabled = true }, 150)

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[SHOW] Overlay shown with ${suggestions.size} suggestions, height=$keyboardHeight")
        }
    }

    /**
     * 隱藏 overlay
     */
    fun hide() {
        if (BuildConfig.DEBUG) {
            val stackTrace = Thread.currentThread().stackTrace
            val caller = stackTrace.getOrNull(3)?.let { "${it.className.substringAfterLast('.')}.${it.methodName}" } ?: "unknown"
            val caller2 = stackTrace.getOrNull(4)?.let { "${it.className.substringAfterLast('.')}.${it.methodName}" } ?: ""
            Log.d(TAG, "[HIDE] hide() called from: $caller <- $caller2, isVisible=$isVisible")
        }
        if (!isVisible) return

        // 直接隱藏，不使用動畫
        visibility = GONE
        isVisible = false
        clearGrid()

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[HIDE] Overlay hidden")
        }
    }

    /**
     * 更新候選詞列表
     */
    fun updateSuggestions(suggestions: List<TaigiWord>) {
        this.suggestions = suggestions
        if (isVisible) {
            // 重置候選詞網格滑動位置（回到頂部）
            resetScrollPosition()
            buildCandidateGrid()
        }
        // 更新 translate 按鍵狀態
        updateTranslateButtonState()
    }

    /**
     * 建立候選詞網格
     */
    private fun buildCandidateGrid() {
        val container = gridContainer ?: return
        container.removeAllViews()

        if (suggestions.isEmpty()) {
            hide()
            return
        }

        // 計算網格排列
        val rows = arrangeRows(suggestions)

        // 建立每一行
        rows.forEachIndexed { rowIndex, row ->
            val rowLayout = createRowLayout(row, rowIndex < rows.size - 1)
            container.addView(rowLayout)
        }
    }

    /**
     * 計算候選詞網格排列
     *
     * 根據候選詞長度動態分配每行的項目數量，而非固定欄位數。
     * 演算法邏輯與 iOS 版本保持一致 (CandidateView.swift:arrangedRows)。
     *
     * 換行條件：
     * 1. 當前行字元數 + 新候選詞字元數 > MAX_CHARS_PER_ROW (20)
     *    且當前行至少有 MIN_ITEMS_PER_ROW (2) 個項目
     * 2. 或當前行已達 MAX_ITEMS_PER_ROW (4) 個項目
     *
     * 候選詞權重（影響視覺比例）：
     * - 1-3 字元：1.0
     * - 4-5 字元：1.1
     * - 6-7 字元：1.4
     * - 8-10 字元：2.2
     * - 11+ 字元：4.0
     *
     * @param suggestions 候選詞列表
     * @return 依照動態排列規則組織的候選詞行列表
     */
    private fun arrangeRows(suggestions: List<TaigiWord>): List<CandidateRow> {
        val rows = mutableListOf<CandidateRow>()
        var currentRow = mutableListOf<CandidateItem>()
        var currentRowCharCount = 0

        suggestions.forEachIndexed { index, word ->
            val charCount = getCharacterCount(word)
            val weight = getItemWeight(word)

            val shouldStartNewRow = (
                (currentRowCharCount + charCount > MAX_CHARS_PER_ROW &&
                    currentRow.isNotEmpty() &&
                    currentRow.size >= MIN_ITEMS_PER_ROW) ||
                    currentRow.size >= MAX_ITEMS_PER_ROW
            )

            if (shouldStartNewRow) {
                rows.add(CandidateRow(currentRow.toList(), currentRow.sumOf { it.weight }))
                currentRow.clear()
                currentRowCharCount = 0
            }

            val item = CandidateItem(word, index, weight)
            currentRow.add(item)
            currentRowCharCount += charCount
        }

        if (currentRow.isNotEmpty()) {
            rows.add(CandidateRow(currentRow.toList(), currentRow.sumOf { it.weight }))
        }

        return rows
    }

    /**
     * 建立一行候選詞視圖
     */
    private fun createRowLayout(row: CandidateRow, showDivider: Boolean): LinearLayout {
        val rowLayout = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }

        // 候選詞容器
        val itemsLayout = LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                val spacing = resources.getDimensionPixelSize(R.dimen.smartbar_button_margin)
                setMargins(spacing * 2, spacing, spacing * 2, spacing)
            }
        }

        // 加入候選詞項目，使用 weight 平均分配寬度
        val spacing = resources.getDimensionPixelSize(R.dimen.smartbar_button_margin)
        row.items.forEach { item ->
            val cellView = createCellView(item)

            // 使用 weight 讓 cell 根據權重分配寬度，填滿整排空間
            val layoutParams = LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.WRAP_CONTENT,
                item.weight.toFloat()
            ).apply {
                // 設定 cell 之間的間距
                if (row.items.indexOf(item) > 0) {
                    marginStart = spacing
                }
            }
            cellView.layoutParams = layoutParams

            itemsLayout.addView(cellView)
        }

        rowLayout.addView(itemsLayout)

        // 分隔線
        if (showDivider) {
            val divider = View(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    1
                ).apply {
                    val margin = resources.getDimensionPixelSize(R.dimen.smartbar_button_margin) * 2
                    setMargins(margin, 0, margin + 60, 0) // 右側預留控制按鈕空間
                }
                // 使用半透明灰色作為分隔線顏色
                setBackgroundColor(0x4D000000.toInt()) // 30% 黑色
            }
            rowLayout.addView(divider)
        }

        return rowLayout
    }

    /**
     * 建立候選詞 cell 視圖
     */
    private fun createCellView(item: CandidateItem): View {
        val charCount = getCharacterCount(item.word)

        return if (charCount >= LONG_WORD_THRESHOLD) {
            createLongCellView(item)
        } else {
            createGridCellView(item)
        }
    }

    /**
     * 建立標準網格 cell
     */
    private fun createGridCellView(item: CandidateItem): View {
        val cellView = LayoutInflater.from(context).inflate(
            R.layout.candidate_grid_cell,
            null,
            false
        )

        val primaryText = cellView.findViewById<TextView>(R.id.cell_primary_text)
        val subtitleText = cellView.findViewById<TextView>(R.id.cell_subtitle_text)

        // 根據設定決定顯示內容
        bindCellContent(item.word, primaryText, subtitleText)

        // 設定點擊事件（含點擊保護）
        cellView.setOnClickListener {
            if (!isClickEnabled) return@setOnClickListener
            onSuggestionSelected?.invoke(item.word, item.originalIndex)
            hide()
            onCollapse?.invoke()  // 同步更新 SmartbarManager 的展開狀態
        }

        return cellView
    }

    /**
     * 建立長詞 cell
     */
    private fun createLongCellView(item: CandidateItem): View {
        val cellView = LayoutInflater.from(context).inflate(
            R.layout.candidate_long_cell,
            null,
            false
        )

        val primaryText = cellView.findViewById<TextView>(R.id.cell_primary_text)
        val subtitleText = cellView.findViewById<TextView>(R.id.cell_subtitle_text)

        // 根據設定決定顯示內容
        bindCellContent(item.word, primaryText, subtitleText)

        // 設定點擊事件（含點擊保護）
        cellView.setOnClickListener {
            if (!isClickEnabled) return@setOnClickListener
            onSuggestionSelected?.invoke(item.word, item.originalIndex)
            hide()
            onCollapse?.invoke()  // 同步更新 SmartbarManager 的展開狀態
        }

        return cellView
    }

    /**
     * 綁定 cell 內容
     * 根據 isTranslateSwapped 決定顯示內容
     * showHanjiMode 固定為 true
     */
    private fun bindCellContent(word: TaigiWord, primaryText: TextView, subtitleText: TextView) {
        // 使用 SmartbarManager 的快取值，避免 DataStore 非同步寫入造成的 race condition
        val smartbarManager = SmartbarManager.getInstance()
        val isTranslateSwapped = smartbarManager.getCachedIsTranslateSwapped()

        when {
            // 沒有漢字：只顯示羅馬字
            word.hanzi.isNullOrEmpty() -> {
                primaryText.text = word.roman
                subtitleText.visibility = View.GONE
            }

            // 翻譯交換模式：漢字為主，羅馬字為副
            isTranslateSwapped -> {
                primaryText.text = word.hanzi
                subtitleText.text = word.roman
                subtitleText.visibility = View.VISIBLE
            }

            // 預設模式：羅馬字為主，漢字為副
            else -> {
                primaryText.text = word.roman
                subtitleText.text = word.hanzi
                subtitleText.visibility = View.VISIBLE
            }
        }

        // 設定字體：根據 fontType 決定
        val fontType = prefs.fontType
        primaryText.typeface = com.siansiansu.taigikeyboard.util.FontUtils.getTypefaceByType(
            fontType = fontType,
            context = context
        )
        subtitleText.typeface = com.siansiansu.taigikeyboard.util.FontUtils.getTypefaceByType(
            fontType = fontType,
            context = context
        )
    }

    /**
     * 計算候選詞字元數
     *
     * 取羅馬字和漢字長度的最大值，確保視覺上能完整顯示較長的那一個。
     * 對應 iOS 版本的 getCharacterCount(for:) 方法。
     */
    private fun getCharacterCount(word: TaigiWord): Int {
        val romanLength = word.roman.length
        val hanziLength = word.hanzi?.length ?: 0
        return maxOf(romanLength, hanziLength)
    }

    /**
     * 計算候選詞權重
     *
     * 權重用於候選詞 cell 的視覺比例分配，較長的詞彙獲得較高權重。
     * 調整權重分級以避免短候選詞按鈕過大。
     */
    private fun getItemWeight(word: TaigiWord): Double {
        val maxLength = getCharacterCount(word)
        return when (maxLength) {
            in 1..5 -> 1.0
            in 6..8 -> 1.2
            in 9..11 -> 1.8
            else -> 3.0
        }
    }

    /**
     * 滾動到上一頁
     */
    private fun scrollToPreviousPage() {
        val newStartIndex = maxOf(0, currentPage * ITEMS_PER_PAGE - ITEMS_PER_PAGE)
        if (newStartIndex >= 0 && newStartIndex < suggestions.size) {
            currentPage = newStartIndex / ITEMS_PER_PAGE
            scrollView?.smoothScrollTo(0, calculateScrollPosition(newStartIndex))

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[PAGE] Scrolled to previous page: $currentPage")
            }
        }
    }

    /**
     * 滾動到下一頁
     */
    private fun scrollToNextPage() {
        val newStartIndex = minOf(suggestions.size - 1, (currentPage + 1) * ITEMS_PER_PAGE)
        if (newStartIndex < suggestions.size) {
            currentPage = newStartIndex / ITEMS_PER_PAGE
            scrollView?.smoothScrollTo(0, calculateScrollPosition(newStartIndex))

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[PAGE] Scrolled to next page: $currentPage")
            }
        }
    }

    /**
     * 計算滾動位置
     * TODO: 根據實際 row height 計算精確位置
     */
    private fun calculateScrollPosition(index: Int): Int {
        // 簡化計算：假設每行約 60dp
        val estimatedRowHeight = (60 * resources.displayMetrics.density).toInt()
        val rowIndex = index / MAX_ITEMS_PER_ROW
        return rowIndex * estimatedRowHeight
    }

    /**
     * 清除網格內容
     */
    private fun clearGrid() {
        gridContainer?.removeAllViews()
        suggestions = emptyList()
        currentPage = 0
    }

    /**
     * 重置候選詞網格滑動位置至起點
     * 確保新候選詞網格總是從頂部開始顯示
     */
    private fun resetScrollPosition() {
        scrollView?.scrollTo(0, 0)
        currentPage = 0
    }

    /**
     * 更新 translate 按鍵的視覺狀態
     * 根據 isTranslateSwapped 設定按鍵的 activated 狀態
     */
    private fun updateTranslateButtonState() {
        val smartbarManager = SmartbarManager.getInstance()
        val isTranslateSwapped = smartbarManager.getCachedIsTranslateSwapped()
        translateButton?.isActivated = isTranslateSwapped
    }

    /**
     * 候選詞行資料結構
     */
    data class CandidateRow(
        val items: List<CandidateItem>,
        val totalWeight: Double
    )

    /**
     * 候選詞項目資料結構
     */
    data class CandidateItem(
        val word: TaigiWord,
        val originalIndex: Int,
        val weight: Double
    )
}
