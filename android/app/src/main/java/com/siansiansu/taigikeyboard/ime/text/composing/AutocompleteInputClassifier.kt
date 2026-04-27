// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.ime.dictionary.InputNormalizer
import com.siansiansu.taigikeyboard.ime.dictionary.InputType

/**
 * Classify raw autocomplete input into an [InputType] tier.
 *
 * Mirrors `ios/Sources/TaigiKeyboard/Autocomplete/Services/AutocompleteInputClassifier.swift`
 * (intent + constants). Android only exposes `determineInputType` for now;
 * the iOS-style `classify(rawInput) -> Classification(inputType, searchKey)`
 * shape is deferred until `LexiconService.search` gains a `rawInput`
 * parameter — today the service re-runs TPS→TL inside its own pipeline,
 * so pre-converting the key at this layer would skip the TPS `er`↔`or`
 * variant expansion in `querySystemDictionaries` and break refactor-freeze.
 */
object AutocompleteInputClassifier {
    /**
     * Classification order matches iOS:
     * 1. Hanzi (CJK Unified + Extensions A–E) — short-circuit, no trie lookup.
     * 2. Diacritic tone marks (NFD combining marks).
     * 3. Numeric tone digit (2/3/5/6/7/8/9) — treat as toned.
     * 4. Otherwise romanization without tone.
     */
    fun determineInputType(text: String): InputType =
        when {
            isHanzi(text) -> InputType.Hanzi
            InputNormalizer.hasToneMarks(text) -> InputType.RomanWithTone
            containsNumericTone(text) -> InputType.RomanWithTone
            else -> InputType.RomanWithoutTone
        }

    /**
     * Digit is a tone marker unless it is `1`, `4`, or `0`.
     * Tones 1 and 4 are unmarked in Taiwanese (open syllable vs checked
     * ending); `0` is reserved for the slot-0 composing candidate.
     */
    private fun containsNumericTone(text: String): Boolean =
        text.any { char ->
            char.isDigit() && char != '1' && char != '4' && char != '0'
        }

    /**
     * True if `input` contains any CJK Unified Ideograph (or Extensions A–E).
     * Inlined from the deleted `ToneConverterModels.isHanzi` helper —
     * single consumer, kept private here.
     */
    private fun isHanzi(input: String): Boolean {
        var i = 0
        while (i < input.length) {
            val codePoint = Character.codePointAt(input, i)
            if (codePoint in 0x4E00..0x9FFF || // CJK Unified Ideographs
                codePoint in 0x3400..0x4DBF || // CJK Extension A
                codePoint in 0x20000..0x2A6DF || // CJK Extension B
                codePoint in 0x2A700..0x2B73F || // CJK Extension C
                codePoint in 0x2B740..0x2B81F || // CJK Extension D
                codePoint in 0x2B820..0x2CEAF // CJK Extension E
            ) {
                return true
            }
            i += Character.charCount(codePoint)
        }
        return false
    }
}
