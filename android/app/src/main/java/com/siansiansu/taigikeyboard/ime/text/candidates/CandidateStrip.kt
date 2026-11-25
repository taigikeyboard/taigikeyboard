// Candidate-strip root composable. Click handling is injected by the caller
// (see CandidateClickHandler); hosted inside SmartbarView's ComposeView.

package com.siansiansu.taigikeyboard.ime.text.candidates

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.LineHeightStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.typeface.TypefaceLoader
import androidx.compose.ui.text.font.Typeface as ComposeTypeface

/** Taigi candidate strip — horizontally scrolling lazy row of candidates. */
@Composable
fun TaigiCandidateStrip(
    state: CandidateStripState,
    onCandidateClick: (TaigiWord, Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    val taigiMode = state.mode as? CandidateMode.Taigi
    if (taigiMode == null) {
        Box(modifier = modifier.fillMaxSize())
        return
    }

    val display = state.display
    val items = taigiMode.items
    val listState = rememberLazyListState()

    LaunchedEffect(state.updateSeq) {
        listState.scrollToItem(0)
    }

    val context = LocalContext.current
    val fontFamily =
        remember(display.fontType) {
            FontFamily(ComposeTypeface(TypefaceLoader.getTypefaceByType(display.fontType, context)))
        }

    // §42: content-level sizing — when NO shown cell carries a subtitle
    // (Romanization Only mode; mixed split cells; an all-roman list) the title is sized for
    // one line instead of reserving the 58/42 split. Subtitle presence never
    // depends on the TPS re-render of `roman`, so `word.roman` suffices here.
    val contentHasSubtitles =
        remember(items, display.candidateDisplayMode, display.isHanjiFirst, display.layoutType) {
            items.any { word ->
                val cell =
                    candidateCellText(
                        hanji = word.hanji,
                        displayRoman = word.roman,
                        isTPSLayout = display.layoutType == "tps",
                        candidateDisplayMode = display.candidateDisplayMode,
                        isHanjiFirst = display.isHanjiFirst,
                        cellScript = word.additionalInfo[TaigiWord.MetadataKeys.CELL_SCRIPT],
                    )
                cell.showsSubtitle
            }
        }

    // Cell-invariant font sizing (content-level, §42): computed once per strip
    // instead of one identical remember slot per cell.
    val res = LocalResources.current
    val configuration = LocalConfiguration.current
    val paddingPx = remember(configuration) { res.getDimensionPixelSize(R.dimen.smartbar_button_padding) }
    val marginPx = remember(configuration) { res.getDimensionPixelSize(R.dimen.smartbar_button_margin) }
    val candidateFontSizes =
        remember(
            display.smartbarHeightPx,
            display.textSizeScale,
            paddingPx,
            marginPx,
            contentHasSubtitles,
            configuration,
        ) {
            computeCandidateFontSizes(
                smartbarHeightPx = display.smartbarHeightPx,
                paddingPx = paddingPx,
                marginPx = marginPx,
                density = res.displayMetrics.density,
                fontScale = configuration.fontScale,
                textSizeScale = display.textSizeScale,
                contentHasSubtitles = contentHasSubtitles,
            )
        }

    // The strip is transparent: the theme surface (solid / gradient) is painted on the
    // common parent, so the candidate bar shares it with the keyboard.
    LazyRow(
        state = listState,
        modifier = modifier.fillMaxSize(),
    ) {
        // No explicit `key` lambda: candidate ids are not unique (e.g.
        // custom-dictionary markers collide on `TaigiWord(id = -2, …)`) and
        // every state push fully replaces the list anyway, so position-based
        // identity is correct here.
        itemsIndexed(items) { index, word ->
            CandidateCell(
                word = word,
                index = index,
                display = display,
                fontFamily = fontFamily,
                fontSizes = candidateFontSizes,
                onClick = { onCandidateClick(word, index) },
            )
        }
    }
}

/**
 * English 3-column candidate strip — fixed-weight row of up to 3 cells.
 * Renders only when [CandidateStripState.mode] is [CandidateMode.English].
 * No scroll state.
 */
@Composable
fun EnglishCandidateStrip(
    state: CandidateStripState,
    onEnglishCandidateClick: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    val englishMode = state.mode as? CandidateMode.English
    if (englishMode == null) {
        Box(modifier = modifier.fillMaxSize())
        return
    }

    val display = state.display
    val items = englishMode.items
    val configuration = LocalConfiguration.current

    // Match legacy SmartbarEnglishCandidate style: sans-serif, independent of
    // the user's Taigi candidate font preference.
    val fontFamily = FontFamily.SansSerif

    val englishTextSp =
        remember(display.smartbarHeightPx, configuration.densityDpi, configuration.fontScale) {
            val scaledDensity = configuration.densityDpi / 160f * configuration.fontScale
            val englishTextPx = display.smartbarHeightPx * 0.36f
            (englishTextPx / scaledDensity).coerceAtLeast(8f)
        }

    Row(
        modifier = modifier.fillMaxSize(),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        for (slot in 0..2) {
            val word = items.getOrNull(slot)
            EnglishCandidateCell(
                modifier = Modifier.weight(1f).fillMaxHeight(),
                roman = word?.roman.orEmpty(),
                visible = word != null,
                textSp = englishTextSp,
                // Role-first like the Taigi cell (:240): a light-only theme's fixed
                // candidateTextColor wins so the text stays dark on a light gradient
                // in system dark mode; null (adaptive default) falls back to the attr.
                titleColor = display.candidateTextColor?.let { Color(it) } ?: Color(display.themeTitleColor),
                pressedHighlight = Color(display.themePressedHighlightColor),
                fontFamily = fontFamily,
                onClick = { onEnglishCandidateClick(slot) },
            )
            if (slot < 2) {
                // Theme candidate text at full opacity, like the expanded-grid lines.
                // CROSS-PLATFORM INVARIANT — mirrors iOS CandidateSuggestionsRow englishCandidateToolbarStyle separator.
                EnglishDivider(
                    color = display.candidateTextColor?.let { Color(it) } ?: Color(display.themePressedHighlightColor),
                )
            }
        }
    }
}

@Composable
private fun CandidateCell(
    word: TaigiWord,
    index: Int,
    display: CandidateDisplayParams,
    fontFamily: FontFamily,
    fontSizes: Pair<Float, Float>,
    onClick: () -> Unit,
) {
    val density = LocalDensity.current
    val configuration = LocalConfiguration.current
    val res = LocalResources.current

    val paddingPx = remember(configuration) { res.getDimensionPixelSize(R.dimen.smartbar_button_padding) }
    val marginPx = remember(configuration) { res.getDimensionPixelSize(R.dimen.smartbar_button_margin) }
    val horizontalSpacing = with(density) { (marginPx * 3).toDp() }
    val verticalSpacing = with(density) { (marginPx * 2).toDp() }
    val innerHorizontalPadding = with(density) { paddingPx.toDp() }
    val innerVerticalPadding = with(density) { (paddingPx / 3).toDp() }

    val cornerRadius = 8.dp

    val interactionSource = remember { MutableInteractionSource() }
    val isPressed by interactionSource.collectIsPressedAsState()

    // First candidate (engine ranker top, index 0) gets a filled keycap-color background as a
    // visual hint — matches the iOS strip + the Rime-family highlighted-candidate convention.
    val backgroundColor =
        candidateCellBackground(
            isPressed = isPressed,
            isFirstCandidate = index == 0,
            firstCandidateColor = Color(display.themeKeyBgColor),
            pressedColor = Color(display.themePressedHighlightColor),
        )

    val isTPSLayout = display.layoutType == "tps"
    val displayRoman = if (isTPSLayout) word.tpsText else word.roman
    val cell =
        candidateCellText(
            hanji = word.hanji,
            displayRoman = displayRoman,
            isTPSLayout = isTPSLayout,
            candidateDisplayMode = display.candidateDisplayMode,
            isHanjiFirst = display.isHanjiFirst,
            cellScript = word.additionalInfo[TaigiWord.MetadataKeys.CELL_SCRIPT],
        )
    val (titleText, subtitleText) = cell
    val showSubtitle = cell.showsSubtitle

    // Hoisted by TaigiCandidateStrip — the sizes are cell-invariant (content-level sizing).
    val (titleSp, subtitleSp) = fontSizes

    val titleColor = display.candidateTextColor?.let { Color(it) } ?: Color(display.themeTitleColor)
    val subtitleColor = display.candidateTextColor?.let { Color(it) } ?: Color(display.themeSubtitleColor)

    Box(
        modifier =
            Modifier
                .fillMaxHeight()
                .padding(horizontal = horizontalSpacing, vertical = verticalSpacing)
                .clickable(
                    interactionSource = interactionSource,
                    indication = null,
                    onClick = onClick,
                ),
        contentAlignment = Alignment.Center,
    ) {
        Box(
            modifier =
                Modifier
                    .matchParentSize()
                    .clip(RoundedCornerShape(cornerRadius))
                    .background(backgroundColor),
        )
        Column(
            modifier =
                Modifier
                    .widthIn(min = 44.dp)
                    .padding(horizontal = innerHorizontalPadding, vertical = innerVerticalPadding)
                    .wrapContentSize(),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            BasicText(
                text = titleText,
                style = candidateLineStyle(titleSp, titleColor, fontFamily),
            )
            if (showSubtitle) {
                BasicText(
                    text = subtitleText!!,
                    style = candidateLineStyle(subtitleSp, subtitleColor, fontFamily),
                    modifier = Modifier.padding(top = with(density) { (marginPx * 2).toDp() }),
                )
            }
        }
    }
}

@Composable
private fun EnglishCandidateCell(
    modifier: Modifier,
    roman: String,
    visible: Boolean,
    textSp: Float,
    titleColor: Color,
    pressedHighlight: Color,
    fontFamily: FontFamily,
    onClick: () -> Unit,
) {
    val interactionSource = remember { MutableInteractionSource() }
    val isPressed by interactionSource.collectIsPressedAsState()

    if (!visible) {
        Box(modifier = modifier)
        return
    }

    Box(
        modifier =
            modifier
                .clickable(
                    interactionSource = interactionSource,
                    indication = null,
                    onClick = onClick,
                ),
        contentAlignment = Alignment.Center,
    ) {
        Box(
            modifier =
                Modifier
                    .matchParentSize()
                    .clip(RoundedCornerShape(8.dp))
                    .background(if (isPressed) pressedHighlight else Color.Transparent),
        )
        BasicText(
            text = roman,
            style =
                TextStyle(
                    color = titleColor,
                    fontSize = textSp.sp,
                    fontFamily = fontFamily,
                    textAlign = TextAlign.Center,
                ),
        )
    }
}

@Composable
private fun EnglishDivider(color: Color) {
    Box(
        modifier =
            Modifier
                .width(1.dp)
                .height(24.dp)
                .background(color),
    )
}

/**
 * Line box of every strip / expanded-grid candidate line, as a multiple of its font size.
 * Applied as an explicit `lineHeight` by [candidateLineStyle], so the line box does not grow
 * with a taller fallback font (the System font's CJK fallback alone is ~1.45×), and
 * [computeCandidateFontSizes] sizes text for exactly this box.
 */
internal const val CANDIDATE_LINE_HEIGHT_FACTOR = 1.15f

/** One candidate line at [fontSp] in a fixed [CANDIDATE_LINE_HEIGHT_FACTOR] line box, glyphs centred in it. */
internal fun candidateLineStyle(
    fontSp: Float,
    color: Color,
    fontFamily: FontFamily,
): TextStyle =
    TextStyle(
        color = color,
        fontSize = fontSp.sp,
        fontFamily = fontFamily,
        textAlign = TextAlign.Center,
        lineHeight = (fontSp * CANDIDATE_LINE_HEIGHT_FACTOR).sp,
        // Tight + Trim.Both: the single line box is exactly lineHeight. Fixed / Minimum (or an
        // untrimmed Tight edge) pad it back up to the fallback font's preferred height, which is
        // the overflow being prevented.
        lineHeightStyle =
            LineHeightStyle(LineHeightStyle.Alignment.Center, LineHeightStyle.Trim.Both, LineHeightStyle.Mode.Tight),
    )

/**
 * Title / subtitle sizes (sp) whose stacked line boxes, paddings and gap fit the strip height.
 * There is no lower bound: a floor would push the stack past the strip and clip it.
 */
internal fun computeCandidateFontSizes(
    smartbarHeightPx: Int,
    paddingPx: Int,
    marginPx: Int,
    density: Float,
    fontScale: Float,
    textSizeScale: Float,
    contentHasSubtitles: Boolean,
): Pair<Float, Float> {
    val scaledDensity = density * fontScale
    // Mirrors CandidateCell: inner vertical padding paddingPx / 3 per side, outer
    // marginPx * 2 per side, subtitle top padding marginPx * 2.
    val verticalPaddingPx = paddingPx / 3 * 2
    val verticalMarginPx = marginPx * 4
    val subtitleGapPx = if (contentHasSubtitles) marginPx * 2 else 0
    // Each line box is rounded up to a whole pixel (LineHeightStyleSpan ceil), at most 1 px a line.
    val lineRoundingPx = if (contentHasSubtitles) 2 else 1
    val availablePx =
        (smartbarHeightPx - verticalPaddingPx - verticalMarginPx - subtitleGapPx - lineRoundingPx)
            .coerceAtLeast(0)
            .toFloat()
    // §42: subtitle-free content sizes the title for ONE line — the full
    // available height, same cap; mixed lists keep the 58/42 split.
    val titleShare = if (contentHasSubtitles) 0.58f else 1.0f
    val titleFitSp = availablePx * titleShare / CANDIDATE_LINE_HEIGHT_FACTOR / scaledDensity
    val subtitleFitSp = availablePx * 0.42f / CANDIDATE_LINE_HEIGHT_FACTOR / scaledDensity
    // The theme's text-size scale (0.85–1.15) shrinks freely but enlarges only up to the fit.
    val scale = minOf(textSizeScale, 1f)
    val titleSp = minOf(titleFitSp * scale, 21f)
    val subtitleSp = minOf(subtitleFitSp * scale, 16f)
    return Pair(titleSp, subtitleSp)
}
