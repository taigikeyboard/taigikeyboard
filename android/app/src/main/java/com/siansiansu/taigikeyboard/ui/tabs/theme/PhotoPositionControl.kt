package com.siansiansu.taigikeyboard.ui.tabs.theme

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculatePan
import androidx.compose.foundation.gestures.calculateZoom
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
import androidx.compose.ui.semantics.clearAndSetSemantics
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
import kotlin.math.sqrt

// The theme editor's photo position control: the drag / pinch surface over the live preview and
// the finger -> focus / zoom math behind it. Mirrors iOS PhotoPositionControl.swift.

/** The axes a photo can move along over the preview. */
data class PhotoAxes(
    val horizontal: Boolean,
    val vertical: Boolean,
)

/**
 * Maps gestures on the live keyboard preview to a photo's focus / zoom. The photo is aspect-filled
 * times its zoom: at zoom 1 at most one axis overflows the surface, zoomed in both do, and only
 * overflowing axes move. The photo follows the finger: its left edge sits at
 * `(surface - cover) * focus`, so a drag of `d` px moves the focus by `d / (surface - cover)`
 * (negative overflow -> dragging right lowers `focusX`). A pinch scales the zoom about the focus
 * point. Mirrors iOS PhotoPositionDrag.
 */
object PhotoPositionDrag {
    /** One TalkBack adjustable step, as a fraction of the overflowing axis. */
    private const val ACCESSIBILITY_STEP = 0.1f

    /** The adjustable steps strictly between 0 and 1, for the TalkBack range. */
    val ACCESSIBILITY_RANGE_STEPS = (1f / ACCESSIBILITY_STEP).roundToInt() - 1

    /** Overflow below this many px is treated as none (rounding in the cover scale). */
    private const val MINIMUM_OVERFLOW = 0.5f

    /**
     * How far the photo at [zoom] overflows the surface on each axis (<= 0 px; 0 = fits exactly).
     * Independent of focus, so the centred cover rect measures it.
     */
    private fun overflow(
        imageWidth: Float,
        imageHeight: Float,
        surfaceWidth: Float,
        surfaceHeight: Float,
        zoom: Float,
    ): Offset {
        val cover =
            ThemeImageBackground.coverRect(
                imageWidth,
                imageHeight,
                SurfaceRect(0f, 0f, surfaceWidth, surfaceHeight),
                focusX = ThemeImageBackground.DEFAULT_FOCUS,
                focusY = ThemeImageBackground.DEFAULT_FOCUS,
                zoom = zoom,
            )
        return Offset(surfaceWidth - cover.width, surfaceHeight - cover.height)
    }

    /** The axes [photo] can move along over the surface (neither when it fits exactly). */
    fun axes(
        photo: ThemeImageBackground,
        imageWidth: Float,
        imageHeight: Float,
        surfaceWidth: Float,
        surfaceHeight: Float,
    ): PhotoAxes = axes(overflowing = overflow(imageWidth, imageHeight, surfaceWidth, surfaceHeight, photo.zoom))

    private fun axes(overflowing: Offset): PhotoAxes = PhotoAxes(horizontal = -overflowing.x >= MINIMUM_OVERFLOW, vertical = -overflowing.y >= MINIMUM_OVERFLOW)

    /** The one axis TalkBack steps: vertical when it moves, else horizontal, else null. */
    fun accessibilityAxis(axes: PhotoAxes): Orientation? =
        when {
            axes.vertical -> Orientation.Vertical
            axes.horizontal -> Orientation.Horizontal
            else -> null
        }

    /** [start] after dragging the photo by [translation] px; an axis that does not overflow keeps its focus. */
    fun dragged(
        start: ThemeImageBackground,
        translation: Offset,
        imageWidth: Float,
        imageHeight: Float,
        surfaceWidth: Float,
        surfaceHeight: Float,
    ): ThemeImageBackground {
        val overflow = overflow(imageWidth, imageHeight, surfaceWidth, surfaceHeight, start.zoom)
        val axes = axes(overflowing = overflow)
        return start.withFocus(
            x = if (axes.horizontal) start.focusX + translation.x / overflow.x else start.focusX,
            y = if (axes.vertical) start.focusY + translation.y / overflow.y else start.focusY,
        )
    }

    /** [start] after a pinch of [magnification] (1 = unchanged); focus kept. */
    fun zoomed(
        start: ThemeImageBackground,
        magnification: Float,
    ): ThemeImageBackground = start.withZoom(start.zoom * magnification)

    /** [photo] with [axis]'s focus set to [value] clamped into 0..1 (also the TalkBack `setProgress` target). */
    fun withAxis(
        photo: ThemeImageBackground,
        axis: Orientation,
        value: Float,
    ): ThemeImageBackground =
        when (axis) {
            Orientation.Horizontal -> photo.withFocus(value, photo.focusY)
            Orientation.Vertical -> photo.withFocus(photo.focusX, value)
        }
}

private val STROKE_WIDTH = 3.dp
private val HALO_WIDTH = 2.dp
private val HEAD_LENGTH = 10.dp

/** Half the arrow length as a fraction of the preview's shorter side. */
private const val HALF_LENGTH_FRACTION = 0.25f

/** The zoom hint's gap radius and tip radius, as fractions of the half length. */
private const val ZOOM_HINT_INNER_FRACTION = 0.35f
private const val ZOOM_HINT_OUTER_FRACTION = 0.85f
private val ZOOM_HINT_DIRECTION = Offset(1f, 1f) / sqrt(2f)
private val HALO_COLOR = Color.Black.copy(alpha = 0.6f)

/**
 * The gesture surface laid over the live preview while the background is a photo: swallows the
 * preview keys' touches, draws a double-headed arrow along each axis the photo can move, and
 * reports every gesture through [PhotoPositionDrag] to [onPhotoChange] — one finger drags (the
 * photo follows it), two fingers pinch the zoom (their pan is ignored, as on iOS). The gesture
 * accumulates into a local photo, so events that land before recomposition never read a stale
 * one. For TalkBack it is one adjustable element labelled [label] that steps the vertical (else
 * horizontal) axis. Mirrors iOS PhotoPositionOverlay.
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
        val axes = PhotoPositionDrag.axes(photo, imageWidth.toFloat(), imageHeight.toFloat(), surfaceWidth, surfaceHeight)
        val accessibilityAxis = PhotoPositionDrag.accessibilityAxis(axes)
        val currentPhoto by rememberUpdatedState(photo)
        val currentOnPhotoChange by rememberUpdatedState(onPhotoChange)

        Box(
            modifier =
                Modifier
                    .matchParentSize()
                    .pointerInput(imageWidth, imageHeight, surfaceWidth, surfaceHeight) {
                        // Hand-rolled, not detectTransformGestures: that waits for touch slop and
                        // pans with a pinch's centroid; here the photo moves on first contact and a
                        // pinch only zooms (iOS parity).
                        awaitEachGesture {
                            awaitFirstDown().consume()
                            var gesturePhoto = currentPhoto
                            var previousPressed = 1
                            do {
                                val event = awaitPointerEvent()
                                val pressed = event.changes.count { it.pressed }
                                val next =
                                    when {
                                        pressed >= 2 -> PhotoPositionDrag.zoomed(gesturePhoto, event.calculateZoom())
                                        // A finger just lifted off a pinch: the centroid jumps, so this
                                        // frame re-bases the drag instead of moving the photo (iOS parity).
                                        pressed != previousPressed -> gesturePhoto
                                        else ->
                                            PhotoPositionDrag.dragged(
                                                gesturePhoto,
                                                event.calculatePan(),
                                                imageWidth.toFloat(),
                                                imageHeight.toFloat(),
                                                surfaceWidth,
                                                surfaceHeight,
                                            )
                                    }
                                previousPressed = pressed
                                event.changes.forEach { it.consume() }
                                if (next != gesturePhoto) {
                                    gesturePhoto = next
                                    currentOnPhotoChange(next)
                                }
                            } while (event.changes.any { it.pressed })
                        }
                    }.then(
                        if (accessibilityAxis == null) {
                            // Nothing to move: hidden from TalkBack.
                            Modifier.clearAndSetSemantics {}
                        } else {
                            val axisValue = if (accessibilityAxis == Orientation.Horizontal) photo.focusX else photo.focusY
                            Modifier.semantics {
                                contentDescription = label
                                stateDescription = "${(axisValue * 100).roundToInt()}%"
                                progressBarRangeInfo = ProgressBarRangeInfo(axisValue, 0f..1f, PhotoPositionDrag.ACCESSIBILITY_RANGE_STEPS)
                                setProgress { target ->
                                    currentOnPhotoChange(PhotoPositionDrag.withAxis(currentPhoto, accessibilityAxis, target))
                                    true
                                }
                            }
                        },
                    ),
        ) { PhotoPositionArrow(axes, Modifier.matchParentSize()) }
    }
}

/**
 * A double-headed arrow through the centre along each of [axes] (a cross when both, nothing when
 * neither), plus a shorter diagonal ↖↘ arrow — the "expand" hint that the photo also pinches —
 * drawn every time, since zoom is always possible. The diagonal leaves a gap at the centre so the
 * move cross stays readable. Its own composable on stable inputs so a drag (which changes the
 * photo, not the axes) skips the redraw. Mirrors iOS PhotoPositionArrow.
 */
@Composable
private fun PhotoPositionArrow(
    axes: PhotoAxes,
    modifier: Modifier,
) {
    Canvas(modifier) {
        val halfLength = min(size.width, size.height) * HALF_LENGTH_FRACTION
        if (axes.horizontal) drawPositionArrow(Offset(1f, 0f), inner = 0f, outer = halfLength)
        if (axes.vertical) drawPositionArrow(Offset(0f, 1f), inner = 0f, outer = halfLength)
        drawPositionArrow(ZOOM_HINT_DIRECTION, inner = halfLength * ZOOM_HINT_INNER_FRACTION, outer = halfLength * ZOOM_HINT_OUTER_FRACTION)
    }
}

// A double-headed arrow along the unit vector [along]: on each side of the centre a shaft from
// radius [inner] out to the tip at radius [outer]. White on a dark halo reads on any photo: every
// stroke is drawn twice, halo first.
private fun DrawScope.drawPositionArrow(
    along: Offset,
    inner: Float,
    outer: Float,
) {
    val across = Offset(-along.y, along.x)
    val headLengthPx = HEAD_LENGTH.toPx()
    val strokeWidthPx = STROKE_WIDTH.toPx()
    val haloWidthPx = HALO_WIDTH.toPx()

    for ((color, width) in listOf(HALO_COLOR to strokeWidthPx + haloWidthPx * 2, Color.White to strokeWidthPx)) {
        for (sign in listOf(1f, -1f)) {
            val direction = along * sign
            val tip = center + direction * outer
            drawLine(color, center + direction * inner, tip, strokeWidth = width, cap = StrokeCap.Round)
            // Arrowhead: two strokes swept back from the tip at 45°.
            for (side in listOf(1f, -1f)) {
                drawLine(color, tip, tip - (direction - across * side) * headLengthPx, strokeWidth = width, cap = StrokeCap.Round)
            }
        }
    }
}
