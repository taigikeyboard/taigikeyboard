package com.siansiansu.taigikeyboard.ui.tabs.home

import androidx.annotation.DrawableRes
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.content.ContentType
import com.siansiansu.taigikeyboard.content.FeatureContent
import com.siansiansu.taigikeyboard.content.ParagraphAttachment
import com.siansiansu.taigikeyboard.content.VersionHistory
import com.siansiansu.taigikeyboard.i18n.StringResolver
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.ui.components.resolveDrawableResId

// Detail screen data model and content builders for feature/FAQ/about-developer/version pages
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

    data class VersionCard(
        val version: String,
        val date: String,
        val changes: List<String>,
    ) : DetailItem
}

internal fun buildDetailItems(
    resolver: StringResolver,
    contentType: String,
    contentKeys: Array<String>,
): List<DetailItem> =
    when (contentType) {
        ContentType.ABOUT_DEVELOPER -> buildAboutDeveloperItems(resolver)
        ContentType.VERSION -> buildVersionItems()
        else -> buildGenericItems(resolver, contentKeys)
    }

internal fun buildContentItems(
    content: FeatureContent,
    context: android.content.Context,
): List<DetailItem> {
    val items = mutableListOf<DetailItem>()

    content.paragraphs.forEach { paragraph ->
        items.add(DetailItem.Paragraph(paragraph.text))

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
                        attachment.text,
                        R.drawable.ic_open_in_new,
                        attachment.url,
                    ),
                )
            }

            is ParagraphAttachment.Navigation -> {
                items.add(
                    DetailItem.NavigationLink(
                        attachment.text,
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
        ContentType.KEY_ABOUT_DEVELOPER -> resolver.resolve(StringKey.HOME_ABOUT_DEVELOPER)
        ContentType.KEY_VERSION_HISTORY -> resolver.resolve(StringKey.HOME_VERSION_HISTORY)
        else -> null
    }

private fun buildAboutDeveloperItems(resolver: StringResolver): List<DetailItem> =
    listOf(
        DetailItem.Paragraph(resolver.resolve(StringKey.HOME_FREE_PROMISE)),
        DetailItem.ExternalLink(
            resolver.resolve(StringKey.COMMON_VIEW_WEBSITE),
            R.drawable.ic_open_in_new,
            "https://www.taigikeyboard.tw/",
        ),
    )

private fun buildVersionItems(): List<DetailItem> =
    VersionHistory.entries.map { entry ->
        DetailItem.VersionCard(entry.version, entry.date, entry.changes)
    }

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
