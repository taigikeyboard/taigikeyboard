import Foundation
import KeyboardKit

/// 候選詞大小寫轉換器
///
/// 根據當前 keyboardCase 狀態轉換候選詞的顯示文字，使候選詞反映：
/// - 已輸入部分的大小寫
/// - 即將輸入的字元大小寫（shift/caps lock 狀態）
enum SuggestionCaseTransformer {

    // MARK: - Public API

    /// 根據 keyboardCase 轉換候選詞列表
    ///
    /// - Parameters:
    ///   - suggestions: 原始候選詞列表
    ///   - composingText: 已輸入的組字文字
    ///   - keyboardCase: 當前鍵盤大小寫狀態
    ///   - inputMode: 輸入模式（POJ/TL）
    /// - Returns: 轉換後的候選詞列表
    static func transform(
        _ suggestions: [Autocomplete.Suggestion],
        composingText: String,
        keyboardCase: Keyboard.KeyboardCase,
        inputMode: InputMode
    ) -> [Autocomplete.Suggestion] {
        suggestions.map { suggestion in
            transformSuggestion(
                suggestion,
                composingText: composingText,
                keyboardCase: keyboardCase,
                inputMode: inputMode
            )
        }
    }

    // MARK: - Private Methods

    /// 轉換單個候選詞
    private static func transformSuggestion(
        _ suggestion: Autocomplete.Suggestion,
        composingText: String,
        keyboardCase: Keyboard.KeyboardCase,
        inputMode: InputMode
    ) -> Autocomplete.Suggestion {
        // 組字文字候選詞（第 0 個位置）不需轉換，因為它已經是使用者輸入的文字
        if suggestion.additionalInfo["isComposingText"] == "true" {
            return suggestion
        }

        // NextWord 候選詞不需轉換（非組字狀態）
        if suggestion.additionalInfo["isNextWord"] == "true" {
            return suggestion
        }

        let transformedText = transformText(
            originalText: suggestion.text,
            composingText: composingText,
            keyboardCase: keyboardCase,
            inputMode: inputMode
        )

        return Autocomplete.Suggestion(
            text: transformedText,
            title: transformedText,
            subtitle: suggestion.subtitle,
            additionalInfo: suggestion.additionalInfo
        )
    }

    /// 轉換文字大小寫
    ///
    /// 轉換邏輯：
    /// - `.capsLocked`: 全部大寫
    /// - `.uppercased`: 已輸入部分保持原樣，下一個字元大寫，其餘小寫
    /// - `.lowercased`: 已輸入部分保持原樣，其餘小寫
    private static func transformText(
        originalText: String,
        composingText: String,
        keyboardCase: Keyboard.KeyboardCase,
        inputMode: InputMode
    ) -> String {
        // Caps Lock：全部大寫
        if keyboardCase == .capsLocked {
            return toUppercase(originalText, inputMode: inputMode)
        }

        // 計算已輸入的字母數量（排除數字聲調）
        let typedLetterCount = countLetters(in: composingText)
        let originalLetterCount = countLetters(in: originalText)

        guard typedLetterCount > 0 else {
            // 沒有輸入，返回原始文字
            return originalText
        }

        // 候選詞比已輸入短或相等：整個候選詞都按 composingText 的大小寫轉換
        if typedLetterCount >= originalLetterCount {
            return matchCase(
                target: originalText,
                source: composingText,
                inputMode: inputMode
            )
        }

        // 分割：已輸入部分 vs 未輸入部分
        let (typedPortion, remainingPortion) = splitByLetterCount(
            originalText,
            letterCount: typedLetterCount
        )

        // 已輸入部分：保持與 composingText 相同的大小寫
        let preservedTyped = matchCase(
            target: typedPortion,
            source: composingText,
            inputMode: inputMode
        )

        // 未輸入部分：根據 keyboardCase 決定
        let transformedRemaining: String
        if keyboardCase == .uppercased {
            // 下一個字母大寫，其餘小寫
            transformedRemaining = capitalizeFirstLetter(remainingPortion, inputMode: inputMode)
        } else {
            // lowercased：全部小寫
            transformedRemaining = toLowercase(remainingPortion, inputMode: inputMode)
        }

        return preservedTyped + transformedRemaining
    }

    // MARK: - Helper Methods

    /// 計算字串中的字母數量（排除數字和符號）
    private static func countLetters(in text: String) -> Int {
        text.filter { $0.isLetter }.count
    }

    /// 根據字母數量分割字串
    ///
    /// - Parameters:
    ///   - text: 要分割的文字
    ///   - letterCount: 第一部分應包含的字母數量
    /// - Returns: (第一部分, 第二部分)
    private static func splitByLetterCount(_ text: String, letterCount: Int) -> (String, String) {
        var count = 0
        var splitIndex = text.startIndex

        for (index, char) in text.enumerated() {
            if char.isLetter {
                count += 1
                if count == letterCount {
                    splitIndex = text.index(text.startIndex, offsetBy: index + 1)
                    break
                }
            }
        }

        let first = String(text[..<splitIndex])
        let second = String(text[splitIndex...])
        return (first, second)
    }

    /// 將目標文字的大小寫與來源文字匹配
    ///
    /// - Parameters:
    ///   - target: 要轉換的目標文字
    ///   - source: 來源文字（提供大小寫參考）
    ///   - inputMode: 輸入模式
    /// - Returns: 轉換後的文字
    private static func matchCase(target: String, source: String, inputMode: InputMode) -> String {
        var result = ""
        var sourceLetters = source.filter { $0.isLetter }

        for char in target {
            if char.isLetter, let sourceChar = sourceLetters.first {
                sourceLetters.removeFirst()
                if sourceChar.isUppercase {
                    result += ToneUtilities.uppercaseToneLetter(String(char), mode: inputMode)
                } else {
                    result += ToneUtilities.lowercaseToneLetter(String(char), mode: inputMode)
                }
            } else {
                result += String(char)
            }
        }

        return result
    }

    /// 首字母大寫，其餘小寫
    private static func capitalizeFirstLetter(_ text: String, inputMode: InputMode) -> String {
        var result = ""
        var isFirstLetter = true

        for char in text {
            if char.isLetter {
                if isFirstLetter {
                    result += ToneUtilities.uppercaseToneLetter(String(char), mode: inputMode)
                    isFirstLetter = false
                } else {
                    result += ToneUtilities.lowercaseToneLetter(String(char), mode: inputMode)
                }
            } else {
                result += String(char)
            }
        }

        return result
    }

    /// 全部轉大寫
    private static func toUppercase(_ text: String, inputMode: InputMode) -> String {
        text.map { char in
            ToneUtilities.uppercaseToneLetter(String(char), mode: inputMode)
        }.joined()
    }

    /// 全部轉小寫
    private static func toLowercase(_ text: String, inputMode: InputMode) -> String {
        text.map { char in
            ToneUtilities.lowercaseToneLetter(String(char), mode: inputMode)
        }.joined()
    }
}
