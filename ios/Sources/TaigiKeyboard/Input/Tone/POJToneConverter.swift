import Foundation

/// POJ (白話字) 模式聲調轉換器
enum POJToneConverter {

    /// 轉換 POJ 輸入為聲調標記
    /// - Parameter input: 輸入字串（可包含多個音節，以連字號分隔）
    /// - Returns: 轉換後的字串
    static func convert(_ input: String) -> String {
        let syllables = input.components(separatedBy: "-")
        let converted = syllables.map { syllable in
            syllable.isEmpty ? "" : convertSyllable(syllable)
        }

        return converted.joined(separator: "-")
    }

    /// 轉換單一 POJ 音節
    /// - Parameter syllable: 音節字串（例如：goa2）
    /// - Returns: 轉換後的音節（例如：goá）
    static func convertSyllable(_ syllable: String) -> String {
        guard let (baseForm, toneNumber) = extractToneNumber(from: syllable) else {
            return syllable
        }

        // 聲調 0, 1, 4 不標記
        if toneNumber == 0 || toneNumber == 1 || toneNumber == 4 {
            return baseForm
        }

        guard let vowelRange = VowelAnalyzer.findPOJVowelRange(in: baseForm) else {
            return syllable
        }

        let vowel = String(baseForm[vowelRange])
        let key = "\(vowel)\(toneNumber)"

        guard let toned = ToneMappings.pojNumberToTone[key] else {
            return syllable
        }

        var result = baseForm
        result.replaceSubrange(vowelRange, with: toned)

        return result
    }

    /// 從音節中提取聲調數字
    /// - Parameter syllable: 音節字串
    /// - Returns: (基本形式, 聲調數字)，若無數字則聲調為 1
    static func extractToneNumber(from syllable: String) -> (baseForm: String, tone: Int)? {
        guard !syllable.isEmpty else { return (syllable, 1) }

        // 只接受最後一個字符為數字
        let lastChar = syllable.last!
        if let tone = Int(String(lastChar)), (1 ... 9).contains(tone) {
            let baseForm = String(syllable.dropLast())
            return (baseForm, tone)
        }

        // 若末尾不是數字，視為第一聲（無聲調標記）
        return (syllable, 1)
    }

    /// 預處理 POJ 輸入
    /// - Parameter input: 原始輸入
    /// - Returns: 預處理後的字串
    static func preprocess(_ input: String) -> String {
        var result = input
        let settings = SharedSettings.shared

        // oo → o͘ 轉換
        if settings.enableDoubleTapOO {
            result = result.replacingOccurrences(of: "oo", with: "o͘")
            result = result.replacingOccurrences(of: "Oo", with: "O͘")
            result = result.replacingOccurrences(of: "OO", with: "O͘")
        }

        // nn → ⁿ 轉換（只在 POJ 模式）
        if settings.enableDoubleTapNN {
            result = convertNN(result)
        }

        return result
    }

    /// 轉換 nn 為鼻化音標記
    /// - Parameter input: 輸入字串
    /// - Returns: 轉換後的字串
    static func convertNN(_ input: String) -> String {
        let vowels = "aeiouAEIOU"
        var result = ""
        let chars = Array(input)
        var i = 0

        while i < chars.count {
            if i + 2 < chars.count,
               vowels.contains(chars[i]),
               String(chars[i + 1]).lowercased() == "n",
               String(chars[i + 2]).lowercased() == "n"
            {
                result += String(chars[i]) + "ⁿ"
                i += 3

            } else {
                result += String(chars[i])
                i += 1
            }
        }

        return result
    }
}
