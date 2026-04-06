package com.siansiansu.taigikeyboard.ui.settings

import androidx.annotation.DrawableRes
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.Image
import androidx.compose.ui.graphics.Color
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.R
import androidx.compose.ui.platform.LocalContext
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.LocalizedText
import com.siansiansu.taigikeyboard.localization.Tab1Texts
import com.siansiansu.taigikeyboard.model.FeatureContent
import com.siansiansu.taigikeyboard.model.FeatureContentLoader
import com.siansiansu.taigikeyboard.model.ParagraphAttachment
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import kotlinx.coroutines.delay

// Content item types for the detail screen
sealed interface DetailItem {
    data class Paragraph(val text: LocalizedText) : DetailItem
    data class ImageCard(@param:DrawableRes val imageResId: Int) : DetailItem
    data class Slideshow(val imageResIds: List<Int>, val intervalMs: Long = 1500L) : DetailItem
    data class NavigationLink(val text: LocalizedText, @param:DrawableRes val iconResId: Int, val action: String) : DetailItem
    data class ExternalLink(val text: LocalizedText, @param:DrawableRes val iconResId: Int, val url: String) : DetailItem
    data class VersionCard(val version: String, val date: String, val changes: List<LocalizedText>) : DetailItem
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DetailScreen(
    titleKey: String,
    contentType: String,
    contentKeys: Array<String>,
    languageManager: LanguageManager,
    fontFamily: FontFamily,
    onNavigationAction: (String) -> Unit,
    onExternalUrl: (String) -> Unit,
    onNavigateBack: () -> Unit
) {
    val language by languageManager.currentLanguageFlow.collectAsState()

    val context = LocalContext.current

    // For feature/faq content types, load from JSON; otherwise use existing key-based resolution
    val contentItem = remember(contentType, contentKeys) {
        if (contentKeys.isEmpty()) null
        else when (contentType) {
            "feature" -> FeatureContentLoader.loadFeatures(context).find { it.id == contentKeys[0] }
            "faq" -> FeatureContentLoader.loadFAQs(context).find { it.id == contentKeys[0] }
            else -> null
        }
    }

    val title = remember(language, titleKey, contentItem) {
        contentItem?.let { languageManager.text(it.title) }
            ?: getLocalizedTextByKey(titleKey)?.let { languageManager.text(it) }
            ?: ""
    }

    val items = remember(language, contentType, contentKeys, contentItem) {
        if (contentItem != null) {
            buildContentItems(contentItem, context)
        } else {
            buildDetailItems(contentType, contentKeys, languageManager)
        }
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = title,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Back",
                            tint = MaterialTheme.colorScheme.onSurface
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surfaceContainer
                )
            )
        }
    ) { innerPadding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding),
            contentPadding = PaddingValues(
                start = 20.dp,
                end = 20.dp,
                top = 16.dp,
                bottom = 40.dp
            )
        ) {
            itemsIndexed(items) { index, item ->
                if (index > 0) {
                    Spacer(Modifier.height(12.dp))
                }
                DetailItemContent(
                    item = item,
                    languageManager = languageManager,
                    fontFamily = fontFamily,
                    onNavigationAction = onNavigationAction,
                    onExternalUrl = onExternalUrl
                )
            }
        }
    }
}

@Composable
private fun DetailItemContent(
    item: DetailItem,
    languageManager: LanguageManager,
    fontFamily: FontFamily,
    onNavigationAction: (String) -> Unit,
    onExternalUrl: (String) -> Unit
) {
    when (item) {
        is DetailItem.Paragraph -> ParagraphCard(
            text = languageManager.text(item.text),
            fontFamily = fontFamily
        )
        is DetailItem.ImageCard -> ImageCardContent(item.imageResId)
        is DetailItem.Slideshow -> SlideshowCard(
            imageResIds = item.imageResIds,
            intervalMs = item.intervalMs
        )
        is DetailItem.NavigationLink -> NavigationLinkCard(
            text = languageManager.text(item.text),
            iconResId = item.iconResId,
            fontFamily = fontFamily,
            onClick = { onNavigationAction(item.action) }
        )
        is DetailItem.ExternalLink -> ExternalLinkCard(
            text = languageManager.text(item.text),
            iconResId = item.iconResId,
            fontFamily = fontFamily,
            onClick = { onExternalUrl(item.url) }
        )
        is DetailItem.VersionCard -> VersionEntryCard(
            version = item.version,
            date = item.date,
            changes = item.changes,
            languageManager = languageManager,
            fontFamily = fontFamily
        )
    }
}

@Composable
private fun ParagraphCard(text: String, fontFamily: FontFamily) {
    SettingsCard {
        Text(
            text = text,
            modifier = Modifier.padding(16.dp),
            fontSize = AppStyle.bodyFontSize,
            fontFamily = fontFamily,
            lineHeight = 24.sp,
            color = MaterialTheme.colorScheme.onSurface
        )
    }
}

@Composable
private fun ImageCardContent(@DrawableRes imageResId: Int) {
    SettingsCard {
        Image(
            painter = painterResource(imageResId),
            contentDescription = null,
            modifier = Modifier.fillMaxWidth(),
            contentScale = ContentScale.FillWidth
        )
    }
}

@Composable
private fun SlideshowCard(imageResIds: List<Int>, intervalMs: Long) {
    var currentIndex by remember { mutableIntStateOf(0) }

    if (imageResIds.size > 1) {
        LaunchedEffect(imageResIds) {
            while (true) {
                delay(intervalMs)
                currentIndex = (currentIndex + 1) % imageResIds.size
            }
        }
    }

    SettingsCard {
        Image(
            painter = painterResource(imageResIds[currentIndex]),
            contentDescription = null,
            modifier = Modifier.fillMaxWidth(),
            contentScale = ContentScale.FillWidth
        )
    }
}

@Composable
private fun NavigationLinkCard(
    text: String,
    @DrawableRes iconResId: Int,
    fontFamily: FontFamily,
    onClick: () -> Unit
) {
    SettingsCard {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clickable(onClick = onClick)
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            val iconBlue = MaterialTheme.colorScheme.primary
            Icon(
                painter = painterResource(iconResId),
                contentDescription = null,
                modifier = Modifier.size(24.dp),
                tint = iconBlue
            )
            Spacer(Modifier.width(12.dp))
            Text(
                text = text,
                modifier = Modifier.weight(1f),
                fontSize = AppStyle.bodyFontSize,
                fontFamily = fontFamily,
                color = MaterialTheme.colorScheme.onSurface
            )
            Icon(
                imageVector = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun ExternalLinkCard(
    text: String,
    @DrawableRes iconResId: Int,
    fontFamily: FontFamily,
    onClick: () -> Unit
) {
    SettingsCard {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clickable(onClick = onClick)
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            val iconBlue = MaterialTheme.colorScheme.primary
            Icon(
                painter = painterResource(iconResId),
                contentDescription = null,
                modifier = Modifier.size(24.dp),
                tint = iconBlue
            )
            Spacer(Modifier.width(12.dp))
            Text(
                text = text,
                modifier = Modifier.weight(1f),
                fontSize = AppStyle.bodyFontSize,
                fontFamily = fontFamily,
                color = MaterialTheme.colorScheme.onSurface
            )
            Icon(
                imageVector = Icons.AutoMirrored.Outlined.OpenInNew,
                contentDescription = null,
                modifier = Modifier.size(12.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun VersionEntryCard(
    version: String,
    date: String,
    changes: List<LocalizedText>,
    languageManager: LanguageManager,
    fontFamily: FontFamily
) {
    SettingsCard {
        Column(modifier = Modifier.padding(16.dp)) {
            // Version + date header
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "v$version",
                    modifier = Modifier.weight(1f),
                    fontSize = AppStyle.bodyFontSize,
                    fontWeight = FontWeight.Bold,
                    fontFamily = fontFamily,
                    color = MaterialTheme.colorScheme.primary
                )
                Text(
                    text = date,
                    fontSize = AppStyle.captionFontSize,
                    fontFamily = fontFamily,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            Spacer(Modifier.height(12.dp))

            // Change list
            changes.forEach { change ->
                Row(modifier = Modifier.padding(bottom = 8.dp)) {
                    Text(
                        text = "\u2022",
                        fontSize = AppStyle.bodyFontSize,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(Modifier.width(8.dp))
                    Text(
                        text = languageManager.text(change),
                        fontSize = AppStyle.bodyFontSize,
                        fontFamily = fontFamily,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }
            }
        }
    }
}

// --- Content building ---

private fun buildDetailItems(
    contentType: String,
    contentKeys: Array<String>,
    languageManager: LanguageManager
): List<DetailItem> {
    return when (contentType) {
        "feedback" -> buildFeedbackItems()
        "version" -> buildVersionItems()
        else -> buildGenericItems(contentKeys, languageManager)
    }
}

private fun buildFeedbackItems(): List<DetailItem> {
    return listOf(
        DetailItem.Paragraph(Tab1Texts.emailContact),
        DetailItem.ExternalLink(
            Tab1Texts.supportUs,
            R.drawable.ic_open_in_new,
            "https://p.ecpay.com.tw/AA663DE"
        ),
        DetailItem.Paragraph(Tab1Texts.freePromise)
    )
}

private fun buildVersionItems(): List<DetailItem> {
    return Tab1Texts.versionHistoryEntries.map { entry ->
        DetailItem.VersionCard(entry.version, entry.date, entry.changes)
    }
}

private fun buildContentItems(
    content: FeatureContent,
    context: android.content.Context
): List<DetailItem> {
    val items = mutableListOf<DetailItem>()

    content.paragraphs.forEach { paragraph ->
        items.add(DetailItem.Paragraph(paragraph.text))

        when (val attachment = paragraph.attachment) {
            is ParagraphAttachment.Slideshow -> {
                val resIds = attachment.images.map { name ->
                    context.resources.getIdentifier(name, "drawable", context.packageName)
                }.filter { it != 0 }
                if (resIds.isNotEmpty()) {
                    items.add(DetailItem.Slideshow(resIds, (attachment.interval * 1000).toLong()))
                }
            }
            is ParagraphAttachment.Image -> {
                val resId = context.resources.getIdentifier(
                    attachment.name, "drawable", context.packageName
                )
                if (resId != 0) {
                    items.add(DetailItem.ImageCard(resId))
                }
            }
            is ParagraphAttachment.Link -> {
                items.add(DetailItem.ExternalLink(
                    attachment.text,
                    R.drawable.ic_open_in_new,
                    attachment.url
                ))
            }
            is ParagraphAttachment.Navigation -> {
                val iconResId = context.resources.getIdentifier(
                    attachment.icon.android, "drawable", context.packageName
                )
                items.add(DetailItem.NavigationLink(
                    attachment.text,
                    if (iconResId != 0) iconResId else R.drawable.keyboard_24,
                    attachment.destination
                ))
            }
            null -> { /* text only */ }
        }
    }

    return items
}

private fun buildGenericItems(
    contentKeys: Array<String>,
    languageManager: LanguageManager
): List<DetailItem> {
    val items = mutableListOf<DetailItem>()

    contentKeys.forEach { key ->
        val localizedText = getLocalizedTextByKey(key)
        if (localizedText != null) {
            items.add(DetailItem.Paragraph(localizedText))
        }
    }

    return items
}

private fun getLocalizedTextByKey(key: String): LocalizedText? {
    return when (key) {
        "contact_us" -> Tab1Texts.contactUs
        "feedback_email" -> Tab1Texts.emailContact
        "version_history" -> Tab1Texts.versionHistory
        else -> null
    }
}

