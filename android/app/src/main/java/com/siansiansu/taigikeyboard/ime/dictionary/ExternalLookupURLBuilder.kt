package com.siansiansu.taigikeyboard.ime.dictionary

import java.net.URLEncoder

/**
 * Builds MOE / Chhoe Taigi external dictionary lookup URLs from TL display form.
 *
 * Parallels iOS `ExternalLookupURLBuilder.swift`. Tone-digit conversion semantics
 * follow the external dictionaries' URL format (tone 1 / tone 4 omitted).
 */
object ExternalLookupURLBuilder {
    // Tone mark → tone digit map derived from TaigiPhonetics (single source of truth)
    private val toneMarkToNumber: Map<Char, String> =
        TaigiPhonetics.combiningToToneNum.mapKeys { (codePoint, _) -> codePoint.toChar() }

    /**
     * Build Chhoe Taigi lookup URL for a TL display string.
     * Returns null if the TL string produces an empty digit form.
     */
    fun chhoeURL(tl: String): String? {
        val tlDigit = toTLDigit(tl)
        if (tlDigit.isEmpty()) return null
        val encoded = URLEncoder.encode(tlDigit, "UTF-8")
        return "https://chhoe.taigi.info/s?s=su&f=e&lmjf=ki&lmj=$encoded"
    }

    /**
     * Build MOE Dictionary lookup URL for a TL display string.
     * Returns null if the TL string produces an empty digit form.
     */
    fun moeURL(tl: String): String? {
        val tlDigit = toTLDigit(tl)
        if (tlDigit.isEmpty()) return null
        val encoded = URLEncoder.encode(tlDigit, "UTF-8")
        return "https://sutian.moe.edu.tw/zh-hant/tshiau/?lui=tai_su&tsha=$encoded"
    }

    /**
     * Convert TL display form (with diacritics) to TL digit form for URL.
     * e.g. "tāi-tsì" → "tai7-tsi3"
     */
    fun toTLDigit(tl: String): String {
        val syllables = tl.lowercase().split("-")
        return syllables.joinToString("-") { normalizeSyllableToDigit(it) }
    }

    private fun normalizeSyllableToDigit(syllable: String): String {
        if (syllable.isEmpty()) return ""

        // Quick path: already-digit-toned input keeps the digit (or strips
        // tone 1 / 4 for external dictionary URL semantics). Nasal-only
        // substitution suffices for the digit branch; full preprocessing
        // happens below for the diacritic path.
        val withNasalConverted = syllable.replace("\u207F", "nn").replace("\u1D3A", "nn")
        if (withNasalConverted.last().isDigit()) {
            val tone = withNasalConverted.last().toString()
            if (tone == "1" || tone == "4") return withNasalConverted.dropLast(1)
            return withNasalConverted
        }

        val withOoConverted = TaigiUnicode.nfdPreprocessed(syllable)

        var toneNumber = ""
        val withoutTone = StringBuilder()

        for (char in withOoConverted) {
            val tone = toneMarkToNumber[char]
            if (tone != null) {
                toneNumber = tone
            } else {
                withoutTone.append(char)
            }
        }

        // Tone 1 (open) and 4 (checked) are omitted in external dictionary URLs
        if (toneNumber.isEmpty() || toneNumber == "1" || toneNumber == "4") {
            return withoutTone.toString()
        }

        return withoutTone.toString() + toneNumber
    }
}
