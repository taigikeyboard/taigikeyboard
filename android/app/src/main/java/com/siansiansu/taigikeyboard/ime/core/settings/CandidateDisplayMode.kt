// 中文: 候選詞顯示模式 — 漢羅並排(預設)/ 羅馬字 / 漢羅合用。羅馬字模式下 isTranslateSwapped / outputBothScripts 的有效值強制為 false;漢羅合用下 isTranslateSwapped 強制為 true。

package com.siansiansu.taigikeyboard.ime.core.settings

/**
 * How candidate cells render in TL / POJ (TPS ignores the mode).
 *
 * Mirrors iOS `Settings/SettingsModels.swift` `CandidateDisplayMode`. Pure
 * Kotlin enum — no platform dependencies. [storageValue] is the raw string
 * persisted under `PreferenceKeys.CANDIDATE_DISPLAY_MODE`; identical across
 * platforms so a settings backup round-trips.
 */
enum class CandidateDisplayMode(
    val storageValue: String,
) {
    /** Today's two-line cell: title / subtitle, translate-swap decides which script leads. */
    SIDE_BY_SIDE("sideBySide"),

    /** Roman-only cell: the engine `roman` field alone, commit = roman. */
    ROMAN_ONLY("romanOnly"),

    /** One-label cell `漢字 羅馬字` (single space), commit = hanji; 文/A inert. */
    COMBINED("combined"),
    ;

    /**
     * Effective `isTranslateSwapped` under this mode. COMBINED forces `true`
     * — the pair is a compatibility projection of "cell leads with hanji,
     * commit writes hanji" (spec §1), not a claim about the stored flag.
     * ROMAN_ONLY forces `false`. Neither touches storage, so returning to
     * SIDE_BY_SIDE restores the user's stored choice.
     */
    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift isTranslateSwapped derivation.
    // Drift causes silent divergence (one platform commits roman under 漢羅合用, or hanji under roman-only).
    fun effectiveTranslateSwapped(stored: Boolean): Boolean = this == COMBINED || (stored && showsHanji)

    /**
     * Effective `outputBothScripts` under this mode. Only ROMAN_ONLY
     * suppresses it; COMBINED keeps the stored value (bracket form becomes
     * `漢字 (羅馬字)`, same as today's swapped mode).
     */
    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift isOutputBothScripts derivation.
    // Drift causes silent divergence (spurious bracket annotation under roman-only).
    fun effectiveOutputBothScripts(stored: Boolean): Boolean = stored && showsHanji

    /** Whether the cell shows any hanji — false only for [ROMAN_ONLY]; also gates the 括號標註 toggle's enabled state. */
    val showsHanji: Boolean get() = this != ROMAN_ONLY

    /** Only side-by-side has a lead script the 文/A key can flip; the other two fix it, so the key is inert. */
    val allowsSwapToggle: Boolean get() = this == SIDE_BY_SIDE

    companion object {
        /** Coerce a stored raw string into a mode; unknown / absent values fall back to [SIDE_BY_SIDE]. */
        fun fromStorage(raw: String?): CandidateDisplayMode = entries.firstOrNull { it.storageValue == raw } ?: SIDE_BY_SIDE
    }
}
