package com.siansiansu.taigikeyboard.model

import com.siansiansu.taigikeyboard.localization.LocalizedText

// Tab1 content models (parsed from tab1-features.json / tab1-faq.json)

data class FeatureContent(
    val id: String,
    val title: LocalizedText,
    val icon: PlatformIcon,
    val summary: LocalizedText? = null,
    val paragraphs: List<FeatureParagraph>
)

data class FeatureParagraph(
    val text: LocalizedText,
    val attachment: ParagraphAttachment? = null
)

data class PlatformIcon(
    val ios: String,
    val android: String
)

sealed interface ParagraphAttachment {
    data class Slideshow(
        val images: List<String>,
        val interval: Double = 2.0
    ) : ParagraphAttachment

    data class Image(
        val name: String
    ) : ParagraphAttachment

    data class Link(
        val text: LocalizedText,
        val url: String
    ) : ParagraphAttachment

    data class Navigation(
        val text: LocalizedText,
        val destination: String,
        val icon: PlatformIcon
    ) : ParagraphAttachment
}
