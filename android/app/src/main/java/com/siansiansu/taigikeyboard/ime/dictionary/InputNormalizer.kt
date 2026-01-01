package com.siansiansu.taigikeyboard.ime.dictionary

import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
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

    private const val TAG = "InputNormalizer"

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
     * 入聲韻尾（-p, -t, -k, -h）
     * 無調符且以這些結尾的音節為第 4 聲
     */
    private val CHECKED_ENDINGS = setOf('p', 't', 'k', 'h')

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

        // 判斷是否需要補上預設聲調（1 或 4）
        // 只有當輸入包含調符時，才對無調符音節補上預設聲調
        // 避免對不完整輸入（如單字母 "g"）錯誤加上聲調
        val shouldAddDefaultTones = hasToneMarks(lowercased)

        // 以連字符分割音節，逐音節處理
        val syllables = lowercased.split("-")
        val result = syllables.map { syllable ->
            normalizeSyllable(syllable, addDefaultTone = shouldAddDefaultTones)
        }

        // 合併（不含連字符）
        val normalized = result.joinToString("")

        if (BuildConfig.DEBUG && input != normalized) {
            Log.d(TAG, "[NORMALIZE] input='$input' -> '$normalized'")
        }

        return normalized
    }

    /**
     * 正規化單一音節
     *
     * 步驟：
     * 1. 轉換 POJ 鼻音符號 ⁿ → nn
     * 2. NFD 分解（將預組合字符分解為基礎字符 + 組合標記）
     * 3. 轉換 POJ o͘（U+0358）→ oo
     * 4. 提取聲調標記，轉為數字
     * 5. 無調符時根據韻尾判斷聲調 1 或 4（僅當 addDefaultTone = true）
     * 6. 組合：音節 + 聲調數字
     *
     * @param syllable 音節字串
     * @param addDefaultTone 是否對無調符音節補上預設聲調（1 或 4）
     *
     * 注意：不做 POJ→TL 拼法轉換（如 ch→ts），因為 Trie 使用前綴區分（tl:/poj:）
     */
    private fun normalizeSyllable(syllable: String, addDefaultTone: Boolean): String {
        if (syllable.isEmpty()) return ""

        // 轉換 POJ 鼻音符號 ⁿ (U+207F) → nn
        val withNasalConverted = syllable.replace("\u207F", "nn")

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
            val tone = TONE_MARK_TO_NUMBER[char]
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
                toneNumber = if (lastChar in CHECKED_ENDINGS) {
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
