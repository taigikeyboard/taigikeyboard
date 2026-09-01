// 中文: 候選詞顯示模式 — 漢羅並排(預設)/ 羅馬字。羅馬字模式下 isTranslateSwapped / outputBothScripts 的有效值強制為 false。

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
    ;

    /**
     * Effective value of a stored script flag (`isTranslateSwapped` /
     * `outputBothScripts`) under this mode. Roman-only suppresses both
     * without touching storage, so switching back restores the user's
     * stored choice.
     */
    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift isTranslateSwapped / isOutputBothScripts derivation.
    // Drift causes silent divergence (one platform keeps hanji-first commits under roman-only).
    fun effectiveScriptFlag(stored: Boolean): Boolean = stored && this != ROMAN_ONLY

    companion object {
        /** Coerce a stored raw string into a mode; unknown / absent values fall back to [SIDE_BY_SIDE]. */
        fun fromStorage(raw: String?): CandidateDisplayMode = entries.firstOrNull { it.storageValue == raw } ?: SIDE_BY_SIDE
    }
}
