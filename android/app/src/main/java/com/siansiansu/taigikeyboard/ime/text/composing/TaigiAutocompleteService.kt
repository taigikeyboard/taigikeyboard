package com.siansiansu.taigikeyboard.ime.text.composing

import android.content.Context
import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.dictionary.*
import com.siansiansu.taigikeyboard.ime.core.PrefHelper

/**
 * 台語自動完成服務
 *
 * 負責：
 * - 根據組字文字搜尋候選詞
 * - 判斷輸入類型（漢字、帶聲調羅馬字、無聲調羅馬字）
 * - 整合使用者頻率排序
 * - showHanjiMode 關閉時對羅馬字去重
 */
class TaigiAutocompleteService(
    private val context: Context,
    private val inputMode: ToneConverterModels.InputMode
) {
    companion object {
        private const val TAG = "TaigiAutocompleteService"
    }

    private val prefs: PrefHelper by lazy { PrefHelper(context) }

    /**
     * 搜尋候選詞
     *
     * @param composingText 組字文字（保留原始大小寫）
     * @return 候選詞列表（第 0 個位置為當前組字文字，第 1 個位置開始為建議候選詞）
     *         參考 iOS: AutocompleteService.swift:69-101
     */
    suspend fun getSuggestions(composingText: String): List<TaigiWord> {
        if (composingText.isEmpty()) {
            return emptyList()
        }

        return try {
            // 保留原始輸入（含大小寫）用於候選詞首字元大小寫判斷
            val originalInput = composingText

            // 前處理：聲調轉換
            val preprocessedText = ToneConverter.convertToToneMarks(composingText, inputMode)

            // 判斷輸入類型
            val inputType = determineInputType(preprocessedText)

            // 搜尋詞典（使用預設 limit = DictionaryConstants.DEFAULT_SEARCH_LIMIT = 100）
            val words = LexiconService.search(
                input = preprocessedText,
                originalInput = originalInput,
                inputType = inputType,
                inputMode = inputMode,
                context = context
            )

            // 當 showHanjiMode = false 時，對羅馬字進行去重
            // 參考 iOS: AutocompleteService.swift:262-268
            val dedupedWords = if (!prefs.showHanjiMode) {
                deduplicateRomanWords(words)
            } else {
                words
            }

            // 在第 0 個位置插入當前組字文字候選詞
            // 參考 iOS: AutocompleteService.swift:99-101
            val composingTextWord = createComposingTextWord(composingText)

            // TODO: 整合 UserFrequencyService 排序
            buildList {
                add(composingTextWord)
                addAll(dedupedWords)
            }
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "getSuggestions failed for: $composingText", e)
            }
            emptyList()
        }
    }

    /**
     * 建立當前組字文字的候選詞物件
     * 這個候選詞會被放在候選詞列的第 0 個位置，顯示使用者目前正在輸入的內容
     *
     * 參考 iOS: AutocompleteService.swift:113-124
     *
     * @param composingText 當前組字文字（保留原始大小寫）
     * @return 組字文字的候選詞物件
     */
    private fun createComposingTextWord(composingText: String): TaigiWord {
        return TaigiWord(
            id = 0,  // 組字文字候選詞使用特殊 ID 0
            roman = composingText,
            hanzi = null,  // 組字文字候選詞不顯示漢字
            lengthScore = null  // 組字文字候選詞不需要長度分數
        )
    }

    /**
     * 對候選詞按羅馬字進行去重，保留第一個出現的
     *
     * 參考 iOS: AutocompleteService.swift:273-286
     *
     * @param words 原始候選詞列表
     * @return 去重後的候選詞列表
     */
    private fun deduplicateRomanWords(words: List<TaigiWord>): List<TaigiWord> {
        val seenRoman = mutableSetOf<String>()
        val result = mutableListOf<TaigiWord>()

        for (word in words) {
            if (!seenRoman.contains(word.roman)) {
                seenRoman.add(word.roman)
                result.add(word)
            }
        }

        return result
    }

    /**
     * 判斷輸入類型
     */
    private fun determineInputType(text: String): InputType {
        return when {
            containsHanzi(text) -> InputType.Hanzi
            containsToneMarks(text) -> InputType.RomanWithTone
            else -> InputType.RomanWithoutTone
        }
    }

    /**
     * 檢查是否包含漢字
     */
    private fun containsHanzi(text: String): Boolean {
        return text.any { char ->
            val codePoint = char.code
            codePoint in 0x4E00..0x9FFF ||
            codePoint in 0xF900..0xFAFF ||
            codePoint in 0x3400..0x4DBF
        }
    }

    /**
     * 檢查是否包含聲調標記
     *
     * Swift 和 Kotlin 對字符串遍歷的處理不同：
     * - Swift 以 grapheme cluster 為單位 (如 "o̍" 視為一個字符)
     * - Kotlin 以 UTF-16 code unit 為單位 (如 "o̍" 視為 "o" + "̍" 兩個字符)
     *
     * 因此需要額外檢查 Unicode 組合標記 (U+0300-U+036F)
     */
    private fun containsToneMarks(text: String): Boolean {
        val pojToneChars = ToneConverterModels.pojToneMapping.keys
        val tlToneChars = ToneConverterModels.tlToneMapping.keys

        return text.any { char ->
            val code = char.code

            // 排除 U+0358 (combining dot above right，用於 o͘)
            // U+0358 是元音區別符號，不是聲調標記
            if (code == 0x0358) {
                return@any false
            }

            // 檢查 Unicode 組合標記範圍（Combining Diacritical Marks）
            // 涵蓋台語使用的所有聲調標記：̀ ̂ ̄ ̋ ̌ ̍ 等
            code in 0x0300..0x036F ||
            // 檢查預組合聲調字符（如 á, é, í 等）
            char.toString() in pojToneChars || char.toString() in tlToneChars
        }
    }
}
