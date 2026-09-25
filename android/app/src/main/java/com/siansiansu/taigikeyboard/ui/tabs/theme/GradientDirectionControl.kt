package com.siansiansu.taigikeyboard.ui.tabs.theme

import android.view.HapticFeedbackConstants
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.drag
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
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.setProgress
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.center
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.toOffset
import com.siansiansu.taigikeyboard.ime.core.SurfaceVector
import com.siansiansu.taigikeyboard.ime.core.ThemeGradient
import kotlin.math.abs
import kotlin.math.ceil
import kotlin.math.floor
import kotlin.math.hypot
import kotlin.math.min
import kotlin.math.roundToInt

// The theme editor's gradient direction control: the drag surface over the live preview and
// the finger -> angle math behind it. Mirrors iOS GradientDirectionControl.swift.

/**
 * Maps a finger on the live keyboard preview to a [ThemeGradient.angle]: the direction from
 * the preview's centre to the finger, in the CSS / Figma convention the model uses (`0` =
 * bottom->top, clockwise). Angles within [SNAP_TOLERANCE] of a 45° preset snap to it;
 * everything else is a whole degree. Mirrors iOS GradientDirectionDrag.
 */
private const val FULL_TURN = 360f

object GradientDirectionDrag {
    private const val PRESET_STEP = ThemeGradient.PRESET_STEP

    /** Half-width of the snap window around each preset, in degrees. */
    const val SNAP_TOLERANCE = 6f

    /** Radius around the centre where the direction is too jittery to trust. */
    val DEAD_ZONE = 8.dp

    /**
     * The angle of the point `(pointX, pointY)` seen from `(centerX, centerY)`, snapped /
     * rounded into `0 until 360`; null while the finger is inside the dead zone.
     */
    fun angle(
        centerX: Float,
        centerY: Float,
        pointX: Float,
        pointY: Float,
        deadZonePx: Float,
    ): Float? {
        val dx = pointX - centerX
        val dy = pointY - centerY
        if (hypot(dx, dy) < deadZonePx) return null
        return snapped(ThemeGradient.degrees(dx, dy))
    }

    /**
     * [degrees] snapped to the nearest preset when within [SNAP_TOLERANCE], else rounded to a
     * whole degree; always in `0 until 360`.
     */
    fun snapped(degrees: Float): Float {
        val nearestPreset = (degrees / PRESET_STEP).roundToInt() * PRESET_STEP
        return wrapped(if (abs(degrees - nearestPreset) <= SNAP_TOLERANCE) nearestPreset else degrees.roundToInt().toFloat())
    }

    fun isPreset(angle: Float): Boolean = angle % PRESET_STEP == 0f

    /**
     * The next preset clockwise or counter-clockwise from [angle] — the TalkBack adjustment
     * step, so the direction stays settable without the drag.
     */
    fun steppedPreset(
        angle: Float,
        clockwise: Boolean,
    ): Float {
        val steps = angle / PRESET_STEP
        return wrapped((if (clockwise) floor(steps) + 1 else ceil(steps) - 1) * PRESET_STEP)
    }

    /** [degrees] reduced into `0 until 360` for any sign. */
    private fun wrapped(degrees: Float): Float = ((degrees % FULL_TURN) + FULL_TURN) % FULL_TURN
}

private val STROKE_WIDTH = 3.dp
private val HALO_WIDTH = 2.dp
private val HEAD_LENGTH = 12.dp
private val CENTRE_DOT_RADIUS = 4.dp

/** Half the axis length as a fraction of the preview's shorter side. */
private const val AXIS_HALF_LENGTH = 0.3f
private const val HEAD_SWEEP_DEGREES = 150f
private val HALO_COLOR = Color.Black.copy(alpha = 0.6f)

/** The presets strictly between 0° and 360°, for the TalkBack range. */
private val PRESET_RANGE_STEPS = (FULL_TURN / ThemeGradient.PRESET_STEP).toInt() - 1

/**
 * The drag surface laid over the live preview while the background is a gradient: swallows
 * the preview keys' touches, draws the current direction as an axis through the centre with an
 * arrowhead at the gradient's end, and reports every drag position through
 * [GradientDirectionDrag] to [onAngleChange]. Landing on a preset ticks. The pointer is the
 * whole control (USER 2026-09-19: no Direction row — seeing the pointer is enough); for TalkBack
 * it is one adjustable element labelled [label] that steps through the 45° presets.
 * Mirrors iOS GradientDirectionOverlay.
 */
@Composable
fun GradientDirectionOverlay(
    label: String,
    angle: Float,
    onAngleChange: (Float) -> Unit,
    modifier: Modifier = Modifier,
) {
    val view = LocalView.current
    val deadZonePx = with(LocalDensity.current) { GradientDirectionDrag.DEAD_ZONE.toPx() }
    val currentAngle by rememberUpdatedState(angle)
    val currentOnAngleChange by rememberUpdatedState(onAngleChange)
    // Touch samples that land on the angle already set are dropped so a finger resting on a
    // preset ticks once, not once per sample.
    val commit: (Float) -> Unit = { next ->
        if (next != currentAngle) {
            if (GradientDirectionDrag.isPreset(next)) view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
            currentOnAngleChange(next)
        }
    }

    Canvas(
        modifier =
            modifier
                .pointerInput(deadZonePx) {
                    awaitEachGesture {
                        val center = size.center.toOffset()

                        fun update(position: Offset) {
                            GradientDirectionDrag.angle(center.x, center.y, position.x, position.y, deadZonePx)?.let(commit)
                        }
                        val down = awaitFirstDown()
                        down.consume()
                        update(down.position)
                        drag(down.id) { change ->
                            change.consume()
                            update(change.position)
                        }
                    }
                }.semantics {
                    contentDescription = label
                    stateDescription = "${angle.toInt()}°"
                    progressBarRangeInfo = ProgressBarRangeInfo(angle, 0f..360f, PRESET_RANGE_STEPS)
                    setProgress { target ->
                        commit(GradientDirectionDrag.steppedPreset(angle, clockwise = target > angle))
                        true
                    }
                },
    ) { drawDirectionAxis(angle) }
}

private fun SurfaceVector.toOffset(): Offset = Offset(dx, dy)

// White on a dark halo reads on any gradient: every shape is drawn twice, halo first.
private fun DrawScope.drawDirectionAxis(angle: Float) {
    val axis = ThemeGradient.direction(angle).toOffset() * (min(size.width, size.height) * AXIS_HALF_LENGTH)
    val tip = center + axis
    val tail = center - axis
    val headLengthPx = HEAD_LENGTH.toPx()
    val heads = listOf(HEAD_SWEEP_DEGREES, -HEAD_SWEEP_DEGREES).map { sweep -> tip + ThemeGradient.direction(angle + sweep).toOffset() * headLengthPx }
    val strokeWidthPx = STROKE_WIDTH.toPx()
    val haloWidthPx = HALO_WIDTH.toPx()
    val dotRadiusPx = CENTRE_DOT_RADIUS.toPx()

    for ((color, width) in listOf(HALO_COLOR to strokeWidthPx + haloWidthPx * 2, Color.White to strokeWidthPx)) {
        drawLine(color, tail, tip, strokeWidth = width, cap = StrokeCap.Round)
        heads.forEach { drawLine(color, tip, it, strokeWidth = width, cap = StrokeCap.Round) }
    }
    drawCircle(HALO_COLOR, radius = dotRadiusPx + haloWidthPx, center = center)
    drawCircle(Color.White, radius = dotRadiusPx, center = center)
}
