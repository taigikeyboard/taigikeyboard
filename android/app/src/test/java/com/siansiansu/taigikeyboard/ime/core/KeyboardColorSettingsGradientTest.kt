package com.siansiansu.taigikeyboard.ime.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Tests for the [KeyboardColorSettings] JSON envelope (blank / legacy role keys) and the
 * gradient-derived candidate tints. Background decoding (legacy keys, `type` discriminator,
 * angle) lives in [ThemeBackgroundTest].
 */
class KeyboardColorSettingsGradientTest {
    @Test
    fun fromJson_legacyRoleKeys_decode() {
        // A colorSettings JSON written before the background field existed.
        val legacy = """{"backgroundColor":-16777216,"keyTextColor":-1}"""
        val restored = KeyboardColorSettings.fromJson(legacy)
        assertNull(restored.backgroundGradient)
        assertEquals(ThemeBackground.Solid(0xFF000000.toInt()), restored.background)
        assertEquals(0xFFFFFFFF.toInt(), restored.keyTextColor)
    }

    @Test
    fun fromJson_emptyOrBlank_returnsDefault() {
        assertEquals(KeyboardColorSettings(), KeyboardColorSettings.fromJson("{}"))
        assertEquals(KeyboardColorSettings(), KeyboardColorSettings.fromJson(""))
    }

    // Candidate tints derive from the gradient first stop: highlight LIGHTENED toward white ×0.5,
    // pressed DEEPENED toward black ×0.65 (each 0-255 component truncated). Mirrors iOS.
    @Test
    fun candidateTints_lightenHighlight_deepenPressed() {
        // Sakura top E6C2D0 → highlight F2E0E7, pressed 957E87
        val pinkTop = 0xFFE6C2D0.toInt()
        assertEquals(0xFFF2E0E7.toInt(), lightenedArgb(pinkTop, CANDIDATE_HIGHLIGHT_LIGHTEN_FACTOR))
        assertEquals(0xFF957E87.toInt(), deepenedArgb(pinkTop, CANDIDATE_PRESSED_DEEPEN_FACTOR))
        // Sea Breeze top BFD2EA → highlight DFE8F4, pressed 7C8898
        val blueTop = 0xFFBFD2EA.toInt()
        assertEquals(0xFFDFE8F4.toInt(), lightenedArgb(blueTop, CANDIDATE_HIGHLIGHT_LIGHTEN_FACTOR))
        assertEquals(0xFF7C8898.toInt(), deepenedArgb(blueTop, CANDIDATE_PRESSED_DEEPEN_FACTOR))
    }

    @Test
    fun deepenedArgb_forcesOpaqueAlpha() {
        // Input alpha is ignored; output is always opaque (alpha 0xFF). factor 1.0 keeps RGB.
        assertEquals(0xFFFFFFFF.toInt(), deepenedArgb(0x00FFFFFF, 1.0))
        // lighten by 0 keeps RGB but still forces opaque alpha.
        assertEquals(0xFF112233.toInt(), lightenedArgb(0x00112233, 0.0))
    }

    // trace: user-theme seed = solid D4D5DD surface, key text 000000, key fill FFFFFF →
    //   fixedKeyFill FFFFFF; no gradient → tints (FFFFFF, FFFFFF deepened ×0.65:
    //   255×0.65 = 165.75 → 165 = A5) = (FFFFFF, A5A5A5). Night-mode free. Mirrors iOS.
    @Test
    fun candidateTints_userThemeUsesKeyFill() {
        val seed = UserThemeSeed.colors
        assertEquals(0xFFFFFFFF.toInt(), seed.fixedKeyFill)
        assertEquals(0xFFFFFFFF.toInt() to 0xFFA5A5A5.toInt(), seed.candidateTints)
    }

    // trace: gradient first stop E6C2D0 wins over the key fill → (F2E0E7, 957E87), as before.
    @Test
    fun candidateTints_gradientUsesFirstStop() {
        val colors =
            KeyboardColorSettings(
                background = ThemeBackground.Gradient(ThemeGradient(listOf(0xFFE6C2D0.toInt(), 0xFFFFFFFF.toInt()))),
                keyTextColor = 0xFF1C1C1E.toInt(),
            ).withKeyFill(0xFFFFFFFF.toInt())
        assertEquals(0xFFF2E0E7.toInt() to 0xFF957E87.toInt(), colors.candidateTints)
    }

    // trace: fixedKeyFill gate = custom surface + visible fill + key text.
    //   Default (all null) → null; transparent fill, no surface → null;
    //   gradient with transparent fill (Outlined/Borderless) → null, tints still from the stop.
    @Test
    fun fixedKeyFill_onlyForConcreteKeyPalettes() {
        assertNull(KeyboardColorSettings().fixedKeyFill)
        assertNull(KeyboardColorSettings().candidateTints)
        assertNull(KeyboardColorSettings().withKeyFill(0x00000000).fixedKeyFill)
        val clearOnGradient =
            KeyboardColorSettings(
                background = ThemeBackground.Gradient(ThemeGradient(listOf(0xFFE6C2D0.toInt(), 0xFFFFFFFF.toInt()))),
                keyTextColor = 0xFF1C1C1E.toInt(),
            ).withKeyFill(0x00000000)
        assertNull(clearOnGradient.fixedKeyFill)
    }
}
