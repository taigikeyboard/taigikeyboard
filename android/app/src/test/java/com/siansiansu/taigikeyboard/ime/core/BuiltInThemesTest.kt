package com.siansiansu.taigikeyboard.ime.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Tests for the built-in theme catalog. The id invariants are load-bearing:
 * [ThemeId.isUserTheme] decides whether an id routes to the user-theme list, and
 * [ThemeId.DEFAULT] is the legacy-buffer sentinel. Mirrors iOS BuiltInThemesTests.
 */
class BuiltInThemesTest {
    @Test
    fun families_flattenIntoAll() {
        assertFalse(BuiltInThemes.families.isEmpty())
        val flattened = BuiltInThemes.families.flatMap { it.themes }
        assertEquals(flattened.map { it.id }, BuiltInThemes.all.map { it.id })
    }

    @Test
    fun all_eachThemeHasPreviewImageName() {
        BuiltInThemes.all.forEach {
            assertNotNull("Built-in '${it.id}' must carry a screenshot slot", it.previewImageName)
        }
    }

    @Test
    fun standardHead_isDefaultSentinel() {
        val head = BuiltInThemes.families.first().themes.first()
        assertEquals(ThemeId.DEFAULT, head.id)
        assertEquals("預設", head.displayName)
    }

    @Test
    fun all_onlyHeadIsDefaultSentinel() {
        assertEquals(1, BuiltInThemes.all.count { it.id == ThemeId.DEFAULT })
    }

    @Test
    fun all_idsAreUnique() {
        val ids = BuiltInThemes.all.map { it.id }
        assertEquals(ids.size, ids.toSet().size)
    }

    @Test
    fun all_idsAreNotUserThemeIds() {
        BuiltInThemes.all.forEach {
            assertFalse("Built-in id '${it.id}' must not parse as a UUID", ThemeId.isUserTheme(it.id))
        }
    }

    @Test
    fun themeById_returnsMatchOrNull() {
        assertEquals("standardBlue", BuiltInThemes.theme("standardBlue")?.id)
        assertNull(BuiltInThemes.theme("no_such_theme"))
        assertEquals("預設", BuiltInThemes.theme(ThemeId.DEFAULT)?.displayName)
    }

    @Test
    fun colorsForScheme_scaffoldDegradesToDefault() {
        val theme = BuiltInThemes.theme("swiftyBlue")!!
        assertEquals(KeyboardColorSettings(), theme.colors(isDark = false))
        assertEquals(KeyboardColorSettings(), theme.colors(isDark = true))
    }

    @Test
    fun standardGradientThemes_carryGradient() {
        for (id in listOf("standardPink", "standardGold", "standardBlue", "standardGreen", "standardPurple")) {
            val theme = BuiltInThemes.theme(id)!!
            assertTrue("$id light must carry a gradient", theme.colors(false).hasBackgroundGradient)
            assertTrue("$id dark must carry a gradient", theme.colors(true).hasBackgroundGradient)
        }
    }

    @Test
    fun colorsForScheme_darkOnlyFallsBackToDark() {
        val dark = KeyboardColorSettings(backgroundColor = 0xFF112233.toInt())
        val theme = BuiltInTheme("test_dark_only", "Dark Only", light = null, dark = dark)
        assertEquals(dark, theme.colors(isDark = false))
        assertEquals(dark, theme.colors(isDark = true))
    }

    @Test
    fun colorsForScheme_lightOnlyFallsBackToLight() {
        val light = KeyboardColorSettings(backgroundColor = 0xFFAABBCC.toInt())
        val theme = BuiltInTheme("test_light_only", "Light Only", light = light, dark = null)
        assertEquals(light, theme.colors(isDark = true))
        assertEquals(light, theme.colors(isDark = false))
    }

    @Test
    fun colorsForScheme_picksRequestedVariant() {
        val light = KeyboardColorSettings(backgroundColor = 0xFF111111.toInt())
        val dark = KeyboardColorSettings(backgroundColor = 0xFF222222.toInt())
        val theme = BuiltInTheme("test_both", "Both", light = light, dark = dark)
        assertEquals(light, theme.colors(isDark = false))
        assertEquals(dark, theme.colors(isDark = true))
    }

    @Test
    fun standardBlue_functionKeysShareNeutralWhiteFill_light() {
        val colors = BuiltInThemes.theme("standardBlue")!!.colors(isDark = false)
        // Function keys forced to the same fill as letter keys; light fill = opaque white.
        assertEquals(colors.normalKeyFillColor, colors.specialKeyFillColor)
        assertEquals(0xFFFFFFFF.toInt(), colors.normalKeyFillColor)
        // The gradient owns the background; the candidate bar is transparent over it.
        assertNull(colors.backgroundColor)
        assertNull(colors.candidateBackgroundColor)
    }

    @Test
    fun standardBlue_gradientStopsAreExpectedArgb_light() {
        val gradient = BuiltInThemes.theme("standardBlue")!!.colors(isDark = false).backgroundGradient!!
        assertEquals(listOf(0xFFBFD2EA.toInt(), 0xFFDCE2EC.toInt()), gradient.stops)
    }
}
