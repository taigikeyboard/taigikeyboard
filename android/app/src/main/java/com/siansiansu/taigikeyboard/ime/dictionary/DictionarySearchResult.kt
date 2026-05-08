// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

// 中文: Tab3 字典查詢結果值型別 — 額外帶 sources(顯示哪些字典有此詞)。
// 中文: chhoeUrl()/moeUrl() 委派給 ExternalLookupURLBuilder。

package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * Search result with source information for dictionary exploration.
 *
 * URL helpers (`chhoeUrl()` / `moeUrl()`) delegate to
 * [ExternalLookupURLBuilder] so that URL-formatting logic has one home.
 */
data class DictionarySearchResult(
    val id: Int,
    val roman: String, // Display form (POJ or TL based on user setting)
    val tl: String, // Raw TL from database (for Chhoe Taigi URL)
    val hanzi: String?,
    val frequency: Int,
    val sources: List<DictionarySource>,
) {
    fun chhoeUrl(): String? = ExternalLookupURLBuilder.chhoeURL(tl)

    fun moeUrl(): String? = ExternalLookupURLBuilder.moeURL(tl)
}
