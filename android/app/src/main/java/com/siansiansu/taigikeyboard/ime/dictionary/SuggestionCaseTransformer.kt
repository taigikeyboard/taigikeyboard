package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * 候選詞大小寫轉換器
 *
 * 根據當前 caps/capsLock 狀態轉換候選詞的顯示文字，使候選詞反映：
 * - 已輸入部分的大小寫
 * - 即將輸入的字元大小寫（shift/caps lock 狀態）
 */
object SuggestionCaseTransformer {

    /**
     * 根據 caps/capsLock 狀態轉換候選詞列表
     *
     * @param suggestions 原始候選詞列表
     * @param composingText 已輸入的組字文字
     * @param caps 是否為大寫模式（Shift 按下）
     * @param capsLock 是否為 Caps Lock 模式
     * @param inputMode 輸入模式（POJ/TL）
     * @return 轉換後的候選詞列表
     */
    fun transform(
        suggestions: List<TaigiWord>,
        composingText: String,
        caps: Boolean,
        capsLock: Boolean,
        inputMode: ToneConverterModels.InputMode
    ): List<TaigiWord> {
        return suggestions.map { word ->
            transformWord(word, composingText, caps, capsLock, inputMode)
        }
    }

    /**
     * 轉換單個候選詞
     */
    private fun transformWord(
        word: TaigiWord,
        composingText: String,
        caps: Boolean,
        capsLock: Boolean,
        inputMode: ToneConverterModels.InputMode
    ): TaigiWord {
        // NextWord 候選詞（id < 0）不需轉換
        if (word.id < 0) {
            return word
        }

        // 組字文字候選詞（id == 0）不需轉換
        // 因為它已經是使用者輸入的文字
        // 使用 id 標記而非字串比較，避免誤匹配同名辭典詞（aligned with iOS flag-based approach）
        if (word.id == 0) {
            return word
        }

        val transformedRoman = ToneUtilities.adjustNasalMarkerCase(
            transformText(
                originalText = word.roman,
                composingText = composingText,
                caps = caps,
                capsLock = capsLock,
                inputMode = inputMode
            )
        )

        return word.copy(roman = transformedRoman)
    }

    /**
     * 轉換文字大小寫
     *
     * 轉換邏輯：
     * - capsLock: 全部大寫
     * - caps (Shift): 已輸入部分保持原樣，下一個字元大寫，其餘小寫
     * - 一般: 已輸入部分保持原樣，其餘小寫
     */
    private fun transformText(
        originalText: String,
        composingText: String,
        caps: Boolean,
        capsLock: Boolean,
        inputMode: ToneConverterModels.InputMode
    ): String {
        // Caps Lock：全部大寫
        if (capsLock) {
            return toUppercase(originalText, inputMode)
        }

        // 計算已輸入的字母數量（排除數字聲調）
        val typedLetterCount = countLetters(composingText)
        val originalLetterCount = countLetters(originalText)

        if (typedLetterCount <= 0) {
            // 沒有輸入，返回原始文字
            return originalText
        }

        // 候選詞比已輸入短或相等：整個候選詞都按 composingText 的大小寫轉換
        if (typedLetterCount >= originalLetterCount) {
            return matchCase(
                target = originalText,
                source = composingText,
                inputMode = inputMode
            )
        }

        // 分割：已輸入部分 vs 未輸入部分
        val (typedPortion, remainingPortion) = splitByLetterCount(
            originalText,
            typedLetterCount
        )

        // 已輸入部分：保持與 composingText 相同的大小寫
        val preservedTyped = matchCase(
            target = typedPortion,
            source = composingText,
            inputMode = inputMode
        )

        // 未輸入部分：根據 caps 狀態決定
        val transformedRemaining = if (caps) {
            // 下一個字母大寫，其餘小寫
            capitalizeFirstLetter(remainingPortion, inputMode)
        } else {
            // 一般模式：全部小寫
            toLowercase(remainingPortion, inputMode)
        }

        return preservedTyped + transformedRemaining
    }

    // MARK: - Helper Methods

    /**
     * 計算字串中的字母數量（排除數字和符號）
     */
    private fun countLetters(text: String): Int {
        return text.count { it.isLetter() }
    }

    /**
     * 根據字母數量分割字串
     *
     * @param text 要分割的文字
     * @param letterCount 第一部分應包含的字母數量
     * @return Pair(第一部分, 第二部分)
     */
    private fun splitByLetterCount(text: String, letterCount: Int): Pair<String, String> {
        var count = 0
        var splitIndex = 0

        for ((index, char) in text.withIndex()) {
            if (char.isLetter()) {
                count++
                if (count == letterCount) {
                    splitIndex = index + 1
                    break
                }
            }
        }

        val first = text.substring(0, splitIndex)
        val second = text.substring(splitIndex)
        return Pair(first, second)
    }

    /**
     * 將目標文字的大小寫與來源文字匹配
     *
     * @param target 要轉換的目標文字
     * @param source 來源文字（提供大小寫參考）
     * @param inputMode 輸入模式
     * @return 轉換後的文字
     */
    private fun matchCase(
        target: String,
        source: String,
        inputMode: ToneConverterModels.InputMode
    ): String {
        val result = StringBuilder()
        val sourceLetters = source.filter { it.isLetter() }.toMutableList()

        for (char in target) {
            if (char.isLetter() && sourceLetters.isNotEmpty()) {
                val sourceChar = sourceLetters.removeAt(0)
                if (sourceChar.isUpperCase()) {
                    result.append(ToneUtilities.uppercaseToneLetter(char.toString(), inputMode))
                } else {
                    result.append(ToneUtilities.lowercaseToneLetter(char.toString(), inputMode))
                }
            } else {
                result.append(char)
            }
        }

        return result.toString()
    }

    /**
     * 首字母大寫，其餘小寫
     */
    private fun capitalizeFirstLetter(
        text: String,
        inputMode: ToneConverterModels.InputMode
    ): String {
        val result = StringBuilder()
        var isFirstLetter = true

        for (char in text) {
            if (char.isLetter()) {
                if (isFirstLetter) {
                    result.append(ToneUtilities.uppercaseToneLetter(char.toString(), inputMode))
                    isFirstLetter = false
                } else {
                    result.append(ToneUtilities.lowercaseToneLetter(char.toString(), inputMode))
                }
            } else {
                result.append(char)
            }
        }

        return result.toString()
    }

    /**
     * 全部轉大寫
     */
    private fun toUppercase(text: String, inputMode: ToneConverterModels.InputMode): String {
        return text.map { char ->
            ToneUtilities.uppercaseToneLetter(char.toString(), inputMode)
        }.joinToString("")
    }

    /**
     * 全部轉小寫
     */
    private fun toLowercase(text: String, inputMode: ToneConverterModels.InputMode): String {
        return text.map { char ->
            ToneUtilities.lowercaseToneLetter(char.toString(), inputMode)
        }.joinToString("")
    }
}
