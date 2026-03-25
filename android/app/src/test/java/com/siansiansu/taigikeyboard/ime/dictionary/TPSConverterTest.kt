package com.siansiansu.taigikeyboard.ime.dictionary

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class TPSConverterTest {

    // MARK: - Syllable Boundary: ㄏ (initial) vs ㆷ (entering tone coda)

    @Test
    fun testToTL_syllableBoundary_initialAfterVowel() {
        // ㄍㄛㄏ: ㄏ is initial h-, NOT coda -h. Must NOT match "koh" (閣).
        assertEquals("ㄍㄛㄏ should insert space before ㄏ (initial)",
            "ko h", TPSConverter.toTL("ㄍㄛㄏ"))
    }

    @Test
    fun testToTL_syllableBoundary_enteringToneCoda() {
        // ㄍㄛㆷ: ㆷ is entering tone -h coda. Should produce "koh4".
        assertEquals("ㄍㄛㆷ should match koh4 via entering tone coda",
            "koh4", TPSConverter.toTL("ㄍㄛㆷ"))
    }

    @Test
    fun testToTL_syllableBoundary_multiSyllable() {
        // ㄍㄛㄏㄧㆲˊ: ko + hiong5, not "kohiong5"
        assertEquals("Consonant after vowel should start new syllable",
            "ko hiong5", TPSConverter.toTL("ㄍㄛㄏㄧㆲˊ"))
    }

    @Test
    fun testToTL_syllableBoundary_allCheckedCodas() {
        assertEquals("ㆴ = -p coda", "kap4", TPSConverter.toTL("ㄍㄚㆴ"))
        assertEquals("ㆵ = -t coda", "kat4", TPSConverter.toTL("ㄍㄚㆵ"))
        assertEquals("ㆻ = -k coda", "kak4", TPSConverter.toTL("ㄍㄚㆻ"))
        assertEquals("ㆷ = -h coda", "kah4", TPSConverter.toTL("ㄍㄚㆷ"))
    }

    @Test
    fun testToTL_syllableBoundary_toneResetsState() {
        // ㄍˋㄏㄧㆲˊ: consonant-only + tone → "k 2", then ㄏ starts new syllable
        assertEquals("k 2 hiong5", TPSConverter.toTL("ㄍˋㄏㄧㆲˊ"))
    }

    // MARK: - Syllable Boundary: ㄫ (initial) vs ㆭ (syllabic ng)

    @Test
    fun testToTL_syllableBoundary_syllabicNg() {
        // ㆭˊ: ㆭ is syllabic ng (vowel table) → "ng5" → matches 黃
        assertEquals("ㆭˊ (syllabic ng) should produce 'ng5'",
            "ng5", TPSConverter.toTL("ㆭˊ"))
    }

    @Test
    fun testToTL_syllableBoundary_initialNg() {
        // ㄫˊ: ㄫ is initial ng (consonant table) → should NOT produce "ng5"
        assertEquals("ㄫˊ (initial ng) should NOT match syllabic ng5 (黃)",
            "ng 5", TPSConverter.toTL("ㄫˊ"))
    }

    @Test
    fun testToTL_syllableBoundary_syllabicM() {
        // ㆬˋ: ㆬ is syllabic m (vowel table) → "m2"
        assertEquals("ㆬˋ (syllabic m) should produce 'm2'",
            "m2", TPSConverter.toTL("ㆬˋ"))
    }

    @Test
    fun testToTL_syllableBoundary_initialM() {
        // ㄇˋ: ㄇ is initial m (consonant table) → should NOT produce "m2"
        assertEquals("ㄇˋ (initial m) should NOT match syllabic m2",
            "m 2", TPSConverter.toTL("ㄇˋ"))
    }

    @Test
    fun testToTL_syllableBoundary_initialNgWithVowel() {
        // ㄫㄚˋ: normal syllable — initial ng + vowel a + tone 2
        assertEquals("ㄫㄚˋ should produce normal syllable 'nga2'",
            "nga2", TPSConverter.toTL("ㄫㄚˋ"))
    }

    // MARK: - Palatalized vs Non-Palatalized Affricates

    @Test
    fun testToTL_palatalized_compound() {
        assertEquals("ㄑㄧ = tshi", "tshi3", TPSConverter.toTL("ㄑㄧ˪"))
        assertEquals("ㄐㄧ = tsi", "tsi3", TPSConverter.toTL("ㄐㄧ˪"))
        assertEquals("ㄒㄧ = si", "si3", TPSConverter.toTL("ㄒㄧ˪"))
        assertEquals("ㆢㄧ = ji", "ji3", TPSConverter.toTL("ㆢㄧ˪"))
    }

    @Test
    fun testToTL_nonPalatalized_withI() {
        // Non-palatalized + ㄧ is invalid TPS, should insert space
        assertEquals("ㄘ + ㄧ invalid", "tsh i3", TPSConverter.toTL("ㄘㄧ˪"))
        assertEquals("ㄗ + ㄧ invalid", "ts i3", TPSConverter.toTL("ㄗㄧ˪"))
        assertEquals("ㄙ + ㄧ invalid", "s i3", TPSConverter.toTL("ㄙㄧ˪"))
        assertEquals("ㆡ + ㄧ invalid", "j i3", TPSConverter.toTL("ㆡㄧ˪"))
    }

    @Test
    fun testToTL_nonPalatalized_otherVowels() {
        // Non-palatalized with non-ㄧ vowels should work normally
        assertEquals("ㄘ + ㄚ is valid", "tsha2", TPSConverter.toTL("ㄘㄚˋ"))
        assertEquals("ㄗ + ㄨ is valid", "tsu5", TPSConverter.toTL("ㄗㄨˊ"))
    }

    // palatalizationReplacement

    @Test
    fun testPalatalization_allPairs_beforeI() {
        assertEquals("ㄙ + ㄧ → ㄒ", "ㄒ", TPSConverter.palatalizationReplacement("ㄧ", 'ㄙ'))
        assertEquals("ㄗ + ㄧ → ㄐ", "ㄐ", TPSConverter.palatalizationReplacement("ㄧ", 'ㄗ'))
        assertEquals("ㄘ + ㄧ → ㄑ", "ㄑ", TPSConverter.palatalizationReplacement("ㄧ", 'ㄘ'))
        assertEquals("ㆡ + ㄧ → ㆢ", "ㆢ", TPSConverter.palatalizationReplacement("ㄧ", 'ㆡ'))
    }

    @Test
    fun testPalatalization_nasalizedI_triggers() {
        // ㆪ (nasalized inn) also triggers palatalization
        assertEquals("ㄙ + ㆪ → ㄒ", "ㄒ", TPSConverter.palatalizationReplacement("ㆪ", 'ㄙ'))
    }

    @Test
    fun testPalatalization_noTrigger() {
        // Non-ㄧ vowel → null
        assertNull("ㄙ + ㄚ → null", TPSConverter.palatalizationReplacement("ㄚ", 'ㄙ'))
        // Already palatalized → null
        assertNull("ㄒ + ㄧ → null", TPSConverter.palatalizationReplacement("ㄧ", 'ㄒ'))
        // Non-affricate → null
        assertNull("ㄍ + ㄧ → null", TPSConverter.palatalizationReplacement("ㄧ", 'ㄍ'))
        // Null last char → null
        assertNull("null + ㄧ → null", TPSConverter.palatalizationReplacement("ㄧ", null))
    }
}
