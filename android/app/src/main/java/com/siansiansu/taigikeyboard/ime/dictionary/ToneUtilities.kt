// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode

/**
 * Tone letter case conversion utilities
 *
 * Provides uppercasing, lowercasing, and nasal marker case adjustment
 * for POJ/TL romanization characters with combining tone marks.
 *
 * Corresponds to iOS ToneUtilities.swift
 */
object ToneUtilities {
    /**
     * Convert tone letter to uppercase based on input mode
     * For multi-character strings (e.g., "ph", "tsh"), only capitalize the first letter
     * Used for sentence case (auto-capitalization)
     */
    fun uppercaseToneLetter(
        char: String,
        mode: InputMode,
    ): String = uppercaseInternal(char, mode, allChars = false)

    /**
     * Convert string to fully uppercase (for Caps Lock mode)
     * All characters are uppercased, e.g., "tsh" → "TSH"
     */
    fun fullUppercaseToneLetter(
        char: String,
        mode: InputMode,
    ): String = uppercaseInternal(char, mode, allChars = true)

    private fun uppercaseInternal(
        char: String,
        mode: InputMode,
        allChars: Boolean,
    ): String {
        // Nasal marker: ⁿ → ᴺ
        if (char == "\u207F") return "\u1D3A"

        val mapping =
            when (mode) {
                InputMode.POJ -> ToneConverterModels.pojLowercaseToUppercaseMapping
                InputMode.TL -> ToneConverterModels.tlLowercaseToUppercaseMapping
                InputMode.ENGLISH -> null
            }
        // Check if there's a direct mapping first
        mapping?.get(char)?.let { return it }

        return if (allChars) {
            char.uppercase()
        } else if (char.length > 1) {
            char.replaceFirstChar { it.uppercaseChar() }
        } else {
            char.uppercase()
        }
    }

    /**
     * Convert tone letter to lowercase based on input mode
     */
    fun lowercaseToneLetter(
        char: String,
        mode: InputMode,
    ): String {
        // Nasal marker: ᴺ → ⁿ
        if (char == "\u1D3A") return "\u207F"

        val mapping =
            when (mode) {
                InputMode.POJ -> ToneConverterModels.pojUppercaseToLowercaseMapping
                InputMode.TL -> ToneConverterModels.tlUppercaseToLowercaseMapping
                InputMode.ENGLISH -> null
            }
        return mapping?.get(char) ?: char.lowercase()
    }

    /**
     * Adjust nasal marker (ⁿ/ᴺ) case to match the preceding letter's case.
     * Rule: ⁿ follows lowercase letters, ᴺ follows uppercase letters.
     *
     * Ported from iOS ToneUtilities.adjustNasalMarkerCase().
     */
    fun adjustNasalMarkerCase(text: String): String {
        val nasalLower = '\u207F' // ⁿ
        val nasalUpper = '\u1D3A' // ᴺ

        // Early exit: skip iteration if no nasal markers present
        if (nasalLower !in text && nasalUpper !in text) return text

        val result = StringBuilder()
        var lastLetterIsUppercase = false

        for (char in text) {
            if (char == nasalLower || char == nasalUpper) {
                result.append(if (lastLetterIsUppercase) nasalUpper else nasalLower)
            } else {
                if (char.isLetter()) {
                    lastLetterIsUppercase = char.isUpperCase()
                }
                result.append(char)
            }
        }

        return result.toString()
    }
}
