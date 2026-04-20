package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Test

/**
 * SuggestionCaseTransformer unit tests
 *
 * Ported from iOS CaseTransformationServiceTests.swift
 * Tests candidate case transformation based on keyboard state (caps/capsLock)
 * and composing text.
 */
class SuggestionCaseTransformerTest {
    private fun word(
        id: Int,
        roman: String,
        hanzi: String? = null,
    ): TaigiWord = TaigiWord(id = id, roman = roman, hanzi = hanzi, lengthScore = null)

    // MARK: - Caps Lock (all uppercase)

    @Test
    fun testTransform_capsLock_allUppercase_POJ() {
        val words = listOf(word(1, "tâi-gí", "台語"))
        val result = SuggestionCaseTransformer.transform(words, "tai", caps = false, capsLock = true, inputMode = InputMode.POJ)
        assertEquals("CapsLock should produce all uppercase", "TÂI-GÍ", result[0].roman)
    }

    @Test
    fun testTransform_capsLock_allUppercase_TL() {
        val words = listOf(word(1, "tâi-gí", "台語"))
        val result = SuggestionCaseTransformer.transform(words, "tai", caps = false, capsLock = true, inputMode = InputMode.TL)
        assertEquals("CapsLock TL should produce all uppercase", "TÂI-GÍ", result[0].roman)
    }

    // MARK: - Caps (Shift) — capitalize next letter

    @Test
    fun testTransform_caps_capitalizeNextLetter() {
        val words = listOf(word(1, "tâi-gí", "台語"))
        val result = SuggestionCaseTransformer.transform(words, "Tai", caps = true, capsLock = false, inputMode = InputMode.POJ)
        // Typed "Tai" -> first 3 letters match case "Tâi", caps=true -> next letter uppercase "G", rest lowercase "í"
        assertEquals("Caps should capitalize next letter after typed portion", "Tâi-Gí", result[0].roman)
    }

    @Test
    fun testTransform_caps_singleLetterRemaining() {
        val words = listOf(word(1, "hó", "好"))
        val result = SuggestionCaseTransformer.transform(words, "h", caps = true, capsLock = false, inputMode = InputMode.POJ)
        assertEquals("Caps with 1 remaining letter", "hÓ", result[0].roman)
    }

    // MARK: - No caps (lowercase remainder)

    @Test
    fun testTransform_noCaps_lowercaseRemainder() {
        val words = listOf(word(1, "tâi-gí", "台語"))
        val result = SuggestionCaseTransformer.transform(words, "Tai", caps = false, capsLock = false, inputMode = InputMode.POJ)
        // Typed "Tai" -> "Tâi", no caps -> remainder lowercase "-gí"
        assertEquals("No caps should keep remainder lowercase", "Tâi-gí", result[0].roman)
    }

    // MARK: - matchCase (preserve typed case)

    @Test
    fun testTransform_matchCase_preservesTypedCase() {
        val words = listOf(word(1, "tâi-gí", "台語"))
        val result = SuggestionCaseTransformer.transform(words, "Tai", caps = false, capsLock = false, inputMode = InputMode.POJ)
        assertEquals("Typed 'Tai' should produce 'Tâi-gí'", "Tâi-gí", result[0].roman)
    }

    @Test
    fun testTransform_matchCase_allTyped() {
        val words = listOf(word(1, "hó", "好"))
        val result = SuggestionCaseTransformer.transform(words, "HO2", caps = false, capsLock = false, inputMode = InputMode.POJ)
        // composingText "HO2" has 2 letters -> candidate "hó" has 1 letter (h + combining ó)
        // typedLetterCount(2) >= originalLetterCount -> matchCase whole candidate
        assertEquals("All typed should match entire candidate case", "HÓ", result[0].roman)
    }

    // MARK: - Empty composingText

    @Test
    fun testTransform_emptyComposingText_unchanged() {
        val words = listOf(word(1, "tâi-gí", "台語"))
        val result = SuggestionCaseTransformer.transform(words, "", caps = false, capsLock = false, inputMode = InputMode.POJ)
        assertEquals("Empty composing text should return unchanged", "tâi-gí", result[0].roman)
    }

    // MARK: - ID-based skip/transform logic

    @Test
    fun testTransform_skipNegativeId() {
        // NextWord entries (id < 0, id != -2) should skip transform
        val words = listOf(word(-1, "tâi-gí"))
        val result = SuggestionCaseTransformer.transform(words, "TAI", caps = false, capsLock = true, inputMode = InputMode.POJ)
        assertEquals("Negative id (NextWord) should skip transform", "tâi-gí", result[0].roman)
    }

    @Test
    fun testTransform_customDictionaryId_notSkipped() {
        // Custom dictionary entries (id == -2) should still be transformed
        val words = listOf(word(-2, "tâi-gí"))
        val result = SuggestionCaseTransformer.transform(words, "tai", caps = false, capsLock = true, inputMode = InputMode.POJ)
        assertEquals("Custom dictionary id (-2) should transform", "TÂI-GÍ", result[0].roman)
    }

    @Test
    fun testTransform_composingTextId_unchanged() {
        // Composing text candidate (id == 0) should not be transformed
        val words = listOf(word(0, "tai"))
        val result = SuggestionCaseTransformer.transform(words, "TAI", caps = false, capsLock = true, inputMode = InputMode.POJ)
        assertEquals("id==0 (composing text) should not transform", "tai", result[0].roman)
    }

    // MARK: - Tone letter case

    @Test
    fun testTransform_toneLetterCase_POJ() {
        val words = listOf(word(1, "ô-pêh-sai"))
        val result = SuggestionCaseTransformer.transform(words, "O", caps = false, capsLock = false, inputMode = InputMode.POJ)
        assertEquals("POJ tone letter capitalization", "Ô-pêh-sai", result[0].roman)
    }

    @Test
    fun testTransform_toneLetterCase_TL() {
        val words = listOf(word(1, "ôo-peh-sai"))
        val result = SuggestionCaseTransformer.transform(words, "O", caps = false, capsLock = false, inputMode = InputMode.TL)
        assertEquals("TL tone letter capitalization", "Ôo-peh-sai", result[0].roman)
    }

    // MARK: - Nasal marker case adjustment

    @Test
    fun testTransform_nasalMarkerCase_adjusted() {
        // Uppercase nasal ᴺ should follow uppercase vowel
        val words = listOf(word(1, "pêⁿ"))
        val result = SuggestionCaseTransformer.transform(words, "PE", caps = false, capsLock = true, inputMode = InputMode.POJ)
        // CapsLock -> all uppercase -> PÊᴺ (nasal adjusted to uppercase)
        assertEquals("Nasal marker should match case", "PÊ\u1D3A", result[0].roman)
    }

    // MARK: - Multiple words

    @Test
    fun testTransform_multipleWords_allTransformed() {
        val words =
            listOf(
                word(1, "tâi-gí", "台語"),
                word(2, "hó", "好"),
                word(3, "lí", "你"),
            )
        val result = SuggestionCaseTransformer.transform(words, "tai", caps = false, capsLock = true, inputMode = InputMode.POJ)
        assertEquals("Word 1 uppercased", "TÂI-GÍ", result[0].roman)
        assertEquals("Word 2 uppercased", "HÓ", result[1].roman)
        assertEquals("Word 3 uppercased", "LÍ", result[2].roman)
    }

    // MARK: - splitByLetterCount edge cases

    @Test
    fun testTransform_digitsNotCountedAsLetters() {
        // composingText "ka2" has 2 letters (k, a), digit 2 is not counted
        val words = listOf(word(1, "ká", "腳"))
        val result = SuggestionCaseTransformer.transform(words, "Ka2", caps = false, capsLock = false, inputMode = InputMode.POJ)
        // 2 typed letters >= candidate letters -> matchCase whole thing
        assertEquals("Digits not counted as letters", "Ká", result[0].roman)
    }

    @Test
    fun testTransform_emptyRoman_unchanged() {
        val words = listOf(word(1, ""))
        val result = SuggestionCaseTransformer.transform(words, "tai", caps = false, capsLock = true, inputMode = InputMode.POJ)
        assertEquals("Empty roman should stay empty", "", result[0].roman)
    }

    // MARK: - Phase 0 §9 — case transformation invariants.
    // Labels match `docs/architecture/behavioral-invariants.md` §9.

    /**
     * Given the same inputs, `SuggestionCaseTransformer.transform` returns
     * byte-identical output across invocations. The function reads no
     * external state (no clock, no settings singleton) — callers forward
     * `caps`, `capsLock`, and `inputMode` explicitly.
     */
    @Test
    fun test_INVARIANT_case_transformer_is_deterministic() {
        val words = listOf(word(1, "tâi-gí", "台語"))
        // `InputMode` has only `POJ`, `TL`, and `ENGLISH` on Android —
        // cross the flag matrix across all three + a couple of composing
        // variants to pin determinism without mode-specific branching.
        val fixtures =
            listOf(
                Triple(false, false, InputMode.POJ),
                Triple(true, false, InputMode.POJ),
                Triple(false, true, InputMode.POJ),
                Triple(false, false, InputMode.TL),
                Triple(true, false, InputMode.TL),
                Triple(false, true, InputMode.TL),
                Triple(false, false, InputMode.ENGLISH),
            )
        for ((caps, capsLock, mode) in fixtures) {
            val first = SuggestionCaseTransformer.transform(words, "Tai", caps, capsLock, mode)
            val second = SuggestionCaseTransformer.transform(words, "Tai", caps, capsLock, mode)
            assertEquals(
                "Same inputs must produce same output for (caps=$caps, capsLock=$capsLock, mode=$mode)",
                first.map { it.roman },
                second.map { it.roman },
            )
        }
    }

    /**
     * `caps` toggles uppercase behavior; flipping the flag must change
     * the output. The transformer reads ONLY the forwarded flag — never
     * a global setting. We assert inequality (flag is observed) rather
     * than pinning exact forms, because the match-case logic depends on
     * the typed composing text's case and matching tests already pin
     * the concrete transformations.
     */
    @Test
    fun test_INVARIANT_case_transformer_honors_auto_cap_flag() {
        val words = listOf(word(1, "tâi-gí", "台語"))

        val withoutCaps =
            SuggestionCaseTransformer.transform(
                words,
                "T",
                caps = false,
                capsLock = false,
                inputMode = InputMode.POJ,
            )
        val withCaps =
            SuggestionCaseTransformer.transform(
                words,
                "T",
                caps = true,
                capsLock = false,
                inputMode = InputMode.POJ,
            )
        assertNotEquals(
            "flipping caps flag must change output when composing starts with a letter",
            withoutCaps[0].roman,
            withCaps[0].roman,
        )

        // CapsLock overrides caps — both caps settings produce the same
        // output when capsLock is true. Pins that capsLock is the stronger
        // flag (observable by callers).
        val lockedA = SuggestionCaseTransformer.transform(words, "T", false, true, InputMode.POJ)
        val lockedB = SuggestionCaseTransformer.transform(words, "T", true, true, InputMode.POJ)
        assertEquals(
            "capsLock dominates caps flag",
            lockedA[0].roman,
            lockedB[0].roman,
        )
    }
}
