// 候選詞大小寫轉換的 per-word bridge。skip 規則(組字文字 / NextWord 候選)
// 留在平台端,真正轉換交給 RustEngineBridge.transformSuggestionCase。

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
        _ suggestions: [AutocompleteSuggestion],
        composingText: String,
        keyboardCase: Keyboard.KeyboardCase,
        inputMode: InputMode,
    ) -> [AutocompleteSuggestion] {
        suggestions.map { suggestion in
            transformSuggestion(
                suggestion,
                composingText: composingText,
                keyboardCase: keyboardCase,
                inputMode: inputMode,
            )
        }
    }

    // 對單一候選詞套用 case 轉換。先看 additionalInfo 跳過組字文字 / NextWord 候選,
    // 其餘交給 Rust bridge 處理。
    private static func transformSuggestion(
        _ suggestion: AutocompleteSuggestion,
        composingText: String,
        keyboardCase: Keyboard.KeyboardCase,
        inputMode: InputMode,
    ) -> AutocompleteSuggestion {
        // Skip rules — match Android `id < 0 && id != -2` numeric markers
        // via iOS's `additionalInfo` flag-based equivalent.
        if suggestion.additionalInfo["isComposingText"] == "true" {
            return suggestion
        }
        if suggestion.additionalInfo["isNextWord"] == "true" {
            return suggestion
        }
        // v3.5.8 §10.2 Opt 2A: Continuous candidates are already cased
        // per-segment from the user's own raw input by the engine
        // (`dispatch::recase_roman`, Model B). The legacy global-caps /
        // typed-prefix transform is invalid under Model B and would
        // clobber that, so bypass it for Continuous-flagged suggestions
        // (flagged at TaigiAutocompleteService.swift `buildContinuousSuggestions`).
        // §10.2 Opt 2A — Continuous 候選已由引擎依使用者 raw 逐段 case,
        // legacy 全域大寫轉換在 Model B 下無效,故跳過不再覆寫。
        // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/SuggestionCaseTransformer.kt IS_CONTINUOUS skip.
        // Drift causes silent divergence (continuous candidate re-cased away from raw).
        if suggestion.additionalInfo["isContinuous"] == "true" {
            return suggestion
        }

        let transformedText = RustEngineBridge.transformSuggestionCase(
            original: suggestion.text,
            composing: composingText,
            letterCase: keyboardCase.asLetterCase,
            mode: inputMode,
        )

        return AutocompleteSuggestion(
            text: transformedText,
            title: transformedText,
            subtitle: suggestion.subtitle,
            additionalInfo: suggestion.additionalInfo,
        )
    }
}
