// 候選大小寫轉換 thin wrapper — Path G 後保留為 platform skip-rule 執行器。
// skip 規則:composing-text 候選 (id == 0) 與 NextWord/English (id < 0 且 ≠ -2) 不轉;
// custom dict (id == -2) 仍會轉。實際大小寫邏輯在 Rust phonetics::case_transform。

package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.engine.CaseTransformBridge
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode

/**
 * 候選詞大小寫轉換器
 *
 * Thin per-word bridge over `CaseTransformBridge.transformSuggestion`.
 * Skip rules (composing-text candidate `id == 0`, NextWord/English
 * `id < 0 && id != -2`) stay platform-side via the existing numeric-id
 * markers — only transform-eligible suggestions reach the engine.
 * Algorithm correctness lives in the Rust crate
 * (`engine/phonetics::case_transform`); see the slice audit doc.
 */
object SuggestionCaseTransformer {
    fun transform(
        suggestions: List<TaigiWord>,
        composingText: String,
        caps: Boolean,
        capsLock: Boolean,
        inputMode: InputMode,
    ): List<TaigiWord> =
        suggestions.map { word ->
            transformWord(word, composingText, caps, capsLock, inputMode)
        }

    private fun transformWord(
        word: TaigiWord,
        composingText: String,
        caps: Boolean,
        capsLock: Boolean,
        inputMode: InputMode,
    ): TaigiWord {
        // v3.5.8 §10.2 Opt 2A: Continuous candidates are already cased
        // per-segment from the user's own raw input by the engine
        // (`dispatch::recase_roman`, Model B). They carry synthetic
        // id >= 1 so the numeric-id skips below do NOT cover them; the
        // legacy global-caps / typed-prefix transform is invalid under
        // Model B and would clobber the engine casing, so bypass it for
        // Continuous-flagged words (flagged at TaigiAutocompleteService
        // `buildContinuousSuggestionsForCandidates`).
        // §10.2 Opt 2A — Continuous 候選已由引擎依使用者 raw 逐段 case,
        // 合成 id>=1 不被下方數字 skip 蓋到,legacy 全域大寫在 Model B 下無效,故跳過。
        // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Autocomplete/Services/SuggestionCaseTransformer.swift isContinuous skip.
        // Drift causes silent divergence (continuous candidate re-cased away from raw).
        if (word.additionalInfo[TaigiWord.MetadataKeys.IS_CONTINUOUS] == "true") return word

        // NextWord / English suggestions (id < 0) skip transform,
        // but custom dictionary entries (id == -2) should be transformed.
        if (word.id < 0 && word.id != -2) return word

        // Composing text candidate (id == 0) doesn't need transform —
        // it's already the user's typed text.
        if (word.id == 0) return word

        val transformedRoman = CaseTransformBridge.transformSuggestion(
            original = word.roman,
            composing = composingText,
            letterCase = CaseTransformBridge.LetterCase.from(caps = caps, capsLock = capsLock),
            mode = inputMode,
        )
        return word.copy(roman = transformedRoman)
    }
}
