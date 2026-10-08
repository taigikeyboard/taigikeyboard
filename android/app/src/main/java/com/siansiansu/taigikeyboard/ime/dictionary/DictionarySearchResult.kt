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
    val id: Int,
    val roman: String, // Display form (POJ or TL based on user setting)
    val tl: String, // Raw TL from database (for Chhoe Taigi URL)
    val hanji: String?,
    val sources: List<DictionarySource>,
) {
    companion object {
        /** Sentinel id for rows synthesised from the user's custom dictionary. Mirrors iOS `customDictMarkerId`. */
        const val CUSTOM_DICT_MARKER_ID = -2
    }

    fun chhoeUrl(): String? = ExternalLookupURLBuilder.chhoeURL(tl)

    fun moeUrl(): String? = ExternalLookupURLBuilder.moeURL(tl)
}
