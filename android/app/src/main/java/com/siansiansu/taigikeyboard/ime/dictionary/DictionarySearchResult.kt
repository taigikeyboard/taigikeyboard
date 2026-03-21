package com.siansiansu.taigikeyboard.ime.dictionary

import java.net.URLEncoder
import java.text.Normalizer

/**
 * Dictionary source enum matching DB column names
 */
enum class DictionarySource(val columnName: String, val displayName: String) {
    KAUTIAN("kautian", "教典"),
    TAIGITV("taigitv", "台語新詞"),
    ITAIGI("itaigi", "iTaigi"),
    SITBUT("sitbut", "植物名彙"),
    TAIHOA("taihoa", "台華對照"),
    TAIJIT("taijit", "臺日"),
    KUNGGE("kungge", "工藝辭典"),
    STTI("stti", "學科術語"),
    KHPOO("khpoo", "補充資料"),
    KHIIN("khiin", "補充資料"),
    LKK("lkk", "漢羅合用"),
    DEV("dev", "補充資料"),
    CUSTOM("custom", "補充資料");
}

/**
 * Search result with source information for dictionary exploration
 */
data class DictionarySearchResult(
    val id: Int,
    val roman: String,          // Display form (POJ or TL based on user setting)
    val tl: String,             // Raw TL from database (for Chhoe Taigi URL)
    val hanzi: String?,
    val frequency: Int,
    val sources: List<DictionarySource>
) {
    /**
     * Build Chhoe Taigi lookup URL using TL digit form
     */
    fun chhoeUrl(): String? {
        val tlDigit = toTLDigit(tl)
        if (tlDigit.isEmpty()) return null
        val encoded = URLEncoder.encode(tlDigit, "UTF-8")
        return "https://chhoe.taigi.info/s?s=su&f=e&lmjf=ki&lmj=$encoded"
    }

    /**
     * Build MOE Dictionary lookup URL using TL digit form
     */
    fun moeUrl(): String? {
        val tlDigit = toTLDigit(tl)
        if (tlDigit.isEmpty()) return null
        val encoded = URLEncoder.encode(tlDigit, "UTF-8")
        return "https://sutian.moe.edu.tw/zh-hant/tshiau/?lui=tai_su&tsha=$encoded"
    }

    companion object {
        // Tone mark map from TaigiPhonetics
        private val toneMarkToNumber: Map<Char, String> =
            TaigiPhonetics.combiningToToneNum.mapKeys { (codePoint, _) -> codePoint.toChar() }

        private val checkedEndings = setOf('p', 't', 'k', 'h')

        /**
         * Convert TL display form (with diacritics) to TL digit form for URL
         * e.g. "tāi-tsì" → "tai7-tsi3"
         */
        fun toTLDigit(tl: String): String {
            val syllables = tl.lowercase().split("-")
            return syllables.joinToString("-") { syllable ->
                normalizeSyllableToDigit(syllable)
            }
        }

        private fun normalizeSyllableToDigit(syllable: String): String {
            if (syllable.isEmpty()) return ""

            val withNasalConverted = syllable
                .replace("\u207F", "nn")
                .replace("\u1D3A", "nn")

            // Already has digit tone — strip tone 1 and 4 for external dictionary URLs
            if (withNasalConverted.last().isDigit()) {
                val tone = withNasalConverted.last().toString()
                if (tone == "1" || tone == "4") return withNasalConverted.dropLast(1)
                return withNasalConverted
            }

            val nfd = Normalizer.normalize(withNasalConverted, Normalizer.Form.NFD)
            val withOoConverted = nfd.replace("\u0358", "o")

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
}
