package com.siansiansu.taigikeyboard.ime.popup

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.text.keyboard.AnchorSide
import androidx.compose.ui.text.font.Typeface as ComposeTypeface

/** Preview popup — TextView + optional three-dots indicator below. */
@Composable
fun KeyPopupBox(state: PreviewState.Visible) {
    val display = state.display
    val density = LocalDensity.current
    val keyHeightDp = with(density) { display.keyHeightPx.toDp() }
    val threeDotsSizeDp = with(density) { display.threeDotsSizePx.toDp() }
    val popupTextSp = with(density) { display.popupTextSizePx.toSp() }

    PopupBackground(display = display, modifier = Modifier.fillMaxSize()) {
        Column(modifier = Modifier.fillMaxSize()) {
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .height(keyHeightDp),
                contentAlignment = Alignment.Center,
            ) {
                BasicText(
                    text = state.label,
                    style =
                        TextStyle(
                            color = Color(display.fgColorArgb),
                            fontSize = popupTextSp,
                            textAlign = TextAlign.Center,
                        ),
                )
            }
            Box(
                modifier = Modifier.fillMaxWidth(),
                contentAlignment = Alignment.CenterEnd,
            ) {
                if (state.showThreeDots) {
                    Image(
                        painter = painterResource(R.drawable.ic_more_horiz),
                        contentDescription = stringRes(StringKey.KEYBOARD_MORE_POPUP_HINT),
                        modifier = Modifier.size(threeDotsSizeDp),
                        colorFilter =
                            ColorFilter.tint(Color(display.fgColorArgb), BlendMode.SrcAtop),
                    )
                } else {
                    // Reserve space identical to legacy `View.INVISIBLE` so the
                    // preview popup height stays stable across show/hide of
                    // the indicator.
                    Spacer(modifier = Modifier.size(threeDotsSizeDp))
                }
            }
        }
    }
}

/** Extended popup — multi-row glyph picker built from solver-supplied row counts. */
@Composable
fun KeyPopupExtendedBox(state: ExtendedState.Visible) {
    val display = state.display
    val density = LocalDensity.current
    val widthDp = with(density) { state.totalWidthPx.toDp() }
    val heightDp = with(density) { state.totalHeightPx.toDp() }
    val cellWidthDp = with(density) { state.cellWidthPx.toDp() }
    val cellHeightDp = with(density) { state.cellHeightPx.toDp() }

    val customFontFamily =
        remember(display.typeface) {
            FontFamily(ComposeTypeface(display.typeface))
        }

    PopupBackground(
        display = display,
        modifier = Modifier.size(widthDp, heightDp),
    ) {
        Column(modifier = Modifier.fillMaxSize()) {
            if (state.row1count > 0) {
                ExtendedRow(
                    cells = state.cells.subList(0, state.row1count),
                    cellAbsoluteOffset = 0,
                    activeIndex = state.activeIndex,
                    cellWidthDp = cellWidthDp,
                    cellHeightDp = cellHeightDp,
                    display = display,
                    customFontFamily = customFontFamily,
                    anchorSide = state.anchorSide,
                )
            }
            ExtendedRow(
                cells = state.cells.subList(state.row1count, state.cells.size),
                cellAbsoluteOffset = state.row1count,
                activeIndex = state.activeIndex,
                cellWidthDp = cellWidthDp,
                cellHeightDp = cellHeightDp,
                display = display,
                customFontFamily = customFontFamily,
                anchorSide = AnchorSide.LEFT,
            )
        }
    }
}

@Composable
private fun ExtendedRow(
    cells: List<PopupCell>,
    cellAbsoluteOffset: Int,
    activeIndex: Int,
    cellWidthDp: androidx.compose.ui.unit.Dp,
    cellHeightDp: androidx.compose.ui.unit.Dp,
    display: PopupDisplayParams,
    customFontFamily: FontFamily,
    anchorSide: AnchorSide,
) {
    val arrangement =
        if (anchorSide == AnchorSide.LEFT) Arrangement.Start else Arrangement.End
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(cellHeightDp),
        horizontalArrangement = arrangement,
    ) {
        cells.forEachIndexed { idx, cell ->
            ExtendedCell(
                cell = cell,
                isActive = (cellAbsoluteOffset + idx) == activeIndex,
                cellWidthDp = cellWidthDp,
                cellHeightDp = cellHeightDp,
                display = display,
                customFontFamily = customFontFamily,
            )
        }
    }
}

@Composable
private fun ExtendedCell(
    cell: PopupCell,
    isActive: Boolean,
    cellWidthDp: androidx.compose.ui.unit.Dp,
    cellHeightDp: androidx.compose.ui.unit.Dp,
    display: PopupDisplayParams,
    customFontFamily: FontFamily,
) {
    val density = LocalDensity.current
    val cornerRadiusDp = with(density) { display.cornerRadiusPx.toDp() }
    val cornerShape = remember(cornerRadiusDp) { RoundedCornerShape(cornerRadiusDp) }
    val cellBg =
        if (isActive) Color(display.extBgColorActiveArgb) else Color(display.extBgColorArgb)

    Box(
        modifier =
            Modifier
                .width(cellWidthDp)
                .height(cellHeightDp)
                .clip(cornerShape)
                .background(cellBg),
        contentAlignment = Alignment.Center,
    ) {
        when {
            cell.icon != null -> {
                val cellHeightPx = with(density) { cellHeightDp.toPx() }
                val cellWidthPx = with(density) { cellWidthDp.toPx() }
                val drawablePadding = (display.iconPaddingFraction * cellHeightPx).toInt()
                val sideMin = minOf(cellWidthPx, cellHeightPx).toInt()
                val iconSizePx = (sideMin - 2 * drawablePadding).coerceAtLeast(0)
                Image(
                    painter = painterResource(cell.icon.drawableRes),
                    contentDescription = null,
                    modifier = Modifier.size(with(density) { iconSizePx.toDp() }),
                    colorFilter =
                        ColorFilter.tint(Color(display.fgColorArgb), BlendMode.SrcAtop),
                )
            }
            cell.label != null -> {
                val fontFamily =
                    if (cell.useCustomTypeface) customFontFamily else FontFamily.Default
                val textPx = display.popupTextSizePx * cell.textScale
                BasicText(
                    text = cell.label,
                    style =
                        TextStyle(
                            color = Color(display.fgColorArgb),
                            fontSize = with(density) { textPx.toSp() },
                            fontFamily = fontFamily,
                            textAlign = TextAlign.Center,
                        ),
                )
            }
        }
    }
}

/**
 * Two-layer rounded-rect popup background — outer shadow ring + 1dp padding +
 * inner filled rect, both with the same corner radius. Not [Modifier.shadow]
 * (which is an elevation blur, not a flat colored ring).
 */
@Composable
private fun PopupBackground(
    display: PopupDisplayParams,
    modifier: Modifier = Modifier,
    content: @Composable BoxScope.() -> Unit,
) {
    val density = LocalDensity.current
    val cornerRadiusDp = with(density) { display.cornerRadiusPx.toDp() }
    val ringWidthDp = with(density) { display.ringWidthPx.toDp() }
    val cornerShape = remember(cornerRadiusDp) { RoundedCornerShape(cornerRadiusDp) }
    Box(
        modifier =
            modifier
                .background(Color(display.shadowColorArgb), cornerShape)
                .padding(ringWidthDp)
                .background(Color(display.bgColorArgb), cornerShape),
        content = content,
    )
}
