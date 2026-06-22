// JVM unit test for DisplayLanguage tag resolution. StringResolver needs an Android Context, so its
// resolution paths are covered by on-device dogfood, not here.

package com.siansiansu.taigikeyboard.i18n

import org.junit.Assert.assertEquals
import org.junit.Test

class DisplayLanguageTest {
    @Test
    fun INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER_pinsHanjiEnglishAndJapanese() {
        // CROSS-PLATFORM INVARIANT — must equal the iOS productionLanguages roster (behavioral-invariants.md §37).
        assertEquals(listOf("hanji", "en", "ja"), DisplayLanguage.productionLanguages.map { it.tag })
    }

    @Test
    fun fromTag_selectableTags_survive() {
        // Production languages resolve to themselves in every build.
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag("hanji"))
        assertEquals(DisplayLanguage.ENGLISH, DisplayLanguage.fromTag("en"))
        assertEquals(DisplayLanguage.JAPANESE, DisplayLanguage.fromTag("ja"))
    }

    @Test
    fun fromTag_unauthoredTags_clampToHanji() {
        // poj is a valid identity but unauthored (no strings yet), so it is not selectable in any build and
        // clamps to Hanji — the picker selection and the rendered strings always agree. Persisted tag
        // untouched. tailo is now authored + debug-selectable (separate test below).
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag("poj"))
    }

    @Test
    fun fromTag_tailoInDebugBuild_resolvesTailo() {
        // Tâi-lô is authored but not yet a production language; it is debug-selectable so a debug build can
        // dogfood its strings + rendering before it joins productionLanguages (mirrors ja R4-1).
        // testDebugUnitTest runs with BuildConfig.DEBUG == true, so the tl tag survives.
        assertEquals(DisplayLanguage.TAILO, DisplayLanguage.fromTag("tailo"))
    }

    @Test
    fun INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER_debugSelectableRoster() {
        // Pin the exact DEBUG picker order: Automatic first, the production roster, then the debug-only
        // tailo preview + pseudo probe. Release drops the last two (asserted by the source #if; not
        // reachable from a DEBUG unit-test build). Guards against reorders / accidental promotion.
        assertEquals(
            listOf("system", "hanji", "en", "ja", "tailo", "pseudo"),
            DisplayLanguage.selectableLanguages.map { it.tag },
        )
    }

    @Test
    fun fromTag_unknownTag_fallsBackToHanji() {
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag("xx"))
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag(""))
    }

    @Test
    fun fromTag_pseudoInDebugBuild_resolvesPseudo() {
        // testDebugUnitTest runs with BuildConfig.DEBUG == true, so the debug-only probe tag survives.
        assertEquals(DisplayLanguage.PSEUDO, DisplayLanguage.fromTag("pseudo"))
    }

    @Test
    fun defaultTag_resolvesToHanji() {
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag(DisplayLanguage.DEFAULT_TAG))
    }

    @Test
    fun INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER_excludesSystemSelectionPolicy() {
        // CROSS-PLATFORM INVARIANT — the authored roster stays [hanji, en, ja]; SYSTEM is a selection
        // policy with no strings and must NOT join productionLanguages (behavioral-invariants.md §37).
        assertEquals(listOf("hanji", "en", "ja"), DisplayLanguage.productionLanguages.map { it.tag })
        assertEquals(false, DisplayLanguage.SYSTEM in DisplayLanguage.productionLanguages)
    }

    @Test
    fun INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER_selectableLeadsWithSystem() {
        // The picker offers Automatic (SYSTEM) first, ahead of the authored roster.
        assertEquals(DisplayLanguage.SYSTEM, DisplayLanguage.selectableLanguages.first())
        assertEquals(DisplayLanguage.SYSTEM, DisplayLanguage.fromTag("system"))
    }

    @Test
    fun INVARIANT_DISPLAY_LANGUAGE_AUTOMATIC_RESOLUTION_mapsDeviceSubtagToAuthoredLanguage() {
        // Japanese device → Japanese; Chinese device → Hanji; everything else (incl. absent locale) → English.
        assertEquals(DisplayLanguage.JAPANESE, DisplayLanguage.resolveAutomatic("ja"))
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.resolveAutomatic("zh"))
        assertEquals(DisplayLanguage.ENGLISH, DisplayLanguage.resolveAutomatic("en"))
        assertEquals(DisplayLanguage.ENGLISH, DisplayLanguage.resolveAutomatic("fr"))
        assertEquals(DisplayLanguage.ENGLISH, DisplayLanguage.resolveAutomatic(""))
    }

    @Test
    fun INVARIANT_DISPLAY_LANGUAGE_AUTOMATIC_RESOLUTION_effectiveLanguageOnlyResolvesSystem() {
        // An explicitly-picked language is itself regardless of the device locale; SYSTEM follows the device.
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.HANJI.effectiveLanguage("ja"))
        assertEquals(DisplayLanguage.JAPANESE, DisplayLanguage.SYSTEM.effectiveLanguage("ja"))
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.SYSTEM.effectiveLanguage("zh"))
        assertEquals(DisplayLanguage.ENGLISH, DisplayLanguage.SYSTEM.effectiveLanguage("de"))
    }
}
