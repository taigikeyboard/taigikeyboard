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

    /// 計算候選詞排序分數
    ///
    /// 簡化公式（v3，與 Android 一致）：
    /// `userFreqScore + recencyBonus + exactBonus + baseFreqScore`
    ///
    /// 設計理念：
    /// - userFreqScore 主導排序（穩定性優先）
    /// - recencyBonus 和 exactBonus 只做微調（不會讓低頻詞超過高頻詞）
    /// - 移除長度懲罰（讓使用者行為決定排序）
    ///
    /// - Parameters:
    ///   - word: 候選詞
    ///   - normalizedInput: 正規化後的輸入（小寫、去連字符）
    ///   - frequencyData: 使用者頻率資料（包含 count 和 lastUsedMillis）
    /// - Returns: 排序分數（越高越優先）
    static func calculateScore(
        word: TaigiWord,
        normalizedInput: String,
        frequencyData: UserFrequencyService.FrequencyData
    ) -> Int {
        let candidateRoman = word.roman
            .replacingOccurrences(of: "-", with: "")
            .lowercased()

        // 使用者頻率（主導因素，上限 100，max 10000）
        let cappedUserFreq = min(frequencyData.count, 100)
        let userFreqScore = cappedUserFreq * 100

        // Recency 加分（微調，最近 1 小時內用過 +200）
        let currentTime = Int64(Date().timeIntervalSince1970 * 1000)
        let oneHourMillis: Int64 = 60 * 60 * 1000
        let recencyBonus: Int
        if frequencyData.lastUsedMillis > 0 &&
            (currentTime - frequencyData.lastUsedMillis) < oneHourMillis {
            recencyBonus = 200
        } else {
            recencyBonus = 0
        }

        // 完全匹配加分（微調，+100）
        let exactBonus = (candidateRoman == normalizedInput) ? 100 : 0

        // 詞庫頻率（新詞 fallback，約 0-100）
        let baseFreqScore = (word.lengthScore ?? 0) / 10

        return userFreqScore + recencyBonus + exactBonus + baseFreqScore
    }

    /// 根據分數排序詞彙（與 Android 一致）
    /// - Parameters:
    ///   - words: 要排序的詞彙陣列
    ///   - normalizedInput: 正規化後的輸入（用於完全匹配判斷）
    ///   - frequencyDataMap: 詞彙頻率資料字典
    /// - Returns: 排序後的詞彙陣列
    static func sortByScore(
        _ words: [TaigiWord],
        normalizedInput: String,
        frequencyDataMap: [String: UserFrequencyService.FrequencyData]
    ) -> [TaigiWord] {
        words.sorted { word1, word2 in
            let freq1 = frequencyDataMap[word1.displayText] ?? .empty
            let freq2 = frequencyDataMap[word2.displayText] ?? .empty

            let score1 = calculateScore(word: word1, normalizedInput: normalizedInput, frequencyData: freq1)
            let score2 = calculateScore(word: word2, normalizedInput: normalizedInput, frequencyData: freq2)

            return score1 > score2
        }
    }

    /// 根據使用者頻率排序詞彙（舊版，保留向後相容）
    @available(*, deprecated, message: "Use sortByScore with frequencyDataMap instead")
    static func sortByScore(
        _ words: [TaigiWord],
        normalizedInput: String,
        frequencies: [String: Int]
    ) -> [TaigiWord] {
        // 轉換為 FrequencyData（無 lastUsed）
        let frequencyDataMap = frequencies.mapValues {
            UserFrequencyService.FrequencyData(count: $0, lastUsedMillis: 0)
        }
        return sortByScore(words, normalizedInput: normalizedInput, frequencyDataMap: frequencyDataMap)
    }

    /// 根據使用者頻率排序詞彙（舊版，保留向後相容）
    @available(*, deprecated, message: "Use sortByScore instead for consistency with Android")
    static func sortByFrequency(
        _ words: [TaigiWord],
        frequencies: [String: Int]
    ) -> [TaigiWord] {
        sortByScore(words, normalizedInput: "", frequencies: frequencies)
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
