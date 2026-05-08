// 中文: 候選列主 Composable — LazyRow 渲染 Taigi 候選 + English 建議 row;
// 中文: 點擊由外部 callback 注入(見 CandidateClickHandler)。被 SmartbarView 內 ComposeView 主機 host。

package com.siansiansu.taigikeyboard.ime.text.smartbar

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
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
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.typeface.TypefaceLoader
import androidx.compose.foundation.text.BasicText
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

    val backgroundModifier =
        if (display.candidateBackgroundColor != null) {
            Modifier.background(Color(display.candidateBackgroundColor))
        } else {
            Modifier
        }

    LazyRow(
        state = listState,
        modifier =
            modifier
                .fillMaxSize()
                .then(backgroundModifier),
        contentPadding = PaddingValues(end = 48.dp),
    ) {
        // No explicit `key` lambda: candidate ids are not unique (custom-dictionary
        // hits collide on `TaigiWord(id = -2, …)` per LexiconService.kt:129) and
        // every state push fully replaces the list anyway, so position-based
        // identity is correct here.
        itemsIndexed(items) { index, word ->
            CandidateCell(
                word = word,
                index = index,
                display = display,
                fontFamily = fontFamily,
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
                titleColor = Color(display.themeTitleColor),
                pressedHighlight = Color(display.themePressedHighlightColor),
                fontFamily = fontFamily,
                onClick = { onEnglishCandidateClick(slot) },
            )
            if (slot < 2) {
                EnglishDivider(color = Color(display.themePressedHighlightColor))
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
    val composingVerticalInset = with(density) { (paddingPx / 2).toDp() }

    val isNextWord = word.id < 0
    val showComposing = index == 0 && !isNextWord
    val cornerRadius = if (showComposing) 16.dp else 8.dp

    val interactionSource = remember { MutableInteractionSource() }
    val isPressed by interactionSource.collectIsPressedAsState()

    val backgroundColor =
        when {
            isPressed -> Color(display.themePressedHighlightColor)
            showComposing -> Color(display.themeKeyBgColor)
            else -> Color.Transparent
        }

    val isTPSLayout = display.layoutType == "tps"
    val displayRoman =
        remember(word.roman, isTPSLayout, display.orMapsToER) {
            if (isTPSLayout) {
                RustEngineBridge.tlDisplayToTps(word.roman, display.orMapsToER)
            } else {
                word.roman
            }
        }
    val titleText: String
    val subtitleText: String?
    when {
        word.hanzi.isNullOrEmpty() -> {
            titleText = displayRoman
            subtitleText = null
        }
        isTPSLayout -> {
            titleText = word.hanzi
            subtitleText = null
        }
        display.isTranslateSwapped -> {
            titleText = word.hanzi
            subtitleText = displayRoman
        }
        else -> {
            titleText = displayRoman
            subtitleText = word.hanzi
        }
    }
    val showSubtitle = !subtitleText.isNullOrEmpty() && subtitleText != titleText

    val (titleSp, subtitleSp) =
        remember(display.smartbarHeightPx, display.textSizeScale, paddingPx, marginPx) {
            computeCandidateFontSizes(
                smartbarHeightPx = display.smartbarHeightPx,
                paddingPx = paddingPx,
                marginPx = marginPx,
                density = res.displayMetrics.density,
                fontScale = configuration.fontScale,
                textSizeScale = display.textSizeScale,
            )
        }

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
                    .let { mod -> if (showComposing) mod.padding(vertical = composingVerticalInset) else mod }
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
                style =
                    TextStyle(
                        color = titleColor,
                        fontSize = titleSp.sp,
                        fontFamily = fontFamily,
                        textAlign = TextAlign.Center,
                    ),
            )
            if (showSubtitle) {
                BasicText(
                    text = subtitleText!!,
                    style =
                        TextStyle(
                            color = subtitleColor,
                            fontSize = subtitleSp.sp,
                            fontFamily = fontFamily,
                            textAlign = TextAlign.Center,
                        ),
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

private fun computeCandidateFontSizes(
    smartbarHeightPx: Int,
    paddingPx: Int,
    marginPx: Int,
    density: Float,
    fontScale: Float,
    textSizeScale: Float,
): Pair<Float, Float> {
    val scaledDensity = density * fontScale
    val verticalPaddingPx = paddingPx * 2 / 3
    val verticalMarginPx = marginPx * 4
    val subtitleGapPx = 2 * density
    val availablePx =
        (smartbarHeightPx - verticalPaddingPx - verticalMarginPx - subtitleGapPx)
            .coerceAtLeast(20f * density)
    val lineHeightFactor = 1.15f
    val titlePx = availablePx * 0.58f * textSizeScale / lineHeightFactor
    val subtitlePx = availablePx * 0.42f * textSizeScale / lineHeightFactor
    val titleSp = (titlePx / scaledDensity).coerceIn(10f, 21f)
    val subtitleSp = (subtitlePx / scaledDensity).coerceIn(8f, 16f)
    return Pair(titleSp, subtitleSp)
}
