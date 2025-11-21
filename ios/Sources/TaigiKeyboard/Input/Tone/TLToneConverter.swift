import Foundation

/// TL (台羅) 模式聲調轉換器
enum TLToneConverter {

    /// 轉換 TL 輸入為聲調標記
    /// - Parameter input: 輸入字串（可包含多個音節，以連字號分隔）
    /// - Returns: 轉換後的字串
    static func convert(_ input: String) -> String {
        let syllables = input.components(separatedBy: "-")
        let converted = syllables.map { syllable in
            syllable.isEmpty ? "" : convertSyllable(syllable)
        }

        return converted.joined(separator: "-")
    }

    /// 轉換單一 TL 音節
    /// - Parameter syllable: 音節字串（例如：goa2）
    /// - Returns: 轉換後的音節（例如：goá）
    static func convertSyllable(_ syllable: String) -> String {
        guard let (baseForm, toneNumber) = POJToneConverter.extractToneNumber(from: syllable) else {
            return syllable
        }

        // 聲調 0, 1, 4 不標記
        if toneNumber == 0 || toneNumber == 1 || toneNumber == 4 {
            return baseForm
        }

        guard let vowelRange = VowelAnalyzer.findTLVowelRange(in: baseForm) else {
            return syllable
        }

        let vowel = String(baseForm[vowelRange])

        // TL 特殊處理：oo → o
        let actual = vowel == "oo" ? "o" : vowel

        let key = "\(actual)\(toneNumber)"
        guard let toned = ToneMappings.tlNumberToTone[key] else {
            return syllable
        }

        var result = baseForm

        // TL 特殊處理：只替換 oo 的第一個 o
        if vowel == "oo" {
            let startIndex = vowelRange.lowerBound
            let endIndex = baseForm.index(after: startIndex)
            result.replaceSubrange(startIndex ..< endIndex, with: toned)
        } else {
            result.replaceSubrange(vowelRange, with: toned)
        }

        return result
    }
}
