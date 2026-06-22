// JVM unit test for LocalizedContentText.resolve (Round C1 content-infra). FeatureContentLoader's
// JSON parse needs an Android Context + org.json (JVM-stubbed via isReturnDefaultValues), so the
// decode/strict-parse path is covered by on-device dogfood, not here — only the pure-Kotlin resolve
// fallback contract is unit-tested. Mirrors the iOS LocalizedContentTextTests resolve matrix.

package com.siansiansu.taigikeyboard.content

import com.siansiansu.taigikeyboard.i18n.DisplayLanguage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class LocalizedContentTextTest {
    // All five effective display languages, each authored to a distinct value.
    private val allLanguages =
        LocalizedContentText(hanji = "漢", tailo = "TL", poj = "POJ", ja = "JA", en = "EN")

    // Hanji-only — the production state today (C1): every other language is unauthored.
    private val hanjiOnly = LocalizedContentText(hanji = "漢")

    @Test
    fun INVARIANT_resolve_authoredLanguage_returnsThatLanguage() {
        // CROSS-PLATFORM INVARIANT — must match iOS LocalizedContentText.resolve(for:) fallback order.
        assertEquals("漢", allLanguages.resolve(DisplayLanguage.HANJI))
        assertEquals("TL", allLanguages.resolve(DisplayLanguage.TAILO))
        assertEquals("POJ", allLanguages.resolve(DisplayLanguage.POJ))
        assertEquals("JA", allLanguages.resolve(DisplayLanguage.JAPANESE))
        assertEquals("EN", allLanguages.resolve(DisplayLanguage.ENGLISH))
    }

    @Test
    fun INVARIANT_resolve_unauthoredLanguage_fallsBackToHanji() {
        // The C1 behavior-freeze guarantee: today every key is Hanji-only, so every effective
        // language renders Hanji until C2 authoring fills the other languages.
        for (language in
            listOf(
                DisplayLanguage.HANJI,
                DisplayLanguage.TAILO,
                DisplayLanguage.POJ,
                DisplayLanguage.JAPANESE,
                DisplayLanguage.ENGLISH,
                DisplayLanguage.PSEUDO,
            )) {
            assertEquals("unauthored $language must fall back to hanji", "漢", hanjiOnly.resolve(language))
        }
    }

    @Test
    fun resolve_pseudo_returnsHanji() {
        // Content has no pseudo map; the layout-probe language renders Hanji (contract: pseudo → hanji).
        assertEquals("漢", allLanguages.resolve(DisplayLanguage.PSEUDO))
    }

    @Test
    fun resolve_system_throws() {
        // SYSTEM is a selection policy with no strings; render sites must pass an effective language.
        // Mirrors StringResolver.resolve's fail-fast (iOS uses assert + Hanji-degrade in release).
        assertThrows(IllegalStateException::class.java) {
            allLanguages.resolve(DisplayLanguage.SYSTEM)
        }
    }
}
