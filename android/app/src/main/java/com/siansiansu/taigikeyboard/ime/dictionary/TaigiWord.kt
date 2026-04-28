// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * Data class representing a Taigi word entry
 * @property id Database row ID
 * @property roman Romanized form with tone marks (POJ or TL)
 * @property hanzi Chinese characters representation (nullable)
 * @property lengthScore 詞庫頻率（frequency），用於排序。值越大代表越常用。
 * @property sourceBitmask u16 source-dictionary bitmask from `dictionary.bin`,
 *   `null` for non-dictionary sources (custom dict, autocomplete, spell-check).
 *   Consumed by `engine/ranking/src/score.rs::tier_numerator` (production
 *   path through `RustEngineBridge.processCandidates`) and the retained
 *   `CandidateProcessor.tierNumerator` JVM-test pin.
 */
data class TaigiWord(
    val id: Int,
    val roman: String,
    val hanzi: String?,
    val lengthScore: Int?,
    val sourceBitmask: Int? = null,
) {
    /**
     * Display text prioritizes hanzi over roman
     */
    val displayText: String
        get() = if (!hanzi.isNullOrEmpty()) hanzi else roman
}
