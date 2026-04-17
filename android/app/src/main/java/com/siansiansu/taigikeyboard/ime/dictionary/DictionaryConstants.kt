package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode

/**
 * Constants for dictionary operations
 */
object DictionaryConstants {
    const val DEFAULT_SEARCH_LIMIT = 200
    const val SUBSYSTEM = "com.siansiansu.taigikeyboard"

    const val TRIE_PREFIX_TL = "tl:"
    const val TRIE_PREFIX_POJ = "poj:"
    const val TRIE_PREFIX_HANZI = "hanzi:"

    const val DICT_BIN_NAME = "dictionary.bin"
    const val ASSOC_BIN_NAME = "association.bin"

    fun triePrefix(mode: InputMode): String =
        when (mode) {
            InputMode.POJ -> TRIE_PREFIX_POJ
            else -> TRIE_PREFIX_TL
        }
}
