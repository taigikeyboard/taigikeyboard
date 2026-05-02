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
    fun determineInputType(text: String): InputType =
        LexiconBridge.classifyInput(text).inputType
}
