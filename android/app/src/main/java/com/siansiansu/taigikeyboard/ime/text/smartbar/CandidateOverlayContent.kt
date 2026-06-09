// Compose content for the expanded candidate overlay — a LazyColumn of pixel-packed candidate
// rows + a fixed right-side control panel (collapse / page / translate). Honors the
// user-customizable candidate theme attrs (smartbar_bgColor / candidate fg colors / key_bgColor).
// Migrated from the legacy RecyclerView CandidateOverlayView; mirrors iOS ExpandedCandidateOverlay.

package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.graphics.Paint
import android.util.TypedValue
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.Typeface as ComposeTypeface
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.annotation.DrawableRes
import androidx.annotation.StringRes
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.CANDIDATE_HIGHLIGHT_LIGHTEN_FACTOR
import com.siansiansu.taigikeyboard.ime.core.CANDIDATE_PRESSED_DEEPEN_FACTOR
import com.siansiansu.taigikeyboard.ime.core.deepenedArgb
import com.siansiansu.taigikeyboard.ime.core.lightenedArgb
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.theme.getColorFromAttr
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private const val ITEMS_PER_PAGE = 20
private const val PRIMARY_TEXT_SIZE_SP = 21f
private const val SUBTITLE_TEXT_SIZE_SP = 19f
private const val MINIMUM_CELL_WIDTH_DP = 44f
private const val CELL_HORIZONTAL_PADDING_DP = 20f

// Right reserved width fed to the row packer: 60dp control panel + 8dp gap + 4dp row margins.
private const val RIGHT_RESERVED_DP = 72f

// Click protection window: a show() must briefly swallow cell taps so the expand-button press
// that opened the overlay does not fall through onto a cell underneath it.
private const val CLICK_PROTECTION_MS = 150L

private val CellCornerRadius = 10.dp
private val CellMinHeight = 58.dp
private val CellVerticalPadding = 7.dp
private val CellHorizontalPadding = 8.dp
private val RowHorizontalMargin = 2.dp
private val RowVerticalMargin = 1.dp
private val DividerStartInset = 2.dp
private val DividerEndInset = 62.dp

private val ControlPanelWidth = 60.dp
private val ControlButtonCorner = 8.dp
private val CollapseButtonHeight = 56.dp
private val PageButtonSize = 45.dp

/**
 * Expanded candidate grid + right control panel.
 *
 * Row breaks are computed by [CandidateRowLayout.arrangeRows] from android [Paint] pixel
 * measurement — kept identical to the legacy View so wrapping does not shift. Cell widths are
 * measured as max(roman, hanzi) so toggling translate swaps text without reflowing the grid.
 *
 * @param resetKey bumped on each overlay show(); re-arms click protection and resets scroll/page.
 */
@Composable
fun CandidateOverlayContent(
    suggestions: List<TaigiWord>,
    typeface: android.graphics.Typeface,
    isTPSLayout: Boolean,
    orMapsToER: Boolean,
    isTranslateSwapped: Boolean,
    resetKey: Int,
    backgroundGradient: List<Int>?,
    candidateTextColor: Int?,
    onSuggestionSelected: (TaigiWord, Int) -> Unit,
    onCollapse: () -> Unit,
    onTranslateToggle: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val fontScale = LocalConfiguration.current.fontScale
    val colors = rememberCandidateOverlayColors(resetKey, backgroundGradient, candidateTextColor)
    val fontFamily = remember(typeface) { FontFamily(ComposeTypeface(typeface)) }

    // Click protection re-arms on every show() (resetKey bump); updateSuggestions must NOT re-arm.
    var isClickEnabled by remember(resetKey) { mutableStateOf(false) }
    LaunchedEffect(resetKey) {
        delay(CLICK_PROTECTION_MS)
        isClickEnabled = true
    }

    val displayMetrics = context.resources.displayMetrics
    // Static per session (screen width / density / dimen) — remember so show/translate recompositions
    // don't redo the resource lookup + arithmetic.
    val availableWidthPx =
        remember(context) { displayMetrics.widthPixels - (RIGHT_RESERVED_DP * displayMetrics.density + 0.5f).toInt() }
    val spacingPx = remember(context) { context.resources.getDimensionPixelSize(R.dimen.smartbar_button_margin) }
    val minCellWidthPx = remember(context) { (MINIMUM_CELL_WIDTH_DP * displayMetrics.density + 0.5f).toInt() }
    val cellPaddingPx = remember(context) { (CELL_HORIZONTAL_PADDING_DP * displayMetrics.density + 0.5f).toInt() }

    // Paint pipeline mirrors the legacy CandidateOverlayView measurement (typeface + sp sizes).
    val primaryPaint = remember(typeface, fontScale) { measurementPaint(typeface, PRIMARY_TEXT_SIZE_SP, displayMetrics) }
    val subtitlePaint = remember(typeface, fontScale) { measurementPaint(typeface, SUBTITLE_TEXT_SIZE_SP, displayMetrics) }

    // Rows do NOT depend on isTranslateSwapped: cell width is max(roman, hanzi) so a swap never reflows.
    val rows =
        remember(suggestions, isTPSLayout, orMapsToER, typeface) {
            CandidateRowLayout.arrangeRows(suggestions, availableWidthPx, spacingPx) { word ->
                measureCellWidth(word, isTPSLayout, orMapsToER, primaryPaint, subtitlePaint, minCellWidthPx, cellPaddingPx)
            }
        }

    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()
    var currentPage by remember(resetKey) { mutableIntStateOf(0) }

    // Scroll to top whenever the overlay is shown or its suggestions are replaced (matches the
    // legacy resetScrollPosition() called from both show() and updateSuggestions()).
    LaunchedEffect(resetKey, suggestions) {
        currentPage = 0
        listState.scrollToItem(0)
    }

    fun rowIndexFor(suggestionIndex: Int): Int {
        if (rows.isEmpty() || suggestions.isEmpty()) return 0
        return minOf((suggestionIndex.toFloat() / suggestions.size * rows.size).toInt(), rows.size - 1)
    }

    // Gradient theme: paint the gradient as an opaque backdrop so the overlay stays
    // continuous with the gradient-painted keyboard. The overlay is a SIBLING of the
    // gradient-painted text_input_content (not a child), so it must paint the gradient
    // itself — a transparent overlay would reveal the transparent IME window. Flat
    // themes fall back to the solid `?smartbar_bgColor` chrome.
    val backgroundModifier =
        remember(backgroundGradient, colors.background) {
            if (backgroundGradient != null && backgroundGradient.size >= 2) {
                Modifier.background(Brush.verticalGradient(backgroundGradient.map { Color(it) }))
            } else {
                Modifier.background(colors.background)
            }
        }

    Box(modifier = modifier.fillMaxSize().then(backgroundModifier)) {
        LazyColumn(
            state = listState,
            modifier = Modifier.fillMaxSize(),
            // end=68dp reserves the control-panel lane; top/bottom mirror the legacy recycler padding.
            contentPadding = PaddingValues(top = 6.dp, bottom = 8.dp, end = 68.dp),
        ) {
            itemsIndexed(rows) { rowIndex, row ->
                CandidateRow(
                    row = row,
                    isLastRow = rowIndex == rows.lastIndex,
                    availableWidthPx = availableWidthPx,
                    spacingPx = spacingPx,
                    colors = colors,
                    fontFamily = fontFamily,
                    isTPSLayout = isTPSLayout,
                    orMapsToER = orMapsToER,
                    isTranslateSwapped = isTranslateSwapped,
                    isClickEnabled = isClickEnabled,
                    onCellClick = { word, index ->
                        onSuggestionSelected(word, index)
                        onCollapse()
                    },
                )
            }
        }

        // Faint divider between the grid lane and the control panel. The legacy XML used the framework
        // `?android:attr/dividerVertical` drawable at alpha 0.5; semiTransparentColor (#20…) is a
        // deliberate close approximation (avoids loading a framework drawable into Compose).
        Box(
            modifier =
                Modifier
                    .align(Alignment.TopEnd)
                    .padding(top = 6.dp, end = ControlPanelWidth, bottom = 8.dp)
                    .width(1.dp)
                    .fillMaxHeight()
                    .background(colors.pressed),
        )

        ControlPanel(
            modifier = Modifier.align(Alignment.TopEnd),
            colors = colors,
            showTranslate = !isTPSLayout,
            isTranslateActivated = isTranslateSwapped,
            onCollapse = onCollapse,
            onPageUp = {
                val newStart = maxOf(0, currentPage * ITEMS_PER_PAGE - ITEMS_PER_PAGE)
                if (newStart < suggestions.size) {
                    currentPage = newStart / ITEMS_PER_PAGE
                    scope.launch { listState.scrollToItem(rowIndexFor(newStart)) }
                }
            },
            onPageDown = {
                val newStart = minOf(suggestions.size - 1, (currentPage + 1) * ITEMS_PER_PAGE)
                if (newStart in suggestions.indices) {
                    currentPage = newStart / ITEMS_PER_PAGE
                    scope.launch { listState.scrollToItem(rowIndexFor(newStart)) }
                }
            },
            onTranslateToggle = onTranslateToggle,
        )
    }
}

@Composable
private fun CandidateRow(
    row: List<CandidateRowLayout.RowItem>,
    isLastRow: Boolean,
    availableWidthPx: Int,
    spacingPx: Int,
    colors: CandidateOverlayColors,
    fontFamily: FontFamily,
    isTPSLayout: Boolean,
    orMapsToER: Boolean,
    isTranslateSwapped: Boolean,
    isClickEnabled: Boolean,
    onCellClick: (TaigiWord, Int) -> Unit,
) {
    val density = LocalDensity.current

    // Distribute leftover width equally across all cells — replicates the legacy
    // LinearLayout `width = measuredWidth, weight = 1` (base width + equal share of slack).
    val totalMeasured = row.sumOf { it.measuredWidth }
    val totalSpacing = spacingPx * (row.size - 1).coerceAtLeast(0)
    val leftover = (availableWidthPx - totalMeasured - totalSpacing).coerceAtLeast(0)
    val bonus = if (row.isNotEmpty()) leftover / row.size else 0

    Column(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = RowHorizontalMargin, vertical = RowVerticalMargin),
        ) {
            row.forEachIndexed { cellIndex, item ->
                if (cellIndex > 0) Spacer(Modifier.width(with(density) { spacingPx.toDp() }))
                // Render at exactly the packer's measuredWidth (+ stretch bonus) — the legacy cell's
                // android:minWidth=64dp never bound because LinearLayout used an exact-width LayoutParams,
                // so coercing to 64dp here would overflow a packed row into the control lane.
                val widthPx = item.measuredWidth + bonus
                CandidateCell(
                    item = item,
                    cellWidth = with(density) { widthPx.toDp() },
                    colors = colors,
                    fontFamily = fontFamily,
                    isTPSLayout = isTPSLayout,
                    orMapsToER = orMapsToER,
                    isTranslateSwapped = isTranslateSwapped,
                    onClick = { if (isClickEnabled) onCellClick(item.word, item.originalIndex) },
                )
            }
        }
        if (!isLastRow) {
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .padding(start = DividerStartInset, end = DividerEndInset)
                        .height(1.dp)
                        .background(colors.pressed),
            )
        }
    }
}

@Composable
private fun CandidateCell(
    item: CandidateRowLayout.RowItem,
    cellWidth: Dp,
    colors: CandidateOverlayColors,
    fontFamily: FontFamily,
    isTPSLayout: Boolean,
    orMapsToER: Boolean,
    isTranslateSwapped: Boolean,
    onClick: () -> Unit,
) {
    val word = item.word
    // First candidate (engine ranker top, index 0) gets the filled keycap-color hint — mirrors
    // the strip + iOS overlay. Keyed on the packer's originalIndex, not the dead isComposingText
    // metadata (no producer since the v3.5.8 continuous redesign).
    val isFirstCandidate = item.originalIndex == 0
    val interaction = remember { MutableInteractionSource() }
    val isPressed by interaction.collectIsPressedAsState()

    val backgroundColor =
        when {
            isPressed -> colors.pressed
            isFirstCandidate -> colors.firstCandidateBackground
            else -> Color.Transparent
        }

    Box(
        modifier =
            Modifier
                .width(cellWidth)
                .heightIn(min = CellMinHeight)
                .clickable(interactionSource = interaction, indication = null, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        // The first-candidate cell insets its background vertically (legacy InsetDrawable) so the
        // keycap tint sits as a shorter pill; pressed/normal cells fill the full cell bounds.
        Box(
            modifier =
                Modifier
                    .matchParentSize()
                    .padding(vertical = if (isFirstCandidate && !isPressed) CellVerticalPadding else 0.dp)
                    .clip(RoundedCornerShape(CellCornerRadius))
                    .background(backgroundColor),
        )

        Column(
            modifier = Modifier.padding(vertical = CellVerticalPadding, horizontal = CellHorizontalPadding),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            // Memoize the TPS conversion (FFI) so it does not re-run on every recomposition
            // (translate toggle / scroll re-entry); only word/layout/orMapsToER affect it.
            val displayRoman =
                remember(word.roman, isTPSLayout, orMapsToER) {
                    if (isTPSLayout) RustEngineBridge.tlDisplayToTps(word.roman, orMapsToER) else word.roman
                }
            val (primary, subtitle) =
                when {
                    word.hanzi.isNullOrEmpty() -> displayRoman to null
                    isTPSLayout -> word.hanzi to null
                    isTranslateSwapped -> word.hanzi to displayRoman
                    else -> displayRoman to word.hanzi
                }

            Text(
                text = primary,
                color = colors.primary,
                fontSize = PRIMARY_TEXT_SIZE_SP.sp,
                fontFamily = fontFamily,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.Center,
            )
            if (subtitle != null) {
                Text(
                    text = subtitle,
                    color = colors.subtitle,
                    fontSize = SUBTITLE_TEXT_SIZE_SP.sp,
                    fontFamily = fontFamily,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(top = 2.dp),
                )
            }
        }
    }
}

@Composable
private fun ControlPanel(
    colors: CandidateOverlayColors,
    showTranslate: Boolean,
    isTranslateActivated: Boolean,
    onCollapse: () -> Unit,
    onPageUp: () -> Unit,
    onPageDown: () -> Unit,
    onTranslateToggle: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier.width(ControlPanelWidth).fillMaxHeight().padding(top = 6.dp, end = 8.dp, bottom = 8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        ControlButton(
            iconRes = R.drawable.ic_keyboard_arrow_up,
            contentDescription = R.string.smartbar__expand_toggle__alt,
            colors = colors,
            modifier = Modifier.fillMaxWidth().height(CollapseButtonHeight),
            iconPadding = 12.dp,
            onClick = onCollapse,
        )
        Spacer(Modifier.height(3.dp))
        ControlButton(
            iconRes = R.drawable.ic_arrow_triangle_up,
            contentDescription = R.string.overlay__page_up,
            colors = colors,
            modifier = Modifier.size(PageButtonSize),
            iconPadding = 10.dp,
            onClick = onPageUp,
        )
        Spacer(Modifier.height(16.dp))
        ControlButton(
            iconRes = R.drawable.ic_arrow_triangle_down,
            contentDescription = R.string.overlay__page_down,
            colors = colors,
            modifier = Modifier.size(PageButtonSize),
            iconPadding = 10.dp,
            onClick = onPageDown,
        )
        if (showTranslate) {
            Spacer(Modifier.height(25.dp))
            ControlButton(
                iconRes = R.drawable.ic_translate,
                contentDescription = R.string.overlay__translate_toggle,
                colors = colors,
                modifier = Modifier.size(PageButtonSize),
                iconPadding = 10.dp,
                isActivated = isTranslateActivated,
                onClick = onTranslateToggle,
            )
        }
    }
}

@Composable
private fun ControlButton(
    @DrawableRes iconRes: Int,
    @StringRes contentDescription: Int,
    colors: CandidateOverlayColors,
    modifier: Modifier,
    iconPadding: Dp,
    isActivated: Boolean = false,
    onClick: () -> Unit,
) {
    val interaction = remember { MutableInteractionSource() }
    val isPressed by interaction.collectIsPressedAsState()
    val background = if (isPressed || isActivated) colors.buttonPressed else Color.Transparent

    Box(
        modifier =
            modifier
                .clip(RoundedCornerShape(ControlButtonCorner))
                .background(background)
                .clickable(interactionSource = interaction, indication = null, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            painter = painterResource(iconRes),
            contentDescription = stringResource(contentDescription),
            tint = colors.controlTint,
            modifier = Modifier.fillMaxSize().padding(iconPadding),
        )
    }
}

// --- Colors ---

/** Candidate-overlay theme colors (NOT [KeyboardChromeColors] — candidate uses smartbar_bgColor). */
private data class CandidateOverlayColors(
    val background: Color,
    val primary: Color,
    val subtitle: Color,
    val firstCandidateBackground: Color,
    val pressed: Color,
    val controlTint: Color,
    val buttonPressed: Color,
)

@Composable
private fun rememberCandidateOverlayColors(
    refreshKey: Int,
    backgroundGradient: List<Int>?,
    candidateTextColor: Int?,
): CandidateOverlayColors {
    val context = LocalContext.current
    return remember(refreshKey, context, backgroundGradient, candidateTextColor) {
        // Gradient themes tint first-candidate + pressed with the theme hue (deepened top stop),
        // matching the strip; flat themes keep the neutral key_bgColor / semiTransparentColor attrs.
        val gradientTop = backgroundGradient?.takeIf { it.size >= 2 }?.first()
        // Role-first foreground (mirrors the strip): a light-only theme's fixed
        // candidateTextColor keeps text/control glyphs dark on a light gradient in
        // system dark mode; null (adaptive default) falls back to the night attrs.
        val roleFg = candidateTextColor?.let { Color(it) }
        CandidateOverlayColors(
            background = Color(getColorFromAttr(context, R.attr.smartbar_bgColor)),
            primary = roleFg ?: Color(getColorFromAttr(context, R.attr.smartbar_candidate_fgColor)),
            subtitle = roleFg ?: Color(getColorFromAttr(context, R.attr.smartbar_candidate_subtitle_fgColor)),
            firstCandidateBackground =
                gradientTop?.let { Color(lightenedArgb(it, CANDIDATE_HIGHLIGHT_LIGHTEN_FACTOR)) }
                    ?: Color(getColorFromAttr(context, R.attr.key_bgColor)),
            pressed =
                gradientTop?.let { Color(deepenedArgb(it, CANDIDATE_PRESSED_DEEPEN_FACTOR)) }
                    ?: Color(getColorFromAttr(context, R.attr.semiTransparentColor)),
            controlTint = roleFg ?: Color(getColorFromAttr(context, R.attr.smartbar_fgColor)),
            buttonPressed = Color(getColorFromAttr(context, R.attr.overlay_button_bgColorPressed)),
        )
    }
}

// --- Measurement (android Paint — preserves the legacy pixel row breaks exactly) ---

private fun measurementPaint(
    typeface: android.graphics.Typeface,
    textSizeSp: Float,
    displayMetrics: android.util.DisplayMetrics,
): Paint =
    Paint().apply {
        isAntiAlias = true
        this.typeface = typeface
        textSize = TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, textSizeSp, displayMetrics)
    }

private fun measureCellWidth(
    word: TaigiWord,
    isTPSLayout: Boolean,
    orMapsToER: Boolean,
    primaryPaint: Paint,
    subtitlePaint: Paint,
    minCellWidthPx: Int,
    cellPaddingPx: Int,
): Int {
    if (isTPSLayout) {
        val title =
            if (!word.hanzi.isNullOrEmpty()) word.hanzi else RustEngineBridge.tlDisplayToTps(word.roman, orMapsToER)
        val titleWidth = primaryPaint.measureText(title)
        return maxOf(minCellWidthPx, (titleWidth + cellPaddingPx + 0.5f).toInt())
    }
    val romanWidth = primaryPaint.measureText(word.roman)
    val hanziWidth = if (!word.hanzi.isNullOrEmpty()) subtitlePaint.measureText(word.hanzi) else 0f
    return maxOf(minCellWidthPx, (maxOf(romanWidth, hanziWidth) + cellPaddingPx + 0.5f).toInt())
}
