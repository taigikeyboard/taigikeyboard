package com.siansiansu.taigikeyboard.ime.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Tests for the [KeyboardColorSettings] background-gradient extension + JSON
 * backward compatibility (a colorSettings JSON written before the gradient field
 * existed must still decode). Mirrors the gradient half of iOS's color tests.
 */
class KeyboardColorSettingsGradientTest {
    @Test
    fun gradient_jsonRoundTrip_preservesStops() {
        val settings = KeyboardColorSettings(
            backgroundColor = 0xFF101010.toInt(),
            backgroundGradient = ThemeGradient(listOf(0xFFBFD2EA.toInt(), 0xFFDCE2EC.toInt())),
        )
        val restored = KeyboardColorSettings.fromJson(settings.toJson())
        assertEquals(settings, restored)
        assertEquals(listOf(0xFFBFD2EA.toInt(), 0xFFDCE2EC.toInt()), restored.backgroundGradient?.stops)
    }

    @Test
    fun hasBackgroundGradient_requiresTwoStops() {
        assertFalse(KeyboardColorSettings().hasBackgroundGradient)
        assertFalse(KeyboardColorSettings(backgroundGradient = ThemeGradient(emptyList())).hasBackgroundGradient)
        assertFalse(
            KeyboardColorSettings(backgroundGradient = ThemeGradient(listOf(0xFF111111.toInt()))).hasBackgroundGradient,
        )
        assertTrue(
            KeyboardColorSettings(
                backgroundGradient = ThemeGradient(listOf(0xFF111111.toInt(), 0xFF222222.toInt())),
            ).hasBackgroundGradient,
        )
    }

    @Test
    fun fromJson_legacyWithoutGradient_decodesNullGradient() {
        // A colorSettings JSON written before the gradient field existed.
        val legacy = """{"backgroundColor":-16777216,"keyTextColor":-1}"""
        val restored = KeyboardColorSettings.fromJson(legacy)
        assertNull(restored.backgroundGradient)
        assertFalse(restored.hasBackgroundGradient)
        assertEquals(0xFF000000.toInt(), restored.backgroundColor)
        assertEquals(0xFFFFFFFF.toInt(), restored.keyTextColor)
    }

    @Test
    fun fromJson_emptyOrBlank_returnsDefault() {
        assertEquals(KeyboardColorSettings(), KeyboardColorSettings.fromJson("{}"))
        assertEquals(KeyboardColorSettings(), KeyboardColorSettings.fromJson(""))
    }

    @Test
    fun gradientStops_returnsStopsWhenRenderable() {
        val stops = listOf(0xFFBFD2EA.toInt(), 0xFFDCE2EC.toInt())
        val rendered = KeyboardColorSettings(backgroundGradient = ThemeGradient(stops)).gradientStops()
        assertEquals(stops, rendered?.toList())
    }

    @Test
    fun gradientStops_nullWhenFlatOrUnderTwoStops() {
        assertNull(KeyboardColorSettings().gradientStops())
        assertNull(KeyboardColorSettings(backgroundGradient = ThemeGradient(emptyList())).gradientStops())
        assertNull(
            KeyboardColorSettings(backgroundGradient = ThemeGradient(listOf(0xFF111111.toInt()))).gradientStops(),
        )
    }
}
