import Foundation

/// 元音範圍分析器
///
/// 找出音節中需要標記聲調的元音位置，支援 POJ 和 TL 模式。
enum VowelAnalyzer {

    // MARK: - 公開介面

    /// 找出 POJ 音節中的元音範圍
    /// - Parameter syllable: 音節字串
    /// - Returns: 元音字符的範圍，若無元音則嘗試找半元音
    static func findPOJVowelRange(in syllable: String) -> Range<String.Index>? {
        let lowercased = syllable.lowercased()

        guard let lastVowel = findLastVowel(in: lowercased) else {
            return findSemivowelRange(syllable)
        }

        let hasPrevVowel = hasVowelBefore(lowercased, lastVowel)

        if !hasPrevVowel {
            return toOriginalRange(syllable, lowercased, lastVowel)
        }

        return findDiphthongPosition(syllable, lowercased)
    }

    /// 找出 TL 音節中的元音範圍
    /// - Parameter syllable: 音節字串
    /// - Returns: 元音字符的範圍
    static func findTLVowelRange(in syllable: String) -> Range<String.Index>? {
        let lowercased = syllable.lowercased()

        // 優先找 a
        if let aRange = lowercased.range(of: "a", options: .backwards) {
            return toOriginalRange(syllable, lowercased, aRange)
        }

        // 找 oo（台羅特有）
        if let ooRange = lowercased.range(of: "oo", options: .backwards) {
            return toOriginalRange(syllable, lowercased, ooRange)
        }

        // 找其他單元音
        let singleVowels = ["i", "u", "o", "e"]
        var lastVowelRange: Range<String.Index>? = nil
        var lastPosition = -1

        for vowel in singleVowels {
            if let range = lowercased.range(of: vowel, options: .backwards) {
                let position = lowercased.distance(from: lowercased.startIndex, to: range.lowerBound)
                if position > lastPosition {
                    lastPosition = position
                    lastVowelRange = range
                }
            }
        }

        if let vowelRange = lastVowelRange {
            return toOriginalRange(syllable, lowercased, vowelRange)
        }

        // 找半元音
        let semivowels = ["ng", "n", "m"]
        for semivowel in semivowels {
            if let range = lowercased.range(of: semivowel, options: .backwards) {
                return toOriginalRange(syllable, lowercased, range)
            }
        }

        return nil
    }

    // MARK: - 私有輔助方法

    /// 找出最後一個元音的範圍
    private static func findLastVowel(in text: String) -> Range<String.Index>? {
        let vowels = ["a", "i", "u", "e", "o͘", "o"]
        var lastRange: Range<String.Index>? = nil
        var lastPosition = -1

        for vowel in vowels {
            if let range = text.range(of: vowel, options: .backwards) {
                let position = text.distance(from: text.startIndex, to: range.lowerBound)
                if position > lastPosition {
                    lastPosition = position
                    lastRange = range
                }
            }
        }

        return lastRange
    }

    /// 檢查範圍前是否有元音
    private static func hasVowelBefore(_ text: String, _ range: Range<String.Index>) -> Bool {
        guard range.lowerBound > text.startIndex else { return false }

        let beforeIndex = text.index(before: range.lowerBound)
        let beforeChar = String(text[beforeIndex])

        return "aiueo͘o".contains(beforeChar.lowercased())
    }

    /// 找出複韻母的聲調位置
    private static func findDiphthongPosition(_ syllable: String, _ lowercased: String) -> Range<String.Index>? {
        let lastChar = getChar(lowercased, fromEnd: 1)
        let secondLastChar = getChar(lowercased, fromEnd: 2)

        let hasEnteringTone = "ptkh".contains(lastChar)

        if hasEnteringTone {
            if "iu".contains(secondLastChar) {
                if lowercased.contains("iuh") {
                    return findCharPosition(syllable, lowercased, fromEnd: 2)
                } else {
                    return findCharPosition(syllable, lowercased, fromEnd: 3)
                }
            } else {
                return findCharPosition(syllable, lowercased, fromEnd: 2)
            }
        } else {
            if secondLastChar == "i" {
                return findCharPosition(syllable, lowercased, fromEnd: 1)
            } else {
                return findCharPosition(syllable, lowercased, fromEnd: 2)
            }
        }
    }

    /// 從尾端計數取得字符
    private static func getChar(_ text: String, fromEnd position: Int) -> String {
        var pos = text.count - 1
        var charCount = 0

        while pos >= 0 {
            let index = text.index(text.startIndex, offsetBy: pos)
            let char = String(text[index])

            // 跳過鼻化音標記
            if char == "ⁿ" {
                pos -= 1
                continue
            }

            // 處理 o͘
            if char == "o", pos > 0 {
                let prevIndex = text.index(text.startIndex, offsetBy: pos - 1)
                if String(text[prevIndex]) == "o" {
                    charCount += 1
                    if charCount == position {
                        return "o"
                    }
                    pos -= 2
                    continue
                }
            }

            // 處理 ng
            if char == "g", pos > 0 {
                let prevIndex = text.index(text.startIndex, offsetBy: pos - 1)
                if String(text[prevIndex]) == "n" {
                    charCount += 1
                    if charCount == position {
                        return "ng"
                    }
                    pos -= 2
                    continue
                }
            }

            charCount += 1
            if charCount == position {
                return char
            }
            pos -= 1
        }

        return ""
    }

    /// 找出從尾端計數的字符位置
    private static func findCharPosition(_ syllable: String, _ lowercased: String, fromEnd position: Int) -> Range<String.Index>? {
        var pos = lowercased.count - 1
        var charCount = 0

        while pos >= 0 {
            let index = lowercased.index(lowercased.startIndex, offsetBy: pos)
            let char = String(lowercased[index])

            // 跳過鼻化音標記
            if char == "ⁿ" {
                pos -= 1
                continue
            }

            // 處理 o͘
            if char == "͘", pos > 0 {
                let prevIndex = lowercased.index(lowercased.startIndex, offsetBy: pos - 1)
                if String(lowercased[prevIndex]) == "o" {
                    charCount += 1
                    if charCount == position {
                        let startIdx = lowercased.index(lowercased.startIndex, offsetBy: pos - 1)
                        let endIdx = lowercased.index(lowercased.startIndex, offsetBy: pos + 1)
                        return toOriginalRange(syllable, lowercased, startIdx ..< endIdx)
                    }
                    pos -= 2
                    continue
                }
            }

            // 處理 oo
            if char == "o", pos > 0 {
                let prevIndex = lowercased.index(lowercased.startIndex, offsetBy: pos - 1)
                if String(lowercased[prevIndex]) == "o" {
                    charCount += 1
                    if charCount == position {
                        let startIdx = lowercased.index(lowercased.startIndex, offsetBy: pos - 1)
                        let endIdx = lowercased.index(lowercased.startIndex, offsetBy: pos + 1)
                        return toOriginalRange(syllable, lowercased, startIdx ..< endIdx)
                    }
                    pos -= 2
                    continue
                }
            }

            // 處理 ng
            if char == "g", pos > 0 {
                let prevIndex = lowercased.index(lowercased.startIndex, offsetBy: pos - 1)
                if String(lowercased[prevIndex]) == "n" {
                    charCount += 1
                    if charCount == position {
                        let startIdx = lowercased.index(lowercased.startIndex, offsetBy: pos - 1)
                        let endIdx = lowercased.index(lowercased.startIndex, offsetBy: pos + 1)
                        return toOriginalRange(syllable, lowercased, startIdx ..< endIdx)
                    }
                    pos -= 2
                    continue
                }
            }

            charCount += 1
            if charCount == position {
                let startIdx = lowercased.index(lowercased.startIndex, offsetBy: pos)
                let endIdx = lowercased.index(after: startIdx)
                return toOriginalRange(syllable, lowercased, startIdx ..< endIdx)
            }
            pos -= 1
        }

        return nil
    }

    /// 找出半元音範圍（當無元音時）
    private static func findSemivowelRange(_ syllable: String) -> Range<String.Index>? {
        let lowercased = syllable.lowercased()

        let semivowels = ["ⁿ", "ng", "n", "m"]
        for semivowel in semivowels {
            if let range = lowercased.range(of: semivowel, options: .backwards) {
                return toOriginalRange(syllable, lowercased, range)
            }
        }

        return nil
    }

    /// 將小寫範圍轉換為原始字串的對應範圍
    private static func toOriginalRange(
        _ syllable: String,
        _ lowercased: String,
        _ range: Range<String.Index>
    ) -> Range<String.Index>? {
        let start = lowercased.distance(from: lowercased.startIndex, to: range.lowerBound)
        let end = lowercased.distance(from: lowercased.startIndex, to: range.upperBound)

        guard start >= 0, end <= syllable.count else {
            return nil
        }

        let startIdx = syllable.index(syllable.startIndex, offsetBy: start)
        let endIdx = syllable.index(syllable.startIndex, offsetBy: end)

        return startIdx ..< endIdx
    }
}
