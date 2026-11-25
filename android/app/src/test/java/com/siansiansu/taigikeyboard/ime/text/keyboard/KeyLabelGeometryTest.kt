package com.siansiansu.taigikeyboard.ime.text.keyboard

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** Pins the key-face text placement rules (layout audit A7–A9, A12, A13). */
class KeyLabelGeometryTest {
    private val eps = 0.001f

    // A7: TPS hints were drawn at a fixed baseline (0.28 h), their ink 3–4 px from the key top.
    @Test
    fun hint_inkSitsBetweenTheTopInsetAndTheLabel() {
        // trace: h = 100 → band = [12, labelInkTop 50 - gap 6 = 44] = 32; ink -24..0 (24 tall)
        // fits at scale 1, hugging the band top: ink top 12 → baseline 12 + 24 = 36.
        val fit = KeyLabelGeometry.hintAboveLabel(keyHeight = 100f, labelInkTop = 50f, inkTop = -24f, inkBottom = 0f)
        assertEquals(1f, fit.scale, eps)
        assertEquals(36f, fit.baseline, eps)
        val top = fit.baseline - 24f * fit.scale
        assertTrue("ink top $top below inset", top >= 100f * KeyLabelGeometry.EDGE_INSET - eps)
        assertTrue("ink bottom ${fit.baseline} above label gap", fit.baseline <= 50f - 100f * KeyLabelGeometry.HINT_GAP + eps)
    }

    @Test
    fun hint_tallerThanItsBand_shrinksToFit() {
        // trace: band = 30 - 6 - 12 = 12; ink 24 tall → scale 0.5, ink top at 12 → baseline 24.
        val fit = KeyLabelGeometry.hintAboveLabel(keyHeight = 100f, labelInkTop = 30f, inkTop = -24f, inkBottom = 0f)
        assertEquals(0.5f, fit.scale, eps)
        assertEquals(24f, fit.baseline, eps)
    }

    @Test
    fun hint_marginsScaleWithTheKey() {
        // Doubling the key (and so the text, sized from h) doubles every distance.
        val small = KeyLabelGeometry.hintAboveLabel(80f, 40f, -16f, 2f)
        val large = KeyLabelGeometry.hintAboveLabel(160f, 80f, -32f, 4f)
        assertEquals(small.scale, large.scale, eps)
        assertEquals(small.baseline * 2f, large.baseline, eps)
    }
}
