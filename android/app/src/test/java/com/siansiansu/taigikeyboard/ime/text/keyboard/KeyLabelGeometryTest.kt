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

    // A8: number-row tone marks were drawn at a fixed baseline (0.66 h) in a font 1.3× the label;
    // on short tablet keys the mark touched both the key top and the digit.
    @Test
    fun toneMark_hugsTheDigit_withAGap_andClearsTheTop() {
        // trace: h = 100, digit ink top 32 → band [12, 26] = 14; acute ink -70..-58 (12 tall)
        // fits at scale 1; ink bottom at 26 → ink top 14 → baseline = 14 + 70 = 84.
        val fit = KeyLabelGeometry.hintAboveLabel(100f, 32f, inkTop = -70f, inkBottom = -58f, hugLabel = true)
        assertEquals(1f, fit.scale, eps)
        assertEquals(84f, fit.baseline, eps)
        assertEquals("ink bottom = digit top - gap", 26f, fit.baseline - 58f, eps)
    }

    @Test
    fun toneMark_onAShortKey_shrinksInsteadOfTouching() {
        // trace: band = 24 - 6 - 12 = 6; ink 12 tall → scale 0.5; ink bottom at 18, top at 12.
        val fit = KeyLabelGeometry.hintAboveLabel(100f, 24f, inkTop = -70f, inkBottom = -58f, hugLabel = true)
        assertEquals(0.5f, fit.scale, eps)
        assertEquals(12f, fit.baseline - 70f * fit.scale, eps)
        assertEquals(18f, fit.baseline - 58f * fit.scale, eps)
    }

    // A9 / A13: labels could grow to 90% of the key width — `?123`, `TPS` and wide Bopomofo
    // glyphs came within 4–9 px of the key edges at large display sizes.
    @Test
    fun multiCharacterLabel_keepsASideMarginProportionalToTheKeyHeight() {
        // trace: ?123 key 170×100 → 170 - 2*12 = 146 (old rule: 153, 8.5 px a side).
        assertEquals(146f, KeyLabelGeometry.maxLabelWidth(170f, 100f), eps)
        // Key height 1.5× (text is sized from it) → each margin 1.5×: 170 - 2*18 = 134.
        assertEquals(134f, KeyLabelGeometry.maxLabelWidth(170f, 150f), eps)
    }

    @Test
    fun label_onAVeryNarrowKey_keepsHalfTheKey() {
        assertEquals(20f, KeyLabelGeometry.maxLabelWidth(40f, 100f), eps)
        assertEquals(20f, KeyLabelGeometry.maxGlyphTextSize(40f, 400f), eps)
    }

    // Default letter text is 0.58 h. Key canvas = screen/10 - 6 dp wide, 38 dp × key-height scale.
    @Test
    fun oneGlyphLabel_atDefaultSizes_isNotShrunk() {
        for (screenDp in listOf(360f, 411f)) {
            val w = screenDp / 10f - 6f
            val h = 38f
            assertTrue("${screenDp}dp", KeyLabelGeometry.maxGlyphTextSize(w, h) >= 0.58f * h)
        }
    }

    @Test
    fun oneGlyphLabel_capIsTheSameForEveryGlyph_andKeepsA1emGlyphInside() {
        // Display size large (320 dp wide): w = 26, h = 38 → cap = 26 - 2*2.28 = 21.44 < 22.04,
        // the same for `m` as for `n`, so neighbouring keys keep one size.
        val cap = KeyLabelGeometry.maxGlyphTextSize(26f, 38f)
        assertEquals(21.44f, cap, eps)
        // A 1 em Bopomofo glyph at that size keeps GLYPH_SIDE_INSET on each side.
        assertEquals(38f * KeyLabelGeometry.GLYPH_SIDE_INSET, (26f - cap) / 2f, eps)
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
