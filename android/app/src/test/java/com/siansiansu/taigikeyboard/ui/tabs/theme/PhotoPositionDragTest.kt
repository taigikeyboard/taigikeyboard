package com.siansiansu.taigikeyboard.ui.tabs.theme

import androidx.compose.foundation.gestures.Orientation
import androidx.compose.ui.geometry.Offset
import com.siansiansu.taigikeyboard.ime.core.ThemeImageBackground
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Tests for [PhotoPositionDrag] — the drag-on-preview -> photo focus math behind the theme
 * editor's drag-to-move photo, and the TalkBack axis setter. Mirrors iOS
 * ThemeBackgroundTests.testPhotoPositionDrag_followsFingerOnOverflowingAxis.
 */
class PhotoPositionDragTest {
    private val centred = ThemeImageBackground("a.jpg")

    // A 100x300 photo over a 100x100 preview covers 100x300 -> overflow (0, -200), vertical axis;
    // a 200x100 photo overflows horizontally; a 50x50 photo fits exactly -> nothing to move.
    @Test
    fun axis_isTheOverflowingOne() {
        assertEquals(Orientation.Vertical, PhotoPositionDrag.axis(100f, 300f, 100f, 100f))
        assertEquals(Orientation.Horizontal, PhotoPositionDrag.axis(200f, 100f, 100f, 100f))
        assertNull(PhotoPositionDrag.axis(50f, 50f, 100f, 100f))
    }

    // Dragging down 50 moves the photo with the finger: 0.5 + 50 / -200 = 0.25 (more of the top shows);
    // only the vertical axis moves and Fade is kept; a long drag up clamps at 1.
    @Test
    fun dragged_followsFingerAndClamps() {
        assertEquals(centred.copy(focusY = 0.25f), PhotoPositionDrag.dragged(centred, Offset(30f, 50f), Orientation.Vertical, 100f, 300f, 100f, 100f))
        assertEquals(1f, PhotoPositionDrag.dragged(centred, Offset(0f, -1000f), Orientation.Vertical, 100f, 300f, 100f, 100f).focusY)
        // Horizontal overflow: 200x100 over 100x100 -> overflow -100; drag right 20 -> 0.5 - 0.2 = 0.3.
        assertEquals(0.3f, PhotoPositionDrag.dragged(centred, Offset(20f, 0f), Orientation.Horizontal, 200f, 100f, 100f, 100f).focusX, 1e-6f)
    }

    // TalkBack setProgress writes only the movable axis, clamped into 0..1.
    @Test
    fun withAxis_setsOneAxisClamped() {
        assertEquals(centred.copy(focusY = 1f), PhotoPositionDrag.withAxis(centred, Orientation.Vertical, 1.4f))
        assertEquals(centred.copy(focusX = 0.1f), PhotoPositionDrag.withAxis(centred, Orientation.Horizontal, 0.1f))
    }
}
