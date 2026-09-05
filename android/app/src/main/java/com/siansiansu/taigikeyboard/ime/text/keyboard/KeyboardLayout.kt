// Main keyboard body Composable. A custom Layout block reproduces the legacy FlexboxLayout
// behavior (width multipliers, flex-shrink, SPACE flex-grow); touch dispatch goes through
// pointerInteropFilter -> KeyTouchCoordinator to match legacy KeyboardView.onTouchEvent 1:1.

package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.content.res.Configuration
import android.view.MotionEvent
import androidx.compose.foundation.background
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInteropFilter
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.Measurable
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.popup.PopupHost
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyType
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.theme.getColorFromAttr

/**
 * Top-level keyboard body Composable. Consumes [KeyboardLayoutData] (P2) +
 * [KeyDimensions] (P2 solver output) and lays out one [KeyContent] per key
 * via a custom [Layout] block that mirrors the legacy FlexboxLayout behavior:
 * per-mode width multipliers, flex-shrink (fit overflowing rows), flex-grow
 * (SPACE absorbs slack).
 *
 * Touch dispatch is wired through `Modifier.pointerInteropFilter` into a
 * caller-supplied [KeyTouchCoordinator] so multi-pointer + popup hand-off
 * semantics match `KeyboardView.onTouchEvent` / `KeyView.onFlorisTouchEvent`
 * 1:1.
 */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun KeyboardLayout(
    layoutData: KeyboardLayoutData,
    keyDimensions: KeyDimensions,
    appearance: KeyboardAppearance,
    keyVariation: KeyVariation,
    coordinator: KeyTouchCoordinator,
    popupHost: PopupHost,
    modifier: Modifier = Modifier,
    isPreview: Boolean = false,
) {
    val density = LocalDensity.current
    val keyMarginH = with(density) { 2.dp.roundToPx() }
    val keyMarginV = with(density) { 5.dp.roundToPx() }
    val rowMarginH = keyMarginH

    var pressedKeyId by remember { mutableStateOf<Long?>(null) }
    if (!isPreview) {
        LaunchedEffect(coordinator) {
            coordinator.onPressedKeyChanged = { pressedKeyId = it }
        }
    }

    val context = LocalContext.current
    val themeColors = remember(context) { ThemePalette.from(context) }
    val themeBgColor = remember(context) { Color(getColorFromAttr(context, R.attr.keyboard_bgColor)) }
    // Gradient themes paint the background once on the common View parent
    // (`text_input_content`, via KeyboardThemeSurfaceController); the Compose body
    // stays transparent so that gradient shows continuously candidate-bar -> keys.
    // Flat themes keep the legacy custom-fill-or-theme background.
    val bgColor = when {
        appearance.colorSettings.hasBackgroundGradient -> Color.Transparent
        else -> appearance.colorSettings.backgroundColor?.let { Color(it) } ?: themeBgColor
    }

    val touchModifier = Modifier.pointerInteropFilter { event ->
        if (!isPreview) return@pointerInteropFilter coordinator.onMotionEvent(event)
        // Preview mode: visual pressed state only — no key dispatch.
        // Mirrors legacy KeyboardView.onPreviewTouchEvent.
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_MOVE -> {
                val hit = coordinator.previewHitTest(event.x.toInt(), event.y.toInt())
                if (hit != pressedKeyId) pressedKeyId = hit
                true
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                pressedKeyId = null
                true
            }
            else -> false
        }
    }

    val visibleRows = remember(layoutData, keyVariation) {
        layoutData.rows.map { row ->
            row.filter { keyVisible(it, keyVariation) }
        }
    }

    Layout(
        modifier = modifier
            .background(bgColor)
            .then(touchModifier),
        content = {
            for ((rowIndex, row) in visibleRows.withIndex()) {
                for ((indexInRow, key) in row.withIndex()) {
                    KeyContent(
                        data = key,
                        mode = layoutData.mode,
                        keyboardLayoutType = appearance.keyboardLayoutType,
                        inputMode = appearance.inputMode,
                        caps = appearance.caps,
                        capsLock = appearance.capsLock,
                        isComposing = appearance.isComposing,
                        isTranslateSwapped = appearance.isTranslateSwapped,
                        imeOptions = appearance.imeOptions,
                        confirmKeyLabel = appearance.confirmKeyLabel,
                        colors = appearance.colorSettings,
                        typeface = appearance.typeface,
                        fontSizeScale = appearance.keyFontSizeScale,
                        cornerRadiusDp = appearance.keyCornerRadius,
                        borderWidthDp = appearance.keyBorderWidth,
                        shadowIntensity = appearance.keyShadowIntensity,
                        isPreview = isPreview,
                        pressed = pressedKeyId == idFor(rowIndex, indexInRow),
                        themeColors = themeColors,
                    )
                }
            }
        },
    ) { measurables, constraints ->
        val containerWidth = constraints.maxWidth
        val available = (containerWidth - 2 * rowMarginH).coerceAtLeast(0)
        val rowHeight = keyDimensions.desiredKeyHeight
        val cellSlotHeight = rowHeight + 2 * keyMarginV
        val totalHeight = cellSlotHeight * visibleRows.size

        // Walk measurables in row-major order, slicing per row.
        val placements = ArrayList<Placement>(measurables.size)
        val resolvedBounds = ArrayList<KeyBounds>(measurables.size)
        var measurableCursor = 0
        for ((rowIndex, row) in visibleRows.withIndex()) {
            val rowMeasurables = measurables.subList(measurableCursor, measurableCursor + row.size)
            measurableCursor += row.size
            val rowKeys = row
            val rowResult = layoutRow(
                row = rowKeys,
                rowIndex = rowIndex,
                rowMeasurables = rowMeasurables,
                desiredKeyWidth = keyDimensions.desiredKeyWidth,
                desiredKeyHeight = rowHeight,
                keyMarginH = keyMarginH,
                keyMarginV = keyMarginV,
                rowMarginH = rowMarginH,
                available = available,
                rowTop = rowIndex * cellSlotHeight,
                keyboardLayoutType = appearance.keyboardLayoutType,
                mode = layoutData.mode,
                rootWidth = containerWidth,
            )
            placements.addAll(rowResult.placements)
            resolvedBounds.addAll(rowResult.bounds)
        }

        coordinator.updateBounds(
            newBounds = resolvedBounds,
            keyboardWidth = containerWidth,
            desiredKeyWidth = keyDimensions.desiredKeyWidth,
            desiredKeyHeight = keyDimensions.desiredKeyHeight,
        )

        layout(containerWidth, totalHeight) {
            for (p in placements) {
                p.placeable.placeRelative(p.x, p.y)
            }
        }
    }
}

/** Resolved placement of a single key composable. */
private data class Placement(
    val placeable: androidx.compose.ui.layout.Placeable,
    val x: Int,
    val y: Int,
)

private data class RowLayoutResult(
    val placements: List<Placement>,
    val bounds: List<KeyBounds>,
)

/**
 * Per-row flex algorithm — port of FlexboxLayout's behavior under the legacy
 * `flexShrink` / `flexGrow` rules (see `KeyView.init { layoutParams.apply { ... } }`):
 *
 * - sum desired widths
 * - if overflow: shrink keys with `flexShrink == 1` proportionally
 * - if underflow: distribute slack evenly across keys with `flexGrow == 1`
 *   (in CHARACTERS mode, only SPACE has flexGrow=1)
 */
@Suppress("LongParameterList")
private fun androidx.compose.ui.layout.MeasureScope.layoutRow(
    row: List<KeyData>,
    rowIndex: Int,
    rowMeasurables: List<Measurable>,
    desiredKeyWidth: Int,
    desiredKeyHeight: Int,
    keyMarginH: Int,
    keyMarginV: Int,
    rowMarginH: Int,
    available: Int,
    rowTop: Int,
    keyboardLayoutType: String,
    mode: KeyboardMode,
    rootWidth: Int,
): RowLayoutResult {
    val n = row.size
    if (n == 0) return RowLayoutResult(emptyList(), emptyList())

    val rawWidths = IntArray(n)
    for (i in 0 until n) {
        rawWidths[i] = desiredWidthFor(row[i], mode, desiredKeyWidth, keyboardLayoutType)
    }
    val cellWidthsWithMargins = IntArray(n) { rawWidths[it] + 2 * keyMarginH }
    var totalWidth = cellWidthsWithMargins.sum()

    if (totalWidth > available) {
        val shrinkable = ArrayList<Int>(n)
        var shrinkableSum = 0
        for (i in 0 until n) {
            if (flexShrinkFor(row[i], mode) == 1f) {
                shrinkable.add(i)
                shrinkableSum += rawWidths[i]
            }
        }
        if (shrinkable.isNotEmpty() && shrinkableSum > 0) {
            val overflow = totalWidth - available
            val ratio = overflow.toFloat() / shrinkableSum
            for (i in shrinkable) {
                val shrink = (rawWidths[i] * ratio).toInt()
                rawWidths[i] = (rawWidths[i] - shrink).coerceAtLeast(1)
                cellWidthsWithMargins[i] = rawWidths[i] + 2 * keyMarginH
            }
            totalWidth = cellWidthsWithMargins.sum()
        }
    } else if (totalWidth < available) {
        val growable = ArrayList<Int>(n)
        for (i in 0 until n) {
            if (flexGrowFor(row[i], mode) == 1f) growable.add(i)
        }
        if (growable.isNotEmpty()) {
            val slack = available - totalWidth
            val perKey = slack / growable.size
            val remainder = slack - perKey * growable.size
            for ((idx, i) in growable.withIndex()) {
                val extra = perKey + if (idx == growable.size - 1) remainder else 0
                rawWidths[i] += extra
                cellWidthsWithMargins[i] += extra
            }
            totalWidth = cellWidthsWithMargins.sum()
        }
    }

    val placements = ArrayList<Placement>(n)
    val bounds = ArrayList<KeyBounds>(n)
    var xCursor = rowMarginH + (available - totalWidth).coerceAtLeast(0) / 2
    for (i in 0 until n) {
        val width = rawWidths[i]
        val height = desiredKeyHeight
        val placeable = rowMeasurables[i].measure(
            Constraints.fixed(width, height),
        )
        val keyLeft = xCursor + keyMarginH
        val keyTop = rowTop + keyMarginV
        placements.add(Placement(placeable, keyLeft, keyTop))

        // Hit-box: include margin band; first/last key in row stretches to
        // the keyboard root edge so finger drift to the border still hits
        // the corner key. Mirrors KeyView.updateTouchHitBox.
        val isFirst = i == 0
        val isLast = i == n - 1
        val hitLeft = if (isFirst) 0 else keyLeft - keyMarginH
        val hitRight = if (isLast) rootWidth else keyLeft + width + keyMarginH
        val hitTop = keyTop - keyMarginV
        val hitBottom = keyTop + height + keyMarginV
        bounds.add(
            KeyBounds(
                keyId = idFor(rowIndex, i),
                data = row[i],
                visible = Bounds(keyLeft, keyTop, keyLeft + width, keyTop + height),
                hit = Bounds(hitLeft, hitTop, hitRight, hitBottom),
                rowIndex = rowIndex,
                isFirstInRow = isFirst,
                isLastInRow = isLast,
            ),
        )

        xCursor += width + 2 * keyMarginH
    }
    return RowLayoutResult(placements, bounds)
}

/**
 * Per-key desired width — direct port of `KeyView.onMeasure` per-mode +
 * per-key width logic. Returns the key cell width in pixels (margin not
 * included).
 */
private fun desiredWidthFor(
    key: KeyData,
    mode: KeyboardMode,
    desiredKeyWidth: Int,
    keyboardLayoutType: String,
): Int =
    when (mode) {
        KeyboardMode.NUMERIC, KeyboardMode.PHONE, KeyboardMode.PHONE2 ->
            (desiredKeyWidth * 2.68f).toInt()
        KeyboardMode.NUMERIC_ADVANCED -> when (key.code) {
            44, 46 -> desiredKeyWidth
            KeyCode.VIEW_SYMBOLS, 61 -> (desiredKeyWidth * 1.34f).toInt()
            else -> (desiredKeyWidth * 1.56f).toInt()
        }
        else -> when (key.code) {
            KeyCode.SHIFT, KeyCode.VIEW_CHARACTERS, KeyCode.VIEW_SYMBOLS,
            KeyCode.VIEW_SYMBOLS2, KeyCode.DELETE, KeyCode.ENTER,
            ->
                (desiredKeyWidth * 1.56f).toInt()
            KeyCode.TRANSLATE -> {
                val scale = when (keyboardLayoutType) {
                    "phahTaigi", "moe1" -> 2.0f
                    else -> 1.5f
                }
                (desiredKeyWidth * scale).toInt()
            }
            KeyCode.SPACE -> when (mode) {
                KeyboardMode.SYMBOLS -> (desiredKeyWidth * 0.56f).toInt()
                else -> desiredKeyWidth
            }
            else -> desiredKeyWidth
        }
    }

private fun flexShrinkFor(
    key: KeyData,
    mode: KeyboardMode,
): Float =
    when (mode) {
        KeyboardMode.NUMERIC, KeyboardMode.NUMERIC_ADVANCED,
        KeyboardMode.PHONE, KeyboardMode.PHONE2,
        -> 1f
        else -> when (key.code) {
            KeyCode.SHIFT, KeyCode.VIEW_CHARACTERS, KeyCode.VIEW_SYMBOLS,
            KeyCode.VIEW_SYMBOLS2, KeyCode.DELETE, KeyCode.ENTER, KeyCode.TRANSLATE,
            -> 0f
            else -> 1f
        }
    }

private fun flexGrowFor(
    key: KeyData,
    mode: KeyboardMode,
): Float =
    when (mode) {
        KeyboardMode.NUMERIC, KeyboardMode.PHONE, KeyboardMode.PHONE2 -> 0f
        KeyboardMode.NUMERIC_ADVANCED -> when (key.type) {
            KeyType.NUMERIC -> 1f
            else -> 0f
        }
        else -> when (key.code) {
            KeyCode.SPACE -> 1f
            else -> 0f
        }
    }

/**
 * Visibility predicate — direct port of `KeyView.updateVisibility`. ALL is
 * always visible; NORMAL is visible for NORMAL/PASSWORD variation; otherwise
 * exact match required.
 */
private fun keyVisible(
    key: KeyData,
    keyVariation: KeyVariation,
): Boolean {
    if (key.variation == KeyVariation.ALL) return true
    if (key.variation == KeyVariation.NORMAL &&
        (keyVariation == KeyVariation.NORMAL || keyVariation == KeyVariation.PASSWORD)
    ) {
        return true
    }
    return key.variation == keyVariation
}

/** Stable per-key identity within a single layout. (rowIndex, indexInRow)
 *  is unique by construction — keying on `KeyData.code` would collide for
 *  TPS / MOE2 layouts that contain multiple `code == 0` placeholder keys
 *  in the same row (each carries distinct popup variants). */
internal fun idFor(
    rowIndex: Int,
    indexInRow: Int,
): Long = (rowIndex.toLong() shl 32) or (indexInRow.toLong() and 0xFFFFFFFFL)

/**
 * Aggregates the runtime appearance + caps + composing state the keyboard
 * needs at render time. Resolved on the IME side once per push so per-key
 * Composables stay pure renderers of state. Mirrors the Phase B
 * `CandidateDisplayParams` snapshot pattern.
 */
data class KeyboardAppearance(
    val keyboardLayoutType: String,
    val inputMode: String,
    val caps: Boolean,
    val capsLock: Boolean,
    val isComposing: Boolean,
    val isTranslateSwapped: Boolean,
    val imeOptions: Int,
    val confirmKeyLabel: String,
    val colorSettings: KeyboardColorSettings,
    val typeface: android.graphics.Typeface,
    val keyFontSizeScale: Float,
    val keyCornerRadius: Float,
    val keyBorderWidth: Float,
    /** Per-key drop-shadow intensity (0 = none, 1..4 grow). Resolved from the
     *  active theme; flat for the legacy/default theme. Mirrors iOS
     *  ThemeAppearance.keyShadowIntensity. */
    val keyShadowIntensity: Float,
    /** Drives [KeyboardLayoutSolver.solveKeyDimensions] inside
     *  [KeyboardImeRoot]. Surfaced through appearance because per-key
     *  dimensions only need to change when these prefs flip — not on
     *  per-keystroke recomposition. */
    val heightFactor: KeyboardHeightFactor,
    val keyHeightScale: Float,
) {
    companion object {
        /** Confirm-key label per input mode (used by KeyView). Keycap content, not app chrome — not i18n. */
        fun confirmKeyLabel(
            inputMode: String,
            isTranslateSwapped: Boolean,
        ): String =
            when {
                inputMode == "tps" || isTranslateSwapped -> "選"
                inputMode == "poj" -> "soán"
                else -> "suán"
            }
    }
}

/** Detect landscape orientation — used inside [KeyboardImeRoot] when solving
 *  [KeyDimensionsInput.isLandscape] and inside the settings preview panel. */
@Composable
fun isLandscape(): Boolean = LocalConfiguration.current.orientation == Configuration.ORIENTATION_LANDSCAPE
