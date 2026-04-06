package com.siansiansu.taigikeyboard.ime.dictionary

import org.junit.Assert.assertEquals
import org.junit.Test

class CustomDictionaryServiceTest {

    // MARK: - generateNotone

    private val notoneCases = listOf(
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
                CustomDictionaryService.generateNotone(input)
            )
        }
    }

    // MARK: - generateAbbrev

    private val abbrevCases = listOf(
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
                CustomDictionaryService.generateAbbrev(input)
            )
        }
    }
}
