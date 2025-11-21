import Foundation

/// 文字處理工具
/// 提供詞彙文字處理相關的靜態方法
enum TextProcessor {

    // MARK: - Capitalization

    /// 根據輸入文字的大小寫狀態，處理目標文字的大小寫
    static func capitalize(_ text: String, basedOn input: String) -> String {
        // 檢查設定是否啟用自動大寫
        guard SharedSettings.shared.isAutoCapitalizationEnabled else {
            return text
        }

        guard !input.isEmpty, !text.isEmpty else {
            return text
        }

        let firstChar = input.first!
        let isUpper = firstChar.isUppercase

        guard isUpper else {
            return text
        }

        let textFirst = text.first!

        if textFirst.isLetter {
            // 使用聲調字母大寫轉換，根據輸入模式選擇映射表
            let inputMode = SharedSettings.shared.inputMode
            let first = ToneUtilities.uppercaseToneLetter(String(textFirst), mode: inputMode)
            let rest = String(text.dropFirst())

            return first + rest
        } else {
            return text
        }
    }

    /// 判斷文字是否以羅馬字母開頭
    static func startsWithRomanLetter(_ text: String) -> Bool {
        guard let firstChar = text.first else {
            return false
        }
        return firstChar.isLetter
    }

    // MARK: - Deduplication

    /// 移除重複的詞彙（基於 roman 和 hanzi 組合）
    static func removeDuplicates(_ words: [TaigiWord]) -> [TaigiWord] {
        var seen = Set<String>()
        var result: [TaigiWord] = []

        for word in words {
            let key = "\(word.roman)|\(word.hanzi ?? "")"
            if !seen.contains(key) {
                seen.insert(key)
                result.append(word)
            }
        }

        return result
    }

    // MARK: - Sorting

    /// 根據使用者頻率排序詞彙
    /// - Parameters:
    ///   - words: 要排序的詞彙陣列
    ///   - frequencies: 詞彙頻率字典
    /// - Returns: 排序後的詞彙陣列
    static func sortByFrequency(
        _ words: [TaigiWord],
        frequencies: [String: Int]
    ) -> [TaigiWord] {
        words.sorted { word1, word2 in
            let freq1 = frequencies[word1.displayText] ?? 0
            let freq2 = frequencies[word2.displayText] ?? 0

            // 優先按頻率排序
            if freq1 != freq2 {
                return freq1 > freq2
            }

            // 頻率相同時，按長度排序
            let len1 = word1.roman.count
            let len2 = word2.roman.count
            if len1 != len2 {
                return len1 < len2
            }

            // 長度相同時，按字母順序排序
            return word1.roman < word2.roman
        }
    }

    /// 收集詞彙的使用頻率
    /// - Parameters:
    ///   - words: 詞彙陣列
    ///   - getFrequency: 取得詞彙頻率的閉包
    /// - Returns: 詞彙頻率字典
    static func collectFrequencies(
        for words: [TaigiWord],
        using getFrequency: (String) -> Int
    ) -> [String: Int] {
        let wordTexts = words.compactMap(\.displayText)
        return wordTexts.reduce(into: [:]) { dict, word in
            if dict[word] == nil {
                dict[word] = getFrequency(word)
            }
        }
    }
}
