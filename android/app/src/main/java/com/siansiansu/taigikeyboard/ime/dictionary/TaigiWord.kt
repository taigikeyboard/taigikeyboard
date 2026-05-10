// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

// 中文: 一筆候選詞值型別 —(id, roman, hanzi?, lengthScore?, sourceBitmask?)。
// 中文: displayText 漢字優先,其次羅馬字。對應 Rust protos::TaigiWord(平台 DTO 仍保留以避免每次橋接重建)。

package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * Data class representing a Taigi word entry
 * @property id Database row ID
 * @property roman Romanized form with tone marks (POJ or TL)
 * @property hanzi Chinese characters representation (nullable)
 * @property lengthScore 詞庫頻率（frequency），用於排序。值越大代表越常用。
 * @property sourceBitmask u16 source-dictionary bitmask from `dictionary.bin`,
 *   `null` for non-dictionary sources (custom dict, autocomplete, spell-check).
 *   Consumed by `engine/ranking/src/score.rs::tier_numerator` through
 *   `RustEngineBridge.processCandidates`.
 */
data class TaigiWord(
    val id: Int,
    val roman: String,
    val hanzi: String?,
    val lengthScore: Int?,
    val sourceBitmask: Int? = null,
    /**
     * Per-cell metadata sidechannel — mirror of iOS
     * `Autocomplete.Suggestion.additionalInfo: [String: String]`. Kept as a
     * separate map (not extra fields) so feature flags can grow the contract
     * without touching this DTO again. See [MetadataKeys] for the key dictionary.
     */
    val additionalInfo: Map<String, String> = emptyMap(),
) {
    /**
     * Display text prioritizes hanzi over roman
     */
    val displayText: String
        get() = if (!hanzi.isNullOrEmpty()) hanzi else roman

    /**
     * Reserved [additionalInfo] key strings shared between producers
     * ([com.siansiansu.taigikeyboard.ime.text.composing.TaigiAutocompleteService])
     * and consumers ([com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateClickHandler],
     * [com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarCandidateStrip]).
     * Adding a new feature → add a const here, do not sprinkle string literals.
     */
    object MetadataKeys {
        /** `"true"` on the slot-0 composing-text cell. Drives dashed-border affordance. */
        const val IS_COMPOSING_TEXT = "isComposingText"

        /** `"true"` on Continuous-mode candidate cells (slots 1..n). Routes tap to commitContinuous. */
        const val IS_CONTINUOUS = "isContinuous"

        /** Decimal-string `consumedSpanEnd` (UInt32) — passed verbatim to `commitContinuous`. */
        const val CONSUMED_BYTES = "consumedBytes"

        /** Decimal-string `syllableCount` (UInt32) — passed verbatim to `commitContinuous`. */
        const val SYLLABLE_COUNT = "syllableCount"

        /**
         * Engine-supplied raw display text. Read verbatim by the tap path —
         * protects `commitContinuous` alignment from any platform view-
         * rewrite of `roman` (e.g. TPS layout transforms).
         */
        const val DISPLAY_TEXT = "displayText"
    }
}
