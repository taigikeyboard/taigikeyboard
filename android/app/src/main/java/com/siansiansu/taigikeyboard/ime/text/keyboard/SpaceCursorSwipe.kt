package com.siansiansu.taigikeyboard.ime.text.keyboard

import kotlin.math.abs

// Turns a horizontal drag on the spacebar into signed cursor steps (Gboard / FlorisBoard
// space swipe).
//
// Nothing moves until the finger has travelled [ACTIVATION_UNITS] units sideways, and further
// sideways than vertically — a tap's jitter or an upward flick stays a space press. Activation
// emits the first step and rebases the origin around the finger, so from then on every [unitPx]
// of travel in either direction is one step with no dead zone on the way back.
// FlorisBoard reference: `SwipeGesture.kt:76-84` (unit = 32 dp / 4) and
// `TextKeyboardLayout.kt:840` (the first unit is dropped).
internal class SpaceCursorSwipe(
    private val downX: Int,
    private val downY: Int,
    private val unitPx: Int,
) {
    var isActive: Boolean = false
        private set

    private var originX: Int = 0
    private var emittedSteps: Int = 0

    /** Whether the pointer at ([x], [y]) has crossed the activation threshold horizontally. */
    fun shouldActivate(
        x: Int,
        y: Int,
    ): Boolean {
        val dx = abs(x - downX)
        return dx >= ACTIVATION_UNITS * unitPx && dx > abs(y - downY)
    }

    /** Starts the swipe at [x] and returns its first step (±1). */
    fun activate(x: Int): Int {
        isActive = true
        // Put the finger mid-unit of step ±1, so half a unit of jitter at activation never steps
        // back.
        originX = if (x >= downX) x - unitPx - unitPx / 2 else x + unitPx / 2
        return stepsTo(x)
    }

    /** Signed steps to move since the last call; 0 when the finger has not crossed a unit. */
    fun stepsTo(x: Int): Int {
        // Floor, not truncation: truncation would make the unit on each side of the origin step 0.
        val target = Math.floorDiv(x - originX, unitPx)
        val delta = target - emittedSteps
        emittedSteps = target
        return delta
    }

    private companion object {
        const val ACTIVATION_UNITS = 2
    }
}
