package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode

/**
 * Utility object for generating tone character data for keyboard popups
 * Extracts tone characters from ToneConverterModels in proper order (2,3,5,6,7,8,9)
 */
object ToneCharacterUtils {

    /**
     * Tone order for popups (skipping tone 1 and 4 which have no marks)
     */
    private val TONE_ORDER = listOf(2, 3, 5, 6, 7, 8, 9)

    /**
     * Get tone characters for a base character in specified mode
     * @param baseChar Base character (e.g., "a", "e", "i")
     * @param mode POJ or TL mode
     * @return List of tone-marked characters in order
     */
    fun getToneCharacters(baseChar: String, mode: InputMode): List<String> {
        val mapping = when (mode) {
            InputMode.POJ -> ToneConverterModels.pojNumberToToneMapping
            InputMode.TL -> ToneConverterModels.tlNumberToToneMapping
            InputMode.ENGLISH -> return emptyList()
        }

        return TONE_ORDER.mapNotNull { tone ->
            val key = "$baseChar$tone"
            mapping[key]
        }
    }

    /**
     * Get all vowel base characters that have tone variations
     */
    fun getBaseVowels(mode: InputMode): List<String> {
        return when (mode) {
            InputMode.POJ -> listOf("a", "e", "i", "o", "o͘", "u", "n", "m")
            InputMode.TL -> listOf("a", "e", "i", "o", "oo", "u", "n", "m")
            InputMode.ENGLISH -> emptyList()
        }
    }

    /**
     * Generate popup data structure for JSON export
     * @param mode POJ or TL mode
     * @return Map of base character to list of tone variations
     */
    fun generatePopupData(mode: InputMode): Map<String, List<ToneCharacterData>> {
        val result = mutableMapOf<String, List<ToneCharacterData>>()
        val baseVowels = getBaseVowels(mode)

        for (baseChar in baseVowels) {
            val toneChars = getToneCharacters(baseChar, mode)
            if (toneChars.isNotEmpty()) {
                result[baseChar] = toneChars.map { char ->
                    ToneCharacterData(
                        code = getUnicodeCodePoint(char),
                        label = char
                    )
                }
            }

            // Also add uppercase variants
            val baseCharUpper = baseChar.uppercase()
            val toneCharsUpper = getToneCharacters(baseCharUpper, mode)
            if (toneCharsUpper.isNotEmpty()) {
                result[baseCharUpper] = toneCharsUpper.map { char ->
                    ToneCharacterData(
                        code = getUnicodeCodePoint(char),
                        label = char
                    )
                }
            }
        }

        // Special handling for "ng" (multi-character base)
        val ngTones = getToneCharacters("ng", mode)
        if (ngTones.isNotEmpty()) {
            result["ng"] = ngTones.map { char ->
                ToneCharacterData(
                    code = getUnicodeCodePoint(char),
                    label = char
                )
            }
        }

        val NgTones = getToneCharacters("Ng", mode)
        if (NgTones.isNotEmpty()) {
            result["Ng"] = NgTones.map { char ->
                ToneCharacterData(
                    code = getUnicodeCodePoint(char),
                    label = char
                )
            }
        }

        return result
    }

    /**
     * Get Unicode code point for a character or character sequence
     * For combining characters, returns the code point of the first character
     */
    private fun getUnicodeCodePoint(char: String): Int {
        return if (char.isNotEmpty()) {
            char.codePointAt(0)
        } else {
            0
        }
    }

    /**
     * Data class for tone character entry
     */
    data class ToneCharacterData(
        val code: Int,
        val label: String
    )
}
