// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

// 字典來源 enum — 標示候選/搜尋結果的來源歸屬(教典/iTaigi/...)。
// bit 位置由 dictionary/common/source_bits.py 釘住,engine/lexicon/src/dictionary_reader.rs 讀取。

package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * Dictionary source enum used for search-result attribution.
 * (Bit positions are owned by `dictionary/common/source_bits.py` and read
 *  by `engine/lexicon/src/dictionary_reader.rs`; see
 *  docs/engine/binary-format.md §4.)
 *
 * The localized badge label for each source is resolved at the UI call site
 * (`DictionarySettingsComponents.tagKey()`), not stored here — keeps this
 * shared-core enum free of app i18n types.
 */
enum class DictionarySource {
    KAUTIAN,
    TAIGITV,
    ITAIGI,
    SITBUT,
    TAIHOA,
    TAIJIT,
    KUNGGE,
    STTI,
    KHPOO,
    KHIIN,
    LKK,
    DEV,
    CUSTOM,
}
