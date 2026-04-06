package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * ToneConverterModels unit tests
 *
 * Ported from iOS ToneUtilitiesTests.swift and CaseTransformationServiceTests.swift
 * Tests uppercaseToneLetter, lowercaseToneLetter, adjustNasalMarkerCase, and isHanzi.
 */
class ToneConverterModelsTest {
    // MARK: - uppercaseToneLetter

    @Test
    fun testUppercaseToneLetter_nasalN() {
        // ⁿ (U+207F) -> ᴺ (U+1D3A)
        assertEquals("\u1D3A", ToneUtilities.uppercaseToneLetter("\u207F", InputMode.POJ))
    }

    @Test
    fun testUppercaseToneLetter_regularLetter() {
        assertEquals("uppercase a", "A", ToneUtilities.uppercaseToneLetter("a", InputMode.TL))
        assertEquals("uppercase k", "K", ToneUtilities.uppercaseToneLetter("k", InputMode.TL))
    }

    @Test
    fun testUppercaseToneLetter_toneMarkedLetter() {
        // á -> Á
        val result = ToneUtilities.uppercaseToneLetter("\u00E1", InputMode.TL)
        assertEquals("uppercase á", "\u00C1", result)
    }

    @Test
    fun testUppercaseToneLetter_alreadyUppercase() {
        assertEquals("uppercase A", "A", ToneUtilities.uppercaseToneLetter("A", InputMode.TL))
    }

    // MARK: - lowercaseToneLetter

    @Test
    fun testLowercaseToneLetter_nasalN() {
        // ᴺ (U+1D3A) -> ⁿ (U+207F)
        assertEquals("\u207F", ToneUtilities.lowercaseToneLetter("\u1D3A", InputMode.POJ))
    }

    @Test
    fun testLowercaseToneLetter_regularLetter() {
        assertEquals("lowercase A", "a", ToneUtilities.lowercaseToneLetter("A", InputMode.TL))
        assertEquals("lowercase K", "k", ToneUtilities.lowercaseToneLetter("K", InputMode.TL))
    }

    @Test
    fun testLowercaseToneLetter_toneMarkedLetter() {
        // Á -> á
        val result = ToneUtilities.lowercaseToneLetter("\u00C1", InputMode.TL)
        assertEquals("lowercase Á", "\u00E1", result)
    }

    @Test
    fun testLowercaseToneLetter_alreadyLowercase() {
        assertEquals("lowercase a", "a", ToneUtilities.lowercaseToneLetter("a", InputMode.TL))
    }

    // MARK: - adjustNasalMarkerCase

    @Test
    fun testAdjustNasalMarkerCase_lowercaseVowel_unchanged() {
        // pêⁿ — lowercase vowel → ⁿ stays
        val result = ToneUtilities.adjustNasalMarkerCase("p\u00EA\u207F")
        assertEquals("Lowercase vowel should keep ⁿ", "p\u00EA\u207F", result)
    }

    @Test
    fun testAdjustNasalMarkerCase_uppercaseVowel_becomesUpperNasal() {
        // PÊⁿ → PÊᴺ
        val result = ToneUtilities.adjustNasalMarkerCase("P\u00CA\u207F")
        assertEquals("Uppercase vowel should produce ᴺ", "P\u00CA\u1D3A", result)
    }

    @Test
    fun testAdjustNasalMarkerCase_alreadyCorrectUpper() {
        // PÊᴺ already correct
        val result = ToneUtilities.adjustNasalMarkerCase("P\u00CA\u1D3A")
        assertEquals("Already correct uppercase nasal should stay", "P\u00CA\u1D3A", result)
    }

    @Test
    fun testAdjustNasalMarkerCase_wrongCaseUpper_becomesLower() {
        // Pêᴺ → Pêⁿ (ê is lowercase, so nasal should be ⁿ)
        val result = ToneUtilities.adjustNasalMarkerCase("P\u00EA\u1D3A")
        assertEquals("Lowercase vowel before ᴺ should fix to ⁿ", "P\u00EA\u207F", result)
    }

    @Test
    fun testAdjustNasalMarkerCase_hyphenatedText() {
        // pêⁿ-á stays unchanged
        val result = ToneUtilities.adjustNasalMarkerCase("p\u00EA\u207F-\u00E1")
        assertEquals("Hyphen should not affect nasal case", "p\u00EA\u207F-\u00E1", result)
    }

    @Test
    fun testAdjustNasalMarkerCase_noPrecedingLetter_defaultsToLower() {
        // No preceding letter → default ⁿ
        val result = ToneUtilities.adjustNasalMarkerCase("\u1D3A")
        assertEquals("No preceding letter should default to ⁿ", "\u207F", result)
    }

    // MARK: - isHanzi

    @Test
    fun testIsHanzi_cjkMainRange() {
        assertTrue("isHanzi(\"台\")", ToneConverterModels.isHanzi("台"))
        assertTrue("isHanzi(\"語\")", ToneConverterModels.isHanzi("語"))
        assertTrue("isHanzi(\"人\")", ToneConverterModels.isHanzi("人"))
    }

    @Test
    fun testIsHanzi_extensionB() {
        assertTrue("isHanzi(U+20000) - CJK Extension B", ToneConverterModels.isHanzi("\uD840\uDC00"))
    }

    @Test
    fun testIsHanzi_latinOnly() {
        assertFalse("isHanzi(\"hello\")", ToneConverterModels.isHanzi("hello"))
        assertFalse("isHanzi(\"abc\")", ToneConverterModels.isHanzi("abc"))
    }

    @Test
    fun testIsHanzi_empty() {
        assertFalse("isHanzi(\"\")", ToneConverterModels.isHanzi(""))
    }

    @Test
    fun testIsHanzi_mixed() {
        assertTrue("isHanzi(\"hello台語\")", ToneConverterModels.isHanzi("hello台語"))
    }

    @Test
    fun testIsHanzi_digits() {
        assertFalse("isHanzi(\"12345\")", ToneConverterModels.isHanzi("12345"))
    }

    @Test
    fun testIsHanzi_bopomofo() {
        assertFalse("isHanzi(\"ㄅ\")", ToneConverterModels.isHanzi("ㄅ"))
        assertFalse("isHanzi(\"ㄆ\")", ToneConverterModels.isHanzi("ㄆ"))
    }

    // MARK: - POJ Tone Letter Mappings (uppercase)

    @Test
    fun testPOJ_uppercaseToneLetters() {
        val testCases =
            listOf(
                // a
                "á" to "Á",
                "à" to "À",
                "â" to "Â",
                "ǎ" to "Ǎ",
                "ā" to "Ā",
                "a̍" to "A̍",
                "ă" to "Ă",
                // e
                "é" to "É",
                "è" to "È",
                "ê" to "Ê",
                "ě" to "Ě",
                "ē" to "Ē",
                "e̍" to "E̍",
                "ĕ" to "Ĕ",
                // i
                "í" to "Í",
                "ì" to "Ì",
                "î" to "Î",
                "ǐ" to "Ǐ",
                "ī" to "Ī",
                "i̍" to "I̍",
                "ĭ" to "Ĭ",
                // o
                "ó" to "Ó",
                "ò" to "Ò",
                "ô" to "Ô",
                "ǒ" to "Ǒ",
                "ō" to "Ō",
                "o̍" to "O̍",
                "ŏ" to "Ŏ",
                // u
                "ú" to "Ú",
                "ù" to "Ù",
                "û" to "Û",
                "ǔ" to "Ǔ",
                "ū" to "Ū",
                "u̍" to "U̍",
                "ŭ" to "Ŭ",
                // n
                "ń" to "Ń",
                "ǹ" to "Ǹ",
                "n̂" to "N̂",
                "ň" to "Ň",
                "n̄" to "N̄",
                "n̍" to "N̍",
                "n̋" to "N̋",
                // m
                "ḿ" to "Ḿ",
                "m̀" to "M̀",
                "m̂" to "M̂",
                "m̌" to "M̌",
                "m̄" to "M̄",
                "m̍" to "M̍",
                "m̋" to "M̋",
            )

        for ((input, expected) in testCases) {
            val result = ToneUtilities.uppercaseToneLetter(input, InputMode.POJ)
            assertEquals("POJ uppercase: $input should be $expected", expected, result)
        }
    }

    // MARK: - POJ Tone Letter Mappings (lowercase)

    @Test
    fun testPOJ_lowercaseToneLetters() {
        val testCases =
            listOf(
                // A
                "Á" to "á",
                "À" to "à",
                "Â" to "â",
                "Ǎ" to "ǎ",
                "Ā" to "ā",
                "A̍" to "a̍",
                "Ă" to "ă",
                // E
                "É" to "é",
                "È" to "è",
                "Ê" to "ê",
                "Ě" to "ě",
                "Ē" to "ē",
                "E̍" to "e̍",
                "Ĕ" to "ĕ",
                // I
                "Í" to "í",
                "Ì" to "ì",
                "Î" to "î",
                "Ǐ" to "ǐ",
                "Ī" to "ī",
                "I̍" to "i̍",
                "Ĭ" to "ĭ",
                // O
                "Ó" to "ó",
                "Ò" to "ò",
                "Ô" to "ô",
                "Ǒ" to "ǒ",
                "Ō" to "ō",
                "O̍" to "o̍",
                "Ŏ" to "ŏ",
                // U
                "Ú" to "ú",
                "Ù" to "ù",
                "Û" to "û",
                "Ǔ" to "ǔ",
                "Ū" to "ū",
                "U̍" to "u̍",
                "Ŭ" to "ŭ",
                // N
                "Ń" to "ń",
                "Ǹ" to "ǹ",
                "N̂" to "n̂",
                "Ň" to "ň",
                "N̄" to "n̄",
                "N̍" to "n̍",
                "N̋" to "n̋",
                // M
                "Ḿ" to "ḿ",
                "M̀" to "m̀",
                "M̂" to "m̂",
                "M̌" to "m̌",
                "M̄" to "m̄",
                "M̍" to "m̍",
                "M̋" to "m̋",
            )

        for ((input, expected) in testCases) {
            val result = ToneUtilities.lowercaseToneLetter(input, InputMode.POJ)
            assertEquals("POJ lowercase: $input should be $expected", expected, result)
        }
    }

    // MARK: - TL Tone Letter Mappings (uppercase)

    @Test
    fun testTL_uppercaseToneLetters() {
        val testCases =
            listOf(
                // TL tone 9
                "a̋" to "A̋",
                "e̋" to "E̋",
                "i̋" to "I̋",
                "ő" to "Ő",
                "ű" to "Ű",
                // oo (TL specific)
                "óo" to "Óo",
                "òo" to "Òo",
                "ôo" to "Ôo",
                "ǒo" to "Ǒo",
                "ōo" to "Ōo",
                "o̍o" to "O̍o",
                "őo" to "Őo",
            )

        for ((input, expected) in testCases) {
            val result = ToneUtilities.uppercaseToneLetter(input, InputMode.TL)
            assertEquals("TL uppercase: $input should be $expected", expected, result)
        }
    }

    // MARK: - TL Tone Letter Mappings (lowercase)

    @Test
    fun testTL_lowercaseToneLetters() {
        val testCases =
            listOf(
                // TL tone 9
                "A̋" to "a̋",
                "E̋" to "e̋",
                "I̋" to "i̋",
                "Ő" to "ő",
                "Ű" to "ű",
                // oo (TL specific)
                "Óo" to "óo",
                "Òo" to "òo",
                "Ôo" to "ôo",
                "Ǒo" to "ǒo",
                "Ōo" to "ōo",
                "O̍o" to "o̍o",
                "Őo" to "őo",
            )

        for ((input, expected) in testCases) {
            val result = ToneUtilities.lowercaseToneLetter(input, InputMode.TL)
            assertEquals("TL lowercase: $input should be $expected", expected, result)
        }
    }

    // MARK: - POJ o͘ (o with dot) Mappings

    @Test
    fun testPOJ_oDot_uppercase() {
        val testCases =
            listOf(
                "ó͘" to "Ó͘",
                "ò͘" to "Ò͘",
                "ô͘" to "Ô͘",
                "ǒ͘" to "Ǒ͘",
                "ō͘" to "Ō͘",
                "o̍͘" to "O̍͘",
                "ŏ͘" to "Ŏ͘",
            )

        for ((input, expected) in testCases) {
            val result = ToneUtilities.uppercaseToneLetter(input, InputMode.POJ)
            assertEquals("POJ o͘ uppercase: $input should be $expected", expected, result)
        }
    }

    // MARK: - fullUppercaseToneLetter

    @Test
    fun testFullUppercaseToneLetter_multiChar() {
        // fullUppercaseToneLetter uppercases ALL characters, unlike uppercaseToneLetter
        assertEquals("tsh -> TSH", "TSH", ToneUtilities.fullUppercaseToneLetter("tsh", InputMode.TL))
        assertEquals("ph -> PH", "PH", ToneUtilities.fullUppercaseToneLetter("ph", InputMode.TL))
    }

    @Test
    fun testFullUppercaseToneLetter_singleChar() {
        // Single char behaves same as uppercaseToneLetter
        assertEquals("a -> A", "A", ToneUtilities.fullUppercaseToneLetter("a", InputMode.TL))
        assertEquals("á -> Á", "\u00C1", ToneUtilities.fullUppercaseToneLetter("\u00E1", InputMode.TL))
    }

    @Test
    fun testPOJ_oDot_lowercase() {
        val testCases =
            listOf(
                "Ó͘" to "ó͘",
                "Ò͘" to "ò͘",
                "Ô͘" to "ô͘",
                "Ǒ͘" to "ǒ͘",
                "Ō͘" to "ō͘",
                "O̍͘" to "o̍͘",
                "Ŏ͘" to "ŏ͘",
            )

        for ((input, expected) in testCases) {
            val result = ToneUtilities.lowercaseToneLetter(input, InputMode.POJ)
            assertEquals("POJ o͘ lowercase: $input should be $expected", expected, result)
        }
    }
}
