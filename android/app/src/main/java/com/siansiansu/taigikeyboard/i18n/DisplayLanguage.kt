// App UI display-language identity + per-language string-resolution strategy (i18n plan D2 hybrid).

package com.siansiansu.taigikeyboard.i18n

import com.siansiansu.taigikeyboard.BuildConfig

// Hanji has no OS locale; its strings live in the default res/values/. Pin a Taiwanese-Hanji BCP-47
// that matches NO resource-qualifier dir, so a context built from it always resolves the default
// (Hanji) set — even after P2 adds values-ja / values-en for other languages. Top-level (not in the
// companion) because an enum entry constructor cannot reference its own companion (uninitialized).
const val BCP47_HANJI = "nan-Hant-TW"

/**
 * How a [DisplayLanguage]'s strings are resolved (plan D2 hybrid):
 * - [Native] — backed by an Android resource set selected via a per-locale Context (en/ja/漢字).
 * - [GeneratedMap] — TL/POJ have no OS locale, so strings come from a generated Kotlin map.
 * - [Pseudo] — debug-only length-inflated layout probe, served from a generated map.
 */
sealed interface StringResolution {
    data class Native(
        val bcp47: String,
    ) : StringResolution

    data object GeneratedMap : StringResolution

    data object Pseudo : StringResolution
}

/**
 * App UI display language — orthogonal to the keyboard input mode.
 *
 * Hanji, English, and Japanese are authored and user-selectable ([productionLanguages]); the remaining
 * languages fall back to Hanji until their authoring phase populates them (P3b TL / P3c POJ) and they
 * join [productionLanguages]. [PSEUDO] is a debug-only layout probe, offered only in debug builds.
 *
 * `system` (Automatic) is deliberately absent — it is a locale-negotiation policy, not a string
 * set, deferred to a later round.
 */
enum class DisplayLanguage(
    val tag: String,
    val resolution: StringResolution,
) {
    HANJI("hanji", StringResolution.Native(BCP47_HANJI)),
    TAILO("tailo", StringResolution.GeneratedMap),
    POJ("poj", StringResolution.GeneratedMap),
    JAPANESE("ja", StringResolution.Native("ja")),
    ENGLISH("en", StringResolution.Native("en")),
    PSEUDO("pseudo", StringResolution.Pseudo),
    ;

    /**
     * The language's own name in its own script (endonym), shown in the picker regardless of the
     * current UI language (W3C-recommended) so a user can always find their language. Language-invariant,
     * so it is NOT an i18n key. The endonym strings MUST match across platforms.
     * CROSS-PLATFORM INVARIANT (INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER) — mirrors
     * ios/Sources/TaigiKeyboard/Strings/DisplayLanguage.swift:34 `endonym`. Drift causes silent divergence.
     */
    val endonym: String
        get() =
            when (this) {
                HANJI -> "漢字"
                TAILO -> "Tâi-lô"
                POJ -> "Pe̍h-ōe-jī"
                JAPANESE -> "日本語"
                ENGLISH -> "English"
                PSEUDO -> "PSEUDO · DEBUG"
            }

    companion object {
        // Default tag persisted before the user ever picks a language. Keeps the app on Hanji.
        const val DEFAULT_TAG = "hanji"

        /**
         * Authored, user-selectable production languages. Drives the Settings language picker and clamps
         * [fromTag]. Grows by one entry as each language's authoring phase lands (P3b TL / P3c POJ remain).
         * CROSS-PLATFORM INVARIANT (INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER) — mirrors
         * ios/Sources/TaigiKeyboard/Strings/DisplayLanguage.swift:68 `productionLanguages`. Drift causes silent divergence.
         */
        val productionLanguages: List<DisplayLanguage> = listOf(HANJI, ENGLISH, JAPANESE)

        /**
         * What the picker offers: the production roster, plus the [PSEUDO] layout probe in DEBUG only.
         * Release builds only ever offer [productionLanguages].
         */
        val selectableLanguages: List<DisplayLanguage>
            get() = if (BuildConfig.DEBUG) productionLanguages + PSEUDO else productionLanguages

        /**
         * Maps a persisted tag to a language, clamped to the currently-selectable set: an unknown tag or
         * one whose language is not yet user-selectable (a leftover "pseudo" in release, or a tl/poj
         * tag from a future build) resolves to [HANJI], so the effective language always matches a picker
         * option. The persisted tag itself is left untouched, so it restores once that language ships.
         */
        fun fromTag(tag: String): DisplayLanguage {
            val match = entries.firstOrNull { it.tag == tag } ?: HANJI
            return if (match in selectableLanguages) match else HANJI
        }
    }
}
