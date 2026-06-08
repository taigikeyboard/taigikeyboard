package com.siansiansu.taigikeyboard.ime.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.UUID

/**
 * Tests for [ThemeResolver] — the pure mapping from a selected theme id to the
 * [ThemeAppearance] the renderer consumes. Mirrors iOS ThemeResolverTests.
 */
class ThemeResolverTest {
    private fun customizedColors(): KeyboardColorSettings = KeyboardColorSettings(backgroundColor = RED)

    private fun appearance(
        colors: KeyboardColorSettings = KeyboardColorSettings(),
        shadow: Float = 0f,
    ): ThemeAppearance = ThemeAppearance.DEFAULT.copy(colors = colors, keyShadowIntensity = shadow)

    private fun userTheme(id: String, appearance: ThemeAppearance): UserTheme =
        UserTheme(id = id, name = "T", appearance = appearance, createdAt = 0L, updatedAt = 0L)

    @Test
    fun resolved_default_returnsLegacyAppearance() {
        val resolved = ThemeResolver.resolved(
            themeId = ThemeId.DEFAULT,
            isDark = false,
            legacyAppearance = ThemeAppearance.DEFAULT,
            userThemes = emptyList(),
        )
        assertEquals(ThemeAppearance.DEFAULT, resolved)
    }

    @Test
    fun resolved_default_returnsCustomizedLegacyVerbatim() {
        val legacy = appearance(customizedColors(), shadow = 0.2f).copy(keyHeightScale = 1.1f)
        val resolved = ThemeResolver.resolved(ThemeId.DEFAULT, false, legacy, emptyList())
        assertEquals(legacy, resolved)
    }

    @Test
    fun resolved_knownUserTheme_carriesFullAppearance() {
        val id = UUID.randomUUID().toString()
        val app = appearance(customizedColors(), shadow = 0.3f).copy(keyHeightScale = 1.1f, keyFontSizeScale = 0.9f)
        val resolved = ThemeResolver.resolved(id, true, ThemeAppearance.DEFAULT, listOf(userTheme(id, app)))
        assertEquals(app, resolved)
    }

    @Test
    fun resolved_unknownId_fallsBackToLegacy() {
        val legacy = appearance(customizedColors(), shadow = 0.4f)
        val resolved = ThemeResolver.resolved("no_such_theme", false, legacy, emptyList())
        assertEquals(legacy, resolved)
    }

    @Test
    fun resolved_deletedUserTheme_fallsBackToLegacy() {
        val staleId = UUID.randomUUID().toString()
        val other = userTheme(UUID.randomUUID().toString(), appearance(customizedColors()))
        val resolved = ThemeResolver.resolved(staleId, false, ThemeAppearance.DEFAULT, listOf(other))
        assertEquals(ThemeAppearance.DEFAULT, resolved)
    }

    @Test
    fun resolved_builtInLight_resolvesThroughCatalog() {
        val expected = BuiltInThemes.theme("standardBlue")!!
        val resolved = ThemeResolver.resolved("standardBlue", false, appearance(customizedColors()), emptyList())
        assertEquals(expected.colors(false), resolved.colors)
        assertNotEquals(KeyboardColorSettings(), resolved.colors)
        assertTrue(resolved.colors.hasBackgroundGradient)
        assertEquals(0f, resolved.keyShadowIntensity, 0f)
    }

    @Test
    fun resolved_builtInDark_resolvesThroughCatalog() {
        val expected = BuiltInThemes.theme("standardBlue")!!
        val resolved = ThemeResolver.resolved("standardBlue", true, ThemeAppearance.DEFAULT, emptyList())
        assertEquals(expected.colors(true), resolved.colors)
    }

    @Test
    fun resolved_builtIn_usesFactorySizes() {
        val legacy = appearance(customizedColors(), shadow = 0.5f).copy(keyHeightScale = 1.15f)
        val resolved = ThemeResolver.resolved("standardBlue", true, legacy, emptyList())
        assertEquals(ThemeAppearance.DEFAULT.keyHeightScale, resolved.keyHeightScale, 0f)
        assertEquals(ThemeAppearance.DEFAULT.keyFontSizeScale, resolved.keyFontSizeScale, 0f)
        assertEquals(ThemeAppearance.DEFAULT.candidateTextSizeScale, resolved.candidateTextSizeScale, 0f)
        assertEquals(ThemeAppearance.DEFAULT.keyCornerRadius, resolved.keyCornerRadius, 0f)
        assertEquals(ThemeAppearance.DEFAULT.keyBorderWidth, resolved.keyBorderWidth, 0f)
        assertEquals(0f, resolved.keyShadowIntensity, 0f)
    }

    @Test
    fun resolved_builtIn_winsOverUnrelatedUserThemes() {
        val other = userTheme(UUID.randomUUID().toString(), appearance(customizedColors()))
        val resolved = ThemeResolver.resolved("standardBlue", true, ThemeAppearance.DEFAULT, listOf(other))
        assertEquals(BuiltInThemes.theme("standardBlue")!!.colors(true), resolved.colors)
    }

    @Test
    fun resolved_corruptUserThemeWithBuiltInId_doesNotShadowBuiltIn() {
        // A persisted user theme carrying a built-in id must NOT shadow the built-in:
        // the user branch is gated on a UUID-shaped id, so "standardBlue" resolves to
        // the built-in palette, not the bogus user theme.
        val bogus = userTheme("standardBlue", appearance(customizedColors()))
        val resolved = ThemeResolver.resolved("standardBlue", false, ThemeAppearance.DEFAULT, listOf(bogus))
        assertEquals(BuiltInThemes.theme("standardBlue")!!.colors(false), resolved.colors)
    }

    private companion object {
        const val RED = 0xFFFF0000.toInt()
    }
}
