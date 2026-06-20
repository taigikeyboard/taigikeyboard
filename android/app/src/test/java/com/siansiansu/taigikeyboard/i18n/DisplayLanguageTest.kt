// JVM unit test for DisplayLanguage tag resolution. StringResolver needs an Android Context, so its
// resolution paths are covered by on-device dogfood, not here.

package com.siansiansu.taigikeyboard.i18n

import org.junit.Assert.assertEquals
import org.junit.Test

class DisplayLanguageTest {
    @Test
    fun fromTag_knownTags_roundTrip() {
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag("hanji"))
        assertEquals(DisplayLanguage.TAILO, DisplayLanguage.fromTag("tailo"))
        assertEquals(DisplayLanguage.POJ, DisplayLanguage.fromTag("poj"))
        assertEquals(DisplayLanguage.JAPANESE, DisplayLanguage.fromTag("ja"))
        assertEquals(DisplayLanguage.ENGLISH, DisplayLanguage.fromTag("en"))
    }

    @Test
    fun fromTag_unknownTag_fallsBackToHanji() {
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag("xx"))
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag(""))
    }

    @Test
    fun fromTag_pseudoInDebugBuild_resolvesPseudo() {
        // testDebugUnitTest runs with BuildConfig.DEBUG == true, so the probe tag survives.
        assertEquals(DisplayLanguage.PSEUDO, DisplayLanguage.fromTag("pseudo"))
    }

    @Test
    fun defaultTag_resolvesToHanji() {
        assertEquals(DisplayLanguage.HANJI, DisplayLanguage.fromTag(DisplayLanguage.DEFAULT_TAG))
    }
}
