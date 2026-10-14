package com.siansiansu.taigikeyboard.ime.text.keyboard

import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import kotlin.math.roundToInt

/**
 * Pure layout-math solver for keyboard geometry. All inputs are scalar value
 * types so the math is JVM-testable without Android instrumentation.
 *
 * Decoupled responsibilities:
 * - [solveKeyDimensions] — letter-key width and height, margins, plus the live
 *   `keyHeightFactor` smartbar consumes
 * - [keyWidth] / [isFixedWidth] — per-key width: iOS slot fractions for letter and
 *   symbol keyboards, legacy multipliers for the advanced number pad
 * - [solvePopupDimensions] — preview-popup dimensions and offset
 * - [solveExtendedPopupGeometry] — extended-popup anchor side, row split,
 *   anchor offset, and final placement under screen-edge constraints
 */
object KeyboardLayoutSolver {
    fun solveKeyDimensions(input: KeyDimensionsInput): KeyDimensions {
        val orientationFactor = if (input.isLandscape) 0.85f else 1.0f
        val keyHeightFactor = orientationFactor * input.keyHeightScale
        val keyMarginH = input.keyMarginH
        val desiredKeyWidth = (input.containerWidth / LETTER_SLOTS_PER_ROW) - (2 * keyMarginH)
        val desiredKeyHeight = (input.baseKeyHeight * keyHeightFactor).toInt()
        return KeyDimensions(
            desiredKeyWidth = desiredKeyWidth,
            desiredKeyHeight = desiredKeyHeight,
            keyHeightFactor = keyHeightFactor,
            containerWidth = input.containerWidth,
            keyMarginH = keyMarginH,
            keyMarginV = input.keyMarginV,
            isLandscape = input.isLandscape,
        )
    }

    /** Visible width in px (margins excluded) of the key [keyCode] in [mode]. */
    fun keyWidth(
        keyCode: Int,
        mode: KeyboardMode,
        keyboardLayoutType: String,
        dimensions: KeyDimensions,
    ): Int {
        val desiredKeyWidth = dimensions.desiredKeyWidth
        when (mode) {
            // Every number-pad key grows to an equal share of the row (see flexGrowFor).
            KeyboardMode.NUMERIC, KeyboardMode.PHONE, KeyboardMode.PHONE2 -> return desiredKeyWidth
            KeyboardMode.NUMERIC_ADVANCED -> return when (keyCode) {
                44, 46 -> desiredKeyWidth
                KeyCode.VIEW_SYMBOLS, 61 -> (desiredKeyWidth * 1.34f).toInt()
                else -> (desiredKeyWidth * 1.56f).toInt()
            }
            else -> Unit
        }
        val fraction = slotFraction(keyCode, keyboardLayoutType, dimensions.isLandscape)
        return when {
            fraction != null -> (dimensions.containerWidth * fraction).roundToInt() - 2 * dimensions.keyMarginH
            keyCode == KeyCode.SPACE && mode == KeyboardMode.SYMBOLS -> (desiredKeyWidth * 0.56f).toInt()
            else -> desiredKeyWidth
        }
    }

    /** True when [keyCode] takes a fixed slot of the row width and never flex-shrinks. */
    fun isFixedWidth(
        keyCode: Int,
        keyboardLayoutType: String,
        isLandscape: Boolean,
    ): Boolean = slotFraction(keyCode, keyboardLayoutType, isLandscape) != null

    /**
     * Key slot (visible key + both `keyMarginH` bands) as a fraction of the keyboard width,
     * or null for a letter-sized key. CROSS-PLATFORM INVARIANT — mirrors iOS
     * `Layout/LayoutConstants.swift` + `LayoutConverter.widthFor`.
     */
    internal fun slotFraction(
        keyCode: Int,
        keyboardLayoutType: String,
        isLandscape: Boolean,
    ): Float? =
        when (keyCode) {
            KeyCode.SHIFT, KeyCode.DELETE -> SHIFT_DELETE_SLOT
            KeyCode.VIEW_CHARACTERS, KeyCode.VIEW_SYMBOLS, KeyCode.VIEW_SYMBOLS2,
            KeyCode.SWITCH_TO_MEDIA_CONTEXT, KeyCode.LANGUAGE_SWITCH,
            -> if (isLandscape) SYSTEM_SLOT_LANDSCAPE else SYSTEM_SLOT_PORTRAIT
            KeyCode.ENTER -> if (isLandscape) ENTER_SLOT_LANDSCAPE else ENTER_SLOT_PORTRAIT
            KeyCode.TRANSLATE -> when (keyboardLayoutType) {
                "phahTaigi", "moe1" -> 2.0f / LETTER_SLOTS_PER_ROW
                else -> 1.5f / LETTER_SLOTS_PER_ROW
            }
            else -> null
        }

    /** A letter slot is 1/10 of the keyboard width on both platforms. */
    private const val LETTER_SLOTS_PER_ROW = 10
    private const val SHIFT_DELETE_SLOT = 0.13f
    private const val SYSTEM_SLOT_PORTRAIT = 0.13f
    private const val SYSTEM_SLOT_LANDSCAPE = 0.10f
    private const val ENTER_SLOT_PORTRAIT = 0.15f
    private const val ENTER_SLOT_LANDSCAPE = 0.095f

    fun solvePopupDimensions(input: PopupDimensionsInput): PopupDimensions {
        val popupWidth: Int
        val popupHeight: Int
        if (input.isLandscape) {
            popupWidth = (input.desiredKeyWidth * 0.6f).toInt()
            popupHeight = (input.desiredKeyHeight * 3.0f).toInt()
        } else {
            popupWidth = (input.desiredKeyWidth * 1.1f).toInt()
            popupHeight = (input.desiredKeyHeight * 2.5f).toInt()
        }
        val popupDiffX = (input.keyViewMeasuredWidth - popupWidth) / 2
        return PopupDimensions(
            popupWidth = popupWidth,
            popupHeight = popupHeight,
            popupDiffX = popupDiffX,
            popupX = popupDiffX,
            popupY = -popupHeight,
        )
    }

    fun solveExtendedPopupGeometry(input: ExtendedPopupGeometryInput): ExtendedPopupGeometry {
        val anchorSide =
            if (input.keyViewX < input.keyboardViewMeasuredWidth / 2) AnchorSide.LEFT else AnchorSide.RIGHT

        val row0count: Int
        val row1count: Int
        when {
            input.popupCount <= 10 -> {
                row1count = 0
                row0count = input.popupCount
            }
            input.popupCount % 2 == 1 -> {
                row1count = (input.popupCount - 1) / 2
                row0count = (input.popupCount + 1) / 2
            }
            else -> {
                row1count = input.popupCount / 2
                row0count = input.popupCount / 2
            }
        }

        val keyPopupDiffX = (input.keyViewMeasuredWidth - input.keyPopupWidth) / 2

        val anchorOffset =
            if (row0count <= 1) {
                0
            } else {
                var offset =
                    if (row0count % 2 == 1) (row0count - 1) / 2 else (row0count / 2) - 1
                val availableSpace =
                    when (anchorSide) {
                        AnchorSide.LEFT -> input.keyViewX.toInt() + keyPopupDiffX
                        AnchorSide.RIGHT ->
                            input.keyboardViewMeasuredWidth -
                                (input.keyViewX.toInt() + keyPopupDiffX + input.keyPopupWidth)
                    }
                while (offset > 0 && availableSpace < offset * input.keyPopupWidth) {
                    offset -= 1
                }
                offset
            }

        val extWidth = row0count * input.keyPopupWidth
        val extHeight =
            if (row1count > 0) input.keyViewMeasuredHeight * 2 else input.keyViewMeasuredHeight

        val anchorShift =
            when (anchorSide) {
                AnchorSide.LEFT -> -anchorOffset * input.keyPopupWidth
                AnchorSide.RIGHT -> -extWidth + input.keyPopupWidth + anchorOffset * input.keyPopupWidth
            }
        val popupX = keyPopupDiffX + anchorShift
        val popupY = -input.keyPopupHeight - if (row1count > 0) input.keyViewMeasuredHeight else 0

        return ExtendedPopupGeometry(
            anchorSide = anchorSide,
            row0count = row0count,
            row1count = row1count,
            anchorOffset = anchorOffset,
            extWidth = extWidth,
            extHeight = extHeight,
            popupX = popupX,
            popupY = popupY,
        )
    }
}

enum class AnchorSide { LEFT, RIGHT }

data class KeyDimensionsInput(
    val containerWidth: Int,
    val keyMarginH: Int,
    val keyMarginV: Int,
    /** Pixel value from `resources.getDimension(R.dimen.key_height)`. Kept as
     *  `Float` so the multiplier chain truncates to `Int` only once at the end,
     *  matching the legacy in-View arithmetic. */
    val baseKeyHeight: Float,
    val isLandscape: Boolean,
    val keyHeightScale: Float,
)

/** Solved key geometry; also echoes the inputs the per-key layout needs, so the layout
 *  reads margins and orientation from the same values the solver used. */
data class KeyDimensions(
    val desiredKeyWidth: Int,
    val desiredKeyHeight: Int,
    val keyHeightFactor: Float,
    val containerWidth: Int,
    val keyMarginH: Int,
    val keyMarginV: Int,
    val isLandscape: Boolean,
)

data class PopupDimensionsInput(
    val desiredKeyWidth: Int,
    val desiredKeyHeight: Int,
    val keyViewMeasuredWidth: Int,
    val isLandscape: Boolean,
)

data class PopupDimensions(
    val popupWidth: Int,
    val popupHeight: Int,
    val popupDiffX: Int,
    val popupX: Int,
    val popupY: Int,
)

data class ExtendedPopupGeometryInput(
    val popupCount: Int,
    val keyViewX: Float,
    val keyboardViewMeasuredWidth: Int,
    val keyViewMeasuredWidth: Int,
    val keyViewMeasuredHeight: Int,
    val keyPopupWidth: Int,
    val keyPopupHeight: Int,
)

data class ExtendedPopupGeometry(
    val anchorSide: AnchorSide,
    val row0count: Int,
    val row1count: Int,
    val anchorOffset: Int,
    val extWidth: Int,
    val extHeight: Int,
    val popupX: Int,
    val popupY: Int,
)
