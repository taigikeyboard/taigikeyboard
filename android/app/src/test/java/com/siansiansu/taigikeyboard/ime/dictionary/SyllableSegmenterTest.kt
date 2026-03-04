package com.siansiansu.taigikeyboard.ime.dictionary

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.assertFalse
import org.junit.Test

/**
 * SyllableSegmenter unit tests
 *
 * Ported from iOS SyllableSegmenterTests.swift
 */
class SyllableSegmenterTest {

    // MARK: - Basic Segmentation

    @Test
    fun testSegment_multiSyllableContinuous() {
        val result = SyllableSegmenter.segment("gua2si7soo")
        assertEquals("segment(\"gua2si7soo\")", listOf("gua2", "si7", "soo"), result)
    }

    @Test
    fun testSegment_singleSyllable() {
        assertEquals("segment(\"ka2\")", listOf("ka2"), SyllableSegmenter.segment("ka2"))
    }

    @Test
    fun testSegment_singleSyllableNoTone() {
        assertEquals("segment(\"ka\")", listOf("ka"), SyllableSegmenter.segment("ka"))
    }

    @Test
    fun testSegment_empty() {
        assertEquals("segment(\"\")", emptyList<String>(), SyllableSegmenter.segment(""))
    }

    // MARK: - Hyphen Handling

    @Test
    fun testSegment_hyphenSeparated() {
        val result = SyllableSegmenter.segment("gua2-si7")
        assertEquals("segment(\"gua2-si7\")", listOf("gua2-", "si7"), result)
    }

    @Test
    fun testSegment_mixedContinuousAndHyphen() {
        val result = SyllableSegmenter.segment("gua2si7-soo")
        assertEquals("segment(\"gua2si7-soo\")", listOf("gua2", "si7-", "soo"), result)
    }

    @Test
    fun testSegment_multipleHyphens() {
        val result = SyllableSegmenter.segment("ka2-lang5-e5")
        assertEquals("segment(\"ka2-lang5-e5\")", listOf("ka2-", "lang5-", "e5"), result)
    }

    @Test
    fun testSegment_leadingHyphen_preserved() {
        val cases = listOf(
            "-gua2" to listOf("-gua2"),
            "-gua2-si7" to listOf("-gua2-", "si7"),
            "--a" to listOf("--a"),
            "-" to listOf("-"),
            "--" to listOf("--"),
        )
        for ((input, expected) in cases) {
            assertEquals(
                "segment(\"$input\"): leading hyphen must be preserved",
                expected,
                SyllableSegmenter.segment(input)
            )
        }
    }

    // MARK: - POJ Input Forms

    @Test
    fun testSegment_pojChInitial() {
        val result = SyllableSegmenter.segment("chhi2ka1")
        assertEquals("segment(\"chhi2ka1\")", listOf("chhi2", "ka1"), result)
    }

    @Test
    fun testSegment_pojOaFinal() {
        val result = SyllableSegmenter.segment("koa1sue3")
        assertEquals("segment(\"koa1sue3\")", listOf("koa1", "sue3"), result)
    }

    @Test
    fun testSegment_pojEngFinal() {
        val result = SyllableSegmenter.segment("peng5")
        assertEquals("segment(\"peng5\")", listOf("peng5"), result)
    }

    // MARK: - Edge Cases

    @Test
    fun testSegment_unrecognizedFallsBackToSingleChars() {
        val result = SyllableSegmenter.segment("xyz")
        assertEquals("segment(\"xyz\") count", 3, result.size)
        assertEquals("segment(\"xyz\")", listOf("x", "y", "z"), result)
    }

    @Test
    fun testSegment_longInput() {
        val result = SyllableSegmenter.segment("bin5hian5")
        assertEquals("segment(\"bin5hian5\")", listOf("bin5", "hian5"), result)
    }

    // MARK: - Case Preservation

    @Test
    fun testSegment_uppercasePreserved() {
        val result = SyllableSegmenter.segment("Gua2si7")
        assertEquals("segment(\"Gua2si7\") count", 2, result.size)
        assertTrue(
            "Uppercase should be preserved in output: got ${result[0]}",
            result[0].startsWith("G")
        )
    }

    // MARK: - Tone Digits

    @Test
    fun testSegment_allToneDigits() {
        val cases = listOf(
            "ka1" to listOf("ka1"),
            "ka2" to listOf("ka2"),
            "ka3" to listOf("ka3"),
            "kah4" to listOf("kah4"),
            "ka5" to listOf("ka5"),
            "ka7" to listOf("ka7"),
            "kah8" to listOf("kah8"),
            "ka9" to listOf("ka9"),
        )
        for ((input, expected) in cases) {
            assertEquals("segment(\"$input\")", expected, SyllableSegmenter.segment(input))
        }
    }

    // MARK: - Complex Multi-Syllable

    @Test
    fun testSegment_fiveSyllables() {
        val result = SyllableSegmenter.segment("gua2si7hak8sing1e5")
        assertEquals(
            "segment(\"gua2si7hak8sing1e5\")",
            listOf("gua2", "si7", "hak8", "sing1", "e5"),
            result
        )
    }

    // MARK: - Onset Atomicity (MOE2 layout)

    @Test
    fun testSegment_standaloneOnset_notSplit() {
        val cases = listOf(
            "tsh" to listOf("tsh"),
            "ts" to listOf("ts"),
            "ph" to listOf("ph"),
            "th" to listOf("th"),
            "kh" to listOf("kh"),
            "ng" to listOf("ng"),
        )
        for ((input, expected) in cases) {
            assertEquals("Onset '$input' must stay atomic", expected, SyllableSegmenter.segment(input))
        }
    }

    @Test
    fun testSegment_standaloneOnset_poj() {
        val cases = listOf(
            "ch" to listOf("ch"),
            "chh" to listOf("chh"),
        )
        for ((input, expected) in cases) {
            assertEquals("POJ onset '$input' must stay atomic", expected, SyllableSegmenter.segment(input))
        }
    }

    @Test
    fun testSegment_trailingOnset_notSplit() {
        val cases = listOf(
            "ka2tsh" to listOf("ka2", "tsh"),
            "ka2ph" to listOf("ka2", "ph"),
            "gua2si7th" to listOf("gua2", "si7", "th"),
        )
        for ((input, expected) in cases) {
            assertEquals("segment(\"$input\"): trailing onset must stay atomic", expected, SyllableSegmenter.segment(input))
        }
    }

    @Test
    fun testSegment_onsetWithFinal_stillPrefersSyllable() {
        val cases = listOf(
            "tsha2" to listOf("tsha2"),
            "pha3" to listOf("pha3"),
            "thau5" to listOf("thau5"),
            "khi2" to listOf("khi2"),
            "tsai5" to listOf("tsai5"),
            "chhi2ka1" to listOf("chhi2", "ka1"),
            "phong" to listOf("phong"),
        )
        for ((input, expected) in cases) {
            assertEquals(
                "segment(\"$input\"): complete syllable must not be split at onset boundary",
                expected, SyllableSegmenter.segment(input)
            )
        }
    }

    // MARK: - Nasalization Suffix Atomicity (MOE2 layout)

    @Test
    fun testSegment_standaloneNn_notSplit() {
        assertEquals(
            "Standalone 'nn' must stay atomic",
            listOf("nn"), SyllableSegmenter.segment("nn")
        )
    }

    @Test
    fun testSegment_trailingNn_notSplit() {
        assertEquals(
            "Trailing 'nn' after toned syllable must stay atomic",
            listOf("ka2", "nn"), SyllableSegmenter.segment("ka2nn")
        )
    }

    @Test
    fun testSegment_nnInSyllable_stillPrefersSyllable() {
        val cases = listOf(
            "kann2" to listOf("kann2"),
            "ann" to listOf("ann"),
            "phiann3" to listOf("phiann3"),
            "iunn5" to listOf("iunn5"),
        )
        for ((input, expected) in cases) {
            assertEquals(
                "segment(\"$input\"): 'nn' must remain part of complete syllable",
                expected, SyllableSegmenter.segment(input)
            )
        }
    }

    // MARK: - CVC+V Tie-Breaking (without checker)

    @Test
    fun testSegment_cvcvTie_withoutChecker_documentsCurrentBehavior() {
        // Without a wordPrefixChecker, ties resolve by first-arrival.
        // kina2jit8: ki(4)+na2(9)=13 wins over kin(9)+a2(4)=13
        assertEquals(
            "Without checker, first-arrival wins the tie (known limitation)",
            listOf("ki", "na2", "jit8"),
            SyllableSegmenter.segment("kina2jit8")
        )
    }

    @Test
    fun testSegment_ama_withoutChecker_correctByDefault() {
        // ama: a(1)+ma(4)=5 wins over am(4)+a(1)=5 by first-arrival.
        assertEquals(
            "Without checker, ama already segments correctly by first-arrival",
            listOf("a", "ma"),
            SyllableSegmenter.segment("ama")
        )
    }

    // MARK: - CVC+V Tie-Breaking (with mock checker)

    @Test
    fun testSegment_kina2jit8_withChecker_prefersKinA2() {
        val checker: WordPrefixChecker = { key ->
            key.startsWith("kin1a2")
        }
        assertEquals(
            "With checker, kin+a2+jit8 should win tie (今仔日)",
            listOf("kin", "a2", "jit8"),
            SyllableSegmenter.segment("kina2jit8", wordPrefixChecker = checker)
        )
    }

    @Test
    fun testSegment_ama_withChecker_preservesCorrectPath() {
        val checker: WordPrefixChecker = { key ->
            key.startsWith("ama")
        }
        assertEquals(
            "With checker, a+ma should be preserved (阿媽)",
            listOf("a", "ma"),
            SyllableSegmenter.segment("ama", wordPrefixChecker = checker)
        )
    }

    @Test
    fun testSegment_hita2e5_withChecker_prefersHitA2() {
        val checker: WordPrefixChecker = { key ->
            key.startsWith("hit4a2")
        }
        assertEquals(
            "With checker, hit+a2+e5 should win tie (彼个)",
            listOf("hit", "a2", "e5"),
            SyllableSegmenter.segment("hita2e5", wordPrefixChecker = checker)
        )
    }

    @Test
    fun testSegment_noChecker_backwardCompatible() {
        val cases = listOf(
            "gua2si7soo" to listOf("gua2", "si7", "soo"),
            "ka2" to listOf("ka2"),
            "bin5hian5" to listOf("bin5", "hian5"),
            "gua2si7-soo" to listOf("gua2", "si7-", "soo"),
        )
        for ((input, expected) in cases) {
            assertEquals(
                "segment(\"$input\", checker: null) must match existing behavior",
                expected,
                SyllableSegmenter.segment(input, wordPrefixChecker = null)
            )
        }
    }

    // MARK: - isValidPrefix (Mode-Aware Trie Validation)

    @Test
    fun testIsValidPrefix_pojMode_rejectsTLOnlyInput() {
        val cases = listOf(
            Triple("ts", false, "TL-only initial (POJ uses ch)"),
            Triple("tsh", false, "TL-only initial (POJ uses chh)"),
            Triple("gua", false, "TL-only final ua (POJ uses oa)"),
            Triple("ing", false, "TL-only final (POJ uses eng)"),
        )
        for ((input, expected, reason) in cases) {
            assertEquals(
                "isValidPrefix(\"$input\", POJ): $reason",
                expected,
                SyllableSegmenter.isValidPrefix(input, ToneConverterModels.InputMode.POJ)
            )
        }
    }

    @Test
    fun testIsValidPrefix_pojMode_acceptsPOJInput() {
        val cases = listOf(
            Triple("ch", true, "POJ initial prefix"),
            Triple("chh", true, "POJ initial prefix"),
            Triple("goa", true, "POJ final oa"),
            Triple("eng", true, "POJ final"),
            Triple("hoo", true, "oo is shared (keyboard shorthand)"),
            Triple("ka", true, "shared syllable"),
            Triple("phang", true, "shared syllable"),
        )
        for ((input, expected, reason) in cases) {
            assertEquals(
                "isValidPrefix(\"$input\", POJ): $reason",
                expected,
                SyllableSegmenter.isValidPrefix(input, ToneConverterModels.InputMode.POJ)
            )
        }
    }

    @Test
    fun testIsValidPrefix_tlMode_rejectsPOJOnlyInput() {
        val cases = listOf(
            Triple("ch", false, "POJ-only initial (TL uses ts)"),
            Triple("chh", false, "POJ-only initial (TL uses tsh)"),
            Triple("goa", false, "POJ-only final oa (TL uses ua)"),
        )
        for ((input, expected, reason) in cases) {
            assertEquals(
                "isValidPrefix(\"$input\", TL): $reason",
                expected,
                SyllableSegmenter.isValidPrefix(input, ToneConverterModels.InputMode.TL)
            )
        }
    }

    @Test
    fun testIsValidPrefix_tlMode_acceptsTLInput() {
        val cases = listOf(
            Triple("ts", true, "TL initial prefix"),
            Triple("tsh", true, "TL initial prefix"),
            Triple("gua", true, "TL final ua"),
            Triple("ua", true, "TL final"),
            Triple("ing", true, "TL final"),
            Triple("eng", true, "TL final (嬰)"),
            Triple("ka", true, "shared syllable"),
        )
        for ((input, expected, reason) in cases) {
            assertEquals(
                "isValidPrefix(\"$input\", TL): $reason",
                expected,
                SyllableSegmenter.isValidPrefix(input, ToneConverterModels.InputMode.TL)
            )
        }
    }

    @Test
    fun testIsValidPrefix_emptyInput() {
        assertTrue(SyllableSegmenter.isValidPrefix("", ToneConverterModels.InputMode.POJ))
        assertTrue(SyllableSegmenter.isValidPrefix("", ToneConverterModels.InputMode.TL))
    }

    // MARK: - Word Grouping

    @Test
    fun testGroupIntoWords_withChecker_groupsKnownWords() {
        val checker: WordPrefixChecker = { key ->
            key.startsWith("kin1a2")
        }
        val syllables = listOf("gua2", "kin", "a2", "jit8")
        val groups = SyllableSegmenter.groupIntoWords(syllables, checker)
        assertEquals(
            "kin+a2+jit8 should be grouped as one word (今仔日)",
            listOf(listOf("gua2"), listOf("kin", "a2", "jit8")),
            groups
        )
    }

    @Test
    fun testGroupIntoWords_withChecker_groupsAma() {
        val checker: WordPrefixChecker = { key ->
            key.startsWith("ama")
        }
        val syllables = listOf("a", "ma")
        val groups = SyllableSegmenter.groupIntoWords(syllables, checker)
        assertEquals(
            "a+ma should be grouped as one word (阿媽)",
            listOf(listOf("a", "ma")),
            groups
        )
    }

    @Test
    fun testGroupIntoWords_withoutChecker_eachSyllableSeparate() {
        val syllables = listOf("gua2", "kin", "a2", "jit8")
        val groups = SyllableSegmenter.groupIntoWords(syllables, null)
        assertEquals(
            "Without checker, each syllable should be its own group",
            listOf(listOf("gua2"), listOf("kin"), listOf("a2"), listOf("jit8")),
            groups
        )
    }

    @Test
    fun testGroupIntoWords_greediestMatch() {
        val checker: WordPrefixChecker = { key ->
            key.startsWith("kin1a2")
        }
        val syllables = listOf("kin", "a2", "jit8")
        val groups = SyllableSegmenter.groupIntoWords(syllables, checker)
        assertEquals(
            "Greedy matching should pick the longest word group",
            listOf(listOf("kin", "a2", "jit8")),
            groups
        )
    }

    // MARK: - Mode-Specific Segmentation

    @Test
    fun testSegment_gua_tlMode_singleSyllable() {
        val result = SyllableSegmenter.segment("gua", mode = ToneConverterModels.InputMode.TL)
        assertEquals(
            "segment(\"gua\", mode=TL) should be a single syllable, not split into gu+a",
            listOf("gua"),
            result
        )
    }

    @Test
    fun testSegment_gua_combinedMode_singleSyllable() {
        val result = SyllableSegmenter.segment("gua")
        assertEquals(
            "segment(\"gua\") should be a single syllable",
            listOf("gua"),
            result
        )
    }

    @Test
    fun testSegment_gua2_tlMode_singleSyllable() {
        val result = SyllableSegmenter.segment("gua2", mode = ToneConverterModels.InputMode.TL)
        assertEquals(
            "segment(\"gua2\", mode=TL) should be a single syllable",
            listOf("gua2"),
            result
        )
    }

    @Test
    fun testSegment_gua_withChecker_singleSyllable() {
        // Checker should not affect "gua" since there's no tie (9 vs 5)
        val alwaysTrueChecker: WordPrefixChecker = { true }
        val result = SyllableSegmenter.segment("gua", wordPrefixChecker = alwaysTrueChecker, mode = ToneConverterModels.InputMode.TL)
        assertEquals(
            "segment(\"gua\", checker=alwaysTrue, mode=TL) should be a single syllable",
            listOf("gua"),
            result
        )
    }

    @Test
    fun testSegment_uaFinalsInTlTrie() {
        // Verify that -ua- finals are properly recognized in TL mode
        val uaSyllables = listOf("gua", "kua", "hua", "bua", "tua", "lua", "sua", "phua", "thua", "khua")
        for (syllable in uaSyllables) {
            val result = SyllableSegmenter.segment(syllable, mode = ToneConverterModels.InputMode.TL)
            assertEquals(
                "segment(\"$syllable\", mode=TL) should be a single syllable",
                listOf(syllable),
                result
            )
        }
    }

    @Test
    fun testIsValidPrefix_gua_tlMode() {
        assertTrue(
            "isValidPrefix(\"gua\", TL) should be true",
            SyllableSegmenter.isValidPrefix("gua", ToneConverterModels.InputMode.TL)
        )
        assertTrue(
            "isValidPrefix(\"gu\", TL) should be true (prefix of gua, gut, etc)",
            SyllableSegmenter.isValidPrefix("gu", ToneConverterModels.InputMode.TL)
        )
    }


    // MARK: - Mode-Specific Segmentation: POJ vs TL differences

    @Test
    fun testSegment_gua_pojMode_splitsBecauseUaNotInPoj() {
        // POJ uses "oa" not "ua", so "gua" is not a valid POJ syllable.
        // This documents why stale inputMode (POJ when user expects TL) causes wrong segmentation.
        val result = SyllableSegmenter.segment("gua", mode = ToneConverterModels.InputMode.POJ)
        assertEquals(
            "segment(\"gua\", mode=POJ): 'ua' is not a POJ final, should split into gu+a",
            listOf("gu", "a"),
            result
        )
    }

    @Test
    fun testSegment_goa_pojMode_singleSyllable() {
        // POJ equivalent of TL "gua" is "goa"
        val result = SyllableSegmenter.segment("goa", mode = ToneConverterModels.InputMode.POJ)
        assertEquals(
            "segment(\"goa\", mode=POJ) should be a single syllable",
            listOf("goa"),
            result
        )
    }
}
