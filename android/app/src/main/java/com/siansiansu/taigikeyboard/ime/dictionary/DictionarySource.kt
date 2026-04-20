// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * Dictionary source enum used for search-result attribution.
 * (Bit positions are owned by EnabledDictionaries / DictionaryBinaryReader;
 *  see docs/engine/binary-format.md §4.)
 */
enum class DictionarySource(
    val displayName: String,
) {
    KAUTIAN("教典"),
    TAIGITV("台語新詞"),
    ITAIGI("iTaigi"),
    SITBUT("植物名彙"),
    TAIHOA("台華對照"),
    TAIJIT("臺日"),
    KUNGGE("工藝辭典"),
    STTI("學科術語"),
    KHPOO("補充資料"),
    KHIIN("補充資料"),
    LKK("漢羅合用"),
    DEV("補充資料"),
    CUSTOM("補充資料"),
}
