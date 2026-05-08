// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

// 中文: 字典查詢輸入類型 — RomanWithoutTone / RomanWithTone / Hanzi 三種。
// 中文: 對應 proto InputType。由 LexiconBridge.classifyInput 實際分類。

package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * Enum representing the type of input for dictionary search
 */
sealed class InputType {
    /** Search by romanization without tone marks (e.g., "goa") */
    object RomanWithoutTone : InputType()

    /** Search by romanization with tone marks (e.g., "góa") */
    object RomanWithTone : InputType()

    /** Search by Chinese characters (e.g., "我") */
    object Hanzi : InputType()
}
