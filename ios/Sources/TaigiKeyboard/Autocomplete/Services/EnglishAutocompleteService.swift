// English-mode autocomplete service — Apple UITextChecker only, no Taigi dictionary or composing.

import Foundation
import KeyboardKit
import UIKit

/// Provides English completions and spelling suggestions through Apple's `UITextChecker`.
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

    // Completions first; spelling guesses only when there are none. Three suggestions max either way.
    private func getSuggestions(for text: String) -> [AutocompleteSuggestion] {
        let currentWord = extractCurrentWord(from: text)
        guard !currentWord.isEmpty else { return [] }

        var suggestions: [AutocompleteSuggestion] = []

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

    /// Returns the word after the last space — the one currently being typed.
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
