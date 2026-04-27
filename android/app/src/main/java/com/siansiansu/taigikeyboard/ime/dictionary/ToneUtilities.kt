// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.core.settings.InputMode

/**
 * Tone-letter case-conversion utilities for POJ/TL romanization.
 *
 * Mirrors iOS `Input/ToneUtilities.swift`. Handles the `ⁿ` (U+207F) /
 * `ᴺ` (U+1D3A) nasal-marker codepoint pair plus mode-specific tone-letter
 * tables that Kotlin's stdlib `uppercase()` / `lowercase()` would round-trip
 * incorrectly.
 *
 * Restored 2026-04-27 after `D9.4 commit 9` over-deleted the surrounding
 * `ToneConverterModels.kt` (which housed phonetic tables now owned by
 * Rust). The four case-mapping tables are inlined here as `private val`
 * because no other file consumed them.
 */
object ToneUtilities {
    /**
     * Convert tone letter to uppercase based on input mode.
     *
     * For multi-character strings (e.g. "ph", "tsh"), only capitalize the
     * first letter — used for sentence case (auto-capitalization).
     */
    fun uppercaseToneLetter(
        char: String,
        mode: InputMode,
    ): String = uppercaseInternal(char, mode, allChars = false)

    /**
     * Convert string to fully uppercase (Caps Lock mode). All characters
     * uppercased, e.g. "tsh" → "TSH".
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
        if (char == "ⁿ") return "ᴺ"

        val mapping =
            when (mode) {
                InputMode.POJ -> pojLowercaseToUppercaseMapping
                InputMode.TL -> tlLowercaseToUppercaseMapping
                InputMode.ENGLISH -> null
            }
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
     * Convert tone letter to lowercase based on input mode.
     */
    fun lowercaseToneLetter(
        char: String,
        mode: InputMode,
    ): String {
        // Nasal marker: ᴺ → ⁿ
        if (char == "ᴺ") return "ⁿ"

        val mapping =
            when (mode) {
                InputMode.POJ -> pojUppercaseToLowercaseMapping
                InputMode.TL -> tlUppercaseToLowercaseMapping
                InputMode.ENGLISH -> null
            }
        return mapping?.get(char) ?: char.lowercase()
    }

    /**
     * Adjust nasal marker (`ⁿ`/`ᴺ`) case to match the preceding letter.
     * Rule: `ⁿ` follows lowercase letters, `ᴺ` follows uppercase letters.
     *
     * Mirrors iOS `ToneUtilities.adjustNasalMarkerCase`.
     */
    fun adjustNasalMarkerCase(text: String): String {
        val nasalLower = 'ⁿ' // ⁿ
        val nasalUpper = 'ᴺ' // ᴺ

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

    // region Case-mapping tables (private — see file-level KDoc)

    private val pojLowercaseToUppercaseMapping =
        mapOf(
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
            // o͘
            "ó͘" to "Ó͘",
            "ò͘" to "Ò͘",
            "ô͘" to "Ô͘",
            "ǒ͘" to "Ǒ͘",
            "ō͘" to "Ō͘",
            "o̍͘" to "O̍͘",
            "ŏ͘" to "Ŏ͘",
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

    private val tlLowercaseToUppercaseMapping =
        mapOf(
            // a
            "á" to "Á",
            "à" to "À",
            "â" to "Â",
            "ǎ" to "Ǎ",
            "ā" to "Ā",
            "a̍" to "A̍",
            "a̋" to "A̋",
            // e
            "é" to "É",
            "è" to "È",
            "ê" to "Ê",
            "ě" to "Ě",
            "ē" to "Ē",
            "e̍" to "E̍",
            "e̋" to "E̋",
            // i
            "í" to "Í",
            "ì" to "Ì",
            "î" to "Î",
            "ǐ" to "Ǐ",
            "ī" to "Ī",
            "i̍" to "I̍",
            "i̋" to "I̋",
            // o
            "ó" to "Ó",
            "ò" to "Ò",
            "ô" to "Ô",
            "ǒ" to "Ǒ",
            "ō" to "Ō",
            "o̍" to "O̍",
            "ő" to "Ő",
            // oo
            "óo" to "Óo",
            "òo" to "Òo",
            "ôo" to "Ôo",
            "ǒo" to "Ǒo",
            "ōo" to "Ōo",
            "o̍o" to "O̍o",
            "őo" to "Őo",
            // u
            "ú" to "Ú",
            "ù" to "Ù",
            "û" to "Û",
            "ǔ" to "Ǔ",
            "ū" to "Ū",
            "u̍" to "U̍",
            "ű" to "Ű",
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

    private val pojUppercaseToLowercaseMapping: Map<String, String> by lazy {
        pojLowercaseToUppercaseMapping.entries.associate { (k, v) -> v to k }
    }

    private val tlUppercaseToLowercaseMapping: Map<String, String> by lazy {
        tlLowercaseToUppercaseMapping.entries.associate { (k, v) -> v to k }
    }

    // endregion
}
