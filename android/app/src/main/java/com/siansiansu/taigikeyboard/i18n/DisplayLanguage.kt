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
 * - [Automatic] — [DisplayLanguage.SYSTEM]'s sentinel: a selection policy with NO authored strings.
 *   The resolver must never read it; the boundary maps it to a concrete language via
 *   [DisplayLanguage.effectiveLanguage] first. Modelled as its own case (not a Hanji sentinel) so a
 *   missed resolution fails fast instead of silently rendering Hanji.
 */
sealed interface StringResolution {
    data class Native(
        val bcp47: String,
    ) : StringResolution

    data object GeneratedMap : StringResolution

    data object Pseudo : StringResolution

    data object Automatic : StringResolution
}

/**
 * App UI display language — orthogonal to the keyboard input mode.
 *
 * Hanji, English, and Japanese are authored and user-selectable ([productionLanguages]). TL and POJ are
 * authored too but only DEBUG-selectable for now; they fall back to Hanji in release until a review-gated
 * promotion round adds them to [productionLanguages]. [PSEUDO] is a debug-only layout probe.
 *
 * [SYSTEM] (Automatic) is a selection POLICY, not a language: it has no authored strings and never
 * appears in [productionLanguages]. It is persisted (the user can return to it) and resolves to a
 * concrete authored language from the device OS locale via [effectiveLanguage] / [resolveAutomatic]
 * at the resolver boundary. Its [StringResolution.Automatic] resolution must be mapped to an effective
 * language BEFORE the resolver reads it.
 */
enum class DisplayLanguage(
    val tag: String,
    val resolution: StringResolution,
) {
    // SYSTEM leads so the picker (driven by selectableLanguages) offers Automatic first.
    SYSTEM("system", StringResolution.Automatic),
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
     * ios/Sources/TaigiKeyboard/Strings/DisplayLanguage.swift `endonym`. Drift causes silent divergence.
     *
     * [SYSTEM] has no endonym — it is not a language. The picker special-cases it and renders the i18n
     * key `settings.displayLanguageAutomatic` instead; calling [endonym] on it is a programmer error.
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
                SYSTEM -> error("system has no endonym; use settings.displayLanguageAutomatic")
            }

    /**
     * Resolves this selection to the language whose strings should actually render: [SYSTEM] maps to a
     * concrete authored language via [resolveAutomatic]; every other case is itself. [deviceLanguageSubtag]
     * is the lowercased device-OS language subtag (e.g. "ja", "zh", "en"), injected by the caller.
     */
    fun effectiveLanguage(deviceLanguageSubtag: String): DisplayLanguage =
        if (this == SYSTEM) resolveAutomatic(deviceLanguageSubtag) else this

    companion object {
        // Default tag persisted before the user ever picks a language. Keeps the app on Hanji (NOT system).
        const val DEFAULT_TAG = "hanji"

        /**
         * Authored, user-selectable production languages. Drives the picker's authored roster and clamps
         * [fromTag]. Grows by one entry when an authored language is promoted (TL/POJ are authored +
         * debug-selectable but await their review-gated promotion round). SEPARATE from
         * [selectableLanguages], which leads with the [SYSTEM] selection policy — [SYSTEM] has no strings
         * of its own, so it is NOT in this authored roster.
         * CROSS-PLATFORM INVARIANT (INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER) — mirrors
         * ios/Sources/TaigiKeyboard/Strings/DisplayLanguage.swift `productionLanguages`. Drift causes silent divergence.
         */
        val productionLanguages: List<DisplayLanguage> = listOf(HANJI, ENGLISH, JAPANESE)

        /**
         * What the picker offers: the [SYSTEM] (Automatic) selection policy first, then the authored
         * production roster, plus DEBUG-only previews — [TAILO] and [POJ] (authored but not yet production
         * languages; debug-selectable so a debug build can dogfood their strings + rendering before they
         * join [productionLanguages]) and the [PSEUDO] layout probe. Release builds offer only
         * [SYSTEM] + [productionLanguages]. Mirrors ios `selectableLanguages`.
         */
        val selectableLanguages: List<DisplayLanguage>
            get() {
                val base = listOf(SYSTEM) + productionLanguages
                return if (BuildConfig.DEBUG) base + TAILO + POJ + PSEUDO else base
            }

        /**
         * Resolves [SYSTEM]/Automatic to a concrete authored language from the device OS locale's
         * language subtag (lowercased): Japanese device → [JAPANESE], Chinese device → [HANJI],
         * everything else (incl. absent locale) → [ENGLISH]. Pure — the OS read happens at the call
         * site, so this stays unit-testable.
         */
        fun resolveAutomatic(deviceLanguageSubtag: String): DisplayLanguage =
            when {
                deviceLanguageSubtag.startsWith("ja") -> JAPANESE
                deviceLanguageSubtag.startsWith("zh") -> HANJI
                else -> ENGLISH
            }

        /**
         * Maps a persisted tag to a language, clamped to the currently-selectable set: an unknown tag or
         * one whose language is not user-selectable in this build (a "pseudo"/"tailo"/"poj" preview in a
         * release build) resolves to [HANJI], so the effective language always matches a picker option.
         * "system" is now selectable, so it round-trips to [SYSTEM]. The persisted tag itself is left
         * untouched, so it restores once that language ships.
         */
        fun fromTag(tag: String): DisplayLanguage =
            clampToSelectable(entries.firstOrNull { it.tag == tag } ?: HANJI, selectableLanguages)

        /**
         * Clamps a language to a selectable roster: a known case the roster does not offer (a debug-only
         * preview when given a release roster) falls back to [HANJI], so the effective language always
         * matches a picker option. Pure + roster-injectable so the release clamp — unreachable from a DEBUG
         * unit-test build, where every case is selectable — stays testable.
         * CROSS-PLATFORM INVARIANT — mirrors ios .../Strings/DisplayLanguage.swift `clampToSelectable`.
         */
        internal fun clampToSelectable(
            language: DisplayLanguage,
            selectable: List<DisplayLanguage>,
        ): DisplayLanguage = if (language in selectable) language else HANJI
    }
}
