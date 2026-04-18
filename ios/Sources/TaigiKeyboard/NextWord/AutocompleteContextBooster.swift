import Foundation

// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Reorders candidate words to surface those whose first character matches
/// a next-word prediction bigram. The predictions are computed elsewhere
/// (`NextWordService`) and passed in as a set of first-character strings,
/// so this type stays pure: no DB, no async, no side effects.
///
/// Moved here from `AutocompleteService.swift` to co-locate with NextWord.
enum AutocompleteContextBooster {
    /// - Parameters:
    ///   - words: 候選詞列表（已排序）
    ///   - predictedFirstChars: 由上一個選字的 bigram 預測出的首字集合
    /// - Returns: boosted 區段 + 其餘（各自保留原順序）
    static func boost(words: [TaigiWord], predictedFirstChars: Set<String>) -> [TaigiWord] {
        guard !predictedFirstChars.isEmpty else { return words }

        var boosted: [TaigiWord] = []
        var rest: [TaigiWord] = []
        for word in words {
            if let firstChar = word.displayText.first,
               predictedFirstChars.contains(String(firstChar))
            {
                boosted.append(word)
            } else {
                rest.append(word)
            }
        }
        return boosted + rest
    }
}
