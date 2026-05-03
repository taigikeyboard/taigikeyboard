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
    ): List<TaigiWord> = suggestions.map { word ->
        transformWord(word, composingText, caps, capsLock, inputMode)
    }

    private fun transformWord(
        word: TaigiWord,
        composingText: String,
        caps: Boolean,
        capsLock: Boolean,
        inputMode: InputMode,
    ): TaigiWord {
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
