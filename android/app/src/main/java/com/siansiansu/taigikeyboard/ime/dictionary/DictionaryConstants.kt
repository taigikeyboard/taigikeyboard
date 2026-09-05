// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

// 字典操作常數 — 預設搜尋上限、subsystem 名稱、trie key 前綴(tl:/poj:/hanzi:)、bin 檔名。
// triePrefix(mode) 依輸入模式選 prefix:POJ → poj:、其他(TL/TPS/English)→ tl:。

package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.core.settings.InputMode

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
