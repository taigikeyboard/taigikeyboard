package com.siansiansu.taigikeyboard.ui.tabs.tab1

import androidx.annotation.DrawableRes
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.content.ContentType
import com.siansiansu.taigikeyboard.content.FeatureContent
import com.siansiansu.taigikeyboard.content.ParagraphAttachment
import com.siansiansu.taigikeyboard.localization.Tab1Texts
import com.siansiansu.taigikeyboard.util.resolveDrawableResId

// Detail screen data model and content builders for feature/FAQ/feedback/version pages
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
    contentType: String,
    contentKeys: Array<String>,
): List<DetailItem> =
    when (contentType) {
        ContentType.FEEDBACK -> buildFeedbackItems()
        ContentType.VERSION -> buildVersionItems()
        else -> buildGenericItems(contentKeys)
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

internal fun getTextByKey(key: String): String? =
    when (key) {
        ContentType.KEY_CONTACT_US -> Tab1Texts.contactUs
        ContentType.KEY_FEEDBACK_EMAIL -> Tab1Texts.emailContact
        ContentType.KEY_VERSION_HISTORY -> Tab1Texts.versionHistory
        else -> null
    }

private fun buildFeedbackItems(): List<DetailItem> =
    listOf(
        DetailItem.Paragraph(Tab1Texts.emailContact),
        DetailItem.ExternalLink(
            Tab1Texts.supportUs,
            R.drawable.ic_open_in_new,
            "https://p.ecpay.com.tw/AA663DE",
        ),
        DetailItem.Paragraph(Tab1Texts.freePromise),
    )

private fun buildVersionItems(): List<DetailItem> =
    Tab1Texts.versionHistoryEntries.map { entry ->
        DetailItem.VersionCard(entry.version, entry.date, entry.changes)
    }

private fun buildGenericItems(contentKeys: Array<String>): List<DetailItem> {
    val items = mutableListOf<DetailItem>()

    contentKeys.forEach { key ->
        val text = getTextByKey(key)
        if (text != null) {
            items.add(DetailItem.Paragraph(text))
        }
    }

    return items
}
