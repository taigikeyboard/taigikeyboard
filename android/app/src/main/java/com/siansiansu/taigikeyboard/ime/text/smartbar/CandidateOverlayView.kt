package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.content.res.ColorStateList
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
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord

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
        private const val MAX_CHARS_PER_ROW = 20
        private const val MIN_ITEMS_PER_ROW = 2
        private const val MAX_ITEMS_PER_ROW = 4
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
    private val prefs: PrefHelper by lazy { PrefHelper(TaigiKeyboard.getInstance().context) }

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
        adapter = CandidateOverlayAdapter(
            context = context,
            isTranslateSwapped = { SmartbarManager.getInstance().getCachedIsTranslateSwapped() },
            fontType = { prefs.fontType },
            isClickEnabled = { isClickEnabled },
            onCellClick = { word, index ->
                onSuggestionSelected?.invoke(word, index)
                hide()
                onCollapse?.invoke()
            }
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
    fun show(suggestions: List<TaigiWord>, keyboardHeight: Int) {
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
            } ?: FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                keyboardHeight
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
     * Arrange candidates into rows based on character count and weight.
     *
     * Algorithm matches iOS CandidateView.swift:arrangedRows.
     *
     * New-row conditions:
     * 1. Current row chars + new word chars > MAX_CHARS_PER_ROW
     *    AND current row has >= MIN_ITEMS_PER_ROW items
     * 2. OR current row has reached MAX_ITEMS_PER_ROW items
     */
    private fun arrangeRows(suggestions: List<TaigiWord>): List<CandidateOverlayAdapter.CandidateRow> {
        val rows = mutableListOf<CandidateOverlayAdapter.CandidateRow>()
        var currentRow = mutableListOf<CandidateOverlayAdapter.CandidateItem>()
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
                rows.add(CandidateOverlayAdapter.CandidateRow(currentRow.toList(), currentRow.sumOf { it.weight }))
                currentRow.clear()
                currentRowCharCount = 0
            }

            val item = CandidateOverlayAdapter.CandidateItem(word, index, weight)
            currentRow.add(item)
            currentRowCharCount += charCount
        }

        if (currentRow.isNotEmpty()) {
            rows.add(CandidateOverlayAdapter.CandidateRow(currentRow.toList(), currentRow.sumOf { it.weight }))
        }

        return rows
    }

    /**
     * Character count for a candidate word.
     * Max of roman and hanzi length for visual sizing.
     */
    private fun getCharacterCount(word: TaigiWord): Int {
        val romanLength = word.roman.length
        val hanziLength = word.hanzi?.length ?: 0
        return maxOf(romanLength, hanziLength)
    }

    /**
     * Weight for visual proportion in the row.
     */
    private fun getItemWeight(word: TaigiWord): Double {
        val maxLength = getCharacterCount(word)
        return when (maxLength) {
            in 1..3 -> 1.0
            in 4..5 -> 1.1
            in 6..7 -> 1.4
            in 8..10 -> 2.2
            else -> 4.0
        }
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
     */
    private fun updateTranslateButtonState() {
        val smartbarManager = SmartbarManager.getInstance()
        val isTranslateSwapped = smartbarManager.getCachedIsTranslateSwapped()
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
