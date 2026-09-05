// 英打模式的 autocomplete service — 完全用 Apple UITextChecker,不走 Taigi 詞典與組字邏輯。

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

    var canIgnoreWords: Bool {
        false
    }

    var canLearnWords: Bool {
        false
    }

    var ignoredWords: [String] = []
    var learnedWords: [String] = []

    // MARK: - Initialization

    init(language: String = "en_US") {
        self.language = language
    }

    // MARK: - KeyboardKit Protocol Methods

    // KeyboardKit 入口 — 把 UITextChecker 的回傳包成 AutocompleteResult。
    func autocomplete(_ text: String) async throws -> AutocompleteResult {
        let suggestions = getSuggestions(for: text)
        return AutocompleteResult(
            inputText: text,
            suggestions: suggestions,
        )
    }

    func hasIgnoredWord(_ word: String) -> Bool {
        ignoredWords.contains(word)
    }

    func hasLearnedWord(_ word: String) -> Bool {
        learnedWords.contains(word)
    }

    func ignoreWord(_: String) {}
    func learnWord(_: String) {}
    func removeIgnoredWord(_: String) {}
    func unlearnWord(_: String) {}

    // MARK: - Private Helpers

    // 先用 UITextChecker 完成補全,沒結果再 fallback 拼字校正,各最多 3 個。
    private func getSuggestions(for text: String) -> [AutocompleteSuggestion] {
        let currentWord = extractCurrentWord(from: text)
        guard !currentWord.isEmpty else { return [] }

        var suggestions: [AutocompleteSuggestion] = []

        // 取得當前單字的自動完成建議
        let range = NSRange(0 ..< currentWord.utf16.count)
        if let completions = checker.completions(
            forPartialWordRange: range,
            in: currentWord,
            language: language,
        ) {
            suggestions = completions.prefix(3).map { completion in
                AutocompleteSuggestion(text: completion)
            }
        }

        // 若無自動完成建議，嘗試拼字校正
        if suggestions.isEmpty {
            let misspelledRange = checker.rangeOfMisspelledWord(
                in: currentWord,
                range: range,
                startingAt: 0,
                wrap: false,
                language: language,
            )

            if misspelledRange.location != NSNotFound {
                if let guesses = checker.guesses(
                    forWordRange: misspelledRange,
                    in: currentWord,
                    language: language,
                ) {
                    suggestions = guesses.prefix(3).map { guess in
                        AutocompleteSuggestion(text: guess)
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
