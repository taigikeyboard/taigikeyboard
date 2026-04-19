// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import java.text.Normalizer

/**
 * Pure derivation functions for custom-dictionary columns. Kept out of
 * `CustomDictionaryService` so scoring/search paths can depend on these
 * helpers without touching SQLite or Android context. Mirrors iOS
 * `Lexicon/Database/CustomDictionaryDerivation.swift`.
 */
object CustomDictionaryDerivation {
    /**
     * Toneless form used for toneless prefix search.
     * Strips tone diacritics (via NFD), trailing digits, hyphens, and spaces.
     */
    fun generateNotone(roman: String): String {
        val withNasalConverted =
            roman
                .lowercase()
                .replace("\u207F", "nn")
                .replace("\u1D3A", "nn")
        val decomposed = Normalizer.normalize(withNasalConverted, Normalizer.Form.NFD)
        return buildString {
            for (cp in decomposed.codePoints().toArray()) {
                if (Character.getType(cp) == Character.NON_SPACING_MARK.toInt()) continue
                if (cp in '0'.code..'9'.code) continue
                if (cp == '-'.code || cp == ' '.code) continue
                appendCodePoint(cp)
            }
        }.let { Normalizer.normalize(it, Normalizer.Form.NFC) }
    }

    /**
     * Abbreviation: first letter of each syllable (split by `-` or space)
     * with diacritics stripped. Returns empty string for single-syllable input.
     */
    fun generateAbbrev(roman: String): String {
        val syllables = roman.lowercase().split(Regex("[-\\s]+")).filter { it.isNotEmpty() }
        if (syllables.size < 2) return ""
        return syllables.joinToString("") { syllable ->
            val firstChar = syllable.first().toString()
            val decomposed = Normalizer.normalize(firstChar, Normalizer.Form.NFD)
            buildString {
                decomposed.codePoints().forEach { cp ->
                    if (Character.getType(cp) != Character.NON_SPACING_MARK.toInt()) {
                        appendCodePoint(cp)
                    }
                }
            }.let { Normalizer.normalize(it, Normalizer.Form.NFC) }
        }
    }

    /**
     * Numeric-toned form (e.g. "gâu-tsá" → "gau5tsa2") for tone-aware search.
     * Delegates to [InputNormalizer.normalize] in TL mode so the tone-digit
     * rules live in one place.
     */
    fun generateRomanNum(roman: String): String = InputNormalizer.normalize(roman, InputMode.TL)
}
