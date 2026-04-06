package com.siansiansu.taigikeyboard.ime.dictionary

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TPSConverterTest {
    // MARK: - Syllable Boundary: ㄏ (initial) vs ㆷ (entering tone coda)

    @Test
    fun testToTL_syllableBoundary_initialAfterVowel() {
        // ㄍㄛㄏ: ㄏ is initial h-, NOT coda -h. Must NOT match "koh" (閣).
        assertEquals(
            "ㄍㄛㄏ should insert space before ㄏ (initial)",
            "ko h",
            TPSConverter.toTL("ㄍㄛㄏ"),
        )
    }

    @Test
    fun testToTL_syllableBoundary_enteringToneCoda() {
        // ㄍㄛㆷ: ㆷ is entering tone -h coda. Should produce "koh4".
        assertEquals(
            "ㄍㄛㆷ should match koh4 via entering tone coda",
            "koh4",
            TPSConverter.toTL("ㄍㄛㆷ"),
        )
    }

    @Test
    fun testToTL_syllableBoundary_multiSyllable() {
        // ㄍㄛㄏㄧㆲˊ: ko + hiong5, not "kohiong5"
        assertEquals(
            "Consonant after vowel should start new syllable",
            "ko hiong5",
            TPSConverter.toTL("ㄍㄛㄏㄧㆲˊ"),
        )
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
        assertEquals(
            "ㆭˊ (syllabic ng) should produce 'ng5'",
            "ng5",
            TPSConverter.toTL("ㆭˊ"),
        )
    }

    @Test
    fun testToTL_syllableBoundary_initialNg() {
        // ㄫˊ: ㄫ is initial ng (consonant table) → should NOT produce "ng5"
        assertEquals(
            "ㄫˊ (initial ng) should NOT match syllabic ng5 (黃)",
            "ng 5",
            TPSConverter.toTL("ㄫˊ"),
        )
    }

    @Test
    fun testToTL_syllableBoundary_syllabicM() {
        // ㆬˋ: ㆬ is syllabic m (vowel table) → "m2"
        assertEquals(
            "ㆬˋ (syllabic m) should produce 'm2'",
            "m2",
            TPSConverter.toTL("ㆬˋ"),
        )
    }

    @Test
    fun testToTL_syllableBoundary_initialM() {
        // ㄇˋ: ㄇ is initial m (consonant table) → should NOT produce "m2"
        assertEquals(
            "ㄇˋ (initial m) should NOT match syllabic m2",
            "m 2",
            TPSConverter.toTL("ㄇˋ"),
        )
    }

    @Test
    fun testToTL_syllableBoundary_initialNgWithVowel() {
        // ㄫㄚˋ: normal syllable — initial ng + vowel a + tone 2
        assertEquals(
            "ㄫㄚˋ should produce normal syllable 'nga2'",
            "nga2",
            TPSConverter.toTL("ㄫㄚˋ"),
        )
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

    // MARK: - Nasalized vowel + checked tone

    @Test
    fun testToTL_nasalizedVowel_checkedTone() {
        // ㆯ (aunn) + ㆷ˙ (h8) = aunnh8
        assertEquals("haunnh8", TPSConverter.toTL("ㄏㆯㆷ˙"))
        assertEquals("haunnh4", TPSConverter.toTL("ㄏㆯㆷ"))
        // Other nasalized + checked combinations
        assertEquals("annh8", TPSConverter.toTL("ㆩㆷ˙"))
        assertEquals("kaunnh8", TPSConverter.toTL("ㄍㆯㆷ˙"))
    }

    // MARK: - Multi-syllable with entering tones

    @Test
    fun testToTL_multiSyllable_enteringTone() {
        // Entering tone followed by new syllable should have space separator
        assertEquals("kah4 hiong5", TPSConverter.toTL("ㄍㄚㆷㄏㄧㆲˊ"))
        assertEquals("kah8 kah8", TPSConverter.toTL("ㄍㄚㆷ˙ㄍㄚㆷ˙"))
    }

    // ========================================================================
    // MARK: - containsTPS (ported from iOS TPSConverterTests)
    // ========================================================================

    @Test
    fun testContainsTPS_tpsChars() {
        assertTrue("containsTPS(\"ㄅㄚ\")", TPSConverter.containsTPS("ㄅㄚ"))
    }

    @Test
    fun testContainsTPS_latinOnly() {
        assertFalse("containsTPS(\"ka2\")", TPSConverter.containsTPS("ka2"))
    }

    @Test
    fun testContainsTPS_empty() {
        assertFalse("containsTPS(\"\")", TPSConverter.containsTPS(""))
    }

    @Test
    fun testContainsTPS_mixed() {
        assertTrue("containsTPS(\"abcㄅdef\")", TPSConverter.containsTPS("abcㄅdef"))
    }

    // ========================================================================
    // MARK: - isTPSToneMark
    // ========================================================================

    @Test
    fun testIsTPSToneMark_toneChars() {
        assertTrue("ˋ is tone mark", TPSConverter.isTPSToneMark('ˋ'))
        assertTrue("˪ is tone mark", TPSConverter.isTPSToneMark('˪'))
        assertTrue("ˊ is tone mark", TPSConverter.isTPSToneMark('ˊ'))
        assertTrue("ˇ is tone mark", TPSConverter.isTPSToneMark('ˇ'))
        assertTrue("˫ is tone mark", TPSConverter.isTPSToneMark('˫'))
        assertTrue("ˆ is tone mark", TPSConverter.isTPSToneMark('ˆ'))
        assertTrue("˙ is tone mark", TPSConverter.isTPSToneMark('˙'))
    }

    @Test
    fun testIsTPSToneMark_nonTone() {
        assertFalse("ㄅ is not tone mark", TPSConverter.isTPSToneMark('ㄅ'))
        assertFalse("ㄚ is not tone mark", TPSConverter.isTPSToneMark('ㄚ'))
        assertFalse("ㆴ is not tone mark", TPSConverter.isTPSToneMark('ㆴ'))
    }

    @Test
    fun testIsTPSToneMark_latin() {
        assertFalse("'a' is not tone mark", TPSConverter.isTPSToneMark('a'))
        assertFalse("'2' is not tone mark", TPSConverter.isTPSToneMark('2'))
    }

    // ========================================================================
    // MARK: - toTPS (ported from iOS TPSConverterTests)
    // ========================================================================

    @Test
    fun testToTPS_simpleSyllable() {
        assertEquals("pa1", "ㄅㄚ", TPSConverter.toTPS("pa1"))
    }

    @Test
    fun testToTPS_aspirated() {
        assertEquals("pha1", "ㄆㄚ", TPSConverter.toTPS("pha1"))
    }

    @Test
    fun testToTPS_tone2() {
        assertEquals("ka2", "ㄍㄚˋ", TPSConverter.toTPS("ka2"))
    }

    @Test
    fun testToTPS_tone3() {
        assertEquals("ka3", "ㄍㄚ˪", TPSConverter.toTPS("ka3"))
    }

    @Test
    fun testToTPS_tone5() {
        assertEquals("ka5", "ㄍㄚˊ", TPSConverter.toTPS("ka5"))
    }

    @Test
    fun testToTPS_tone7() {
        assertEquals("ka7", "ㄍㄚ˫", TPSConverter.toTPS("ka7"))
    }

    @Test
    fun testToTPS_tone6() {
        assertEquals("ka6", "ㄍㄚˇ", TPSConverter.toTPS("ka6"))
    }

    @Test
    fun testToTPS_tone9() {
        assertEquals("a9", "ㄚˆ", TPSConverter.toTPS("a9"))
    }

    @Test
    fun testToTPS_stopTone_p4() {
        assertEquals("kap4", "ㄍㄚㆴ", TPSConverter.toTPS("kap4"))
    }

    @Test
    fun testToTPS_stopTone_t4() {
        assertEquals("kat4", "ㄍㄚㆵ", TPSConverter.toTPS("kat4"))
    }

    @Test
    fun testToTPS_stopTone_k4() {
        assertEquals("kak4", "ㄍㄚㆻ", TPSConverter.toTPS("kak4"))
    }

    @Test
    fun testToTPS_stopTone_h4() {
        assertEquals("kah4", "ㄍㄚㆷ", TPSConverter.toTPS("kah4"))
    }

    @Test
    fun testToTPS_tone_p8() {
        assertEquals("kap8", "ㄍㄚㆴ˙", TPSConverter.toTPS("kap8"))
    }

    @Test
    fun testToTPS_tone8_nonStop() {
        val result = TPSConverter.toTPS("a8")
        assertTrue("Non-stop tone 8 should contain U+02D9", result.contains("\u02D9"))
    }

    @Test
    fun testToTPS_standaloneM() {
        assertEquals("m1", "ㆬ", TPSConverter.toTPS("m1"))
    }

    @Test
    fun testToTPS_standaloneNg() {
        assertEquals("ng1", "ㆭ", TPSConverter.toTPS("ng1"))
    }

    @Test
    fun testToTPS_standaloneM_withTone() {
        assertEquals("m7", "ㆬ˫", TPSConverter.toTPS("m7"))
    }

    @Test
    fun testToTPS_standaloneNg_withTone() {
        assertEquals("ng5", "ㆭˊ", TPSConverter.toTPS("ng5"))
    }

    @Test
    fun testToTPS_nasalVowel() {
        assertEquals("ann1", "ㆩ", TPSConverter.toTPS("ann1"))
    }

    @Test
    fun testToTPS_multiCharConsonant_tshi() {
        assertEquals("tshi1", "ㄑㄧ", TPSConverter.toTPS("tshi1"))
    }

    @Test
    fun testToTPS_nasalizedInn_afterPalatalizedTs() {
        assertEquals("tsinn5", "ㄐㆪˊ", TPSConverter.toTPS("tsinn5"))
    }

    @Test
    fun testToTPS_nasalizedInn_afterPalatalizedTsh() {
        assertEquals("tshinn5", "ㄑㆪˊ", TPSConverter.toTPS("tshinn5"))
    }

    @Test
    fun testToTPS_hyphenBecomesSpace() {
        val result = TPSConverter.toTPS("gua2-gua2")
        assertTrue("Hyphen should become space", result.contains(" "))
    }

    @Test
    fun testToTPS_oBecomesOO_beforeStopK4() {
        assertEquals("ok4", "ㆦㆻ", TPSConverter.toTPS("ok4"))
    }

    @Test
    fun testToTPS_oBecomesOO_beforeStopP4() {
        assertEquals("op4", "ㆦㆴ", TPSConverter.toTPS("op4"))
    }

    @Test
    fun testToTPS_oBecomesOO_beforeStopT4() {
        assertEquals("ot4", "ㆦㆵ", TPSConverter.toTPS("ot4"))
    }

    @Test
    fun testToTPS_oStaysO_beforeStopH4() {
        assertEquals("oh4", "ㄛㆷ", TPSConverter.toTPS("oh4"))
    }

    @Test
    fun testToTPS_ooStaysOO_beforeNonStop() {
        assertEquals("oo2", "ㆦˋ", TPSConverter.toTPS("oo2"))
    }

    @Test
    fun testToTPS_ing() {
        assertEquals("ing5", "ㄧㄥˊ", TPSConverter.toTPS("ing5"))
    }

    @Test
    fun testToTPS_king() {
        assertEquals("king5", "ㄍㄧㄥˊ", TPSConverter.toTPS("king5"))
    }

    @Test
    fun testToTPS_ung_usesNg() {
        assertEquals("ung7", "ㄨㆭ˫", TPSConverter.toTPS("ung7"))
    }

    @Test
    fun testToTPS_or_default() {
        assertEquals("or2 default", "ㄛˋ", TPSConverter.toTPS("or2"))
    }

    @Test
    fun testToTPS_or_mapsToER() {
        assertEquals("or2 orMapsToER", "ㄜˋ", TPSConverter.toTPS("or2", orMapsToER = true))
    }

    @Test
    fun testToTPS_er_unaffected() {
        assertEquals("er2", "ㄜˋ", TPSConverter.toTPS("er2"))
        assertEquals("er2 orMapsToER", "ㄜˋ", TPSConverter.toTPS("er2", orMapsToER = true))
    }

    @Test
    fun testToTPS_empty() {
        assertEquals("", TPSConverter.toTPS(""))
    }

    // ========================================================================
    // MARK: - adjustTPSInitialKey (ported from iOS TPSConverterTests)
    // ========================================================================

    @Test
    fun testAdjustTPSInitialKey_m_atSyllableStart() {
        assertEquals("ㄇ", TPSConverter.adjustTPSInitialKey("ㄇ", ""))
    }

    @Test
    fun testAdjustTPSInitialKey_m_afterVowel() {
        assertEquals("ㆬ", TPSConverter.adjustTPSInitialKey("ㄇ", "ㄧ"))
    }

    @Test
    fun testAdjustTPSInitialKey_ng_afterI() {
        assertEquals("ㄥ", TPSConverter.adjustTPSInitialKey("ㄫ", "ㄧ"))
    }

    @Test
    fun testAdjustTPSInitialKey_ng_afterOtherVowel() {
        assertEquals("ㆭ", TPSConverter.adjustTPSInitialKey("ㄫ", "ㄚ"))
    }

    @Test
    fun testAdjustTPSInitialKey_m_afterTone() {
        assertEquals("ㄇ", TPSConverter.adjustTPSInitialKey("ㄇ", "ㄇㄚˋ"))
    }

    @Test
    fun testAdjustTPSInitialKey_m_afterCheckedFinal() {
        assertEquals("ㄇ", TPSConverter.adjustTPSInitialKey("ㄇ", "ㄍㄚㆴ"))
    }

    @Test
    fun testAdjustTPSInitialKey_ng_atSyllableStart() {
        assertEquals("ㄫ", TPSConverter.adjustTPSInitialKey("ㄫ", ""))
    }

    @Test
    fun testAdjustTPSInitialKey_n_atSyllableStart() {
        assertEquals("ㄋ", TPSConverter.adjustTPSInitialKey("ㄋ", ""))
    }

    @Test
    fun testAdjustTPSInitialKey_n_afterVowel() {
        assertEquals("ㄣ", TPSConverter.adjustTPSInitialKey("ㄋ", "ㄒㄧ"))
    }

    @Test
    fun testAdjustTPSInitialKey_n_afterTone() {
        assertEquals("ㄋ", TPSConverter.adjustTPSInitialKey("ㄋ", "ㄒㄧㄣˋ"))
    }

    @Test
    fun testAdjustTPSInitialKey_n_afterCheckedFinal() {
        assertEquals("ㄋ", TPSConverter.adjustTPSInitialKey("ㄋ", "ㄍㄚㆴ"))
    }

    @Test
    fun testAdjustTPSInitialKey_n_afterSpace() {
        assertEquals("ㄋ", TPSConverter.adjustTPSInitialKey("ㄋ", "ㄚ "))
    }

    @Test
    fun testAdjustTPSInitialKey_p_atSyllableStart() {
        assertEquals("ㄅ", TPSConverter.adjustTPSInitialKey("ㄅ", ""))
    }

    @Test
    fun testAdjustTPSInitialKey_p_afterVowel() {
        assertEquals("ㆴ", TPSConverter.adjustTPSInitialKey("ㄅ", "ㄍㄚ"))
    }

    @Test
    fun testAdjustTPSInitialKey_p_afterTone() {
        assertEquals("ㄅ", TPSConverter.adjustTPSInitialKey("ㄅ", "ㄚˋ"))
    }

    @Test
    fun testAdjustTPSInitialKey_p_afterCheckedFinal() {
        assertEquals("ㄅ", TPSConverter.adjustTPSInitialKey("ㄅ", "ㄍㄚㆴ"))
    }

    @Test
    fun testAdjustTPSInitialKey_t_afterVowel() {
        assertEquals("ㆵ", TPSConverter.adjustTPSInitialKey("ㄉ", "ㄍㄚ"))
    }

    @Test
    fun testAdjustTPSInitialKey_k_afterVowel() {
        assertEquals("ㆻ", TPSConverter.adjustTPSInitialKey("ㄍ", "ㄍㄚ"))
    }

    @Test
    fun testAdjustTPSInitialKey_h_afterVowel() {
        assertEquals("ㆷ", TPSConverter.adjustTPSInitialKey("ㄏ", "ㄍㄚ"))
    }

    @Test
    fun testAdjustTPSInitialKey_h_atSyllableStart() {
        assertEquals("ㄏ", TPSConverter.adjustTPSInitialKey("ㄏ", ""))
    }

    @Test
    fun testAdjustTPSInitialKey_p_afterSpace() {
        assertEquals("ㄅ", TPSConverter.adjustTPSInitialKey("ㄅ", "ㄚ "))
    }

    @Test
    fun testAdjustTPSInitialKey_nonTargetKey() {
        assertEquals("ㄌ", TPSConverter.adjustTPSInitialKey("ㄌ", "ㄧ"))
    }

    // ========================================================================
    // MARK: - adjustTPSNasalizedVowelKey
    // ========================================================================

    @Test
    fun testAdjustTPSNasalizedVowelKey_default() {
        assertEquals("ㆮ unchanged", "ㆮ", TPSConverter.adjustTPSNasalizedVowelKey("ㆮ", ""))
    }

    @Test
    fun testAdjustTPSNasalizedVowelKey_afterI() {
        assertEquals("ㆮ after ㄧ → ㆯ", "ㆯ", TPSConverter.adjustTPSNasalizedVowelKey("ㆮ", "ㄧ"))
    }

    @Test
    fun testAdjustTPSNasalizedVowelKey_nonAinn() {
        assertEquals("ㆩ unchanged", "ㆩ", TPSConverter.adjustTPSNasalizedVowelKey("ㆩ", "ㄧ"))
    }

    // ========================================================================
    // MARK: - displayRoman
    // ========================================================================

    @Test
    fun testDisplayRoman_tpsLayout() {
        assertEquals("ㄍㄚˋ", TPSConverter.displayRoman("ka2", "tps"))
    }

    @Test
    fun testDisplayRoman_nonTpsLayout() {
        assertEquals("ka2", TPSConverter.displayRoman("ka2", "roman"))
    }

    @Test
    fun testDisplayRoman_tpsLayout_orMapsToER() {
        assertEquals("ㄜˋ", TPSConverter.displayRoman("or2", "tps", orMapsToER = true))
    }

    // ========================================================================
    // MARK: - toTPSFromDisplay
    // ========================================================================

    @Test
    fun testToTPSFromDisplay_diacriticInput() {
        val result = TPSConverter.toTPSFromDisplay("gu\u00E1") // guá
        assertEquals("guá → TPS", TPSConverter.toTPS("gua2"), result)
    }

    @Test
    fun testToTPSFromDisplay_multiSyllable() {
        val result = TPSConverter.toTPSFromDisplay("g\u00E2u-ts\u00E1") // gâu-tsá
        assertEquals("gâu-tsá → TPS", TPSConverter.toTPS("gau5-tsa2"), result)
    }

    @Test
    fun testToTPSFromDisplay_empty() {
        assertEquals("", TPSConverter.toTPSFromDisplay(""))
    }

    // ========================================================================
    // MARK: - Round-Trip
    // ========================================================================

    @Test
    fun testRoundTrip_tpsToTlToTps() {
        val cases =
            listOf(
                "ㄍㄨㄚˋ" to "kua2",
                "ㄉㄧㄠˊ" to "tiau5",
                "ㄍㄚㆻ˙" to "kak8",
            )
        for ((tps, expectedTl) in cases) {
            val tl = TPSConverter.toTL(tps)
            assertEquals("toTL($tps)", expectedTl, tl)
            val backToTps = TPSConverter.toTPS(tl)
            assertEquals("Round-trip toTPS(toTL($tps))", tps, backToTps)
        }
    }

    @Test
    fun testRoundTrip_tlToTpsToTl() {
        val cases = listOf("ka2", "tshiu7", "ing5", "m7", "kah4")
        for (tl in cases) {
            val tps = TPSConverter.toTPS(tl)
            val backToTl = TPSConverter.toTL(tps)
            assertEquals("Round-trip toTL(toTPS($tl))", tl, backToTl)
        }
    }

    // ========================================================================
    // MARK: - syllabicNasalReplacement
    // ========================================================================

    @Test
    fun testSyllabicNasalReplacement_m_beforeToneMark() {
        assertEquals("ㆬ", TPSConverter.syllabicNasalReplacement("˫", 'ㄇ'))
        assertEquals("ㆬ", TPSConverter.syllabicNasalReplacement("ˋ", 'ㄇ'))
        assertEquals("ㆬ", TPSConverter.syllabicNasalReplacement("ˊ", 'ㄇ'))
    }

    @Test
    fun testSyllabicNasalReplacement_ng_beforeToneMark() {
        assertEquals("ㆭ", TPSConverter.syllabicNasalReplacement("ˊ", 'ㄫ'))
        assertEquals("ㆭ", TPSConverter.syllabicNasalReplacement("˫", 'ㄫ'))
    }

    @Test
    fun testSyllabicNasalReplacement_nonTone_noChange() {
        assertNull("vowel is not tone mark", TPSConverter.syllabicNasalReplacement("ㄚ", 'ㄇ'))
    }

    @Test
    fun testSyllabicNasalReplacement_otherConsonant_noChange() {
        assertNull("ㄍ is not ㄇ/ㄫ", TPSConverter.syllabicNasalReplacement("˫", 'ㄍ'))
    }

    @Test
    fun testSyllabicNasalReplacement_alreadySyllabic_noChange() {
        assertNull("ㆬ already syllabic", TPSConverter.syllabicNasalReplacement("˫", 'ㆬ'))
    }

    @Test
    fun testSyllabicNasalReplacement_nullLastChar() {
        assertNull("null last char", TPSConverter.syllabicNasalReplacement("˫", null))
    }
}
