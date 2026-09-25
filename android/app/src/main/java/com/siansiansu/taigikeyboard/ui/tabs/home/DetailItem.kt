package com.siansiansu.taigikeyboard.ui.tabs.home

import androidx.annotation.DrawableRes
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.content.ContentType
import com.siansiansu.taigikeyboard.content.FeatureContent
import com.siansiansu.taigikeyboard.content.ParagraphAttachment
import com.siansiansu.taigikeyboard.i18n.DisplayLanguage
import com.siansiansu.taigikeyboard.i18n.StringResolver
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.ui.components.resolveDrawableResId

// Detail screen data model and content builders for feature/FAQ/about-developer pages
sealed interface DetailItem {
    data class Paragraph(
        val text: String,
    ) : DetailItem

    data class ImageCard(
        @param:DrawableRes val imageResId: Int,
    ) : DetailItem

    data class Slideshow(
        val imageResIds: List<Int>,
        val intervalMs: Long = 1500L,
    ) : DetailItem

    data class NavigationLink(
        val text: String,
        @param:DrawableRes val iconResId: Int,
        val action: String,
    ) : DetailItem

    data class ExternalLink(
        val text: String,
        @param:DrawableRes val iconResId: Int,
        val url: String,
    ) : DetailItem

    data class Footnote(
        val text: String,
    ) : DetailItem
}

internal fun buildDetailItems(
    resolver: StringResolver,
    contentType: String,
    contentKeys: Array<String>,
): List<DetailItem> =
    when (contentType) {
        ContentType.ABOUT_DEVELOPER -> buildAboutKeyboardItems(resolver)
        else -> buildGenericItems(resolver, contentKeys)
    }

internal fun buildContentItems(
    content: FeatureContent,
    context: android.content.Context,
    language: DisplayLanguage,
): List<DetailItem> {
    val items = mutableListOf<DetailItem>()

    content.paragraphs.forEach { paragraph ->
        items.add(DetailItem.Paragraph(paragraph.text.resolve(language)))

        when (val attachment = paragraph.attachment) {
            is ParagraphAttachment.Slideshow -> {
                val resIds =
                    attachment.images
                        .map { name ->
                            resolveDrawableResId(context, name)
                        }.filter { it != 0 }
                if (resIds.isNotEmpty()) {
                    items.add(DetailItem.Slideshow(resIds, (attachment.interval * 1000).toLong()))
                }
            }

            is ParagraphAttachment.Image -> {
                val resId = resolveDrawableResId(context, attachment.name)
                if (resId != 0) {
                    items.add(DetailItem.ImageCard(resId))
                }
            }

            is ParagraphAttachment.Link -> {
                items.add(
                    DetailItem.ExternalLink(
                        attachment.text.resolve(language),
                        R.drawable.ic_open_in_new,
                        attachment.url,
                    ),
                )
            }

            is ParagraphAttachment.Navigation -> {
                items.add(
                    DetailItem.NavigationLink(
                        attachment.text.resolve(language),
                        resolveDrawableResId(context, attachment.icon.android, R.drawable.keyboard_24),
                        attachment.destination,
                    ),
                )
            }

            null -> { /* text only */ }
        }
    }

    return items
}

internal fun getTextByKey(
    resolver: StringResolver,
    key: String,
): String? =
    when (key) {
        ContentType.KEY_ABOUT_DEVELOPER -> resolver.resolve(StringKey.HOME_ABOUT_KEYBOARD)
        else -> null
    }

// The desktop About links without the desktop sponsor link: Google Play has no
// Billing exemption for an individual's donations (USER 2026-09-26).
private val aboutLinks =
    listOf(
        Triple(StringKey.HOME_WEBSITE_LINK, R.drawable.ic_globe, "https://taigikeyboard.tw"),
        Triple(StringKey.HOME_GITHUB_LINK, R.drawable.ic_github, "https://github.com/taigikeyboard"),
        Triple(StringKey.HOME_DISCORD_LINK, R.drawable.ic_discord, "https://discord.gg/kXhtQfWvK"),
        Triple(StringKey.HOME_EMAIL_LINK, R.drawable.ic_email, "mailto:info@taigikeyboard.tw"),
    )

private fun buildAboutKeyboardItems(resolver: StringResolver): List<DetailItem> =
    listOf(
        DetailItem.Paragraph(resolver.resolve(StringKey.HOME_ABOUT_INTRO_PROJECT)),
        DetailItem.Paragraph(resolver.resolve(StringKey.HOME_ABOUT_INTRO_MAINTAINER)),
    ) +
        aboutLinks.map { (key, icon, url) -> DetailItem.ExternalLink(resolver.resolve(key), icon, url) } +
        DetailItem.Footnote(resolver.resolve(StringKey.HOME_COPYRIGHT_LINE))

private fun buildGenericItems(
    resolver: StringResolver,
    contentKeys: Array<String>,
): List<DetailItem> {
    val items = mutableListOf<DetailItem>()

    contentKeys.forEach { key ->
        val text = getTextByKey(resolver, key)
        if (text != null) {
            items.add(DetailItem.Paragraph(text))
        }
    }

    return items
}
