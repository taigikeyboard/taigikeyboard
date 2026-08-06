// JVM unit test for DisplayLanguage tag resolution. StringResolver needs an Android Context, so its
// resolution paths are covered by on-device dogfood, not here.

package com.siansiansu.taigikeyboard.i18n

import org.junit.Assert.assertEquals
import org.junit.Test

class DisplayLanguageTest {
    @Test
    fun INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER_pinsHanjiEnglishJapaneseTailoAndPoj() {
        // CROSS-PLATFORM INVARIANT — must equal the iOS productionLanguages roster + tools/i18n
        // PRODUCTION_LANGUAGES, same order (behavioral-invariants.md §37).
        assertEquals(listOf("hanji", "en", "ja", "tailo", "poj"), DisplayLanguage.productionLanguages.map { it.tag })
    }

    @Test
    fun fromTag_selectableTags_survive() {
        // Production languages resolve to themselves in every build (tailo/poj joined the roster in R5-2/R6-2).
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag("hanji"))
        assertEquals(DisplayLanguage.ENGLISH, DisplayLanguage.fromTag("en"))
        assertEquals(DisplayLanguage.JAPANESE, DisplayLanguage.fromTag("ja"))
        assertEquals(DisplayLanguage.TAILO, DisplayLanguage.fromTag("tailo"))
        assertEquals(DisplayLanguage.POJ, DisplayLanguage.fromTag("poj"))
    }

    @Test
    fun INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER_selectableRoster() {
        assertEquals(
            listOf("system", "hanji", "en", "ja", "tailo", "poj"),
            DisplayLanguage.selectableLanguages.map { it.tag },
        )
    }

    @Test
    fun fromTag_unknownTag_fallsBackToHanji() {
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag("xx"))
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag(""))
    }

    @Test
    fun defaultTag_resolvesToSystemAutomatic() {
        // Default before any user pick = system (Automatic), so a fresh install follows the device OS locale.
        // CROSS-PLATFORM INVARIANT — mirrors iOS SettingsKeyTests defaultTagIsSystemAutomatic.
        assertEquals("system", DisplayLanguage.DEFAULT_TAG)
        assertEquals(DisplayLanguage.SYSTEM, DisplayLanguage.fromTag(DisplayLanguage.DEFAULT_TAG))
    }

    @Test
    fun INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER_excludesSystemSelectionPolicy() {
        // CROSS-PLATFORM INVARIANT — the authored roster is [hanji, en, ja, tailo, poj]; SYSTEM is a
        // selection policy with no strings and must NOT join productionLanguages (behavioral-invariants.md §37).
        assertEquals(listOf("hanji", "en", "ja", "tailo", "poj"), DisplayLanguage.productionLanguages.map { it.tag })
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
        // Japanese device → Japanese; English device → English; everything else (incl. Chinese / absent) → Hanji.
        assertEquals(DisplayLanguage.JAPANESE, DisplayLanguage.resolveAutomatic("ja"))
        assertEquals(DisplayLanguage.ENGLISH, DisplayLanguage.resolveAutomatic("en"))
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.resolveAutomatic("zh"))
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.resolveAutomatic("fr"))
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.resolveAutomatic(""))
    }

    @Test
    fun INVARIANT_DISPLAY_LANGUAGE_AUTOMATIC_RESOLUTION_effectiveLanguageOnlyResolvesSystem() {
        // An explicitly-picked language is itself regardless of the device locale; SYSTEM follows the device.
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.HANJI.effectiveLanguage("ja"))
        assertEquals(DisplayLanguage.JAPANESE, DisplayLanguage.SYSTEM.effectiveLanguage("ja"))
        assertEquals(DisplayLanguage.ENGLISH, DisplayLanguage.SYSTEM.effectiveLanguage("en"))
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.SYSTEM.effectiveLanguage("zh"))
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.SYSTEM.effectiveLanguage("de"))
    }
}
