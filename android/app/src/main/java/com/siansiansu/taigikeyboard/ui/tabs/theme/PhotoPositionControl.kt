package com.siansiansu.taigikeyboard.ui.tabs.theme

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.drag
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.setProgress
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.ime.core.SurfaceRect
import com.siansiansu.taigikeyboard.ime.core.ThemeImageBackground
import kotlin.math.min
import kotlin.math.roundToInt

// The theme editor's photo position control: the drag surface over the live preview and the
// finger -> focus math behind it. Mirrors iOS PhotoPositionControl.swift.

/**
 * Maps a drag on the live keyboard preview to a photo's focus. The photo is aspect-filled, so at
 * most one axis overflows the surface; only that axis moves. The photo follows the finger: its
 * left edge sits at `(surface - cover) * focus`, so a drag of `d` px moves the focus by
 * `d / (surface - cover)` (negative overflow -> dragging right lowers `focusX`).
 * Mirrors iOS PhotoPositionDrag.
 */
object PhotoPositionDrag {
    /** One TalkBack adjustable step, as a fraction of the overflowing axis. */
    private const val ACCESSIBILITY_STEP = 0.1f

    /** The adjustable steps strictly between 0 and 1, for the TalkBack range. */
    val ACCESSIBILITY_RANGE_STEPS = (1f / ACCESSIBILITY_STEP).roundToInt() - 1

    /** Overflow below this many px is treated as none (rounding in the cover scale). */
    private const val MINIMUM_OVERFLOW = 0.5f

    /**
     * How far the aspect-filled photo overflows the surface on each axis (<= 0 px; 0 = fits
     * exactly). Independent of focus, so the centred cover rect measures it.
     */
    private fun overflow(
        imageWidth: Float,
        imageHeight: Float,
        surfaceWidth: Float,
        surfaceHeight: Float,
    ): Offset {
        val cover =
            ThemeImageBackground.coverRect(
                imageWidth,
                imageHeight,
                SurfaceRect(0f, 0f, surfaceWidth, surfaceHeight),
                focusX = ThemeImageBackground.DEFAULT_FOCUS,
                focusY = ThemeImageBackground.DEFAULT_FOCUS,
            )
        return Offset(surfaceWidth - cover.width, surfaceHeight - cover.height)
    }

    /** The axis the photo can move along over the surface, or null when it fits exactly. */
    fun axis(
        imageWidth: Float,
        imageHeight: Float,
        surfaceWidth: Float,
        surfaceHeight: Float,
    ): Orientation? {
        val overflow = overflow(imageWidth, imageHeight, surfaceWidth, surfaceHeight)
        return when {
            -overflow.y >= MINIMUM_OVERFLOW -> Orientation.Vertical
            -overflow.x >= MINIMUM_OVERFLOW -> Orientation.Horizontal
            else -> null
        }
    }

    /**
     * [start] after dragging the photo by [translation] px along [axis] (the one [axis] returned,
     * so its overflow is non-zero), focus clamped into 0..1.
     */
    fun dragged(
        start: ThemeImageBackground,
        translation: Offset,
        axis: Orientation,
        imageWidth: Float,
        imageHeight: Float,
        surfaceWidth: Float,
        surfaceHeight: Float,
    ): ThemeImageBackground {
        val overflow = overflow(imageWidth, imageHeight, surfaceWidth, surfaceHeight)
        return when (axis) {
            Orientation.Horizontal -> withAxis(start, axis, start.focusX + translation.x / overflow.x)
            Orientation.Vertical -> withAxis(start, axis, start.focusY + translation.y / overflow.y)
        }
    }

    /** [photo] with [axis]'s focus set to [value] clamped into 0..1 (also the TalkBack `setProgress` target). */
    fun withAxis(
        photo: ThemeImageBackground,
        axis: Orientation,
        value: Float,
    ): ThemeImageBackground =
        when (axis) {
            Orientation.Horizontal -> photo.copy(focusX = value.coerceIn(0f, 1f))
            Orientation.Vertical -> photo.copy(focusY = value.coerceIn(0f, 1f))
        }
}

private val STROKE_WIDTH = 3.dp
private val HALO_WIDTH = 2.dp
private val HEAD_LENGTH = 10.dp

/** Half the arrow length as a fraction of the preview's shorter side. */
private const val HALF_LENGTH_FRACTION = 0.25f
private val HALO_COLOR = Color.Black.copy(alpha = 0.6f)

/**
 * The drag surface laid over the live preview while the background is a photo: swallows the
 * preview keys' touches, draws a double-headed arrow along the axis the photo can move, and
 * reports every drag through [PhotoPositionDrag] to [onPhotoChange], so the photo follows the
 * finger. When the photo fits the preview exactly there is nothing to move: no arrow, no
 * gesture. For TalkBack it is one adjustable element labelled [label] that steps the movable
 * axis. Mirrors iOS PhotoPositionOverlay.
 */
@Composable
fun PhotoPositionOverlay(
    label: String,
    imageWidth: Int,
    imageHeight: Int,
    photo: ThemeImageBackground,
    onPhotoChange: (ThemeImageBackground) -> Unit,
    modifier: Modifier = Modifier,
) {
    BoxWithConstraints(modifier) {
        val density = LocalDensity.current
        val surfaceWidth = with(density) { maxWidth.toPx() }
        val surfaceHeight = with(density) { maxHeight.toPx() }
        val axis = PhotoPositionDrag.axis(imageWidth.toFloat(), imageHeight.toFloat(), surfaceWidth, surfaceHeight) ?: return@BoxWithConstraints
        val currentPhoto by rememberUpdatedState(photo)
        val currentOnPhotoChange by rememberUpdatedState(onPhotoChange)
        val axisValue = if (axis == Orientation.Horizontal) photo.focusX else photo.focusY

        Box(
            modifier =
                Modifier
                    .matchParentSize()
                    .pointerInput(imageWidth, imageHeight, surfaceWidth, surfaceHeight, axis) {
                        awaitEachGesture {
                            val down = awaitFirstDown()
                            down.consume()
                            val start = currentPhoto
                            drag(down.id) { change ->
                                change.consume()
                                currentOnPhotoChange(
                                    PhotoPositionDrag.dragged(
                                        start,
                                        change.position - down.position,
                                        axis,
                                        imageWidth.toFloat(),
                                        imageHeight.toFloat(),
                                        surfaceWidth,
                                        surfaceHeight,
                                    ),
                                )
                            }
                        }
                    }.semantics {
                        contentDescription = label
                        stateDescription = "${(axisValue * 100).roundToInt()}%"
                        progressBarRangeInfo = ProgressBarRangeInfo(axisValue, 0f..1f, PhotoPositionDrag.ACCESSIBILITY_RANGE_STEPS)
                        setProgress { target ->
                            currentOnPhotoChange(PhotoPositionDrag.withAxis(currentPhoto, axis, target))
                            true
                        }
                    },
        ) { PhotoPositionArrow(axis, Modifier.matchParentSize()) }
    }
}

/**
 * A double-headed arrow through the centre along [axis]. Its own composable on stable inputs so a
 * drag (which changes the photo, not the axis) skips the redraw. Mirrors iOS PhotoPositionArrow.
 */
@Composable
private fun PhotoPositionArrow(
    axis: Orientation,
    modifier: Modifier,
) {
    Canvas(modifier) { drawPositionArrow(axis) }
}

// White on a dark halo reads on any photo: every stroke is drawn twice, halo first.
private fun DrawScope.drawPositionArrow(axis: Orientation) {
    val halfLength = min(size.width, size.height) * HALF_LENGTH_FRACTION
    val along = if (axis == Orientation.Horizontal) Offset(1f, 0f) else Offset(0f, 1f)
    val across = Offset(along.y, along.x)
    val headLengthPx = HEAD_LENGTH.toPx()
    val strokeWidthPx = STROKE_WIDTH.toPx()
    val haloWidthPx = HALO_WIDTH.toPx()

    for ((color, width) in listOf(HALO_COLOR to strokeWidthPx + haloWidthPx * 2, Color.White to strokeWidthPx)) {
        for (sign in listOf(1f, -1f)) {
            val direction = along * sign
            val tip = center + direction * halfLength
            drawLine(color, center, tip, strokeWidth = width, cap = StrokeCap.Round)
            // Arrowhead: two strokes swept back from the tip at 45°.
            for (side in listOf(1f, -1f)) {
                drawLine(color, tip, tip - (direction - across * side) * headLengthPx, strokeWidth = width, cap = StrokeCap.Round)
            }
        }
    }
}
