package com.siansiansu.taigikeyboard.ime.text.candidates

import androidx.compose.ui.graphics.Color
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Pins the candidate-cell background precedence shared by the strip and the expanded
 * overlay: pressed > first candidate (keycap-color hint) > transparent. Mirrors iOS
 * `BuiltInThemesTests.testResolvedBackgroundColor_firstCandidateLightBeatsSelection`.
 *
 * INVARIANT_CANDIDATE_FIRST_KEYCAP_HINT (behavioral-invariants.md §19)
 */
class CandidateCellBackgroundTest {
    private val keycap = Color.Green
    private val pressed = Color.Red

    private fun background(
        isPressed: Boolean,
        isFirstCandidate: Boolean,
    ) = candidateCellBackground(isPressed, isFirstCandidate, firstCandidateColor = keycap, pressedColor = pressed)

    @Test
    fun `first candidate not pressed shows the keycap hint`() {
        assertEquals(keycap, background(isPressed = false, isFirstCandidate = true))
    }

    @Test
    fun `press on the first candidate beats the keycap hint`() {
        assertEquals(pressed, background(isPressed = true, isFirstCandidate = true))
    }

    @Test
    fun `press on another candidate shows the pressed color`() {
        assertEquals(pressed, background(isPressed = true, isFirstCandidate = false))
    }

    @Test
    fun `other candidates stay transparent`() {
        assertEquals(Color.Transparent, background(isPressed = false, isFirstCandidate = false))
    }
}
