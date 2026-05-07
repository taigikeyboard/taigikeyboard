package com.siansiansu.taigikeyboard.ime.popup

import android.graphics.Typeface
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.text.keyboard.AnchorSide

/**
 * Compose-side render state for the preview popup (the small "key was pressed"
 * bubble). Independent from [ExtendedState] because the preview popup window
 * stays visible while the extended popup is shown — only the three-dots
 * indicator is hidden during extension.
 */
sealed interface PreviewState {
    data object Hidden : PreviewState

    data class Visible(
        val label: String,
        val showThreeDots: Boolean,
        val popupWidthPx: Int,
        val popupHeightPx: Int,
        val display: PopupDisplayParams,
    ) : PreviewState
}

/**
 * Compose-side render state for the extended popup (multi-glyph long-press
 * picker).
 */
sealed interface ExtendedState {
    data object Hidden : ExtendedState

    data class Visible(
        val cells: List<PopupCell>,
        val activeIndex: Int,
        val anchorSide: AnchorSide,
        val row0count: Int,
        val row1count: Int,
        val cellWidthPx: Int,
        val cellHeightPx: Int,
        val totalWidthPx: Int,
        val totalHeightPx: Int,
        val display: PopupDisplayParams,
    ) : ExtendedState
}

/**
 * One cell inside the extended popup. Manager pre-resolves text vs icon so the
 * Composable does not look up KeyCode-specific branches at render time.
 *
 * `useCustomTypeface = false` is preserved for SWITCH_TO_TEXT_CONTEXT to match
 * the legacy KeyPopupExtendedSingleView path where typeface was never set on
 * that branch.
 */
data class PopupCell(
    val label: String?,
    val icon: PopupIcon?,
    val textScale: Float,
    val useCustomTypeface: Boolean,
)

enum class PopupIcon(val drawableRes: Int) {
    Settings(R.drawable.ic_settings),
    SentimentSatisfied(R.drawable.ic_sentiment_satisfied),
}

/**
 * Style parameters resolved once per show() / extend() call from the IME
 * Context. Pre-resolution mirrors the Phase B SmartbarManager.currentDisplay()
 * pattern — keeps theme/density/typeface lookups out of the hot recomposition
 * path and lets the Composable be a pure render of state.
 */
data class PopupDisplayParams(
    val fgColorArgb: Int,
    val bgColorArgb: Int,
    val extBgColorArgb: Int,
    val extBgColorActiveArgb: Int,
    val shadowColorArgb: Int,
    val cornerRadiusPx: Float,
    val keyHeightPx: Int,
    val popupTextSizePx: Float,
    val threeDotsSizePx: Int,
    val ringWidthPx: Int,
    val iconPaddingFraction: Float,
    val typeface: Typeface,
)
