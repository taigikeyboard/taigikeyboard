package com.siansiansu.taigikeyboard.content

import com.siansiansu.taigikeyboard.i18n.DisplayLanguage

// Home tab content models (parsed from features.json / faq.json shared assets)

internal const val DEFAULT_SLIDESHOW_INTERVAL_SECONDS = 2.0

/**
 * A localized content string from the shared JSON, e.g. `{"hanji": "...", "en": "...", "ja": "..."}`.
 * Only [hanji] is required; the other languages are authored later (C2). [resolve] returns the active
 * language's string, falling back to [hanji] for any language not yet authored — so until C2 fills the
 * keys, every effective language renders Hanji (display unchanged).
 */
data class LocalizedContentText(
    val hanji: String,
    val tailo: String? = null,
    val poj: String? = null,
    val ja: String? = null,
    val en: String? = null,
) {
    /**
     * The string for [language], falling back to [hanji] when that language is unauthored. [language]
     * is the EFFECTIVE display language (never SYSTEM) — render sites pass `LocalDisplayLanguage.current`,
     * already resolved away from SYSTEM. Mirrors StringResolver.resolve's fail-fast on SYSTEM.
     * CROSS-PLATFORM INVARIANT — mirrors
     * ios .../App/Tabs/Home/Models/FeatureContent.swift `LocalizedContentText.resolve(for:)`.
     * Drift causes silent divergence.
     */
    fun resolve(language: DisplayLanguage): String =
        when (language) {
            DisplayLanguage.HANJI -> hanji
            DisplayLanguage.TAILO -> tailo ?: hanji
            DisplayLanguage.POJ -> poj ?: hanji
            DisplayLanguage.JAPANESE -> ja ?: hanji
            DisplayLanguage.ENGLISH -> en ?: hanji
            DisplayLanguage.SYSTEM ->
                error("LocalizedContentText.resolve must receive an effective language, never SYSTEM")
        }
}

data class FeatureContent(
    val id: String,
    val title: LocalizedContentText,
    val icon: PlatformIcon,
    val summary: LocalizedContentText? = null,
    val paragraphs: List<FeatureParagraph>,
)

data class FeatureParagraph(
    val text: LocalizedContentText,
    val attachment: ParagraphAttachment? = null,
)

data class PlatformIcon(
    val ios: String,
    val android: String,
)

sealed interface ParagraphAttachment {
    data class Slideshow(
        val images: List<String>,
        val interval: Double = DEFAULT_SLIDESHOW_INTERVAL_SECONDS,
    ) : ParagraphAttachment

    data class Image(
        val name: String,
    ) : ParagraphAttachment

    data class Link(
        val text: LocalizedContentText,
        val url: String,
    ) : ParagraphAttachment

    data class Navigation(
        val text: LocalizedContentText,
        val destination: String,
        val icon: PlatformIcon,
    ) : ParagraphAttachment
}
