package com.siansiansu.taigikeyboard.ime.text.composing

import android.view.inputmethod.InputConnection
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverter
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels

/**
 * 管理台語羅馬字組字狀態
 *
 * 負責處理：
 * - 組字文字的追加與刪除
 * - 聲調數字轉換
 * - 字元組合轉換（oo → o͘, nn → ⁿ）
 * - 與 InputConnection 同步組字狀態
 */
class ComposingManager(
    private val inputMode: ToneConverterModels.InputMode
) {
    // 組字狀態
    private var composingText: String = ""
    private var isComposing: Boolean = false

    // 候選詞選擇相關
    var selectedCandidateIndex: Int = 0
        private set

    /**
     * 開始組字
     */
    fun startComposing(char: String, ic: InputConnection) {
        // 確保清空舊的組字狀態
        if (isComposing) {
            ic.finishComposingText()
        }
        composingText = char
        isComposing = true
        selectedCandidateIndex = 0
        updateComposingText(ic)
    }

    /**
     * 追加字元到組字
     */
    fun appendCharacter(char: String, ic: InputConnection) {

        if (!isComposing) {
            startComposing(char, ic)
            return
        }

        selectedCandidateIndex = 0

        // 計算新文字
        var newText = composingText + char

        // 檢查字元組合轉換（oo → o͘, nn → ⁿ）
        newText = checkCharacterCombination(newText, char) ?: newText

        // 檢查聲調轉換
        if (char.toIntOrNull() != null) {
            val toneNumber = char.toInt()
            if (toneNumber in 2..9 && toneNumber != 4) {
                newText = applyToneConversion(newText, toneNumber) ?: newText
            }
        }

        composingText = newText
        updateComposingText(ic)
    }

    /**
     * 追加連字符號
     */
    fun appendHyphen(ic: InputConnection) {
        appendCharacter("-", ic)
    }

    /**
     * 刪除組字的最後一個字元
     */
    fun deleteBackward(ic: InputConnection): Boolean {
        if (!isComposing || composingText.isEmpty()) {
            return false
        }

        // 嘗試聲調還原
        val restoredText = attemptToneRestoration()
        if (restoredText != null) {
            composingText = restoredText
            updateComposingText(ic)
            return true
        }

        // 一般字元刪除
        composingText = composingText.dropLast(1)

        if (composingText.isEmpty()) {
            // 清空組字區（直接刪除組字文字）
            ic.setComposingText("", 1)
            reset(ic)
            return true
        }

        updateComposingText(ic)
        return true
    }

    /**
     * 確認組字（提交文字）
     */
    fun commitComposition(ic: InputConnection) {
        if (!isComposing || composingText.isEmpty()) {
            return
        }

        // 清除內部狀態
        composingText = ""
        isComposing = false
        selectedCandidateIndex = 0

        // finishComposingText() 會將當前 composing text 提交到輸入框
        ic.finishComposingText()
    }

    /**
     * 選擇候選詞
     */
    fun selectSuggestion(suggestion: String, ic: InputConnection) {
        if (!isComposing) {
            return
        }

        // 清除內部狀態
        composingText = ""
        isComposing = false
        selectedCandidateIndex = 0

        // 先設定組字文字，再確認提交
        ic.setComposingText(suggestion, 1)
        ic.finishComposingText()
    }

    /**
     * 移動到下一個候選詞（空白鍵循環選擇）
     */
    fun moveToNextCandidate(totalCandidates: Int): Boolean {
        if (!isComposing || totalCandidates == 0) {
            return false
        }

        selectedCandidateIndex = (selectedCandidateIndex + 1) % totalCandidates
        return true
    }

    /**
     * 確認當前選中的候選詞
     */
    fun confirmSelectedCandidate(candidates: List<String>, ic: InputConnection): Boolean {
        if (!isComposing || selectedCandidateIndex >= candidates.size) {
            return false
        }

        selectSuggestion(candidates[selectedCandidateIndex], ic)
        return true
    }

    /**
     * 重置組字狀態
     */
    fun reset(ic: InputConnection) {
        if (isComposing) {
            ic.finishComposingText()
        }
        composingText = ""
        isComposing = false
        selectedCandidateIndex = 0
    }

    /**
     * 取得當前組字文字
     */
    fun getComposingText(): String? {
        return if (isComposing) composingText else null
    }

    /**
     * 是否正在組字
     */
    fun isComposing(): Boolean = isComposing

    /**
     * 更新 InputConnection 的組字文字
     */
    private fun updateComposingText(ic: InputConnection) {
        ic.setComposingText(composingText, 1)
    }

    /**
     * 檢查字元組合轉換（oo → o͘, nn → ⁿ）
     */
    private fun checkCharacterCombination(currentText: String, input: String): String? {
        // POJ 模式：檢查 oo → o͘
        if (inputMode == ToneConverterModels.InputMode.POJ) {
            if (input.lowercase() == "o" && currentText.length >= 2) {
                val beforeLast = currentText.dropLast(1)
                if (beforeLast.lastOrNull()?.lowercaseChar() == 'o') {
                    val wasUppercase = beforeLast.lastOrNull()?.isUpperCase() == true
                    val replacement = if (wasUppercase) "O͘" else "o͘"
                    return beforeLast.dropLast(1) + replacement
                }
            }
        }

        // POJ 模式：檢查 nn → ⁿ（台羅模式保持 nn）
        if (inputMode == ToneConverterModels.InputMode.POJ &&
            input.lowercase() == "n" &&
            currentText.length >= 3) {

            val lastThree = currentText.takeLast(3)
            if (lastThree.length >= 2) {
                val secondLast = lastThree[lastThree.length - 2]
                val thirdLast = if (lastThree.length >= 3) lastThree[lastThree.length - 3] else null

                if (secondLast.lowercaseChar() == 'n' && thirdLast != null) {
                    val vowels = "aeiouAEIOU"
                    if (thirdLast in vowels) {
                        val vowelWithNasal = "$thirdLast" + "ⁿ"
                        return currentText.dropLast(3) + vowelWithNasal
                    }
                }
            }
        }

        return null
    }

    /**
     * 應用聲調轉換
     */
    private fun applyToneConversion(currentText: String, toneNumber: Int): String? {
        if (toneNumber !in 2..9 || currentText.isEmpty()) {
            return null
        }

        val converted = ToneConverter.convertToToneMarks(currentText, inputMode)
        return if (converted != currentText) converted else null
    }

    /**
     * 嘗試聲調還原（刪除聲調符號時還原為基本字元）
     *
     * 支援兩種 Unicode 編碼：
     * 1. Precomposed characters (單一字符): ń, ǹ, ň 等
     * 2. Combining characters (基本字符 + 組合符號): n + ̂, n + ̄, n + ̍ 等
     */
    private fun attemptToneRestoration(): String? {
        if (composingText.isEmpty()) {
            return null
        }

        val toneMapping = when (inputMode) {
            ToneConverterModels.InputMode.POJ -> ToneConverterModels.pojToneMapping
            ToneConverterModels.InputMode.TL -> ToneConverterModels.tlToneMapping
        }

        // 從後往前搜尋聲調字元
        // 優先檢查較長的組合（處理 combining characters）
        var i = composingText.length - 1
        while (i >= 0) {
            // 嘗試 2 字元組合 (base + combining character)
            // 例如: n̂ = 'n' + U+0302
            if (i >= 1) {
                val twoCharSeq = composingText.substring(i - 1, i + 1)
                val baseChar = toneMapping[twoCharSeq]
                if (baseChar != null) {
                    val before = composingText.substring(0, i - 1)
                    val after = composingText.substring(i + 1)
                    return before + baseChar + after
                }
            }

            // 嘗試 1 字元 (precomposed character)
            // 例如: ń = U+0144
            val oneChar = composingText[i].toString()
            val baseChar = toneMapping[oneChar]
            if (baseChar != null) {
                val before = composingText.substring(0, i)
                val after = composingText.substring(i + 1)
                return before + baseChar + after
            }

            i--
        }

        return null
    }
}
