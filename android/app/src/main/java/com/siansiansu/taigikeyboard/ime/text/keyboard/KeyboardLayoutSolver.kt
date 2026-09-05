// 純算式鍵盤幾何 solver — 所有輸入皆為純值型別,可在 JVM 單元測試直接跑,不需 Android 儀器化。
// 三大職責:per-key 尺寸與 keyHeightFactor、preview popup 尺寸與位移、extended popup 錨點與螢幕邊夾擠。

package com.siansiansu.taigikeyboard.ime.text.keyboard

/**
 * Pure layout-math solver for keyboard geometry. All inputs are scalar value
 * types so the math is JVM-testable without Android instrumentation.
 *
 * Three decoupled responsibilities, mirroring the legacy in-View math:
 * - [solveKeyDimensions] — per-key width and height, plus the live
 *   `keyHeightFactor` smartbar consumes
 * - [solvePopupDimensions] — preview-popup dimensions and offset
 * - [solveExtendedPopupGeometry] — extended-popup anchor side, row split,
 *   anchor offset, and final placement under screen-edge constraints
 */
object KeyboardLayoutSolver {
    fun solveKeyDimensions(input: KeyDimensionsInput): KeyDimensions {
        val orientationFactor = if (input.isLandscape) 0.85f else 1.0f
        val keyHeightFactor = orientationFactor * input.heightFactor.multiplier * input.keyHeightScale
        val keyMarginH = input.keyMarginH
        val desiredKeyWidth = (input.containerWidth / 10) - (2 * keyMarginH)
        val desiredKeyHeight = (input.baseKeyHeight * keyHeightFactor).toInt()
        return KeyDimensions(
            desiredKeyWidth = desiredKeyWidth,
            desiredKeyHeight = desiredKeyHeight,
            keyHeightFactor = keyHeightFactor,
        )
    }

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

enum class KeyboardHeightFactor(
    val multiplier: Float,
) {
    EXTRA_SHORT(0.85f),
    SHORT(0.90f),
    MID_SHORT(0.95f),
    NORMAL(1.00f),
    MID_TALL(1.05f),
    TALL(1.10f),
    EXTRA_TALL(1.15f),
    ;

    companion object {
        fun fromPreferenceString(value: String): KeyboardHeightFactor =
            when (value) {
                "extra_short" -> EXTRA_SHORT
                "short" -> SHORT
                "mid_short" -> MID_SHORT
                "normal" -> NORMAL
                "mid_tall" -> MID_TALL
                "tall" -> TALL
                "extra_tall" -> EXTRA_TALL
                else -> NORMAL
            }
    }
}

enum class AnchorSide { LEFT, RIGHT }

data class KeyDimensionsInput(
    val containerWidth: Int,
    val keyMarginH: Int,
    /** Pixel value from `resources.getDimension(R.dimen.key_height)`. Kept as
     *  `Float` so the multiplier chain truncates to `Int` only once at the end,
     *  matching the legacy in-View arithmetic. */
    val baseKeyHeight: Float,
    val isLandscape: Boolean,
    val heightFactor: KeyboardHeightFactor,
    val keyHeightScale: Float,
)

data class KeyDimensions(
    val desiredKeyWidth: Int,
    val desiredKeyHeight: Int,
    val keyHeightFactor: Float,
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
