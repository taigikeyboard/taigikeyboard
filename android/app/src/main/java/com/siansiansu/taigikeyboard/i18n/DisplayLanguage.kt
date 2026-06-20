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
 * R2a-1 is behaviour-frozen infra: only [HANJI] carries real strings; every other language
 * falls back to Hanji until its authoring phase populates it (P2 en / P3a ja / P3b TL / P3c POJ).
 * [PSEUDO] is a debug-only layout probe, never offered in the production picker.
 *
 * `system` (Automatic) is deliberately absent — it is a locale-negotiation policy, not a string
 * set, and is introduced with the picker in P2; R2a-1 keeps the app pinned to Hanji.
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

    companion object {
        // Default tag persisted before the user ever picks a language. Keeps the app on Hanji.
        const val DEFAULT_TAG = "hanji"

        /**
         * Maps a persisted tag to a language. Unknown tags fall back to [HANJI] (anti-crash).
         * A leftover "pseudo" tag from a debug build resolves to [HANJI] in release, so production
         * never renders the layout-probe strings.
         */
        fun fromTag(tag: String): DisplayLanguage {
            val match = entries.firstOrNull { it.tag == tag } ?: HANJI
            return if (match == PSEUDO && !BuildConfig.DEBUG) HANJI else match
        }
    }
}
