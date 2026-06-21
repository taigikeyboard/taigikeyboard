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
        // tl/poj are valid identities but not yet selectable in any build, so the effective language is
        // Hanji — the picker selection and the rendered strings always agree. Persisted tag untouched.
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag("tailo"))
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag("poj"))
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
}
