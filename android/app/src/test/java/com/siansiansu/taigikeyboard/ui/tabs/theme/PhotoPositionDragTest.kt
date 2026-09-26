package com.siansiansu.taigikeyboard.ui.tabs.theme

import androidx.compose.foundation.gestures.Orientation
import androidx.compose.ui.geometry.Offset
import com.siansiansu.taigikeyboard.ime.core.ThemeImageBackground
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Tests for [PhotoPositionDrag] — the drag / pinch-on-preview -> photo focus / zoom math behind
 * the theme editor's photo position control, and the TalkBack axis setter. Mirrors iOS
 * ThemeBackgroundTests.testPhotoPositionDrag_*.
 */
class PhotoPositionDragTest {
    private val centred = ThemeImageBackground("a.jpg")

    // A 100x300 photo over a 100x100 preview covers 100x300 -> overflow (0, -200), vertical only;
    // a 200x100 photo overflows horizontally; a 50x50 photo fits exactly -> nothing to move;
    // zoomed 2x the 100x300 photo covers 200x600 -> both axes overflow.
    @Test
    fun axes_areTheOverflowingOnes() {
        assertEquals(PhotoAxes(horizontal = false, vertical = true), PhotoPositionDrag.axes(centred, 100f, 300f, 100f, 100f))
        assertEquals(PhotoAxes(horizontal = true, vertical = false), PhotoPositionDrag.axes(centred, 200f, 100f, 100f, 100f))
        assertEquals(PhotoAxes(horizontal = false, vertical = false), PhotoPositionDrag.axes(centred, 50f, 50f, 100f, 100f))
        assertEquals(PhotoAxes(horizontal = true, vertical = true), PhotoPositionDrag.axes(centred.withZoom(2f), 100f, 300f, 100f, 100f))
    }

    // TalkBack steps vertical when it moves, else horizontal, else nothing.
    @Test
    fun accessibilityAxis_prefersVertical() {
        assertEquals(Orientation.Vertical, PhotoPositionDrag.accessibilityAxis(PhotoAxes(horizontal = true, vertical = true)))
        assertEquals(Orientation.Horizontal, PhotoPositionDrag.accessibilityAxis(PhotoAxes(horizontal = true, vertical = false)))
        assertNull(PhotoPositionDrag.accessibilityAxis(PhotoAxes(horizontal = false, vertical = false)))
    }

    // Dragging down 50 moves the photo with the finger: 0.5 + 50 / -200 = 0.25 (more of the top shows);
    // x does not overflow so it keeps 0.5 and Fade is kept; a long drag up clamps at 1.
    @Test
    fun dragged_followsFingerAndClamps() {
        assertEquals(centred.copy(focusY = 0.25f), PhotoPositionDrag.dragged(centred, Offset(30f, 50f), 100f, 300f, 100f, 100f))
        assertEquals(1f, PhotoPositionDrag.dragged(centred, Offset(0f, -1000f), 100f, 300f, 100f, 100f).focusY)
        // Horizontal overflow: 200x100 over 100x100 -> overflow -100; drag right 20 -> 0.5 - 0.2 = 0.3.
        assertEquals(0.3f, PhotoPositionDrag.dragged(centred, Offset(20f, 0f), 200f, 100f, 100f, 100f).focusX, 1e-6f)
    }

    // Zoomed 2x, a 100x300 photo covers 200x600 over 100x100 -> overflow (-100, -500): a drag of
    // (20, 50) moves both axes, x 0.5 + 20 / -100 = 0.3, y 0.5 + 50 / -500 = 0.4; zoom kept.
    @Test
    fun dragged_zoomedMovesBothAxes() {
        val dragged = PhotoPositionDrag.dragged(centred.withZoom(2f), Offset(20f, 50f), 100f, 300f, 100f, 100f)
        assertEquals(0.3f, dragged.focusX, 1e-6f)
        assertEquals(0.4f, dragged.focusY, 1e-6f)
        assertEquals(2f, dragged.zoom)
    }

    // A pinch multiplies the zoom and keeps the focus; 1x..2x clamps.
    @Test
    fun zoomed_multipliesAndClamps() {
        assertEquals(centred.copy(zoom = 1.5f), PhotoPositionDrag.zoomed(centred, 1.5f))
        assertEquals(ThemeImageBackground.ZOOM_MAX, PhotoPositionDrag.zoomed(centred.withZoom(1.5f), 3f).zoom)
        assertEquals(ThemeImageBackground.ZOOM_MIN, PhotoPositionDrag.zoomed(centred, 0.5f).zoom)
    }

    // TalkBack setProgress writes only the stepped axis, clamped into 0..1.
    @Test
    fun withAxis_setsOneAxisClamped() {
        assertEquals(centred.copy(focusY = 1f), PhotoPositionDrag.withAxis(centred, Orientation.Vertical, 1.4f))
        assertEquals(centred.copy(focusX = 0.1f), PhotoPositionDrag.withAxis(centred, Orientation.Horizontal, 0.1f))
    }
}
