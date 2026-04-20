package com.siansiansu.taigikeyboard.ime.dictionary

import org.junit.Assert.assertEquals
import org.junit.Test

/** Pure-derivation tests for the custom-dictionary column helpers. */
class CustomDictionaryServiceTest {
    // MARK: - generateNotone

    private val notoneCases =
        listOf(
            // POJ nasal ⁿ (U+207F) → nn
            "saⁿ" to "sann",
            "phiaⁿ" to "phiann",
            "saⁿ-á" to "sanna",
            // Uppercase nasal ᴺ (U+1D3A) → nn
            "saᴺ" to "sann",
            // Baseline: tone marks stripped (base letter remains)
            "hó" to "ho",
            "gâu-tsá" to "gautsa",
            "lí hó" to "liho",
            // Baseline: digits stripped
            "ho2" to "ho",
            "gau5-tsa2" to "gautsa",
            // Mixed: nasal + digits
            "saⁿ2" to "sann",
        )

    @Test
    fun testGenerateNotone() {
        notoneCases.forEach { (input, expected) ->
            assertEquals(
                "generateNotone(\"$input\") should be \"$expected\"",
                expected,
                CustomDictionaryDerivation.generateNotone(input),
            )
        }
    }

    // MARK: - generateAbbrev

    private val abbrevCases =
        listOf(
            "gâu-tsá" to "gt",
            "saⁿ-á" to "sa",
            "lí hó" to "lh",
            // Single syllable → empty
            "saⁿ" to "",
            "hó" to "",
        )

    @Test
    fun testGenerateAbbrev() {
        abbrevCases.forEach { (input, expected) ->
            assertEquals(
                "generateAbbrev(\"$input\") should be \"$expected\"",
                expected,
                CustomDictionaryDerivation.generateAbbrev(input),
            )
        }
    }

    // MARK: - Phase 0 §10 — custom-dictionary derivation invariants.
    // Labels match `docs/architecture/behavioral-invariants.md` §10.

    /**
     * Every hyphen-separated syllable contributes exactly one character to
     * `generateAbbrev`. Single-syllable input returns an empty abbrev per
     * the `syllables.size < 2` guard. Wraps existing `testGenerateAbbrev`
     * substance under the invariant label.
     */
    @Test
    fun test_INVARIANT_abbrev_key_is_one_char_per_syllable() {
        val multiSyllableFixtures =
            listOf(
                "gâu-tsá" to 2,
                "tâi-gí-bûn" to 3,
                "a-b-c-d" to 4,
                "lí hó" to 2, // space-separated counts too
            )
        for ((input, syllableCount) in multiSyllableFixtures) {
            val abbrev = CustomDictionaryDerivation.generateAbbrev(input)
            assertEquals(
                "abbrev length must equal syllable count for '$input'",
                syllableCount,
                abbrev.length,
            )
        }
        // Single-syllable input must return empty abbrev (documented contract).
        assertEquals(
            "single-syllable abbrev is empty",
            "",
            CustomDictionaryDerivation.generateAbbrev("hó"),
        )
        // Empty input is a fixed point.
        assertEquals(
            "empty abbrev on empty input",
            "",
            CustomDictionaryDerivation.generateAbbrev(""),
        )
    }

    /**
     * `CustomDictionaryDerivation.generateRomanNum` delegates to
     * `InputNormalizer.normalize(..., InputMode.TL)`. The delegation is the
     * Phase 0 §10 guarantee — search keys land in the trie using the same
     * pipeline that normalizes user input, so custom-dictionary entries
     * surface at the same keystroke the original word would.
     */
    @Test
    fun test_INVARIANT_custom_derivation_matches_input_normalizer() {
        val fixtures =
            listOf(
                "", // empty → empty both sides
                "hó", // single syllable with tone mark
                "gâu-tsá", // hyphen + tone marks
                "tâi-gí-bûn", // three syllables
                "lí hó", // whitespace separator
                "saⁿ", // POJ nasal
                "hō͘", // POJ o͘
            )
        for (input in fixtures) {
            assertEquals(
                "generateRomanNum must equal InputNormalizer.normalize(..., TL) for '$input'",
                InputNormalizer.normalize(input, ToneConverterModels.InputMode.TL),
                CustomDictionaryDerivation.generateRomanNum(input),
            )
        }
    }
}
