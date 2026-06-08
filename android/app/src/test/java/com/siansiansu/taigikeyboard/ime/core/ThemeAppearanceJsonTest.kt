package com.siansiansu.taigikeyboard.ime.core

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Tests for [ThemeAppearance] JSON: full round-trip + forward-compatible decode
 * (any field absent in stored JSON falls back to the project default, so a
 * future appearance field never strands an older-build theme). Mirrors iOS
 * testThemeAppearance_decodesPartialJSON_fillsMissingWithDefaults.
 */
class ThemeAppearanceJsonTest {
    @Test
    fun jsonRoundTrip_fullAppearance() {
        val appearance = ThemeAppearance(
            colors = KeyboardColorSettings(backgroundColor = 0xFF00FF00.toInt()),
            keyShadowIntensity = 0.25f,
            keyHeightScale = 1.1f,
            keyFontSizeScale = 0.9f,
            candidateTextSizeScale = 1.05f,
            keyCornerRadius = 12f,
            keyBorderWidth = 1.5f,
        )
        assertEquals(appearance, ThemeAppearance.fromJson(appearance.toJson()))
    }

    @Test
    fun fromJson_partialJson_fillsMissingWithDefaults() {
        val json = JSONObject("""{ "colors": {}, "keyShadowIntensity": 0.25 }""")
        val appearance = ThemeAppearance.fromJson(json)
        assertEquals(0.25f, appearance.keyShadowIntensity, 0f)
        assertEquals(KeyboardColorSettings(), appearance.colors)
        assertEquals(ThemeAppearance.DEFAULT.keyHeightScale, appearance.keyHeightScale, 0f)
        assertEquals(ThemeAppearance.DEFAULT.keyFontSizeScale, appearance.keyFontSizeScale, 0f)
        assertEquals(ThemeAppearance.DEFAULT.candidateTextSizeScale, appearance.candidateTextSizeScale, 0f)
        assertEquals(ThemeAppearance.DEFAULT.keyCornerRadius, appearance.keyCornerRadius, 0f)
        assertEquals(ThemeAppearance.DEFAULT.keyBorderWidth, appearance.keyBorderWidth, 0f)
    }

    @Test
    fun fromJson_unknownKey_ignored() {
        // A legacy theme carrying the now-removed "fontType" key still decodes
        // (font is a global setting, never a theme field).
        val json = JSONObject("""{ "colors": {}, "keyShadowIntensity": 0.3, "fontType": "iansui" }""")
        val appearance = ThemeAppearance.fromJson(json)
        assertEquals(0.3f, appearance.keyShadowIntensity, 0f)
        assertEquals(ThemeAppearance.DEFAULT.keyHeightScale, appearance.keyHeightScale, 0f)
    }

    @Test
    fun fromJson_emptyObject_returnsDefault() {
        assertEquals(ThemeAppearance.DEFAULT, ThemeAppearance.fromJson(JSONObject()))
    }
}
