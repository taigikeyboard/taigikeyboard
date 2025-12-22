package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode

/**
 * Tone converter for Taiwanese romanization systems (POJ and TL)
 * Converts number notation (e.g., "goa2") to tone marks (e.g., "góa")
 */
object ToneConverter {

    /**
     * Convert input with tone numbers to tone marks
     * @param input Input string with tone numbers
     * @param mode POJ or TL mode
     * @return String with tone marks applied
     */
    fun convertToToneMarks(input: String, mode: InputMode): String {
        val preprocessed = preprocess(input, mode)

        return when (mode) {
            InputMode.POJ -> convertPOJ(preprocessed)
            InputMode.TL -> convertTL(preprocessed)
        }
    }

    /**
     * Preprocess input for special character conversions
     */
    private fun preprocess(input: String, mode: InputMode): String {
        var result = input

        if (mode == InputMode.POJ) {
            // TODO: Add settings support for enableDoubleTapOO and enableDoubleTapNN
            // For now, enable these conversions by default

            // Convert oo → o͘
            result = result.replace("oo", "o͘")
            result = result.replace("Oo", "O͘")
            result = result.replace("OO", "O͘")

            // Convert nn → ⁿ (only in POJ mode)
            result = convertNN(result)
        }

        return result
    }

    /**
     * Convert vowel + nn pattern to vowel + ⁿ
     */
    private fun convertNN(input: String): String {
        val vowels = "aeiouAEIOU"
        val result = StringBuilder()
        var i = 0

        while (i < input.length) {
            when {
                i + 2 < input.length &&
                vowels.contains(input[i]) &&
                input[i + 1].lowercaseChar() == 'n' &&
                input[i + 2].lowercaseChar() == 'n' -> {
                    result.append(input[i]).append("ⁿ")
                    i += 3
                }
                else -> {
                    result.append(input[i])
                    i++
                }
            }
        }

        return result.toString()
    }

    /**
     * Convert POJ input to tone marks
     */
    private fun convertPOJ(input: String): String {
        val syllables = input.split("-")
        val converted = syllables.map { syllable ->
            if (syllable.isEmpty()) "" else convertPOJSyllable(syllable)
        }
        return converted.joinToString("-")
    }

    /**
     * Convert a single POJ syllable
     */
    private fun convertPOJSyllable(syllable: String): String {
        val (baseForm, toneNumber) = extractToneNumber(syllable)

        // 聲調 0 無標記，聲調 1/4 保留數字顯示
        if (toneNumber == 0) {
            return baseForm
        }
        if (toneNumber == 1 || toneNumber == 4) {
            return "$baseForm$toneNumber"
        }

        val vowelRange = findVowelRange(baseForm, InputMode.POJ) ?: return syllable
        val vowel = baseForm.substring(vowelRange)

        val key = "$vowel$toneNumber"
        val toned = ToneConverterModels.pojNumberToToneMapping[key] ?: return syllable

        return baseForm.replaceRange(vowelRange, toned)
    }

    /**
     * Convert TL input to tone marks
     */
    private fun convertTL(input: String): String {
        val syllables = input.split("-")
        val converted = syllables.map { syllable ->
            if (syllable.isEmpty()) "" else convertTLSyllable(syllable)
        }
        return converted.joinToString("-")
    }

    /**
     * Convert a single TL syllable
     */
    private fun convertTLSyllable(syllable: String): String {
        val (baseForm, toneNumber) = extractToneNumber(syllable)

        // 聲調 0 無標記，聲調 1/4 保留數字顯示
        if (toneNumber == 0) {
            return baseForm
        }
        if (toneNumber == 1 || toneNumber == 4) {
            return "$baseForm$toneNumber"
        }

        val vowelRange = findVowelRange(baseForm, InputMode.TL) ?: return syllable
        val vowel = baseForm.substring(vowelRange)

        // Handle "oo" in TL (maps to single "o" for tone marking)
        val actual = if (vowel == "oo") "o" else vowel

        val key = "$actual$toneNumber"
        val toned = ToneConverterModels.tlNumberToToneMapping[key] ?: return syllable

        return if (vowel == "oo") {
            // Replace only the first 'o' in "oo"
            baseForm.replaceRange(vowelRange.first until vowelRange.first + 1, toned)
        } else {
            baseForm.replaceRange(vowelRange, toned)
        }
    }

    /**
     * Extract tone number from syllable
     * @return Pair of (baseForm, toneNumber), tone=0 if no digit found (no processing needed)
     */
    fun extractToneNumber(syllable: String): Pair<String, Int> {
        if (syllable.isEmpty()) return Pair(syllable, 0)

        val lastChar = syllable.last()
        if (lastChar.isDigit()) {
            val tone = lastChar.digitToInt()
            if (tone in 1..9) {
                val baseForm = syllable.dropLast(1)
                return Pair(baseForm, tone)
            }
        }

        // No tone number found, return tone=0 (no processing, keep as-is)
        return Pair(syllable, 0)
    }

    /**
     * Find the vowel range in the syllable for tone mark placement
     */
    private fun findVowelRange(syllable: String, mode: InputMode): IntRange? {
        return when (mode) {
            InputMode.POJ -> findPOJVowelRange(syllable)
            InputMode.TL -> findTLVowelRange(syllable)
        }
    }

    /**
     * Find vowel range for POJ
     */
    private fun findPOJVowelRange(syllable: String): IntRange? {
        val lowercased = syllable.lowercase()

        val lastVowel = findLastVowel(lowercased)

        if (lastVowel == null) {
            return findSemivowelRange(syllable)
        }

        val hasPrevVowel = hasVowelBefore(lowercased, lastVowel.first)

        if (!hasPrevVowel) {
            return toOriginalRange(syllable, lowercased, lastVowel)
        }

        return findDiphthongPosition(syllable, lowercased)
    }

    /**
     * Find the last vowel in the text
     */
    private fun findLastVowel(text: String): IntRange? {
        val vowels = listOf("a", "i", "u", "e", "o͘", "o")
        var lastRange: IntRange? = null
        var lastPosition = -1

        for (vowel in vowels) {
            val index = text.lastIndexOf(vowel)
            if (index > lastPosition) {
                lastPosition = index
                // 創建閉區間，包含整個母音
                lastRange = index..(index + vowel.length - 1)
            }
        }

        return lastRange
    }

    /**
     * Check if there's a vowel before the given position
     *
     * 檢查前一個字符是否為母音（包括 o͘ 中的 o）
     */
    private fun hasVowelBefore(text: String, position: Int): Boolean {
        if (position <= 0) return false

        val beforeChar = text[position - 1].lowercaseChar()
        // 檢查是否為基本母音字符（a, i, u, e, o）
        // 注意：這裡檢查的是單個字符，不需要包含組合字符如 o͘
        return beforeChar in "aiueo"
    }

    /**
     * Find diphthong position for tone mark placement
     */
    private fun findDiphthongPosition(syllable: String, lowercased: String): IntRange? {
        val lastChar = getChar(lowercased, fromEnd = 1)
        val secondLastChar = getChar(lowercased, fromEnd = 2)

        val hasEnteringTone = "ptkh".contains(lastChar)

        return if (hasEnteringTone) {
            if ("iu".contains(secondLastChar)) {
                if (lowercased.contains("iuh")) {
                    findCharPosition(syllable, lowercased, fromEnd = 2)
                } else {
                    findCharPosition(syllable, lowercased, fromEnd = 3)
                }
            } else {
                findCharPosition(syllable, lowercased, fromEnd = 2)
            }
        } else {
            if (secondLastChar == "i") {
                findCharPosition(syllable, lowercased, fromEnd = 1)
            } else {
                findCharPosition(syllable, lowercased, fromEnd = 2)
            }
        }
    }

    /**
     * Get character at position from end (skipping nasal markers)
     */
    private fun getChar(text: String, fromEnd: Int): String {
        var pos = text.length - 1
        var charCount = 0

        while (pos >= 0) {
            val char = text[pos].toString()

            // Skip nasal marker
            if (char == "ⁿ") {
                pos--
                continue
            }

            // Handle o͘
            if (char == "o" && pos > 0 && text[pos - 1] == 'o') {
                charCount++
                if (charCount == fromEnd) return "o"
                pos -= 2
                continue
            }

            // Handle ng
            if (char == "g" && pos > 0 && text[pos - 1] == 'n') {
                charCount++
                if (charCount == fromEnd) return "ng"
                pos -= 2
                continue
            }

            charCount++
            if (charCount == fromEnd) return char
            pos--
        }

        return ""
    }

    /**
     * Find character position from end (skipping nasal markers)
     */
    private fun findCharPosition(syllable: String, lowercased: String, fromEnd: Int): IntRange? {
        var pos = lowercased.length - 1
        var charCount = 0

        while (pos >= 0) {
            val char = lowercased[pos].toString()

            // Skip nasal marker
            if (char == "ⁿ") {
                pos--
                continue
            }

            // Handle o͘ (兩個字符)
            if (char == "͘" && pos > 0 && lowercased[pos - 1] == 'o') {
                charCount++
                if (charCount == fromEnd) {
                    return toOriginalRange(syllable, lowercased, (pos - 1)..pos)
                }
                pos -= 2
                continue
            }

            // Handle oo (兩個字符)
            if (char == "o" && pos > 0 && lowercased[pos - 1] == 'o') {
                charCount++
                if (charCount == fromEnd) {
                    return toOriginalRange(syllable, lowercased, (pos - 1)..pos)
                }
                pos -= 2
                continue
            }

            // Handle ng (兩個字符)
            if (char == "g" && pos > 0 && lowercased[pos - 1] == 'n') {
                charCount++
                if (charCount == fromEnd) {
                    return toOriginalRange(syllable, lowercased, (pos - 1)..pos)
                }
                pos -= 2
                continue
            }

            // 單個字符
            charCount++
            if (charCount == fromEnd) {
                return toOriginalRange(syllable, lowercased, pos..pos)
            }
            pos--
        }

        return null
    }

    /**
     * Find vowel range for TL
     */
    private fun findTLVowelRange(syllable: String): IntRange? {
        val lowercased = syllable.lowercase()

        // Priority 1: 'a'
        lowercased.lastIndexOf("a").let {
            if (it >= 0) return toOriginalRange(syllable, lowercased, it..it)
        }

        // Priority 2: 'oo'
        lowercased.lastIndexOf("oo").let {
            if (it >= 0) return toOriginalRange(syllable, lowercased, it..(it + 1))
        }

        // Priority 3: other single vowels
        val singleVowels = listOf("i", "u", "o", "e")
        var lastVowelRange: IntRange? = null
        var lastPosition = -1

        for (vowel in singleVowels) {
            val index = lowercased.lastIndexOf(vowel)
            if (index > lastPosition) {
                lastPosition = index
                lastVowelRange = index..index
            }
        }

        if (lastVowelRange != null) {
            return toOriginalRange(syllable, lowercased, lastVowelRange)
        }

        // Priority 4: semivowels (ng, n, m)
        val semivowels = listOf("ng", "n", "m")
        for (semivowel in semivowels) {
            val index = lowercased.lastIndexOf(semivowel)
            if (index >= 0) {
                return toOriginalRange(syllable, lowercased, index..(index + semivowel.length - 1))
            }
        }

        return null
    }

    /**
     * Find semivowel range (for syllables without regular vowels)
     */
    private fun findSemivowelRange(syllable: String): IntRange? {
        val lowercased = syllable.lowercase()

        val semivowels = listOf("ⁿ", "ng", "n", "m")
        for (semivowel in semivowels) {
            val index = lowercased.lastIndexOf(semivowel)
            if (index >= 0) {
                return toOriginalRange(syllable, lowercased, index..(index + semivowel.length - 1))
            }
        }

        return null
    }

    /**
     * Convert lowercased range to original syllable range
     *
     * 注意：Kotlin 的 IntRange 是閉區間，但 substring/replaceRange 會正確處理
     * 這個函數主要做邊界驗證，並確保範圍在有效範圍內
     */
    private fun toOriginalRange(syllable: String, lowercased: String, range: IntRange): IntRange? {
        val start = range.first
        val end = range.last

        // 驗證邊界（end 是 inclusive 的最後一個索引）
        if (start < 0 || start > syllable.length || end < 0 || end >= syllable.length) {
            return null
        }

        if (start > end) return null

        // 返回相同的範圍（Kotlin IntRange 是閉區間）
        return start..end
    }
}
