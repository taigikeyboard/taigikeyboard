package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import java.text.Normalizer

/**
 * 輸入正規化工具
 *
 * 將使用者輸入統一轉換為 TL 數字聲調格式：
 * - POJ 調符（hó）→ hoo2
 * - TL 調符（hóo）→ hoo2
 * - POJ 數字（ho2）→ hoo2
 * - TL 數字（hoo2）→ hoo2
 * - POJ 無聲調（choa）→ tsua
 *
 * 處理步驟：分割音節 → 逐音節處理（調符轉數字、POJ→TL）→ 合併
 */
object InputNormalizer {

    // 調符 → 聲調數字（參考 KeSi）
    private val TONE_MARK_TO_NUMBER = mapOf(
        '\u0301' to "2",  // ́ COMBINING ACUTE ACCENT
        '\u0300' to "3",  // ̀ COMBINING GRAVE ACCENT
        '\u0302' to "5",  // ̂ COMBINING CIRCUMFLEX ACCENT
        '\u030C' to "6",  // ̌ COMBINING CARON
        '\u0304' to "7",  // ̄ COMBINING MACRON
        '\u030D' to "8",  // ̍ COMBINING VERTICAL LINE ABOVE
        '\u0306' to "9",  // ̆ COMBINING BREVE (POJ)
        '\u030B' to "9",  // ̋ COMBINING DOUBLE ACUTE ACCENT (TL)
    )

    /**
     * 正規化輸入為 Trie 查詢格式（TL 數字聲調）
     *
     * 支援任何輸入格式：
     * - POJ 調符（hó-bô）→ hoo2boo5
     * - TL 調符（hóo-bôo）→ hoo2boo5
     * - POJ 數字（ho2-bo5）→ hoo2boo5
     * - TL 數字（hoo2-boo5）→ hoo2boo5
     *
     * @param input 使用者輸入
     * @param mode POJ 或 TL 模式（目前未使用，POJ/TL 調符相同）
     * @return 正規化後的字串（TL 格式、小寫、無連字符、數字聲調）
     */
    fun normalize(input: String, mode: InputMode): String {
        if (input.isEmpty()) return ""

        // 轉小寫
        val lowercased = input.lowercase()

        // 以連字符分割音節，逐音節處理
        val syllables = lowercased.split("-")
        val result = syllables.map { syllable ->
            normalizeSyllable(syllable)
        }

        // 合併（不含連字符）
        return result.joinToString("")
    }

    /**
     * 正規化單一音節
     *
     * 步驟：
     * 1. 移除聲調 1 和 4（視為無聲調）
     * 2. NFD 分解（將預組合字符分解為基礎字符 + 組合標記）
     * 3. 提取聲調標記，轉為數字
     * 4. 組合：音節 + 聲調數字
     *
     * 注意：不做 POJ→TL 轉換，因為 Trie 使用前綴區分（tl:/poj:）
     */
    private fun normalizeSyllable(syllable: String): String {
        if (syllable.isEmpty()) return ""

        // 移除聲調 1 和 4（視為無聲調）
        // 例如：gua1 → gua, gua12 → gua2, gua42 → gua2
        val withoutTone1And4 = syllable.replace("1", "").replace("4", "")

        // 檢查是否已有數字聲調（如 ho2）
        val existingTone = withoutTone1And4.lastOrNull()?.takeIf { it.isDigit() }
        if (existingTone != null) {
            // 已有數字聲調，直接返回
            return withoutTone1And4
        }

        // NFD 分解（使用移除 1/4 後的字串）
        val nfd = Normalizer.normalize(withoutTone1And4, Normalizer.Form.NFD)

        // 提取聲調標記
        var toneNumber = ""
        val withoutTone = StringBuilder()

        for (char in nfd) {
            val tone = TONE_MARK_TO_NUMBER[char]
            if (tone != null) {
                toneNumber = tone  // 取最後一個聲調標記
            } else {
                withoutTone.append(char)
            }
        }

        // 組合：音節 + 聲調數字
        return withoutTone.toString() + toneNumber
    }

    /**
     * 將 POJ 拼法轉換為 TL 拼法（單一音節）
     *
     * 注意：目前未使用，因 Trie 使用前綴區分（tl:/poj:）
     * 保留供未來可能需要時使用
     *
     * 參考 KeSi tsuan_kongke()
     * 轉換規則（順序重要）：
     * - ch → ts
     * - ou → oo
     * - o͘ → oo
     * - ⁿ → nn
     * - oa → ua
     * - oe → ue
     * - eng → ing
     * - ek → ik
     * - oonn → onn（修正 ou→oo 後產生的錯誤）
     */
    @Suppress("unused")
    private fun pojToTl(syllable: String): String {
        return syllable
            .replace("ch", "ts")
            .replace("ou", "oo")
            .replace("o͘", "oo")
            .replace("ⁿ", "nn")
            .replace("oa", "ua")
            .replace("oe", "ue")
            .replace("eng", "ing")
            .replace("ek", "ik")
            .replace("oonn", "onn")
    }

    /**
     * 移除輸入中的所有聲調（調符和數字）
     *
     * 用於生成無聲調查詢 key（TL 格式）
     */
    fun removeAllTones(input: String, mode: InputMode): String {
        // 先正規化（調符→數字、POJ→TL）
        val normalized = normalize(input, mode)
        // 再移除數字
        return normalized.replace(Regex("[0-9]"), "")
    }

    /**
     * 檢查輸入是否包含聲調標記
     */
    fun hasToneMarks(input: String): Boolean {
        val nfd = Normalizer.normalize(input, Normalizer.Form.NFD)
        return nfd.any { it in TONE_MARK_TO_NUMBER }
    }

    /**
     * 檢查輸入是否包含數字聲調
     */
    fun hasNumericTones(input: String): Boolean {
        return input.any { it.isDigit() }
    }
}
