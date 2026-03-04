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
 */
class TaigiAutocompleteService(
    private val context: Context,
    private val inputMode: ToneConverterModels.InputMode,
    private val prefs: PrefHelper? = null
) {
    companion object {
        private const val TAG = "TaigiAutocompleteService"
    }

    /**
     * 搜尋候選詞
     *
     * @param rawInput 原始輸入（保留數字聲調，用於 Trie 搜尋）
     * @param displayText 顯示文字（聲調已轉換，用於候選詞位置 0 顯示）
     * @param lastSelectedWord 上一個選擇的詞（用於上下文提升）
     * @return 候選詞列表（第 0 個位置為當前組字文字，第 1 個位置開始為建議候選詞）
     *         參考 iOS: AutocompleteService.swift:69-101
     */
    suspend fun autocomplete(rawInput: String, displayText: String, lastSelectedWord: String? = null): List<TaigiWord> {
        // Guard: skip search if inputs are empty (aligned with iOS 3-guard pattern)
        if (rawInput.isEmpty() || displayText.isEmpty()) {
            return emptyList()
        }

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[INPUT] rawInput='$rawInput', displayText='$displayText', mode=$inputMode")
        }

        return try {
            // 判斷輸入類型（用 rawInput 判斷，因為它保留數字聲調）
            val determineStart = System.currentTimeMillis()
            val inputType = determineInputType(rawInput)

            if (BuildConfig.DEBUG) {
                Log.d("PERF", "[3-a] determineInputType: ${System.currentTimeMillis() - determineStart}ms")
                Log.d(TAG, "[INPUT] inputType=$inputType")
            }

            // 搜尋系統詞典
            val searchStart = System.currentTimeMillis()
            val words = LexiconService.search(
                input = rawInput,
                inputType = inputType,
                inputMode = inputMode,
                context = context,
                prefs = prefs
            )
            if (BuildConfig.DEBUG) {
                Log.d("PERF", "[3-b] LexiconService.search call: ${System.currentTimeMillis() - searchStart}ms")
                Log.d(TAG, "[RESULT] LexiconService returned ${words.size} words")
            }

            // Apply context boost: promote candidates matching bigram predictions
            val contextBoostedWords = applyContextBoost(words, lastSelectedWord)

            // 在第 0 個位置插入當前組字文字候選詞（用 displayText 顯示）
            // 參考 iOS: AutocompleteService.swift:99-101
            val buildStart = System.currentTimeMillis()
            val composingTextWord = createComposingTextWord(displayText)

            // 候選詞排序：composingText → 系統詞庫
            val result = buildList {
                add(composingTextWord)
                addAll(contextBoostedWords)
            }
            if (BuildConfig.DEBUG) {
                Log.d("PERF", "[3-c] buildList: ${System.currentTimeMillis() - buildStart}ms")
            }
            result
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[ERROR] autocomplete failed for: $rawInput", e)
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
     * 判斷輸入類型
     *
     * Uses centralized utilities:
     * - ToneConverterModels.isHanzi() for Hanzi detection (broader CJK range including Extensions B-E)
     * - InputNormalizer.hasToneMarks() for tone mark detection (NFD-based, cleaner)
     */
    private fun determineInputType(text: String): InputType {
        return when {
            ToneConverterModels.isHanzi(text) -> InputType.Hanzi
            InputNormalizer.hasToneMarks(text) -> InputType.RomanWithTone
            containsNumericTone(text) -> InputType.RomanWithTone
            else -> InputType.RomanWithoutTone
        }
    }

    /**
     * Apply context boost by promoting candidates whose displayText starts
     * with a bigram-predicted character.
     *
     * Uses lastSelectedWord to query NextWordService for bigram predictions,
     * then partitions candidates: context-matched first, then the rest
     * (preserving original order within each group).
     *
     * Aligned with iOS: AutocompleteService.swift applyContextBoost()
     */
    private suspend fun applyContextBoost(words: List<TaigiWord>, lastSelectedWord: String?): List<TaigiWord> {
        if (lastSelectedWord.isNullOrEmpty()) return words

        val predictions = NextWordService.predict(
            word = lastSelectedWord,
            limit = 30,
            context = context,
            prefs = prefs
        )
        if (predictions.isEmpty()) return words

        // Build context set: predicted hanzi characters
        val contextSet = predictions.map { it.hanzi }.toSet()

        // Partition: candidates whose displayText starts with a predicted character go first
        val boosted = mutableListOf<TaigiWord>()
        val rest = mutableListOf<TaigiWord>()

        for (word in words) {
            val display = word.displayText
            val firstChar = display.firstOrNull()?.toString()
            if (firstChar != null && firstChar in contextSet) {
                boosted.add(word)
            } else {
                rest.add(word)
            }
        }

        return boosted + rest
    }

    /**
     * 檢查是否包含數字聲調（2-9，排除 1、4、0）
     * Tones 1 and 4 are unmarked in Taiwanese, so their presence alone
     * doesn't indicate toned input.
     */
    private fun containsNumericTone(text: String): Boolean {
        return text.any { char ->
            char.isDigit() && char != '1' && char != '4' && char != '0'
        }
    }
}
