package com.siansiansu.taigikeyboard.ime.theme

import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
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
        val head = BuiltInThemes.families
            .first()
            .themes
            .first()
        assertEquals(ThemeId.DEFAULT, head.id)
        assertEquals(StringKey.THEME_PALETTE_DEFAULT, head.displayNameKey)
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
        assertEquals(StringKey.THEME_PALETTE_DEFAULT, BuiltInThemes.theme(ThemeId.DEFAULT)?.displayNameKey)
    }

    // Three key-style families (Filled/Outlined/Borderless), each carrying the same 7 colors.
    @Test
    fun families_threeKeyStyleFamiliesEachWithSevenColors() {
        assertEquals(
            listOf(StringKey.THEME_FAMILY_CLASSIC, StringKey.THEME_FAMILY_FRAMED, StringKey.THEME_FAMILY_CLEAN),
            BuiltInThemes.families.map { it.titleKey },
        )
        BuiltInThemes.families.forEach { family ->
            assertEquals("${family.titleKey} must carry all 7 shared colors", 7, family.themes.size)
            assertEquals(
                listOf(
                    StringKey.THEME_PALETTE_DEFAULT,
                    StringKey.THEME_PALETTE_PINK,
                    StringKey.THEME_PALETTE_GOLD,
                    StringKey.THEME_PALETTE_BLUE,
                    StringKey.THEME_PALETTE_GREEN,
                    StringKey.THEME_PALETTE_PURPLE,
                    StringKey.THEME_PALETTE_CATPPUCCIN,
                ),
                family.themes.map { it.displayNameKey },
            )
        }
        assertEquals(21, BuiltInThemes.all.size)
    }

    // Filled keeps filled keys; Outlined/Borderless recolor keys to transparent (key == background).
    @Test
    fun keyStyleFamilies_framedAndCleanHaveTransparentKeys() {
        val classic = BuiltInThemes.theme("standardBlue")!!.colors(isDark = false)
        assertEquals(0xFFFFFFFF.toInt(), classic.normalKeyFillColor) // opaque white
        for (id in listOf("framedBlue", "cleanBlue")) {
            val colors = BuiltInThemes.theme(id)!!.colors(isDark = false)
            assertEquals("$id normal key must be transparent", 0, colors.normalKeyFillColor)
            assertEquals("$id special key must be transparent", 0, colors.specialKeyFillColor)
        }
    }

    // Only the Outlined family draws the outline; Filled/Borderless leave keyBorderWidth null.
    @Test
    fun keyStyleFamilies_onlyFramedCarriesKeyBorderWidth() {
        assertEquals(1.0f, BuiltInThemes.theme("framedBlue")!!.keyBorderWidth!!, 0f)
        assertNull(BuiltInThemes.theme("standardBlue")!!.keyBorderWidth)
        assertNull(BuiltInThemes.theme("cleanBlue")!!.keyBorderWidth)
    }

    // The three families SHARE colors — only the key fill differs (gradient + text identical).
    @Test
    fun keyStyleFamilies_shareBackgroundAndTextAcrossFamilies() {
        val classic = BuiltInThemes.theme("standardBlue")!!.colors(isDark = false)
        for (id in listOf("framedBlue", "cleanBlue")) {
            val variant = BuiltInThemes.theme(id)!!.colors(isDark = false)
            assertEquals("$id keeps the same gradient", classic.backgroundGradient, variant.backgroundGradient)
            assertEquals("$id keeps the same text color", classic.keyTextColor, variant.keyTextColor)
        }
    }

    // framed/clean gradient themes keep the gradient → candidate tints still derive from it.
    @Test
    fun keyStyleFamilies_framedGradientStillCarriesGradient() {
        for (id in listOf("framedPink", "cleanPink")) {
            assertNotNull("$id must keep the gradient", BuiltInThemes.theme(id)!!.colors(isDark = false).backgroundGradient)
        }
    }

    // framed/clean Default stay adaptive (bg/text null) but carry transparent keys; Filled Default fully adaptive.
    @Test
    fun keyStyleFamilies_defaultVariantsAdaptiveWithTransparentKeys() {
        for (id in listOf("framedDefault", "cleanDefault")) {
            val colors = BuiltInThemes.theme(id)!!.colors(isDark = false)
            assertNull("$id keeps adaptive background", colors.background)
            assertNull("$id keeps adaptive text", colors.keyTextColor)
            assertEquals("$id keys are transparent", 0, colors.normalKeyFillColor)
        }
        assertEquals(KeyboardColorSettings(), BuiltInThemes.theme(ThemeId.DEFAULT)!!.colors(isDark = false))
    }

    @Test
    fun standardGradientThemes_carryGradient() {
        for (id in listOf("standardPink", "standardGold", "standardBlue", "standardGreen", "standardPurple")) {
            val theme = BuiltInThemes.theme(id)!!
            assertNotNull("$id light must carry a gradient", theme.colors(false).backgroundGradient)
            assertNotNull("$id dark must carry a gradient", theme.colors(true).backgroundGradient)
        }
    }

    // Gradient themes are light-only (dark == null) → dark scheme reuses the light
    // palette unchanged (USER: these themes keep their light look in dark mode).
    @Test
    fun standardGradientThemes_stayLightInDarkMode() {
        for (id in listOf("standardPink", "standardGold", "standardBlue", "standardGreen", "standardPurple")) {
            val theme = BuiltInThemes.theme(id)!!
            assertNull("$id must not define a dark variant — it stays light in dark mode", theme.dark)
            assertEquals("$id dark scheme must reuse the light palette", theme.colors(false), theme.colors(true))
        }
    }

    // Catppuccin is the dark-only Catppuccin Mocha theme across all 3 families — light == null
    // so BOTH schemes resolve to the dark variant (always dark, the mirror of the 5
    // light-only gradients). gradient top #1E1E2E (Base) → #181825 (Mantle); key+candidate
    // text #CDD6F4 (Text); Filled keys (letter + function) share #313244 (Surface0);
    // Outlined/Borderless keep transparent keys.
    @Test
    fun catppuccinTheme_isDarkOnlyAcrossFamilies() {
        for (id in listOf("standardCatppuccin", "framedCatppuccin", "cleanCatppuccin")) {
            val theme = BuiltInThemes.theme(id)!!
            assertNull("$id must be dark-only (light == null)", theme.light)
            assertNotNull("$id must define the dark variant", theme.dark)
            val colors = theme.colors(isDark = true)
            assertEquals("$id light request falls back to the dark variant", colors, theme.colors(isDark = false))
            assertNotNull("$id must carry the Catppuccin gradient", colors.backgroundGradient)
            assertEquals("$id gradient top = Mocha Base", 0xFF1E1E2E.toInt(), colors.backgroundGradient?.stops?.first())
            assertEquals("$id gradient bottom = Mocha Mantle", 0xFF181825.toInt(), colors.backgroundGradient?.stops?.last())
            assertEquals("$id key text = Mocha Text", 0xFFCDD6F4.toInt(), colors.keyTextColor)
            assertEquals("$id candidate text = Mocha Text", 0xFFCDD6F4.toInt(), colors.candidateTextColor)
        }
        // Filled Catppuccin: letter + function keys share the Surface0 neutral fill.
        val classic = BuiltInThemes.theme("standardCatppuccin")!!.colors(isDark = true)
        assertEquals("經典 暗眠山貓 letter key = Mocha Surface0", 0xFF313244.toInt(), classic.normalKeyFillColor)
        assertEquals("經典 暗眠山貓 function key shares the Surface0 fill", 0xFF313244.toInt(), classic.specialKeyFillColor)
        // Outlined/Borderless keep keys transparent (gradient shows through).
        for (id in listOf("framedCatppuccin", "cleanCatppuccin")) {
            val colors = BuiltInThemes.theme(id)!!.colors(isDark = true)
            assertEquals("$id letter keys must be transparent", 0, colors.normalKeyFillColor)
            assertEquals("$id function keys must be transparent", 0, colors.specialKeyFillColor)
        }
    }

    @Test
    fun colorsForScheme_darkOnlyFallsBackToDark() {
        val dark = KeyboardColorSettings(background = ThemeBackground.Solid(0xFF112233.toInt()))
        val theme = BuiltInTheme("test_dark_only", StringKey.THEME_PALETTE_DEFAULT, light = null, dark = dark)
        assertEquals(dark, theme.colors(isDark = false))
        assertEquals(dark, theme.colors(isDark = true))
    }

    @Test
    fun colorsForScheme_lightOnlyFallsBackToLight() {
        val light = KeyboardColorSettings(background = ThemeBackground.Solid(0xFFAABBCC.toInt()))
        val theme = BuiltInTheme("test_light_only", StringKey.THEME_PALETTE_DEFAULT, light = light, dark = null)
        assertEquals(light, theme.colors(isDark = true))
        assertEquals(light, theme.colors(isDark = false))
    }

    @Test
    fun colorsForScheme_picksRequestedVariant() {
        val light = KeyboardColorSettings(background = ThemeBackground.Solid(0xFF111111.toInt()))
        val dark = KeyboardColorSettings(background = ThemeBackground.Solid(0xFF222222.toInt()))
        val theme = BuiltInTheme("test_both", StringKey.THEME_PALETTE_DEFAULT, light = light, dark = dark)
        assertEquals(light, theme.colors(isDark = false))
        assertEquals(dark, theme.colors(isDark = true))
    }

    @Test
    fun standardBlue_functionKeysShareNeutralWhiteFill_light() {
        val colors = BuiltInThemes.theme("standardBlue")!!.colors(isDark = false)
        // Function keys forced to the same fill as letter keys; light fill = opaque white.
        assertEquals(colors.normalKeyFillColor, colors.specialKeyFillColor)
        assertEquals(0xFFFFFFFF.toInt(), colors.normalKeyFillColor)
        // The gradient owns the whole surface (no solid colour beside it).
        assertNotNull(colors.backgroundGradient)
    }

    // trace: standardPink = Filled, light slot: gradient (0xE6C2D0, 0xEADCE2), LIGHT_KEY_TEXT 0x1C1C1E,
    // LIGHT_KEY_FILL 0xFFFFFF; seededForUserTheme keeps every set role; border null -> factory 0 -> Filled
    @Test
    fun userThemeBase_filledPink_copiesGradientAndWhiteKeys() {
        val base = BuiltInThemes.theme("standardPink")!!.userThemeBase!!
        assertEquals(listOf(0xFFE6C2D0.toInt(), 0xFFEADCE2.toInt()), base.colors.backgroundGradient?.stops)
        assertEquals(0xFF1C1C1E.toInt(), base.colors.keyTextColor)
        assertEquals(0xFF1C1C1E.toInt(), base.colors.candidateTextColor)
        assertEquals(0xFFFFFFFF.toInt(), base.colors.normalKeyFillColor)
        assertEquals(0xFFFFFFFF.toInt(), base.colors.specialKeyFillColor)
        assertEquals(0f, base.keyBorderWidth)
        assertEquals(ThemeKeyStyle.CLASSIC, base.keyStyle)
        assertEquals(ThemeAppearance.DEFAULT_KEY_HEIGHT_SCALE, base.keyHeightScale)
    }

    // trace: framed* = clear key fill + OUTLINED_BORDER_WIDTH 1.0 -> Outlined; clean* = clear fill, no border -> Borderless
    @Test
    fun userThemeBase_keepsKeyStyleOfTheFamily() {
        val framed = BuiltInThemes.theme("framedPink")!!.userThemeBase!!
        assertEquals(ThemeKeyStyle.FRAMED, framed.keyStyle)
        assertEquals(ThemeKeyStyle.OUTLINED_BORDER_WIDTH, framed.keyBorderWidth)
        val clean = BuiltInThemes.theme("cleanPink")!!.userThemeBase!!
        assertEquals(ThemeKeyStyle.CLEAN, clean.keyStyle)
        assertEquals(0f, clean.keyBorderWidth)
    }

    // trace: Catppuccin builds into the dark slot (light null) -> light ?: dark picks it: Mocha
    // Base->Mantle (0x1E1E2E, 0x181825), text 0xCDD6F4, Surface0 keys 0x313244
    @Test
    fun userThemeBase_catppuccinTakesTheDarkVariant() {
        val base = BuiltInThemes.theme("standardCatppuccin")!!.userThemeBase!!
        assertEquals(listOf(0xFF1E1E2E.toInt(), 0xFF181825.toInt()), base.colors.backgroundGradient?.stops)
        assertEquals(0xFFCDD6F4.toInt(), base.colors.keyTextColor)
        assertEquals(0xFF313244.toInt(), base.colors.normalKeyFillColor)
    }

    // trace: Filled Default has no variant; framed / clean Default carry only clear key fills (no background)
    @Test
    fun userThemeBase_nullForTheAdaptiveDefaults() {
        listOf(ThemeId.DEFAULT, "framedDefault", "cleanDefault").forEach {
            assertNull(it, BuiltInThemes.theme(it)!!.userThemeBase)
        }
    }

    // trace: framedCatppuccin -> Filled twin standardCatppuccin -> Surface0 0x313244 keys, text kept 0xCDD6F4;
    // cleanPink -> standardPink -> LIGHT_KEY_FILL 0xFFFFFF; border 0 (Filled twin), gradient unchanged
    @Test
    fun userThemeFilledBaseline_takesTheFilledTwinsKeys() {
        val catppuccin = BuiltInThemes.userThemeFilledBaseline("framedCatppuccin")
        assertEquals(ThemeKeyStyle.CLASSIC, catppuccin.keyStyle)
        assertEquals(0xFF313244.toInt(), catppuccin.colors.normalKeyFillColor)
        assertEquals(0xFFCDD6F4.toInt(), catppuccin.colors.keyTextColor)
        assertEquals(0f, catppuccin.keyBorderWidth)
        val pink = BuiltInThemes.userThemeFilledBaseline("cleanPink")
        assertEquals(0xFFFFFFFF.toInt(), pink.colors.normalKeyFillColor)
        assertEquals(
            BuiltInThemes
                .theme("cleanPink")!!
                .userThemeBase!!
                .colors.background,
            pink.colors.background,
        )
        assertEquals(BuiltInThemes.userThemeBaseline("standardGold"), BuiltInThemes.userThemeFilledBaseline("standardGold"))
        assertEquals(ThemeAppearance.USER_THEME_SEED, BuiltInThemes.userThemeFilledBaseline(null))
    }

    @Test
    fun userThemeBaseline_builtInBaseElseSeed() {
        assertEquals(BuiltInThemes.theme("framedGold")!!.userThemeBase, BuiltInThemes.userThemeBaseline("framedGold"))
        assertEquals(ThemeAppearance.USER_THEME_SEED, BuiltInThemes.userThemeBaseline(null))
        assertEquals(ThemeAppearance.USER_THEME_SEED, BuiltInThemes.userThemeBaseline("noLongerShipped"))
    }

    @Test
    fun standardBlue_gradientStopsAreExpectedArgb_light() {
        val gradient = BuiltInThemes.theme("standardBlue")!!.colors(isDark = false).backgroundGradient
        assertEquals(listOf(0xFFBFD2EA.toInt(), 0xFFDCE2EC.toInt()), gradient?.stops)
        assertEquals("built-in gradients stay vertical", ThemeGradient.DEFAULT_ANGLE, gradient?.angle)
    }
}
