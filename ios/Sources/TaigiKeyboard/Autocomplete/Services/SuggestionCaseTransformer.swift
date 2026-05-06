import Foundation
import KeyboardKit

/// 候選詞大小寫轉換器
///
/// Thin per-word bridge over `RustEngineBridge.transformSuggestionCase`.
/// Skip rules (composing-text candidate / NextWord candidate) stay
/// platform-side via `additionalInfo` flags — only transform-eligible
/// suggestions reach the engine. Algorithm correctness lives in the Rust
/// crate (`engine/phonetics::case_transform`); see the slice audit doc.
enum SuggestionCaseTransformer {
    /// 根據 keyboardCase 轉換候選詞列表
    static func transform(
        _ suggestions: [Autocomplete.Suggestion],
        composingText: String,
        keyboardCase: Keyboard.KeyboardCase,
        inputMode: InputMode,
    ) -> [Autocomplete.Suggestion] {
        suggestions.map { suggestion in
            transformSuggestion(
                suggestion,
                composingText: composingText,
                keyboardCase: keyboardCase,
                inputMode: inputMode,
            )
        }
    }

    private static func transformSuggestion(
        _ suggestion: Autocomplete.Suggestion,
        composingText: String,
        keyboardCase: Keyboard.KeyboardCase,
        inputMode: InputMode,
    ) -> Autocomplete.Suggestion {
        // Skip rules — match Android `id < 0 && id != -2` numeric markers
        // via iOS's `additionalInfo` flag-based equivalent.
        if suggestion.additionalInfo["isComposingText"] == "true" {
            return suggestion
        }
        if suggestion.additionalInfo["isNextWord"] == "true" {
            return suggestion
        }

        let transformedText = RustEngineBridge.transformSuggestionCase(
            original: suggestion.text,
            composing: composingText,
            letterCase: keyboardCase.asLetterCase,
            mode: inputMode,
        )

        return Autocomplete.Suggestion(
            text: transformedText,
            title: transformedText,
            subtitle: suggestion.subtitle,
            additionalInfo: suggestion.additionalInfo,
        )
    }
}
