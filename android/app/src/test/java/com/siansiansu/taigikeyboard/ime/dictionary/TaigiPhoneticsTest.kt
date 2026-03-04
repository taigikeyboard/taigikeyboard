package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** TaigiPhonetics unit tests
 *  Ported from references/taigi-converter/tests/{phonetics,tl,poj}.test.js */
class TaigiPhoneticsTest {

    // MARK: - A. stripToneMark

    @Test
    fun testStripToneMark_acuteAccentTone2() {
        val (bare, tone) = TaigiPhonetics.stripToneMark("\u00E1")  // á
        assertEquals("a", bare)
        assertEquals("2", tone)
    }

    @Test
    fun testStripToneMark_graveAccentTone3() {
        val (bare, tone) = TaigiPhonetics.stripToneMark("\u00E0")  // à
        assertEquals("a", bare)
        assertEquals("3", tone)
    }

    @Test
    fun testStripToneMark_circumflexTone5() {
        val (bare, tone) = TaigiPhonetics.stripToneMark("\u00E2")  // â
        assertEquals("a", bare)
        assertEquals("5", tone)
    }

    @Test
    fun testStripToneMark_macronTone7() {
        val (bare, tone) = TaigiPhonetics.stripToneMark("\u0101")  // ā
        assertEquals("a", bare)
        assertEquals("7", tone)
    }

    @Test
    fun testStripToneMark_verticalLineTone8() {
        val (bare, tone) = TaigiPhonetics.stripToneMark("a\u030D")
        assertEquals("a", bare)
        assertEquals("8", tone)
    }

    @Test
    fun testStripToneMark_breveTone9_POJ() {
        val (bare, tone) = TaigiPhonetics.stripToneMark("\u0103")  // ă (a + breve)
        assertEquals("a", bare)
        assertEquals("9", tone)
    }

    @Test
    fun testStripToneMark_doubleAcuteTone9_TL() {
        val (bare, tone) = TaigiPhonetics.stripToneMark("a\u030B")
        assertEquals("a", bare)
        assertEquals("9", tone)
    }

    @Test
    fun testStripToneMark_noMark() {
        val (bare, tone) = TaigiPhonetics.stripToneMark("a")
        assertEquals("a", bare)
        assertEquals("", tone)
    }

    @Test
    fun testStripToneMark_trailingDigit() {
        val (bare, tone) = TaigiPhonetics.stripToneMark("ka2")
        assertEquals("ka", bare)
        assertEquals("2", tone)
    }

    @Test
    fun testStripToneMark_multiCharSyllable() {
        // tshiū = tshiu + macron on u
        val (bare, tone) = TaigiPhonetics.stripToneMark("tshi\u016B")
        assertEquals("tshiu", bare)
        assertEquals("7", tone)
    }

    // MARK: - B. normalizeToTL

    @Test
    fun testNormalizeToTL_cases() {
        val cases = listOf(
            "ch" to "ts",
            "chh" to "tsh",
            "oa" to "ua",
            "oe" to "ue",
            "eng" to "ing",
            "ek" to "ik",
            "ou" to "oo",
            "o\u0358" to "oo",     // o͘ -> oo
            "\u207F" to "nn",      // ⁿ -> nn
            "oonn" to "onn",
        )
        for ((input, expected) in cases) {
            assertEquals(
                "normalizeToTL($input) should be $expected",
                expected, TaigiPhonetics.normalizeToTL(input)
            )
        }
    }

    // MARK: - C. isStopTone

    @Test
    fun testIsStopTone_stopEndings() {
        val cases = listOf(
            "ap" to true,
            "at" to true,
            "ak" to true,
            "ah" to true,
            "a" to false,
            "an" to false,
            "ang" to false,
            "annh" to true,  // nasal with h
        )
        for ((final_, expected) in cases) {
            assertEquals(
                "isStopTone($final_) should be $expected",
                expected, TaigiPhonetics.isStopTone(final_)
            )
        }
    }

    // MARK: - D. splitInitialFinal

    @Test
    fun testSplitInitialFinal_validSyllables() {
        val cases = listOf(
            Triple("ka", "k", "a"),
            Triple("tshiu", "tsh", "iu"),
            Triple("a", "", "a"),       // no initial
            Triple("ng", "", "ng"),     // syllabic ng
            Triple("m", "", "m"),       // syllabic m
            Triple("phang", "ph", "ang"),
            Triple("iang", "", "iang"),
            Triple("oo", "", "oo"),
        )
        for ((input, expectedInitial, expectedFinal) in cases) {
            val result = TaigiPhonetics.splitInitialFinal(input)
            assertNotNull("splitInitialFinal($input) should not be null", result)
            assertEquals("initial of $input", expectedInitial, result!!.first)
            assertEquals("final of $input", expectedFinal, result.second)
        }
    }

    @Test
    fun testSplitInitialFinal_invalidReturnsNull() {
        assertNull("splitInitialFinal(\"xyz\") should be null", TaigiPhonetics.splitInitialFinal("xyz"))
    }

    // MARK: - E. parseSyllable

    @Test
    fun testParseSyllable_simpleCases() {
        val cases = listOf(
            // (input, initial, final, tone)
            arrayOf("ka2", "k", "a", "2"),
            arrayOf("kang1", "k", "ang", "1"),
            arrayOf("a1", "", "a", "1"),
            // Tone mark
            arrayOf("k\u00E1", "k", "a", "2"),  // ká
            // Inferred tones
            arrayOf("kah", "k", "ah", "4"),   // stop tone -> 4
            arrayOf("ka", "k", "a", "1"),      // non-stop -> 1
            // Aspirated initial
            arrayOf("pha3", "ph", "a", "3"),
            // tsh initial
            arrayOf("tshiu7", "tsh", "iu", "7"),
        )
        for (case_ in cases) {
            val (input, expectedInitial, expectedFinal, expectedTone) = case_
            val result = TaigiPhonetics.parseSyllable(input)
            assertNotNull("parseSyllable($input) should not be null", result)
            assertEquals("initial of $input", expectedInitial, result!!.first)
            assertEquals("final of $input", expectedFinal, result.second)
            assertEquals("tone of $input", expectedTone, result.third)
        }
    }

    @Test
    fun testParseSyllable_pojForms() {
        val cases = listOf(
            arrayOf("chhi2", "tsh", "i", "2"),    // ch->ts, chh->tsh
            arrayOf("koa1", "k", "ua", "1"),       // oa->ua
            arrayOf("koe1", "k", "ue", "1"),       // oe->ue
            arrayOf("peng5", "p", "ing", "5"),     // eng->ing
        )
        for (case_ in cases) {
            val (input, expectedInitial, expectedFinal, expectedTone) = case_
            val result = TaigiPhonetics.parseSyllable(input)
            assertNotNull("parseSyllable($input) should not be null", result)
            assertEquals("initial of $input", expectedInitial, result!!.first)
            assertEquals("final of $input", expectedFinal, result.second)
            assertEquals("tone of $input", expectedTone, result.third)
        }
    }

    @Test
    fun testParseSyllable_syllabicConsonants() {
        val ngResult = TaigiPhonetics.parseSyllable("ng5")
        assertNotNull("parseSyllable(\"ng5\") should not be null", ngResult)
        assertEquals("initial of ng5", "", ngResult!!.first)
        assertEquals("final of ng5", "ng", ngResult.second)
        assertEquals("tone of ng5", "5", ngResult.third)

        val mResult = TaigiPhonetics.parseSyllable("m7")
        assertNotNull("parseSyllable(\"m7\") should not be null", mResult)
        assertEquals("initial of m7", "", mResult!!.first)
        assertEquals("final of m7", "m", mResult.second)
        assertEquals("tone of m7", "7", mResult.third)
    }

    @Test
    fun testParseSyllable_invalidReturnsNull() {
        assertNull("parseSyllable(\"xyz\") should be null", TaigiPhonetics.parseSyllable("xyz"))
    }

    // MARK: - F. toTL

    @Test
    fun testToTL_allTones() {
        val cases = listOf(
            arrayOf("k", "a", "1", "ka"),             // tone 1: no mark
            arrayOf("k", "a", "2", "k\u00E1"),        // ká
            arrayOf("k", "a", "3", "k\u00E0"),        // kà
            arrayOf("k", "ah", "4", "kah"),            // tone 4: no mark
            arrayOf("k", "a", "5", "k\u00E2"),        // kâ
            arrayOf("k", "a", "7", "k\u0101"),        // kā
            arrayOf("k", "ah", "8", "ka\u030Dh"),     // ka̍h
        )
        for (case_ in cases) {
            val (initial, final_, tone, expected) = case_
            val result = TaigiPhonetics.toTL(initial = initial, final_ = final_, tone = tone)
            assertEquals("toTL($initial, $final_, $tone)", expected, result)
        }
    }

    @Test
    fun testToTL_tone9_doubleAcute() {
        val result = TaigiPhonetics.toTL(initial = "k", final_ = "a", tone = "9")
        assertTrue(
            "TL tone 9 should use double acute accent (U+030B)",
            result.codePoints().toArray().contains(0x030B)
        )
    }

    @Test
    fun testToTL_vowelPriority() {
        val cases = listOf(
            // a takes priority
            arrayOf("k", "ai", "2", "k\u00E1i"),
            // oo: mark between o's
            arrayOf("k", "oo", "5", "k\u00F4o"),
            // e
            arrayOf("t", "e", "7", "t\u0113"),
            // o
            arrayOf("k", "o", "2", "k\u00F3"),
            // ui -> mark on i
            arrayOf("k", "ui", "3", "ku\u00EC"),
            // iu -> mark on u
            arrayOf("tsh", "iu", "7", "tshi\u016B"),
            // ng -> mark on n
            arrayOf("", "ng", "5", "n\u0302g"),
            // m -> mark on m
            arrayOf("", "m", "7", "m\u0304"),
        )
        for (case_ in cases) {
            val (initial, final_, tone, expected) = case_
            val result = TaigiPhonetics.toTL(initial = initial, final_ = final_, tone = tone)
            assertEquals("toTL($initial, $final_, $tone)", expected, result)
        }
    }

    @Test
    fun testToTL_noInitial() {
        val result = TaigiPhonetics.toTL(initial = "", final_ = "a", tone = "2")
        assertEquals("\u00E1", result)  // á
    }

    @Test
    fun testToTL_complexFinal_iang() {
        // a takes priority in iang
        val result = TaigiPhonetics.toTL(initial = "k", final_ = "iang", tone = "5")
        assertEquals("ki\u00E2ng", result)  // kiâng
    }

    // MARK: - G. toPOJ

    @Test
    fun testToPOJ_initialConversion() {
        // ts -> ch
        val result1 = TaigiPhonetics.toPOJ(initial = "ts", final_ = "u", tone = "2")
        assertEquals("ch\u00FA", result1)  // chú

        // tsh -> chh
        val result2 = TaigiPhonetics.toPOJ(initial = "tsh", final_ = "iu", tone = "7")
        assertEquals("chhi\u016B", result2)  // chhiū
    }

    @Test
    fun testToPOJ_finalConversions() {
        // nn -> ⁿ
        val annResult = TaigiPhonetics.toPOJ(initial = "k", final_ = "ann", tone = "2")
        assertTrue("nn should become ⁿ in POJ: got $annResult", annResult.contains("\u207F"))

        // oo -> o͘
        val ooResult = TaigiPhonetics.toPOJ(initial = "k", final_ = "oo", tone = "1")
        assertTrue(
            "oo should become o͘ in POJ: got $ooResult",
            ooResult.codePoints().toArray().contains(0x0358)
        )

        // ua -> oa
        val uaResult = TaigiPhonetics.toPOJ(initial = "k", final_ = "ua", tone = "1")
        assertTrue("ua should become oa in POJ: got $uaResult", uaResult.contains("oa"))

        // ue -> oe
        val ueResult = TaigiPhonetics.toPOJ(initial = "k", final_ = "ue", tone = "1")
        assertTrue("ue should become oe in POJ: got $ueResult", ueResult.contains("oe"))

        // ing -> eng
        val ingResult = TaigiPhonetics.toPOJ(initial = "p", final_ = "ing", tone = "1")
        assertEquals("peng", ingResult)

        // ik -> ek
        val ikResult = TaigiPhonetics.toPOJ(initial = "p", final_ = "ik", tone = "4")
        assertTrue("ik should become ek in POJ: got $ikResult", ikResult.contains("ek"))
    }

    @Test
    fun testToPOJ_toneMarks() {
        assertEquals("ka", TaigiPhonetics.toPOJ(initial = "k", final_ = "a", tone = "1"))
        assertEquals("k\u00E1", TaigiPhonetics.toPOJ(initial = "k", final_ = "a", tone = "2"))
        assertEquals("k\u00E2", TaigiPhonetics.toPOJ(initial = "k", final_ = "a", tone = "5"))
        assertEquals("k\u0101", TaigiPhonetics.toPOJ(initial = "k", final_ = "a", tone = "7"))
    }

    @Test
    fun testToPOJ_tone9_breve() {
        val result = TaigiPhonetics.toPOJ(initial = "k", final_ = "a", tone = "9")
        // POJ tone 9 uses breve: ă (U+0103)
        assertTrue("POJ tone 9 should use breve: got $result", result.contains("\u0103"))
    }

    @Test
    fun testToPOJ_triphthong_iau_markOnA() {
        val result = TaigiPhonetics.toPOJ(initial = "", final_ = "iau", tone = "5")
        assertTrue("iau mark should be on a: got $result", result.contains("\u00E2"))
    }

    @Test
    fun testToPOJ_diphthong_ai_markOnFirst() {
        val result = TaigiPhonetics.toPOJ(initial = "k", final_ = "ai", tone = "2")
        assertEquals("k\u00E1i", result)
    }

    @Test
    fun testToPOJ_nonTsInitialUnchanged() {
        assertEquals("k\u00E1", TaigiPhonetics.toPOJ(initial = "k", final_ = "a", tone = "2"))
        assertEquals("p\u00E1", TaigiPhonetics.toPOJ(initial = "p", final_ = "a", tone = "2"))
        assertEquals("h\u00E1", TaigiPhonetics.toPOJ(initial = "h", final_ = "a", tone = "2"))
    }

    // MARK: - H. convertSyllable

    @Test
    fun testConvertSyllable_tlMode() {
        // Tones 2-9 (except 4) get marks
        val result = TaigiPhonetics.convertSyllable("ka2", InputMode.TL)
        assertEquals("k\u00E1", result)

        // Tone 1 keeps digit
        assertEquals("ka1", TaigiPhonetics.convertSyllable("ka1", InputMode.TL))
        // Tone 4 keeps digit
        assertEquals("kah4", TaigiPhonetics.convertSyllable("kah4", InputMode.TL))
    }

    @Test
    fun testConvertSyllable_pojMode() {
        val result = TaigiPhonetics.convertSyllable("ka2", InputMode.POJ)
        assertEquals("k\u00E1", result)
    }

    @Test
    fun testConvertSyllable_casePreservation() {
        // Uppercase first letter should be preserved
        val result = TaigiPhonetics.convertSyllable("Ka2", InputMode.TL)
        assertTrue("Case should be preserved: got $result", result.first().isUpperCase())
    }

    @Test
    fun testConvertSyllable_noToneDigit() {
        // No trailing digit -> returned as-is
        assertEquals("ka", TaigiPhonetics.convertSyllable("ka", InputMode.TL))
    }

    @Test
    fun testConvertSyllable_englishPassthrough() {
        // English mode should return syllable as-is
        assertEquals("hello2", TaigiPhonetics.convertSyllable("hello2", InputMode.ENGLISH))
        assertEquals("ka2", TaigiPhonetics.convertSyllable("ka2", InputMode.ENGLISH))
    }

    @Test
    fun testConvertSyllable_invalidSyllable() {
        // Invalid syllable returned as-is
        assertEquals("xyz5", TaigiPhonetics.convertSyllable("xyz5", InputMode.TL))
    }

    // MARK: - I. convertToToneMarks

    @Test
    fun testConvertToToneMarks_multiSyllable() {
        val result = TaigiPhonetics.convertToToneMarks("ka2-lang5", InputMode.TL)
        assertEquals("k\u00E1-l\u00E2ng", result)
    }

    @Test
    fun testConvertToToneMarks_mixedTones() {
        val result = TaigiPhonetics.convertToToneMarks("gua2-si7-hak8-sing1", InputMode.TL)
        // gua2 -> guá, si7 -> sī, hak8 -> ha̍k (keep digit for 1)
        assertTrue("gua2 should produce guá", result.contains("gu\u00E1"))
        assertTrue("si7 should produce sī", result.contains("s\u012B"))
    }

    @Test
    fun testConvertToToneMarks_emptyString() {
        assertEquals("", TaigiPhonetics.convertToToneMarks("", InputMode.TL))
    }

    @Test
    fun testConvertToToneMarks_singleSyllable() {
        val result = TaigiPhonetics.convertToToneMarks("ho2", InputMode.TL)
        assertEquals("h\u00F3", result)
    }

    // MARK: - J. tlDisplayToPOJDisplay

    @Test
    fun testTlDisplayToPOJDisplay_tsConversion() {
        // TL ts -> POJ ch
        val result = TaigiPhonetics.tlDisplayToPOJDisplay("ts\u00E1i")  // tsái
        assertTrue("ts should become ch: got $result", result.startsWith("ch"))
    }

    @Test
    fun testTlDisplayToPOJDisplay_casePreservation() {
        val result = TaigiPhonetics.tlDisplayToPOJDisplay("T\u00E2i")  // Tâi
        assertTrue("Case should be preserved: got $result", result.first().isUpperCase())
    }

    @Test
    fun testTlDisplayToPOJDisplay_ooHandling() {
        // TL oo -> POJ o͘
        val result = TaigiPhonetics.tlDisplayToPOJDisplay("h\u00F4o")  // hôo
        assertTrue(
            "oo should become o͘ in POJ: got $result",
            result.codePoints().toArray().contains(0x0358)
        )
    }

    @Test
    fun testTlDisplayToPOJDisplay_hyphenatedMultiSyllable() {
        val result = TaigiPhonetics.tlDisplayToPOJDisplay("t\u00E2i-g\u00ED")  // tâi-gí
        assertTrue("Hyphens should be preserved", result.contains("-"))
    }

    @Test
    fun testTlDisplayToPOJDisplay_empty() {
        assertEquals("", TaigiPhonetics.tlDisplayToPOJDisplay(""))
    }
}
