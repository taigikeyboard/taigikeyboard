package com.siansiansu.taigikeyboard.model

// Tab1 content models (parsed from tab1-features.json / tab1-faq.json)

data class FeatureContent(
    val id: String,
    val title: String,
    val icon: PlatformIcon,
    val summary: String? = null,
    val paragraphs: List<FeatureParagraph>,
)

data class FeatureParagraph(
    val text: String,
    val attachment: ParagraphAttachment? = null,
)

data class PlatformIcon(
    val ios: String,
    val android: String,
)

sealed interface ParagraphAttachment {
    data class Slideshow(
        val images: List<String>,
        val interval: Double = 2.0,
    ) : ParagraphAttachment

    data class Image(
        val name: String,
    ) : ParagraphAttachment

    data class Link(
        val text: String,
        val url: String,
    ) : ParagraphAttachment

    data class Navigation(
        val text: String,
        val destination: String,
        val icon: PlatformIcon,
    ) : ParagraphAttachment
}
