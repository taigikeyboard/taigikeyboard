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
    fun normalize(
        input: String,
        mode: InputMode,
    ): String {
        if (input.isEmpty()) return ""

        val lowercased = preprocessTPS(input).lowercase()

        // Only add default tones when input contains diacritics
        val shouldAddDefaultTones = hasToneMarks(lowercased)

        val syllables = lowercased.split("-", " ")
        val result =
            syllables.map { syllable ->
                normalizeSyllable(syllable, addDefaultTone = shouldAddDefaultTones)
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
    private fun normalizeSyllable(
        syllable: String,
        addDefaultTone: Boolean,
    ): String {
        if (syllable.isEmpty()) return ""

        // 轉換 POJ 鼻音符號 ⁿ (U+207F) / ᴺ (U+1D3A) → nn
        val withNasalConverted = syllable.replace("\u207F", "nn").replace("\u1D3A", "nn")

        // NFD 分解 + 轉換 POJ o͘ (U+0358) → oo
        // 必須在數字聲調檢查前處理，否則 "ho͘2"（齒盤輸入）會帶 U+0358 直接返回，
        // 導致 trie 查詢失敗（trie 用 ASCII "hoo2"）
        val nfd = Normalizer.normalize(withNasalConverted, Normalizer.Form.NFD)
        val withOoConverted = nfd.replace("\u0358", "o")

        // 檢查是否已有數字聲調（如 hoo2）
        val existingTone = withOoConverted.lastOrNull()?.takeIf { it.isDigit() }
        if (existingTone != null) {
            return withOoConverted
        }

        // 提取聲調標記
        var toneNumber = ""
        val withoutTone = StringBuilder()

        for (char in withOoConverted) {
            val tone = toneMarkToNumber[char]
            if (tone != null) {
                toneNumber = tone // 取最後一個聲調標記
            } else {
                withoutTone.append(char)
            }
        }

        // 無調符時根據韻尾判斷聲調（僅當 addDefaultTone = true）
        if (addDefaultTone && toneNumber.isEmpty()) {
            val lastChar = withoutTone.lastOrNull()
            if (lastChar != null) {
                toneNumber =
                    if (lastChar in checkedEndings) {
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
     * Build a search key from raw input.
     *
     * Converts TPS input to TL romanization for trie lookup.
     * Non-TPS input is returned as-is.
     *
     * @param input Raw user input
     * @param mode POJ or TL mode
     * @return Search key for trie prefix matching
     */
    fun buildSearchKey(
        input: String,
        mode: InputMode,
    ): String {
        if (input.isEmpty()) return ""
        return preprocessTPS(input)
    }

    /**
     * 檢查輸入是否包含聲調標記
     */
    fun hasToneMarks(input: String): Boolean {
        val nfd = Normalizer.normalize(input, Normalizer.Form.NFD)
        return nfd.any { it in toneMarkToNumber }
    }

    /** Convert TPS input to TL; return original if no TPS characters found. */
    private fun preprocessTPS(input: String): String = if (TPSConverter.containsTPS(input)) TPSConverter.toTL(input) else input
}
