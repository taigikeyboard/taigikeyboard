// English-mode autocomplete service — Apple UITextChecker only, no Taigi dictionary or composing.

import Foundation
import KeyboardKit
import UIKit

/// Provides English completions and spelling suggestions through Apple's `UITextChecker`.
class EnglishAutocompleteService: EngineFreeAutocompleteService {
    // MARK: - Properties

    private let checker = UITextChecker()
    private let language: String

    // MARK: - Initialization

    init(language: String = "en_US") {
        self.language = language
    }

    // MARK: - KeyboardKit Protocol Methods

    func autocomplete(text: String, in _: Locale) async throws -> AutocompleteResult {
        let suggestions = getSuggestions(for: text)
        return AutocompleteResult(
            inputText: text,
            suggestions: suggestions,
        )
    }

    // MARK: - Private Helpers

    // Completions first; spelling guesses only when there are none. Three suggestions max either way.
    private func getSuggestions(for text: String) -> [AutocompleteSuggestion] {
        let currentWord = Self.extractCurrentWord(from: text)
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

    /// The word being typed at the end of `text`: what follows the last whitespace or `.,!?;:`
    /// once trailing whitespace is dropped. Apostrophe and hyphen do not split, so `don't` stays
    /// one token.
    /// CROSS-PLATFORM INVARIANT — mirrors Android `EnglishAutocompleteService.extractCurrentWord`.
    static func extractCurrentWord(from text: String) -> String {
        let word = text.reversed()
            .drop(while: { $0.isWhitespace })
            .prefix(while: { !$0.isWhitespace && !wordSeparators.contains($0) })
        return String(word.reversed())
    }

    private static let wordSeparators: Set<Character> = [".", ",", "!", "?", ";", ":"]
}
