package com.siansiansu.taigikeyboard.ime.text.layout

import androidx.compose.foundation.background
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
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
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.LayoutCoordinates
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
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardLayoutSolver

// Draws a key table as plain keys: the custom-layout editor board, and the custom-layout card preview
// on the Layout tab and in the keyboard's layout picker. Not the keyboard view, so a key can be
// selected or dragged and nothing is typed. Mirrors iOS Layout/LayoutKeyGrid.swift.

/**
 * The editor board's state and callbacks. Points are in window coordinates
 * ([LayoutCoordinates.localToWindow]), as are the bounds [onKeyBounds] reports.
 */
class LayoutKeyGridInteraction(
    val selected: LayoutPlace?,
    /** The key being dragged, drawn faded at its place. */
    val lifted: LayoutPlace?,
    /** The key a drag would land on. */
    val highlighted: LayoutPlace?,
    /** TalkBack name of a removed key's place (the grid has no strings of its own). */
    val removedPlaceLabel: String,
    val onTap: (LayoutPlace) -> Unit,
    /** A key moved past the touch slop. Move / end / cancel name the key, so a second finger never takes the drag over. */
    val onDragStart: (LayoutPlace, Offset) -> Unit,
    val onDragMove: (LayoutPlace, Offset) -> Unit,
    /** The finger lifted after a drag, at this point. */
    val onDragEnd: (LayoutPlace, Offset) -> Unit,
    /** The drag ended without a release (the system took the gesture, or the key left the board). */
    val onDragCancel: (LayoutPlace) -> Unit,
    /** A key reports its bounds after each layout, and null when it leaves the board. */
    val onKeyBounds: (LayoutPlace, Rect?) -> Unit,
)

/**
 * [rows] drawn on a board; [base] sets the 文/A key's width. [isDark] is the caller's effective
 * light / dark state (the app theme, or the keyboard's [com.siansiansu.taigikeyboard.ime.theme.isKeyboardNightMode]).
 * With an [interaction] every key with a place takes taps and drags; without one the grid is an inert
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
                        // new node, so it reports its bounds again and the old place's leave.
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

/** A key's face: its text, or its keyboard icon. Also the key the editor floats under a drag. */
@Composable
fun LayoutKeyFace(
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

/** One line that shrinks to fit (`tsh` / `chh`), as on the keyboard (KeyContent). */
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
        autoSize = TextAutoSize.StepBased(minFontSize = 6.sp, maxFontSize = maxFontSize),
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
fun layoutKeyColor(isDark: Boolean): Color = colorResource(if (isDark) R.color.keyboard_default_key_dark else R.color.keyboard_default_key_light)

fun layoutKeyTextColor(isDark: Boolean): Color = if (isDark) Color.White else Color.Black

@Composable
private fun boardColor(isDark: Boolean): Color = colorResource(if (isDark) R.color.keyboard_default_background_dark else R.color.keyboard_default_background_light)

/** Key widths in letter-key units; a row of ten letter keys fills the board. */
private const val ROW_UNITS = 10f

/**
 * Where a gesture reads its node's position from, kept out of composition state: pointer positions
 * are node-local, drags and drops work in window coordinates.
 */
internal class WindowAnchor {
    var coordinates: LayoutCoordinates? = null

    fun toWindow(position: Offset): Offset = coordinates?.localToWindow(position) ?: position
}

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
    val isTarget = place != null && place == interaction?.highlighted
    val corner = gap * 1.5f
    val shape = RoundedCornerShape(corner)
    val accent = MaterialTheme.colorScheme.primary
    val dashColor = layoutKeyTextColor(isDark).copy(alpha = 0.5f)
    val keyColor = layoutKeyColor(isDark)
    val fill =
        when {
            isSelected -> accent
            key.isRemoved -> Color.Transparent
            else -> keyColor
        }

    val keyModifier =
        if (place != null && interaction != null) {
            Modifier.interactive(place, key, isSelected, interaction)
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
                    .alpha(if (place != null && place == interaction?.lifted) 0.3f else 1f)
                    .background(fill, shape)
                    .drawBehind {
                        val cornerRadius = CornerRadius(corner.toPx())
                        when {
                            isTarget -> drawRoundRect(accent, cornerRadius = cornerRadius, style = Stroke(width = 2.5.dp.toPx()))
                            key.isRemoved && !isSelected ->
                                drawRoundRect(
                                    dashColor,
                                    cornerRadius = cornerRadius,
                                    style = Stroke(width = 1.dp.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(3.dp.toPx(), 3.dp.toPx()))),
                                )
                        }
                    }
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
 * One gesture per key: a press that lifts within the touch slop is a tap; past it the key drags until
 * the finger lifts ([LayoutKeyGridInteraction.onDragEnd]) or the gesture is cancelled
 * ([LayoutKeyGridInteraction.onDragCancel]), never a tap. Keyed on the place only: the latest
 * callbacks are read through [rememberUpdatedState], so the redraws a drag causes never restart it.
 * TalkBack and hardware keys (Enter / Space) tap through semantics and key events.
 */
@Composable
private fun Modifier.interactive(
    place: LayoutPlace,
    key: GridKey,
    isSelected: Boolean,
    interaction: LayoutKeyGridInteraction,
): Modifier {
    val current by rememberUpdatedState(interaction)
    val anchor = remember { WindowAnchor() }
    DisposableEffect(place) {
        onDispose { current.onKeyBounds(place, null) }
    }
    return this
        .onGloballyPositioned {
            anchor.coordinates = it
            current.onKeyBounds(place, it.boundsInWindow())
        }.pointerInput(place) {
            awaitEachGesture {
                val down = awaitFirstDown()
                var isDragging = false
                var isFinished = false
                val toWindow = anchor::toWindow
                try {
                    while (true) {
                        val change = awaitPointerEvent().changes.firstOrNull { it.id == down.id } ?: break
                        if (!change.pressed) {
                            // A system cancel arrives as an up already consumed: it ends the gesture
                            // without a tap or a drop.
                            if (change.changedToUp()) {
                                isFinished = true
                                change.consume()
                                if (isDragging) current.onDragEnd(place, toWindow(change.position)) else current.onTap(place)
                            }
                            break
                        }
                        if (!isDragging && (change.position - down.position).getDistance() > viewConfiguration.touchSlop) {
                            isDragging = true
                            current.onDragStart(place, toWindow(change.position))
                        }
                        if (isDragging) {
                            change.consume()
                            current.onDragMove(place, toWindow(change.position))
                        }
                    }
                } finally {
                    if (isDragging && !isFinished) current.onDragCancel(place)
                }
            }
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
