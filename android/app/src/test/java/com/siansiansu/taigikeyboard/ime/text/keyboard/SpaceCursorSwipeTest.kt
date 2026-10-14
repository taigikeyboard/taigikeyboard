package com.siansiansu.taigikeyboard.ime.text.keyboard

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

// Pins the spacebar-swipe step mapping: activation threshold, one step per unit, reversal.
class SpaceCursorSwipeTest {
    private val unit = 8

    private fun swipe() = SpaceCursorSwipe(downX = 100, downY = 50, unitPx = unit)

    @Test
    fun activatesOnlyAfterTwoUnitsSideways() {
        val swipe = swipe()
        assertFalse(swipe.shouldActivate(115, 50))
        assertTrue(swipe.shouldActivate(116, 50))
        assertTrue(swipe.shouldActivate(84, 50))
        assertFalse(swipe.isActive)
    }

    @Test
    fun mostlyVerticalDragDoesNotActivate() {
        val swipe = swipe()
        // dx 16 reaches the threshold but dy 20 dominates — an upward flick, not a swipe.
        assertFalse(swipe.shouldActivate(116, 30))
        assertFalse(swipe.shouldActivate(116, 66))
        assertTrue(swipe.shouldActivate(116, 65))
    }

    @Test
    fun rightSwipeStepsOncePerUnitAndReverses() {
        val swipe = swipe()
        // trace: activate(116) → origin = 116 - 8 - 4 = 104, first step +1.
        assertEquals(1, swipe.activate(116))
        assertTrue(swipe.isActive)
        // floorDiv(119 - 104, 8) = 1 → no new step; 120 → 2.
        assertEquals(0, swipe.stepsTo(119))
        assertEquals(1, swipe.stepsTo(120))
        // floorDiv(136 - 104, 8) = 4 → two steps in one MOVE batch.
        assertEquals(2, swipe.stepsTo(136))
        // Back past the origin: floorDiv(7, 8) = 0, floorDiv(-1, 8) = -1.
        assertEquals(-4, swipe.stepsTo(111))
        assertEquals(-1, swipe.stepsTo(103))
        // floorDiv(-8, 8) = -1, floorDiv(-9, 8) = -2: one unit per step on the far side too.
        assertEquals(0, swipe.stepsTo(96))
        assertEquals(-1, swipe.stepsTo(95))
    }

    @Test
    fun jitterRightAfterActivationDoesNotStepBack() {
        val swipe = swipe()
        swipe.activate(116)
        // floorDiv(112 - 104, 8) = 1 — half a unit back keeps the first step.
        assertEquals(0, swipe.stepsTo(112))
        assertEquals(-1, swipe.stepsTo(111))
    }

    @Test
    fun leftSwipeMirrorsRight() {
        val swipe = swipe()
        // trace: activate(84) → origin = 84 + 4 = 88, first step -1.
        assertEquals(-1, swipe.activate(84))
        // floorDiv(84 - 88, 8) = -1 → no new step; floorDiv(-8, 8) = -1; floorDiv(-9, 8) = -2.
        assertEquals(0, swipe.stepsTo(84))
        assertEquals(0, swipe.stepsTo(80))
        assertEquals(-1, swipe.stepsTo(79))
        // Back to the origin: floorDiv(0, 8) = 0 → two steps right.
        assertEquals(2, swipe.stepsTo(88))
    }
}
