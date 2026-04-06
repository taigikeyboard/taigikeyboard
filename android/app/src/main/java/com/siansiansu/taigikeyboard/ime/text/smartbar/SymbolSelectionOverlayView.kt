package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.util.AttributeSet
import android.util.Log
import android.util.TypedValue
import android.view.Gravity
import android.view.LayoutInflater
import android.view.View
import android.widget.Button
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R

/**
 * Symbol selection overlay view.
 *
 * Covers the keyboard area with a tabbed symbol grid (7 category tabs),
 * matching the LayoutSelectionOverlayView pattern.
 */
class SymbolSelectionOverlayView : FrameLayout {
    companion object {
        private const val TAG = "SymbolSelectionOverlay"
    }

    private var isShowing: Boolean = false
    private var selectedTab: SymbolCategory = SymbolCategory.FULL_WIDTH

    private var tabBar: LinearLayout? = null
    private val tabButtons = mutableListOf<Button>()
    private var gridContainer: LinearLayout? = null

    var onSymbolSelected: ((String) -> Unit)? = null

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)

    init {
        LayoutInflater.from(context).inflate(R.layout.symbol_selection_overlay, this, true)
        visibility = GONE
    }

    override fun onFinishInflate() {
        super.onFinishInflate()

        tabBar = findViewById(R.id.symbol_overlay_tab_bar)
        gridContainer = findViewById(R.id.symbol_overlay_grid_container)

        buildTabButtons()
    }

    private fun buildTabButtons() {
        val bar = tabBar ?: return
        bar.removeAllViews()
        tabButtons.clear()

        val density = resources.displayMetrics.density

        for ((index, category) in SymbolCategory.entries.withIndex()) {
            val btn =
                Button(context).apply {
                    text = category.label
                    textSize = 13f
                    isAllCaps = false
                    minWidth = 0
                    minimumWidth = 0
                    setPadding(0, 0, 0, 0)
                    layoutParams =
                        LinearLayout
                            .LayoutParams(
                                0,
                                (44 * density).toInt(),
                                1f,
                            ).apply {
                                marginStart = if (index > 0) (4 * density).toInt() else 0
                            }
                    setBackgroundResource(R.drawable.smartbar_button_background)
                    setOnClickListener {
                        selectedTab = category
                        updateTabStyles()
                        buildGrid(SymbolData.rows(category), category.columnCount, category.fontSize)
                    }
                }
            tabButtons.add(btn)
            bar.addView(btn)
        }
    }

    /**
     * Show the overlay.
     * @param keyboardHeight Total keyboard height (smartbar + keyboard) to size the overlay.
     */
    fun show(keyboardHeight: Int) {
        if (isShowing) return

        if (keyboardHeight > 0) {
            val smartbarHeight = resources.getDimensionPixelSize(R.dimen.smartbar_height)
            val overlayHeight = keyboardHeight - smartbarHeight
            layoutParams = (layoutParams as? FrameLayout.LayoutParams)?.apply {
                height = overlayHeight
                topMargin = smartbarHeight
            } ?: FrameLayout
                .LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    overlayHeight,
                ).apply {
                    gravity = Gravity.TOP
                    topMargin = smartbarHeight
                }
        }

        selectedTab = SymbolCategory.FULL_WIDTH
        updateTabStyles()
        buildGrid(SymbolData.rows(SymbolCategory.FULL_WIDTH), SymbolCategory.FULL_WIDTH.columnCount, SymbolCategory.FULL_WIDTH.fontSize)

        visibility = VISIBLE
        isShowing = true

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[SHOW] Symbol selection overlay shown, height=$keyboardHeight")
        }
    }

    /**
     * Hide the overlay.
     */
    fun hide() {
        if (!isShowing) return

        visibility = GONE
        isShowing = false

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[HIDE] Symbol selection overlay hidden")
        }
    }

    fun isVisible(): Boolean = isShowing

    private fun updateTabStyles() {
        val accentColor = resolveAccentColor()
        val fgColor = resolveForegroundColor()
        val density = resources.displayMetrics.density

        SymbolCategory.entries.forEachIndexed { index, category ->
            val btn = tabButtons.getOrNull(index) ?: return@forEachIndexed
            if (category == selectedTab) {
                btn.background =
                    GradientDrawable().apply {
                        shape = GradientDrawable.RECTANGLE
                        cornerRadius = 6 * density
                        setColor(accentColor)
                    }
                btn.setTextColor(Color.WHITE)
            } else {
                btn.setBackgroundResource(R.drawable.smartbar_button_background)
                btn.setTextColor(fgColor)
            }
        }
    }

    private fun buildGrid(
        rows: List<List<String>>,
        columnCount: Int,
        fontSize: Float,
    ) {
        val container = gridContainer ?: return
        container.removeAllViews()

        val density = resources.displayMetrics.density
        val fgColor = resolveForegroundColor()
        val cellHeight = (40 * density).toInt()
        val spacing = (4 * density).toInt()

        for (row in rows) {
            val rowLayout =
                LinearLayout(context).apply {
                    orientation = LinearLayout.HORIZONTAL
                    layoutParams =
                        LinearLayout
                            .LayoutParams(
                                LinearLayout.LayoutParams.MATCH_PARENT,
                                LinearLayout.LayoutParams.WRAP_CONTENT,
                            ).apply {
                                bottomMargin = spacing
                            }
                }

            for (i in 0 until columnCount) {
                val symbol = row.getOrNull(i)
                if (symbol != null) {
                    val cell =
                        TextView(context).apply {
                            text = symbol
                            setTextColor(fgColor)
                            textSize = fontSize
                            gravity = Gravity.CENTER
                            layoutParams =
                                LinearLayout
                                    .LayoutParams(
                                        0,
                                        cellHeight,
                                        1f,
                                    ).apply {
                                        marginStart = if (i > 0) spacing else 0
                                    }
                            isClickable = true
                            setOnClickListener {
                                onSymbolSelected?.invoke(symbol)
                            }
                        }
                    rowLayout.addView(cell)
                } else {
                    // Empty placeholder cell
                    val spacer =
                        View(context).apply {
                            layoutParams =
                                LinearLayout
                                    .LayoutParams(
                                        0,
                                        cellHeight,
                                        1f,
                                    ).apply {
                                        marginStart = if (i > 0) spacing else 0
                                    }
                        }
                    rowLayout.addView(spacer)
                }
            }

            container.addView(rowLayout)
        }
    }

    private fun resolveAccentColor(): Int {
        val typedValue = TypedValue()
        return if (context.theme.resolveAttribute(R.attr.smartbar_accentColor, typedValue, true)) {
            typedValue.data
        } else {
            Color.parseColor("#007AFF")
        }
    }

    private fun resolveForegroundColor(): Int {
        val typedValue = TypedValue()
        return if (context.theme.resolveAttribute(R.attr.smartbar_fgColor, typedValue, true)) {
            typedValue.data
        } else {
            Color.BLACK
        }
    }
}
