package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.util.AttributeSet
import android.util.Log
import android.util.TypedValue
import android.view.Gravity
import android.view.LayoutInflater
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.graphics.toColorInt
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.localization.LayoutTexts

/**
 * Layout selection overlay view.
 *
 * Covers the keyboard area with layout preview cards,
 * matching the iOS LayoutSelectionOverlay behavior.
 */
class LayoutSelectionOverlayView : FrameLayout {
    companion object {
        private const val TAG = "LayoutSelectionOverlay"
        private const val CARD_WIDTH_DP = 120
        private const val CARD_SPACING_DP = 12
        private const val CHECKMARK_SIZE_DP = 36
        private const val CORNER_RADIUS_DP = 10
        private const val AUTO_COLLAPSE_DELAY_MS = 300L
    }

    private data class LayoutOption(
        val key: String,
        val labelProvider: () -> String,
        val previewRes: Int,
        val isDisabled: Boolean = false,
    )

    // A7: IME-only overlay; `context` resolves to the `TaigiKeyboard` service.
    private val prefs: PrefHelper get() = (context as TaigiKeyboard).prefs
    private var isShowing: Boolean = false

    private var romanizationRow: LinearLayout? = null
    private var phoneticRow: LinearLayout? = null

    var onLayoutSelected: ((String) -> Unit)? = null

    private val romanizationLayouts: List<LayoutOption> by lazy {
        listOf(
            LayoutOption("phahTaigi", { LayoutTexts.phahTaigiLayout }, R.drawable.layout_phahtaigi_preview),
            LayoutOption("qwerty", { LayoutTexts.standardLayout }, R.drawable.layout_standard_preview),
            LayoutOption("moe1", { LayoutTexts.moe1Layout }, R.drawable.layout_moe1_preview),
            LayoutOption("moe2", { LayoutTexts.moe2Layout }, R.drawable.layout_moe2_preview),
        )
    }

    private val phoneticLayouts: List<LayoutOption> by lazy {
        listOf(
            LayoutOption("tps", { LayoutTexts.tpsLayout }, R.drawable.layout_tps_preview),
        )
    }

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)

    init {
        LayoutInflater.from(context).inflate(R.layout.layout_selection_overlay, this, true)
        visibility = GONE
    }

    override fun onFinishInflate() {
        super.onFinishInflate()

        romanizationRow = findViewById(R.id.layout_overlay_romanization_row)
        phoneticRow = findViewById(R.id.layout_overlay_phonetic_row)

        // Set section headers
        findViewById<TextView>(R.id.layout_overlay_romanization_header)?.text =
            LayoutTexts.romanizationKeyboard
        findViewById<TextView>(R.id.layout_overlay_phonetic_header)?.text =
            LayoutTexts.taigiPhonetic
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

        buildCards()
        visibility = VISIBLE
        isShowing = true

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[SHOW] Layout selection overlay shown, height=$keyboardHeight")
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
            Log.d(TAG, "[HIDE] Layout selection overlay hidden")
        }
    }

    fun isVisible(): Boolean = isShowing

    private fun buildCards(selectedLayout: String? = null) {
        val currentLayout = selectedLayout ?: prefs.keyboardLayoutType

        romanizationRow?.removeAllViews()
        phoneticRow?.removeAllViews()

        romanizationLayouts.forEachIndexed { index, layout ->
            val card = createLayoutCard(layout, currentLayout == layout.key)
            romanizationRow?.addView(card)
            if (index < romanizationLayouts.size - 1) {
                romanizationRow?.addView(createSpacer())
            }
        }

        phoneticLayouts.forEachIndexed { index, layout ->
            val card = createLayoutCard(layout, currentLayout == layout.key)
            phoneticRow?.addView(card)
            if (index < phoneticLayouts.size - 1) {
                phoneticRow?.addView(createSpacer())
            }
        }
    }

    private fun createLayoutCard(
        layout: LayoutOption,
        isSelected: Boolean,
    ): View {
        val density = resources.displayMetrics.density
        val cardWidthPx = (CARD_WIDTH_DP * density).toInt()
        val cornerRadiusPx = CORNER_RADIUS_DP * density
        val checkmarkSizePx = (CHECKMARK_SIZE_DP * density).toInt()

        val column =
            LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                layoutParams =
                    LinearLayout.LayoutParams(
                        cardWidthPx,
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                    )
                gravity = Gravity.CENTER_HORIZONTAL
            }

        // Preview image container (FrameLayout for overlays)
        val imageContainer =
            FrameLayout(context).apply {
                layoutParams =
                    LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                    )
            }

        // Preview image
        val imageView =
            ImageView(context).apply {
                layoutParams =
                    FrameLayout.LayoutParams(
                        FrameLayout.LayoutParams.MATCH_PARENT,
                        FrameLayout.LayoutParams.WRAP_CONTENT,
                    )
                scaleType = ImageView.ScaleType.FIT_CENTER
                adjustViewBounds = true
                setImageResource(layout.previewRes)

                // Rounded corners via clip
                clipToOutline = true
                outlineProvider =
                    object : android.view.ViewOutlineProvider() {
                        override fun getOutline(
                            view: View,
                            outline: android.graphics.Outline,
                        ) {
                            outline.setRoundRect(0, 0, view.width, view.height, cornerRadiusPx)
                        }
                    }
            }
        imageContainer.addView(imageView)

        if (layout.isDisabled) {
            // Dark overlay for disabled cards (40% black, matching Layout tab)
            val darkOverlay =
                View(context).apply {
                    layoutParams =
                        FrameLayout.LayoutParams(
                            FrameLayout.LayoutParams.MATCH_PARENT,
                            FrameLayout.LayoutParams.MATCH_PARENT,
                        )
                    setBackgroundColor(Color.argb(102, 0, 0, 0)) // 0x66000000
                    clipToOutline = true
                    outlineProvider =
                        object : android.view.ViewOutlineProvider() {
                            override fun getOutline(
                                view: View,
                                outline: android.graphics.Outline,
                            ) {
                                outline.setRoundRect(0, 0, view.width, view.height, cornerRadiusPx)
                            }
                        }
                }
            imageContainer.addView(darkOverlay)

            // "Coming Soon" text with capsule background (matching Layout tab)
            val comingSoonText =
                TextView(context).apply {
                    layoutParams =
                        FrameLayout.LayoutParams(
                            FrameLayout.LayoutParams.WRAP_CONTENT,
                            FrameLayout.LayoutParams.WRAP_CONTENT,
                            Gravity.CENTER,
                        )
                    text = LayoutTexts.comingSoon
                    setTextColor(resolveForegroundColor())
                    textSize = 11f
                    setTypeface(typeface, android.graphics.Typeface.BOLD)
                    val hPad = (12 * density).toInt()
                    val vPad = (4 * density).toInt()
                    setPadding(hPad, vPad, hPad, vPad)
                    background =
                        GradientDrawable().apply {
                            shape = GradientDrawable.RECTANGLE
                            cornerRadius = 50 * density
                            setColor(Color.argb(180, 128, 128, 128))
                        }
                }
            imageContainer.addView(comingSoonText)
        } else if (isSelected) {
            // Semi-transparent dark overlay for selected cards
            val selectedOverlay =
                View(context).apply {
                    layoutParams =
                        FrameLayout.LayoutParams(
                            FrameLayout.LayoutParams.MATCH_PARENT,
                            FrameLayout.LayoutParams.MATCH_PARENT,
                        )
                    setBackgroundColor(Color.argb(64, 0, 0, 0)) // 25% black
                    clipToOutline = true
                    outlineProvider =
                        object : android.view.ViewOutlineProvider() {
                            override fun getOutline(
                                view: View,
                                outline: android.graphics.Outline,
                            ) {
                                outline.setRoundRect(0, 0, view.width, view.height, cornerRadiusPx)
                            }
                        }
                }
            imageContainer.addView(selectedOverlay)

            // Checkmark circle
            val checkmarkContainer =
                FrameLayout(context).apply {
                    layoutParams =
                        FrameLayout.LayoutParams(
                            checkmarkSizePx,
                            checkmarkSizePx,
                            Gravity.CENTER,
                        )
                    background =
                        GradientDrawable().apply {
                            shape = GradientDrawable.OVAL
                            setColor(resolveAccentColor())
                        }
                }

            val checkmarkIcon =
                ImageView(context).apply {
                    layoutParams =
                        FrameLayout.LayoutParams(
                            (16 * density).toInt(),
                            (16 * density).toInt(),
                            Gravity.CENTER,
                        )
                    setImageResource(R.drawable.ic_check)
                    imageTintList = ColorStateList.valueOf(Color.WHITE)
                }
            checkmarkContainer.addView(checkmarkIcon)
            imageContainer.addView(checkmarkContainer)
        }

        // Border for selected cards
        if (isSelected && !layout.isDisabled) {
            val borderDrawable =
                GradientDrawable().apply {
                    shape = GradientDrawable.RECTANGLE
                    cornerRadius = cornerRadiusPx
                    setStroke((2.5f * density).toInt(), resolveAccentColor())
                    setColor(Color.TRANSPARENT)
                }
            val borderView =
                View(context).apply {
                    layoutParams =
                        FrameLayout.LayoutParams(
                            FrameLayout.LayoutParams.MATCH_PARENT,
                            FrameLayout.LayoutParams.MATCH_PARENT,
                        )
                    background = borderDrawable
                }
            imageContainer.addView(borderView)
        }

        column.addView(imageContainer)

        // Label text
        val label =
            TextView(context).apply {
                layoutParams =
                    LinearLayout
                        .LayoutParams(
                            LinearLayout.LayoutParams.WRAP_CONTENT,
                            LinearLayout.LayoutParams.WRAP_CONTENT,
                        ).apply {
                            topMargin = (6 * density).toInt()
                        }
                text = layout.labelProvider()
                textSize = 12f
                setTypeface(typeface, android.graphics.Typeface.BOLD)
                gravity = Gravity.CENTER
                maxLines = 1
                setTextColor(resolveForegroundColor())
                alpha = if (layout.isDisabled) 0.5f else 1f
            }
        column.addView(label)

        // Click handler
        if (!layout.isDisabled) {
            column.setOnClickListener {
                prefs.keyboardLayoutType = layout.key
                buildCards(layout.key) // Refresh selection state with known value
                // Directly trigger keyboard rebuild (bypass DataStore Flow delay)
                onLayoutSelected?.invoke(layout.key)
                // Auto-collapse after short delay
                it.postDelayed({
                    hide()
                }, AUTO_COLLAPSE_DELAY_MS)
            }
        }

        return column
    }

    private fun createSpacer(): View {
        val spacingPx = (CARD_SPACING_DP * resources.displayMetrics.density).toInt()
        return View(context).apply {
            layoutParams = LinearLayout.LayoutParams(spacingPx, 1)
        }
    }

    private fun resolveAccentColor(): Int {
        val typedValue = TypedValue()
        return if (context.theme.resolveAttribute(R.attr.smartbar_accentColor, typedValue, true)) {
            typedValue.data
        } else {
            "#007AFF".toColorInt() // Fallback iOS blue
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
