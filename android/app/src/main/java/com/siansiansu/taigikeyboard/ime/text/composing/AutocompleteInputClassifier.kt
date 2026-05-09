// 中文: Autocomplete 輸入分類器薄殼 — 把原始 keystroke 序列分類為
// 中文: Hanzi / RomanWithTone / RomanNoTone(InputType)。實際分類邏輯已在 Rust
// 中文: lexicon::classify_input;此檔僅將 LexiconBridge.classifyInput 的結果取出 inputType。

package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.engine.LexiconBridge
import com.siansiansu.taigikeyboard.ime.dictionary.InputType

/**
 * Classify raw autocomplete input into an [InputType] tier.
 *
 * Thin wrapper over [LexiconBridge.classifyInput]. Precedence contract:
 * `INVARIANT_LEX_INPUT_CLASSIFICATION_PRECEDENCE`.
 */
object AutocompleteInputClassifier {
    fun determineInputType(text: String): InputType = LexiconBridge.classifyInput(text).inputType
}
