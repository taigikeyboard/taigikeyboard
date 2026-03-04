package com.siansiansu.taigikeyboard.ime.dictionary

import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import java.text.Normalizer

/**
 * Input normalizer
 *
 * Converts user input to numeric tone format (mode-native spelling):
 * - POJ diacritics (hó) → ho2 (stays POJ)
 * - TL diacritics (hóo) → hoo2
 * - POJ numeric (ho2) → ho2 (stays POJ)
 * - TL numeric (hoo2) → hoo2
 *
 * Output retains the input mode's spelling. The trie prefix (tl:/poj:)
 * is added at the query boundary, not here.
 *
 * Pipeline: split syllables → per-syllable (diacritics→digits) → join
 */
object InputNormalizer {

    private const val TAG = "InputNormalizer"

    // Derive tone mark map from TaigiPhonetics (single source of truth)
    private val toneMarkToNumber: Map<Char, String> =
        TaigiPhonetics.combiningToToneNum.mapKeys { (codePoint, _) -> codePoint.toChar() }

    /**
     * 入聲韻尾（-p, -t, -k, -h）
     * 無調符且以這些結尾的音節為第 4 聲
     */
    private val checkedEndings = setOf('p', 't', 'k', 'h')

    /**
     * Normalize input to Trie query format (TL numeric tones)
     *
     * @param input User input
     * @param mode POJ or TL mode
     * @return Normalized string (TL format, lowercase, no hyphens, numeric tones)
     */
    fun normalize(input: String, mode: InputMode): String {
        if (input.isEmpty()) return ""

        val lowercased = input.lowercase()

        // Only add default tones when input contains diacritics
        val shouldAddDefaultTones = hasToneMarks(lowercased)

        val syllables = lowercased.split("-")
        val result = syllables.map { syllable ->
            normalizeSyllable(syllable, addDefaultTone = shouldAddDefaultTones)
        }

        // Validate each syllable against the mode-appropriate trie (aligned with iOS)
        if (mode != InputMode.ENGLISH) {
            for (syllable in result) {
                if (syllable.isEmpty()) continue
                val base = if (syllable.last().isDigit()) syllable.dropLast(1) else syllable
                if (base.isEmpty()) continue
                if (!SyllableSegmenter.isValidPrefix(base, mode)) {
                    return ""
                }
            }
        }

        val normalized = result.joinToString("")

        if (BuildConfig.DEBUG && input != normalized) {
            Log.d(TAG, "[NORMALIZE] input='$input' -> '$normalized'")
        }

        return normalized
    }

    /**
     * Normalize a single syllable: strip diacritics → numeric tone
     *
     * Steps:
     * 1. Convert POJ nasal ⁿ → nn
     * 2. NFD decompose
     * 3. Convert POJ o͘ (U+0358) → oo
     * 4. Extract combining tone mark → digit
     * 5. Add default tone 1 or 4 based on checked endings (only when addDefaultTone = true)
     * 6. Join: syllable + tone digit
     *
     * Note: POJ→TL spelling conversion (ch→ts) is done in normalize(), not here.
     */
    private fun normalizeSyllable(syllable: String, addDefaultTone: Boolean): String {
        if (syllable.isEmpty()) return ""

        // 轉換 POJ 鼻音符號 ⁿ (U+207F) / ᴺ (U+1D3A) → nn
        val withNasalConverted = syllable.replace("\u207F", "nn").replace("\u1D3A", "nn")

        // 檢查是否已有數字聲調（如 ho2）
        val existingTone = withNasalConverted.lastOrNull()?.takeIf { it.isDigit() }
        if (existingTone != null) {
            // 已有數字聲調，直接返回
            return withNasalConverted
        }

        // NFD 分解
        val nfd = Normalizer.normalize(withNasalConverted, Normalizer.Form.NFD)

        // 轉換 POJ o͘：把 U+0358 (COMBINING DOT ABOVE RIGHT) 替換成 o
        // 需在 NFD 分解後處理，因為 ó͘ 分解後是 o + ́ + ͘
        val withOoConverted = nfd.replace("\u0358", "o")

        // 提取聲調標記
        var toneNumber = ""
        val withoutTone = StringBuilder()

        for (char in withOoConverted) {
            val tone = toneMarkToNumber[char]
            if (tone != null) {
                toneNumber = tone  // 取最後一個聲調標記
            } else {
                withoutTone.append(char)
            }
        }

        // 無調符時根據韻尾判斷聲調（僅當 addDefaultTone = true）
        if (addDefaultTone && toneNumber.isEmpty()) {
            val lastChar = withoutTone.lastOrNull()
            if (lastChar != null) {
                toneNumber = if (lastChar in checkedEndings) {
                    // 入聲韻尾（-p, -t, -k, -h）→ 第 4 聲
                    "4"
                } else {
                    // 開音節 → 第 1 聲
                    "1"
                }
            }
        }

        // 組合：音節 + 聲調數字
        return withoutTone.toString() + toneNumber
    }

    /**
     * Build a search key from continuous input using SyllableSegmenter.
     *
     * For continuous input without hyphens (e.g., "gua2si7soo"), segments into
     * valid syllables, adds default tones to non-final segments
     * (tone 1 for open syllables, tone 4 for stop consonants), then joins
     * without hyphens to match the trie key format.
     *
     * This enables autocomplete for continuous input like "guasisoo" ->
     * segmented as ["gua", "si", "soo"] -> normalized as "gua1si1soo".
     *
     * For hyphenated input, falls back to the regular normalize() behavior.
     *
     * @param input Raw user input
     * @param mode POJ or TL mode
     * @return Normalized search key for trie prefix matching
     */
    fun buildSearchKey(input: String, mode: InputMode): String {
        if (input.isEmpty()) return ""

        // Aligned with iOS AutocompleteService.buildSearchKey:
        // - No lowercasing (preserve original case)
        // - Segment raw input, strip trailing hyphens, add default tones
        // - Join with "-" separator

        val prefix = DictionaryConstants.triePrefix(mode)
        val checker: WordPrefixChecker? = if (TrieService.isReady) {
            { key -> TrieService.prefixSearch(prefix + key.lowercase(), 1).isNotEmpty() }
        } else null
        val segments = SyllableSegmenter.segment(input, wordPrefixChecker = checker, mode = mode)

        // Single segment: return input unchanged (match iOS)
        if (segments.size <= 1) return input

        val hasTones = input.any { it.isDigit() }

        val processed = segments.mapIndexed { index, seg ->
            val base = if (seg.endsWith("-")) seg.dropLast(1) else seg
            if (base.isEmpty()) return@mapIndexed ""

            val isLast = index == segments.size - 1

            // Add default tones to non-final segments only when input already has tone digits
            if (hasTones && !isLast && !base.last().isDigit()) {
                base + if (TaigiPhonetics.isStopTone(base)) "4" else "1"
            } else {
                base
            }
        }

        return processed.joinToString("-")
    }

    /**
     * 檢查輸入是否包含聲調標記
     */
    fun hasToneMarks(input: String): Boolean {
        val nfd = Normalizer.normalize(input, Normalizer.Form.NFD)
        return nfd.any { it in toneMarkToNumber }
    }

}
