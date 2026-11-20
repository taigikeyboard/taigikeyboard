// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * Search result with source information for dictionary exploration.
 *
 * URL helpers (`chhoeUrl()` / `moeUrl()`) delegate to
 * [ExternalLookupURLBuilder] so that URL-formatting logic has one home.
 */
data class DictionarySearchResult(
    val roman: String, // Display form (POJ or TL based on user setting)
    val tl: String, // TL the web dictionaries are asked with (a custom entry's own romanization)
    val hanji: String?,
    val sources: List<DictionarySource>,
) {
    fun chhoeUrl(): String? = ExternalLookupURLBuilder.chhoeURL(tl)

    fun moeUrl(): String? = ExternalLookupURLBuilder.moeURL(tl)
}
