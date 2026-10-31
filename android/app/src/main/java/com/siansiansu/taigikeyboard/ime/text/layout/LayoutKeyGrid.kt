package com.siansiansu.taigikeyboard.ime.text.layout

import androidx.compose.foundation.background
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.TextAutoSize
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.boundsInWindow
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.colorResource
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.min
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardLayoutSolver

// Draws a key table as plain keys: the custom-layout editor board, and the custom-layout card preview
// on the Layout tab and in the keyboard's layout picker. Not the keyboard view, so a key can be
// selected and nothing is typed. Mirrors iOS Layout/LayoutKeyGrid.swift.

/** The editor board's state and callbacks. */
class LayoutKeyGridInteraction(
    val selected: LayoutPlace?,
    /** TalkBack name of a removed key's place (the grid has no strings of its own). */
    val removedPlaceLabel: String,
    val onTap: (LayoutPlace) -> Unit,
    /** The selected key's horizontal centre in window coordinates, reported as it is laid out. */
    val onSelectedKeyCenterX: (Float) -> Unit = {},
)

/**
 * [rows] drawn on a board; [base] sets the 文/A key's width. [isDark] is the caller's effective
 * light / dark state (the app theme, or the keyboard's [com.siansiansu.taigikeyboard.ime.theme.isKeyboardNightMode]).
 * With an [interaction] every key with a place takes taps; without one the grid is an inert
 * preview with no TalkBack content of its own (the caller names it).
 */
@Composable
fun LayoutKeyGrid(
    rows: List<List<GridKey>>,
    base: String,
    isDark: Boolean,
    modifier: Modifier = Modifier,
    interaction: LayoutKeyGridInteraction? = null,
) {
    BoxWithConstraints(
        modifier
            .background(boardColor(isDark))
            .padding(2.dp),
    ) {
        val unit = maxWidth / ROW_UNITS
        val rowHeight = maxHeight / rows.size.coerceAtLeast(1)
        val gap = min(unit, rowHeight) * 0.1f
        val fontSize = with(LocalDensity.current) { (rowHeight * 0.4f).toSp() }
        Column(
            modifier = Modifier.fillMaxSize().then(if (interaction == null) Modifier.clearAndSetSemantics {} else Modifier),
        ) {
            rows.forEach { row ->
                val widths = widths(row, base)
                Row(
                    modifier = Modifier.fillMaxWidth().height(rowHeight),
                    horizontalArrangement = Arrangement.Center,
                ) {
                    row.forEachIndexed { column, gridKey ->
                        // One node per place: a key that takes another place (a base switch) is a
                        // new node, so its tap gesture (keyed on the place) starts fresh.
                        key(gridKey.place ?: column) {
                            GridKeyView(
                                key = gridKey,
                                isDark = isDark,
                                fontSize = fontSize,
                                gap = gap,
                                interaction = interaction,
                                modifier = Modifier.width(unit * widths[column]).fillMaxHeight(),
                            )
                        }
                    }
                }
            }
        }
    }
}

/** The dashed outline of an empty place: a removed key, an empty long-press slot. */
fun Modifier.dashedOutline(
    color: Color,
    corner: Dp,
    isVisible: Boolean = true,
): Modifier =
    drawBehind {
        if (isVisible) {
            drawRoundRect(
                color,
                cornerRadius = CornerRadius(corner.toPx()),
                style = Stroke(width = 1.dp.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(3.dp.toPx(), 3.dp.toPx()))),
            )
        }
    }

/** A key's face: its text, or its keyboard icon. */
@Composable
private fun LayoutKeyFace(
    face: GridKey.Face,
    color: Color,
    fontSize: TextUnit,
    modifier: Modifier = Modifier,
) {
    Box(modifier, contentAlignment = Alignment.Center) {
        when (face) {
            is GridKey.Face.Character -> AutoSizeLabel(face.label, color, fontSize)
            is GridKey.Face.Function -> {
                val icon = iconRes(face.key)
                when {
                    icon != null -> {
                        val size = with(LocalDensity.current) { fontSize.toDp() }
                        Icon(painterResource(icon), contentDescription = null, tint = color, modifier = Modifier.size(size))
                    }
                    face.key == FunctionKey.NUMERIC -> AutoSizeLabel(stringResource(R.string.key__view_symbols), color, fontSize * 0.8f)
                    // Space: a blank key.
                    else -> Unit
                }
            }
        }
    }
}

/**
 * One line that shrinks to fit (`tsh` / `chh`), as on the keyboard (KeyContent). The floor is relative,
 * as iOS `minimumScaleFactor(0.5)`, so a small picker card still fits a 3-letter key.
 */
@Composable
private fun AutoSizeLabel(
    text: String,
    color: Color,
    maxFontSize: TextUnit,
) {
    BasicText(
        text = text,
        style = TextStyle(color = color),
        maxLines = 1,
        autoSize = TextAutoSize.StepBased(minFontSize = maxFontSize * 0.5f, maxFontSize = maxFontSize),
    )
}

/**
 * TalkBack name of a key: its text, or a symbol TalkBack reads by its Unicode name (as iOS
 * `LayoutKeyGrid.accessibilityName`).
 */
fun accessibilityName(face: GridKey.Face): String =
    when (face) {
        is GridKey.Face.Character -> face.label
        is GridKey.Face.Function ->
            when (face.key) {
                FunctionKey.SHIFT -> "⇧"
                FunctionKey.BACKSPACE -> "⌫"
                FunctionKey.RETURN -> "⏎"
                FunctionKey.EMOJI -> "😊"
                FunctionKey.SPACE -> "␣"
                FunctionKey.TRANSLATE -> "文"
                FunctionKey.NUMERIC -> "123"
            }
    }

// The default keyboard theme's tones (res/values/colors.xml `keyboard_default_*`, which themes.xml
// uses): every key, function keys included, white on a gray board.
@Composable
private fun layoutKeyColor(isDark: Boolean): Color = colorResource(if (isDark) R.color.keyboard_default_key_dark else R.color.keyboard_default_key_light)

private fun layoutKeyTextColor(isDark: Boolean): Color = if (isDark) Color.White else Color.Black

@Composable
private fun boardColor(isDark: Boolean): Color = colorResource(if (isDark) R.color.keyboard_default_background_dark else R.color.keyboard_default_background_light)

/** Key widths in letter-key units; a row of ten letter keys fills the board. */
private const val ROW_UNITS = 10f

@Composable
private fun GridKeyView(
    key: GridKey,
    isDark: Boolean,
    fontSize: TextUnit,
    gap: Dp,
    interaction: LayoutKeyGridInteraction?,
    modifier: Modifier,
) {
    val place = key.place
    val isSelected = place != null && place == interaction?.selected
    val corner = gap * 1.5f
    val shape = RoundedCornerShape(corner)
    val dashColor = layoutKeyTextColor(isDark).copy(alpha = 0.5f)
    val keyColor = layoutKeyColor(isDark)
    val fill =
        when {
            isSelected -> MaterialTheme.colorScheme.primary
            key.isRemoved -> Color.Transparent
            else -> keyColor
        }

    val keyModifier =
        if (place != null && interaction != null) {
            Modifier
                .interactive(place, key, isSelected, interaction)
                .onGloballyPositioned { if (isSelected) interaction.onSelectedKeyCenterX(it.boundsInWindow().center.x) }
        } else {
            Modifier
        }

    Box(modifier.padding(gap / 2).then(keyModifier)) {
        LayoutKeyFace(
            face = key.face,
            color = if (isSelected) Color.White else layoutKeyTextColor(isDark),
            fontSize = fontSize,
            modifier =
                Modifier
                    .fillMaxSize()
                    .background(fill, shape)
                    .dashedOutline(dashColor, corner, isVisible = key.isRemoved && !isSelected)
                    // The digit row is the base's and inert; a removed place shows no face.
                    .alpha(
                        when {
                            key.isRemoved -> 0f
                            place == null -> 0.5f
                            else -> 1f
                        },
                    ),
        )
    }
}

/**
 * A tap reports the key to the editor. Keyed on the place only: the latest callbacks are read through
 * [rememberUpdatedState], so a redraw never restarts the gesture. TalkBack and hardware keys
 * (Enter / Space) tap through semantics and key events.
 */
@Composable
private fun Modifier.interactive(
    place: LayoutPlace,
    key: GridKey,
    isSelected: Boolean,
    interaction: LayoutKeyGridInteraction,
): Modifier {
    val current by rememberUpdatedState(interaction)
    return this
        .pointerInput(place) {
            detectTapGestures(onTap = { current.onTap(place) })
        }.onKeyEvent { event ->
            val isActivation = event.key == Key.Enter || event.key == Key.NumPadEnter || event.key == Key.Spacebar
            if (isActivation && event.type == KeyEventType.KeyUp) current.onTap(place)
            isActivation
        }.focusable()
        .clearAndSetSemantics {
            contentDescription = if (key.isRemoved) current.removedPlaceLabel else accessibilityName(key.face)
            role = Role.Button
            selected = isSelected
            onClick {
                current.onTap(place)
                true
            }
        }
}

/**
 * Key widths of one row in letter-key units, from the keyboard's own slot fractions
 * ([KeyboardLayoutSolver.slotFraction], portrait). Space takes what the row leaves (at least two keys
 * wide); a row still wider than the board shrinks to fit it.
 */
private fun widths(
    row: List<GridKey>,
    base: String,
): List<Float> {
    fun isSpace(key: GridKey) = (key.face as? GridKey.Face.Function)?.key == FunctionKey.SPACE

    fun fixedWidth(key: GridKey): Float =
        when (val face = key.face) {
            is GridKey.Face.Character -> 1f
            is GridKey.Face.Function -> KeyboardLayoutSolver.slotFraction(face.key.code, base, isLandscape = false)?.times(ROW_UNITS) ?: 1f
        }
    val spaceWidth = (ROW_UNITS - row.filterNot(::isSpace).sumOf { fixedWidth(it).toDouble() }.toFloat()).coerceAtLeast(2f)
    val widths = row.map { if (isSpace(it)) spaceWidth else fixedWidth(it) }
    val scale = (ROW_UNITS / widths.sum()).coerceAtMost(1f)
    return widths.map { it * scale }
}

private fun iconRes(key: FunctionKey): Int? =
    when (key) {
        FunctionKey.SHIFT -> R.drawable.ic_keyboard_arrow_up
        FunctionKey.BACKSPACE -> R.drawable.ic_backspace
        FunctionKey.RETURN -> R.drawable.ic_keyboard_return
        FunctionKey.EMOJI -> R.drawable.ic_sentiment_satisfied
        // The keyboard's 文/A icon in its default state (KeyContent).
        FunctionKey.TRANSLATE -> R.drawable.ic_translate
        FunctionKey.NUMERIC, FunctionKey.SPACE -> null
    }
