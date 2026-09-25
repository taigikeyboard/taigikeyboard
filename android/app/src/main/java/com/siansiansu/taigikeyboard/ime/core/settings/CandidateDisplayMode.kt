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

    /**
     * Hanji with Romanization (§42 second exception): each hanji-bearing candidate lists
     * adjacent one-script Hanji + romanization cells, no subtitle; a tap commits
     * that cell's script. 文/A stays, as the punctuation-width toggle only.
     */
    COMBINED("combined"),
    ;

    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift isTranslateSwapped derivation.
    // Drift causes silent divergence (one platform commits roman under Hanji with Romanization, or hanji under roman-only).

    /**
     * Effective `isTranslateSwapped` under this mode. COMBINED forces `true`
     * — the pair is a compatibility projection of "cell leads with hanji,
     * commit writes hanji" (spec §1), not a claim about the stored flag.
     * ROMAN_ONLY forces `false`. Neither touches storage, so returning to
     * SIDE_BY_SIDE restores the user's stored choice.
     */
    fun effectiveTranslateSwapped(stored: Boolean): Boolean = this == COMBINED || (stored && showsHanji)

    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift isOutputBothScripts derivation.
    // Drift causes silent divergence (spurious bracket annotation under roman-only).

    /**
     * Effective `outputBothScripts` under this mode. Only ROMAN_ONLY
     * suppresses it; COMBINED keeps the stored value (bracket form becomes
     * `Hanji (romanization)`, same as today's swapped mode).
     */
    fun effectiveOutputBothScripts(stored: Boolean): Boolean = stored && showsHanji

    /** Whether the cell shows any hanji — false only for [ROMAN_ONLY]; also gates the Annotate in Brackets toggle's enabled state. */
    val showsHanji: Boolean get() = this != ROMAN_ONLY

    /**
     * Whether the 文/A key is shown (bottom row + expanded overlay) and its tap
     * writes the stored swap — exactly where hanji is on screen. Under COMBINED
     * the cells are split per script, so the key only picks the punctuation
     * width (USER 2026-09-13: "Hanji with Romanization needs the isTranslateSwapped button");
     * ROMAN_ONLY hides it and the stored swap waits for the way back.
     */
    val allowsSwapToggle: Boolean get() = showsHanji

    /**
     * Whether the character / symbol layouts type full-width punctuation
     * (`，。` over `,.`) for a stored swap flag — the stored flag masked like
     * Annotate in Brackets, NOT the candidate projection [effectiveTranslateSwapped], which
     * COMBINED forces on while 文/A still picks the width. TPS has its own JSON.
     * Mirrored on iOS / macOS / Windows beside [effectiveOutputBothScripts].
     */
    fun effectiveFullWidthPunctuation(stored: Boolean): Boolean = stored && showsHanji

    companion object {
        /** Coerce a stored raw string into a mode; unknown / absent values fall back to [SIDE_BY_SIDE]. */
        fun fromStorage(raw: String?): CandidateDisplayMode = entries.firstOrNull { it.storageValue == raw } ?: SIDE_BY_SIDE
    }
}
