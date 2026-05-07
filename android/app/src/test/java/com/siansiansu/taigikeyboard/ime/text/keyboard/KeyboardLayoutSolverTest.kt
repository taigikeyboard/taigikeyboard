package com.siansiansu.taigikeyboard.ime.text.keyboard

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** Pure-JVM tests for [KeyboardLayoutSolver]. */
class KeyboardLayoutSolverTest {
    // --- KeyboardHeightFactor.fromPreferenceString ---

    @Test
    fun `fromPreferenceString maps each documented value`() {
        assertEquals(KeyboardHeightFactor.EXTRA_SHORT, KeyboardHeightFactor.fromPreferenceString("extra_short"))
        assertEquals(KeyboardHeightFactor.SHORT, KeyboardHeightFactor.fromPreferenceString("short"))
        assertEquals(KeyboardHeightFactor.MID_SHORT, KeyboardHeightFactor.fromPreferenceString("mid_short"))
        assertEquals(KeyboardHeightFactor.NORMAL, KeyboardHeightFactor.fromPreferenceString("normal"))
        assertEquals(KeyboardHeightFactor.MID_TALL, KeyboardHeightFactor.fromPreferenceString("mid_tall"))
        assertEquals(KeyboardHeightFactor.TALL, KeyboardHeightFactor.fromPreferenceString("tall"))
        assertEquals(KeyboardHeightFactor.EXTRA_TALL, KeyboardHeightFactor.fromPreferenceString("extra_tall"))
    }

    @Test
    fun `fromPreferenceString defaults unknown values to NORMAL`() {
        assertEquals(KeyboardHeightFactor.NORMAL, KeyboardHeightFactor.fromPreferenceString(""))
        assertEquals(KeyboardHeightFactor.NORMAL, KeyboardHeightFactor.fromPreferenceString("garbage"))
    }

    @Test
    fun `each KeyboardHeightFactor exposes the legacy multiplier`() {
        assertEquals(0.85f, KeyboardHeightFactor.EXTRA_SHORT.multiplier)
        assertEquals(0.90f, KeyboardHeightFactor.SHORT.multiplier)
        assertEquals(0.95f, KeyboardHeightFactor.MID_SHORT.multiplier)
        assertEquals(1.00f, KeyboardHeightFactor.NORMAL.multiplier)
        assertEquals(1.05f, KeyboardHeightFactor.MID_TALL.multiplier)
        assertEquals(1.10f, KeyboardHeightFactor.TALL.multiplier)
        assertEquals(1.15f, KeyboardHeightFactor.EXTRA_TALL.multiplier)
    }

    // --- solveKeyDimensions ---

    @Test
    fun `solveKeyDimensions divides container by 10 and subtracts twice keyMarginH`() {
        val result = KeyboardLayoutSolver.solveKeyDimensions(
            KeyDimensionsInput(
                containerWidth = 1080,
                keyMarginH = 5,
                baseKeyHeight = 200f,
                isLandscape = false,
                heightFactor = KeyboardHeightFactor.NORMAL,
                keyHeightScale = 1.0f,
            ),
        )

        assertEquals(98, result.desiredKeyWidth)
        assertEquals(200, result.desiredKeyHeight)
        assertEquals(1.0f, result.keyHeightFactor)
    }

    @Test
    fun `solveKeyDimensions applies the landscape orientation multiplier`() {
        val result = KeyboardLayoutSolver.solveKeyDimensions(
            KeyDimensionsInput(
                containerWidth = 2000,
                keyMarginH = 0,
                baseKeyHeight = 200f,
                isLandscape = true,
                heightFactor = KeyboardHeightFactor.NORMAL,
                keyHeightScale = 1.0f,
            ),
        )

        assertEquals(0.85f, result.keyHeightFactor)
        assertEquals(170, result.desiredKeyHeight)
    }

    @Test
    fun `solveKeyDimensions multiplies orientation factor by heightFactor and scale`() {
        val result = KeyboardLayoutSolver.solveKeyDimensions(
            KeyDimensionsInput(
                containerWidth = 1000,
                keyMarginH = 0,
                baseKeyHeight = 100f,
                isLandscape = false,
                heightFactor = KeyboardHeightFactor.EXTRA_TALL,
                keyHeightScale = 1.2f,
            ),
        )

        // 1.0 * 1.15 * 1.2 = 1.38
        assertEquals(1.38f, result.keyHeightFactor, 1e-5f)
        assertEquals(138, result.desiredKeyHeight)
    }

    @Test
    fun `solveKeyDimensions truncates fractional desiredKeyHeight via toInt`() {
        val result = KeyboardLayoutSolver.solveKeyDimensions(
            KeyDimensionsInput(
                containerWidth = 0,
                keyMarginH = 0,
                baseKeyHeight = 7f,
                isLandscape = false,
                heightFactor = KeyboardHeightFactor.MID_SHORT,
                keyHeightScale = 1.0f,
            ),
        )

        // 7 * 0.95 = 6.65 → toInt = 6
        assertEquals(6, result.desiredKeyHeight)
    }

    @Test
    fun `solveKeyDimensions preserves fractional baseKeyHeight precision through one truncation`() {
        // Regression for Codex PR #227 r3202664511: a previous draft truncated
        // baseKeyHeight to Int before applying multipliers, which on densities
        // where R.dimen.key_height resolves to a non-integer Float can drift
        // by 1px versus the legacy `(getDimension * factor).toInt()` shape.
        val result = KeyboardLayoutSolver.solveKeyDimensions(
            KeyDimensionsInput(
                containerWidth = 0,
                keyMarginH = 0,
                baseKeyHeight = 200.99f,
                isLandscape = false,
                heightFactor = KeyboardHeightFactor.EXTRA_TALL,
                keyHeightScale = 1.0f,
            ),
        )

        // Float math: 200.99 * 1.15 = 231.1385 → toInt = 231 (legacy).
        // With the bug:    200    * 1.15 = 230.0    → toInt = 230.
        assertEquals(231, result.desiredKeyHeight)
    }

    // --- solvePopupDimensions ---

    @Test
    fun `solvePopupDimensions in portrait scales width and height by the portrait multipliers`() {
        val result = KeyboardLayoutSolver.solvePopupDimensions(
            PopupDimensionsInput(
                desiredKeyWidth = 100,
                desiredKeyHeight = 200,
                keyViewMeasuredWidth = 100,
                isLandscape = false,
            ),
        )

        assertEquals(110, result.popupWidth)
        assertEquals(500, result.popupHeight)
        // diffX = (100 - 110) / 2 = -5
        assertEquals(-5, result.popupDiffX)
        assertEquals(-5, result.popupX)
        assertEquals(-500, result.popupY)
    }

    @Test
    fun `solvePopupDimensions in landscape scales width and height by the landscape multipliers`() {
        val result = KeyboardLayoutSolver.solvePopupDimensions(
            PopupDimensionsInput(
                desiredKeyWidth = 100,
                desiredKeyHeight = 200,
                keyViewMeasuredWidth = 100,
                isLandscape = true,
            ),
        )

        assertEquals(60, result.popupWidth)
        assertEquals(600, result.popupHeight)
        // diffX = (100 - 60) / 2 = 20
        assertEquals(20, result.popupDiffX)
        assertEquals(20, result.popupX)
        assertEquals(-600, result.popupY)
    }

    // --- solveExtendedPopupGeometry — row split ---

    @Test
    fun `solveExtendedPopupGeometry with popupCount up to 10 stays single-row`() {
        for (count in 0..10) {
            val result = KeyboardLayoutSolver.solveExtendedPopupGeometry(
                ExtendedPopupGeometryInput(
                    popupCount = count,
                    keyViewX = 0f,
                    keyboardViewMeasuredWidth = 1080,
                    keyViewMeasuredWidth = 100,
                    keyViewMeasuredHeight = 200,
                    keyPopupWidth = 110,
                    keyPopupHeight = 500,
                ),
            )

            assertEquals("popupCount=$count", count, result.row0count)
            assertEquals("popupCount=$count", 0, result.row1count)
        }
    }

    @Test
    fun `solveExtendedPopupGeometry with odd count above 10 puts extra key on row0`() {
        val result = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 11,
                keyViewX = 0f,
                keyboardViewMeasuredWidth = 1080,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )

        assertEquals(6, result.row0count)
        assertEquals(5, result.row1count)
    }

    @Test
    fun `solveExtendedPopupGeometry with even count above 10 splits evenly`() {
        val result = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 12,
                keyViewX = 0f,
                keyboardViewMeasuredWidth = 1080,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )

        assertEquals(6, result.row0count)
        assertEquals(6, result.row1count)
    }

    // --- solveExtendedPopupGeometry — anchor side ---

    @Test
    fun `solveExtendedPopupGeometry anchors LEFT when keyViewX is in left half`() {
        val result = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 3,
                keyViewX = 100f,
                keyboardViewMeasuredWidth = 1080,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )

        assertEquals(AnchorSide.LEFT, result.anchorSide)
    }

    @Test
    fun `solveExtendedPopupGeometry anchors RIGHT when keyViewX is at or past midpoint`() {
        val result = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 3,
                keyViewX = 540f,
                keyboardViewMeasuredWidth = 1080,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )

        assertEquals(AnchorSide.RIGHT, result.anchorSide)
    }

    // --- solveExtendedPopupGeometry — anchor offset clamp ---

    @Test
    fun `solveExtendedPopupGeometry anchorOffset is zero when row0 holds at most one key`() {
        val result = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 1,
                keyViewX = 100f,
                keyboardViewMeasuredWidth = 1080,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )

        assertEquals(0, result.anchorOffset)
    }

    @Test
    fun `solveExtendedPopupGeometry anchorOffset clamps down when available space is tight`() {
        // row0count=5 (popupCount=5), default offset=2, anchor LEFT.
        // Derived diffX = (100 - 110) / 2 = -5.
        // availableSpace = keyViewX + diffX = 100 + (-5) = 95.
        // 95 < 2*110 → offset=1; 95 < 1*110 → offset=0; loop exits.
        val result = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 5,
                keyViewX = 100f,
                keyboardViewMeasuredWidth = 1080,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )

        assertEquals(0, result.anchorOffset)
    }

    @Test
    fun `solveExtendedPopupGeometry anchorOffset keeps default when space is ample`() {
        // row0count=5, default offset=2, LEFT anchor, large availableSpace.
        val result = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 5,
                keyViewX = 500f,
                keyboardViewMeasuredWidth = 2000,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )

        // availableSpace = 500 + (-5) = 495; 495 >= 2*110=220 → offset stays at 2.
        assertEquals(2, result.anchorOffset)
    }

    @Test
    fun `solveExtendedPopupGeometry anchorOffset for even row0count uses size-half-minus-one default`() {
        // popupCount=12 → row0count=6, row1count=6. Even-length default
        // offset = (6/2) - 1 = 2.
        val result = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 12,
                keyViewX = 800f,
                keyboardViewMeasuredWidth = 2000,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )

        // availableSpace = 800 + (-5) = 795 >= 2*110=220 → offset stays at 2.
        assertEquals(2, result.anchorOffset)
    }

    @Test
    fun `solveExtendedPopupGeometry RIGHT anchor uses right-edge availableSpace`() {
        // popupCount=5 → row0count=5, default offset=2. RIGHT anchor.
        // Derived diffX = (100 - 110) / 2 = -5.
        // availableSpace = kbdWidth - (keyViewX + diffX + keyPopupWidth)
        //                = 1080 - (900 + (-5) + 110) = 1080 - 1005 = 75.
        // 75 < 2*110=220 → offset=1; 75 < 1*110=110 → offset=0.
        val result = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 5,
                keyViewX = 900f,
                keyboardViewMeasuredWidth = 1080,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )

        assertEquals(AnchorSide.RIGHT, result.anchorSide)
        assertEquals(0, result.anchorOffset)
    }

    // --- solveExtendedPopupGeometry — extWidth / extHeight ---

    @Test
    fun `solveExtendedPopupGeometry extHeight doubles when row1 is non-empty`() {
        val singleRow = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 5,
                keyViewX = 500f,
                keyboardViewMeasuredWidth = 2000,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )
        val doubleRow = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 12,
                keyViewX = 500f,
                keyboardViewMeasuredWidth = 2000,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )

        assertEquals(200, singleRow.extHeight)
        assertEquals(400, doubleRow.extHeight)
        assertEquals(5 * 110, singleRow.extWidth)
        assertEquals(6 * 110, doubleRow.extWidth)
    }

    // --- solveExtendedPopupGeometry — placement ---

    @Test
    fun `solveExtendedPopupGeometry LEFT anchor shifts popupX left by anchorOffset times keyPopupWidth`() {
        // popupCount=5 → offset default=2 with ample space; LEFT anchor.
        // popupX = (100-110)/2 + (-2*110) = -5 + -220 = -225.
        val result = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 5,
                keyViewX = 500f,
                keyboardViewMeasuredWidth = 2000,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )

        assertTrue("expected LEFT", result.anchorSide == AnchorSide.LEFT)
        assertEquals(-225, result.popupX)
        assertEquals(-500, result.popupY)
    }

    @Test
    fun `solveExtendedPopupGeometry RIGHT anchor uses negative-extWidth-plus-keyPopupWidth-plus-offset shift`() {
        // popupCount=5 → row0count=5, extWidth=550, default offset=2.
        // keyViewX=1900, kbdWidth=2000 → RIGHT.
        // availableSpace = 2000 - (1900-5+110) = 2000 - 2005 = -5; tight.
        // -5 < 2*110 → offset=1; -5 < 1*110 → offset=0.
        // popupX = (100-110)/2 + (-550 + 110 + 0*110) = -5 + -440 = -445.
        val result = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 5,
                keyViewX = 1900f,
                keyboardViewMeasuredWidth = 2000,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )

        assertEquals(AnchorSide.RIGHT, result.anchorSide)
        assertEquals(0, result.anchorOffset)
        assertEquals(-445, result.popupX)
    }

    @Test
    fun `solveExtendedPopupGeometry popupY accounts for the second row when present`() {
        val doubleRow = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = 12,
                keyViewX = 500f,
                keyboardViewMeasuredWidth = 2000,
                keyViewMeasuredWidth = 100,
                keyViewMeasuredHeight = 200,
                keyPopupWidth = 110,
                keyPopupHeight = 500,
            ),
        )

        // popupY = -500 - 200 = -700
        assertEquals(-700, doubleRow.popupY)
    }
}
