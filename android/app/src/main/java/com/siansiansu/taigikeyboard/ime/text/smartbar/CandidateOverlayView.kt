// 中文: 候選 overlay(展開鍵盤上方覆蓋的全畫面候選格)— 使用 RecyclerView 列回收以省 inflate 成本。
// 中文: 僅可見列會 inflate;捲動時 ViewHolder 回收。Smartbar 一般 LazyRow 不夠用時的擴充顯示。

package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Paint
import android.util.AttributeSet
import android.util.Log
import android.util.TypedValue
import android.view.LayoutInflater
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.LinearLayout
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.typeface.TypefaceLoader

/**
 * Candidate overlay view — grid display over the keyboard.
 *
 * Uses a vertical RecyclerView for row-level recycling.
 * Only visible rows are inflated; ViewHolders are reused during scroll.
 */
class CandidateOverlayView : FrameLayout {
    companion object {
        private const val TAG = "CandidateOverlayView"
        private const val ITEMS_PER_PAGE = 20
        private const val MINIMUM_CELL_WIDTH_DP = 44f
        private const val CELL_HORIZONTAL_PADDING_DP = 20f

        // Primary text size in sp (matches candidate_grid_cell.xml)
        private const val PRIMARY_TEXT_SIZE_SP = 21f

        // Subtitle text size in sp (matches candidate_grid_cell.xml)
        private const val SUBTITLE_TEXT_SIZE_SP = 19f

        // Right-side padding: 68dp (60dp panel + 8dp gap) + 4dp row margins
        private const val RIGHT_RESERVED_DP = 72f
    }

    // UI
    private var recyclerView: RecyclerView? = null
    private var layoutManager: LinearLayoutManager? = null
    private var adapter: CandidateOverlayAdapter? = null
    private var controlsContainer: LinearLayout? = null
    private var collapseButton: ImageButton? = null
    private var pageUpButton: ImageButton? = null
    private var pageDownButton: ImageButton? = null
    private var translateButton: ImageButton? = null

    // State
    private var isVisible: Boolean = false
    private var currentPage: Int = 0
    private var suggestions: List<TaigiWord> = emptyList()

    // A7: IME-only view; `context` resolves to the `TaigiKeyboard` service,
    // so the Application-owned PrefHelper is reachable without `getInstance()`.
    private val prefs: PrefHelper by lazy { (context as TaigiKeyboard).prefs }

    // Text measurement
    private val primaryPaint = Paint().apply { isAntiAlias = true }
    private val subtitlePaint = Paint().apply { isAntiAlias = true }
    private var measurementFontType: String? = null
    private val density: Float get() = resources.displayMetrics.density
    private val minimumCellWidthPx: Int get() = (MINIMUM_CELL_WIDTH_DP * density + 0.5f).toInt()
    private val cellHorizontalPaddingPx: Int get() = (CELL_HORIZONTAL_PADDING_DP * density + 0.5f).toInt()

    // Click protection: prevent expand button click from propagating to cells
    private var isClickEnabled: Boolean = true

    // Callbacks
    var onCollapse: (() -> Unit)? = null
    var onSuggestionSelected: ((TaigiWord, Int) -> Unit)? = null
    var onTranslateToggle: (() -> Unit)? = null

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)

    init {
        LayoutInflater.from(context).inflate(R.layout.candidate_overlay, this, true)
        visibility = GONE
    }

    override fun onFinishInflate() {
        super.onFinishInflate()

        recyclerView = findViewById(R.id.overlay_recycler_view)
        controlsContainer = findViewById(R.id.overlay_controls)
        collapseButton = findViewById(R.id.overlay_collapse_button)
        pageUpButton = findViewById(R.id.overlay_page_up_button)
        pageDownButton = findViewById(R.id.overlay_page_down_button)
        translateButton = findViewById(R.id.overlay_translate_button)

        // Set up RecyclerView
        layoutManager = LinearLayoutManager(context, LinearLayoutManager.VERTICAL, false)
        adapter =
            CandidateOverlayAdapter(
                context = context,
                isTranslateSwapped = { (context as TaigiKeyboard).smartbarManager.getCachedIsTranslateSwapped() },
                fontType = { prefs.fontType },
                layoutType = { prefs.keyboardLayoutType },
                orMapsToER = { prefs.tpsOrMapsToER },
                isClickEnabled = { isClickEnabled },
                onCellClick = { word, index ->
                    onSuggestionSelected?.invoke(word, index)
                    hide()
                    onCollapse?.invoke()
                },
            )
        recyclerView?.apply {
            this.layoutManager = this@CandidateOverlayView.layoutManager
            this.adapter = this@CandidateOverlayView.adapter
            itemAnimator = null // Disable animations for instant updates
        }

        applyIconTint()

        collapseButton?.setOnClickListener {
            hide()
            onCollapse?.invoke()
        }

        pageUpButton?.setOnClickListener { scrollToPreviousPage() }
        pageDownButton?.setOnClickListener { scrollToNextPage() }
        translateButton?.setOnClickListener {
            onTranslateToggle?.invoke()
            updateTranslateButtonState()
        }

        updateTranslateButtonState()
    }

    /**
     * Show overlay.
     * @param suggestions candidate list
     * @param keyboardHeight total keyboard height (constrains overlay height)
     */
    fun show(
        suggestions: List<TaigiWord>,
        keyboardHeight: Int,
    ) {
        // Click protection: disable immediately
        isClickEnabled = false

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[SHOW] show() called, isVisible=$isVisible, suggestions=${suggestions.size}")
        }
        if (isVisible) return

        this.suggestions = suggestions
        this.currentPage = 0

        // Set overlay height to keyboard height
        if (keyboardHeight > 0) {
            layoutParams = (layoutParams as? FrameLayout.LayoutParams)?.apply {
                height = keyboardHeight
            } ?: FrameLayout
                .LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    keyboardHeight,
                ).apply {
                    gravity = android.view.Gravity.TOP
                }
        }

        resetScrollPosition()
        submitRows()
        updateTranslateButtonState()

        visibility = VISIBLE
        isVisible = true

        // Click protection: release after 150ms
        postDelayed({ isClickEnabled = true }, 150)

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[SHOW] Overlay shown with ${suggestions.size} suggestions, height=$keyboardHeight")
        }
    }

    /**
     * Hide overlay.
     */
    fun hide() {
        if (BuildConfig.DEBUG) {
            val stackTrace = Thread.currentThread().stackTrace
            val caller = stackTrace.getOrNull(3)?.let { "${it.className.substringAfterLast('.')}.${it.methodName}" } ?: "unknown"
            val caller2 = stackTrace.getOrNull(4)?.let { "${it.className.substringAfterLast('.')}.${it.methodName}" } ?: ""
            Log.d(TAG, "[HIDE] hide() called from: $caller <- $caller2, isVisible=$isVisible")
        }
        if (!isVisible) return

        visibility = GONE
        isVisible = false
        adapter?.submitList(emptyList())
        suggestions = emptyList()
        currentPage = 0

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[HIDE] Overlay hidden")
        }
    }

    /**
     * Update candidate list.
     */
    @android.annotation.SuppressLint("NotifyDataSetChanged")
    fun updateSuggestions(suggestions: List<TaigiWord>) {
        this.suggestions = suggestions
        if (isVisible) {
            resetScrollPosition()
            submitRows()
            // Force rebind: isTranslateSwapped is external state not in DiffUtil data
            adapter?.notifyDataSetChanged()
        }
        updateTranslateButtonState()
    }

    /**
     * Arrange suggestions into rows and submit to adapter.
     */
    private fun submitRows() {
        if (suggestions.isEmpty()) {
            hide()
            return
        }
        adapter?.submitList(arrangeRows(suggestions))
    }

    /**
     * Arrange candidates into rows using pixel-based measurement.
     *
     * Algorithm matches iOS ExpandedCandidateOverlay.arrangedRows:
     * measure each cell's text width with Paint, pack cells into rows
     * until the next cell would exceed available width.
     */
    private fun arrangeRows(suggestions: List<TaigiWord>): List<CandidateOverlayAdapter.CandidateRow> {
        ensurePaintsConfigured()

        val screenWidthPx = resources.displayMetrics.widthPixels
        val reservedPx = (RIGHT_RESERVED_DP * density + 0.5f).toInt()
        val availableWidth = screenWidthPx - reservedPx
        val spacing = context.resources.getDimensionPixelSize(R.dimen.smartbar_button_margin)

        val rows = mutableListOf<CandidateOverlayAdapter.CandidateRow>()
        var currentRow = mutableListOf<CandidateOverlayAdapter.CandidateItem>()
        var currentRowWidth = 0

        suggestions.forEachIndexed { index, word ->
            val cellWidth = measureCellWidth(word)
            val spacingNeeded = if (currentRow.isEmpty()) 0 else spacing

            if (currentRow.isNotEmpty() && (currentRowWidth + spacingNeeded + cellWidth) > availableWidth) {
                rows.add(CandidateOverlayAdapter.CandidateRow(currentRow.toList()))
                currentRow = mutableListOf()
                currentRowWidth = 0
            }

            currentRow.add(CandidateOverlayAdapter.CandidateItem(word, index, cellWidth))
            currentRowWidth += (if (currentRow.size == 1) 0 else spacing) + cellWidth
        }

        if (currentRow.isNotEmpty()) {
            rows.add(CandidateOverlayAdapter.CandidateRow(currentRow.toList()))
        }

        return rows
    }

    /**
     * Ensure Paint objects have the correct typeface and text sizes.
     * Re-configures when font type changes.
     */
    private fun ensurePaintsConfigured() {
        val currentFontType = prefs.fontType
        if (measurementFontType != currentFontType) {
            val typeface = TypefaceLoader.getTypefaceByType(currentFontType, context)
            primaryPaint.typeface = typeface
            primaryPaint.textSize = PRIMARY_TEXT_SIZE_SP * resources.displayMetrics.scaledDensity
            subtitlePaint.typeface = typeface
            subtitlePaint.textSize = SUBTITLE_TEXT_SIZE_SP * resources.displayMetrics.scaledDensity
            measurementFontType = currentFontType
        }
    }

    /**
     * Measure cell width in pixels based on actual text rendering.
     *
     * - TPS layout: only measure hanzi (subtitle is hidden)
     * - Non-TPS: measure both roman and hanzi widths to cover swap states,
     *   so layout doesn't reflow on translate toggle (matching iOS behavior)
     */
    private fun measureCellWidth(word: TaigiWord): Int {
        val isTPSLayout = prefs.keyboardLayoutType == "tps" || prefs.inputMode == "tps"

        if (isTPSLayout) {
            // TPS: only hanzi title (or TPS-converted fallback), no subtitle
            val titleText = if (!word.hanzi.isNullOrEmpty()) {
                word.hanzi
            } else {
                // word.roman is display form (per TaigiWord docs), so use display-aware op.
                // Codex v3 §8 / v4 §8: pre-D9.4 displayRoman wrongly called numeric `toTPS`
                // on display input. D9.4 fixes via tlDisplayToTps.
                RustEngineBridge.tlDisplayToTps(word.roman, prefs.tpsOrMapsToER)
            }
            val titleWidth = primaryPaint.measureText(titleText)
            return maxOf(minimumCellWidthPx, (titleWidth + cellHorizontalPaddingPx + 0.5f).toInt())
        }

        // Non-TPS: measure both roman and hanzi to cover swap states
        val romanWidth = primaryPaint.measureText(word.roman)
        val hanziWidth =
            if (!word.hanzi.isNullOrEmpty()) {
                subtitlePaint.measureText(word.hanzi)
            } else {
                0f
            }
        val maxTextWidth = maxOf(romanWidth, hanziWidth)
        return maxOf(minimumCellWidthPx, (maxTextWidth + cellHorizontalPaddingPx + 0.5f).toInt())
    }

    /**
     * Scroll to previous page.
     */
    private fun scrollToPreviousPage() {
        val newStartIndex = maxOf(0, currentPage * ITEMS_PER_PAGE - ITEMS_PER_PAGE)
        if (newStartIndex >= 0 && newStartIndex < suggestions.size) {
            currentPage = newStartIndex / ITEMS_PER_PAGE
            val rowIndex = calculateRowIndex(newStartIndex)
            layoutManager?.scrollToPositionWithOffset(rowIndex, 0)

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[PAGE] Scrolled to previous page: $currentPage")
            }
        }
    }

    /**
     * Scroll to next page.
     */
    private fun scrollToNextPage() {
        val newStartIndex = minOf(suggestions.size - 1, (currentPage + 1) * ITEMS_PER_PAGE)
        if (newStartIndex < suggestions.size) {
            currentPage = newStartIndex / ITEMS_PER_PAGE
            val rowIndex = calculateRowIndex(newStartIndex)
            layoutManager?.scrollToPositionWithOffset(rowIndex, 0)

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[PAGE] Scrolled to next page: $currentPage")
            }
        }
    }

    /**
     * Estimate which RecyclerView row contains the given suggestion index.
     */
    private fun calculateRowIndex(suggestionIndex: Int): Int {
        val totalRows = adapter?.itemCount ?: 0
        if (totalRows == 0 || suggestions.isEmpty()) return 0
        // Approximate: distribute suggestion indices evenly across rows
        return minOf((suggestionIndex.toFloat() / suggestions.size * totalRows).toInt(), totalRows - 1)
    }

    /**
     * Reset scroll position to top.
     */
    private fun resetScrollPosition() {
        recyclerView?.scrollToPosition(0)
        currentPage = 0
    }

    /**
     * Update translate button visual state.
     * Hidden for TPS layout (always hanzi-only, no translate toggle).
     */
    private fun updateTranslateButtonState() {
        // TPS layout: hide translate button entirely
        if (prefs.keyboardLayoutType == "tps" || prefs.inputMode == "tps") {
            translateButton?.visibility = View.GONE
            return
        }

        translateButton?.visibility = View.VISIBLE
        val isTranslateSwapped = (context as TaigiKeyboard).smartbarManager.getCachedIsTranslateSwapped()
        translateButton?.isActivated = isTranslateSwapped
    }

    /**
     * Apply theme tint to all control buttons.
     */
    private fun applyIconTint() {
        val typedValue = TypedValue()
        context.theme.resolveAttribute(R.attr.smartbar_fgColor, typedValue, true)
        val tintColor = ColorStateList.valueOf(typedValue.data)

        collapseButton?.imageTintList = tintColor
        pageUpButton?.imageTintList = tintColor
        pageDownButton?.imageTintList = tintColor
        translateButton?.imageTintList = tintColor
    }
}
