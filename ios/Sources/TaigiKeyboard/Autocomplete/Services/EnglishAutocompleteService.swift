import Foundation
import KeyboardKit
import UIKit

/// 英文自動完成服務
///
/// 使用 Apple 的 UITextChecker 提供英文自動完成與拼字建議。
/// 完全依賴 KeyboardKit 框架，不使用組字邏輯。
class EnglishAutocompleteService: KeyboardKit.AutocompleteService {

    // MARK: - Properties

    private let checker = UITextChecker()
    private let language: String

    // MARK: - KeyboardKit Protocol Properties

    var locale: Locale = .current

    var canIgnoreWords: Bool { false }
    var canLearnWords: Bool { false }

    var ignoredWords: [String] = []
    var learnedWords: [String] = []

    // MARK: - Initialization

    init(language: String = "en_US") {
        self.language = language
    }

    // MARK: - KeyboardKit Protocol Methods

    func autocomplete(_ text: String) async throws -> Autocomplete.Result {
        let suggestions = getSuggestions(for: text)
        return Autocomplete.Result(
            inputText: text,
            suggestions: suggestions
        )
    }

    func hasIgnoredWord(_ word: String) -> Bool { ignoredWords.contains(word) }
    func hasLearnedWord(_ word: String) -> Bool { learnedWords.contains(word) }

    func ignoreWord(_ word: String) { }
    func learnWord(_ word: String) { }
    func removeIgnoredWord(_ word: String) { }
    func unlearnWord(_ word: String) { }

    // MARK: - Private Helpers

    private func getSuggestions(for text: String) -> [Autocomplete.Suggestion] {
        let currentWord = extractCurrentWord(from: text)
        guard !currentWord.isEmpty else { return [] }

        var suggestions: [Autocomplete.Suggestion] = []

        // 取得當前單字的自動完成建議
        let range = NSRange(0..<currentWord.utf16.count)
        if let completions = checker.completions(
            forPartialWordRange: range,
            in: currentWord,
            language: language
        ) {
            suggestions = completions.prefix(3).map { completion in
                Autocomplete.Suggestion(text: completion)
            }
        }

        // 若無自動完成建議，嘗試拼字校正
        if suggestions.isEmpty {
            let misspelledRange = checker.rangeOfMisspelledWord(
                in: currentWord,
                range: range,
                startingAt: 0,
                wrap: false,
                language: language
            )

            if misspelledRange.location != NSNotFound {
                if let guesses = checker.guesses(
                    forWordRange: misspelledRange,
                    in: currentWord,
                    language: language
                ) {
                    suggestions = guesses.prefix(3).map { guess in
                        Autocomplete.Suggestion(text: guess)
                    }
                }
            }
        }

        return suggestions
    }

    /// 從輸入文字中提取當前單字（最後一個空白後的文字）
    private func extractCurrentWord(from text: String) -> String {
        let trimmed = text.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return "" }

        if let lastSpaceIndex = trimmed.lastIndex(of: " ") {
            let wordStartIndex = trimmed.index(after: lastSpaceIndex)
            return String(trimmed[wordStartIndex...])
        }

        return trimmed
    }
}
