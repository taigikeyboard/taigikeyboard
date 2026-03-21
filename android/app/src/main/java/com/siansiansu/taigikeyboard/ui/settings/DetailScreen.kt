package com.siansiansu.taigikeyboard.ui.settings

import androidx.annotation.DrawableRes
import androidx.compose.foundation.Image
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
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.LocalizedText
import com.siansiansu.taigikeyboard.localization.Tab1Texts
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

    val title = remember(language, titleKey) {
        getLocalizedTextByKey(titleKey)?.let { languageManager.text(it) } ?: ""
    }

    val items = remember(language, contentType, contentKeys) {
        buildDetailItems(contentType, contentKeys, languageManager)
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
            fontSize = 17.sp,
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
            Icon(
                painter = painterResource(iconResId),
                contentDescription = null,
                modifier = Modifier.size(24.dp),
                tint = MaterialTheme.colorScheme.primary
            )
            Spacer(Modifier.width(12.dp))
            Text(
                text = text,
                modifier = Modifier.weight(1f),
                fontSize = 17.sp,
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
            Icon(
                painter = painterResource(iconResId),
                contentDescription = null,
                modifier = Modifier.size(24.dp),
                tint = MaterialTheme.colorScheme.primary
            )
            Spacer(Modifier.width(12.dp))
            Text(
                text = text,
                modifier = Modifier.weight(1f),
                fontSize = 17.sp,
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
                    fontSize = 18.sp,
                    fontWeight = FontWeight.Bold,
                    fontFamily = fontFamily,
                    color = MaterialTheme.colorScheme.primary
                )
                Text(
                    text = date,
                    fontSize = 14.sp,
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
                        fontSize = 16.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(Modifier.width(8.dp))
                    Text(
                        text = languageManager.text(change),
                        fontSize = 16.sp,
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
            R.drawable.ic_star,
            "https://p.ecpay.com.tw/AA663DE"
        )
    )
}

private fun buildVersionItems(): List<DetailItem> {
    return Tab1Texts.versionHistoryEntries.map { entry ->
        DetailItem.VersionCard(entry.version, entry.date, entry.changes)
    }
}

private fun buildGenericItems(
    contentKeys: Array<String>,
    languageManager: LanguageManager
): List<DetailItem> {
    val items = mutableListOf<DetailItem>()

    contentKeys.forEach { key ->
        val paragraphs = getLocalizedTextListByKey(key)
        if (paragraphs != null) {
            paragraphs.forEachIndexed { index, localizedText ->
                // Special: next word feature - first paragraph with slideshow
                if (key == "feature_next_word_detail" && index == 0) {
                    items.add(DetailItem.Paragraph(localizedText))
                    items.add(DetailItem.Slideshow(
                        listOf(R.drawable.nextword_1, R.drawable.nextword_2, R.drawable.nextword_3)
                    ))
                } else {
                    items.add(DetailItem.Paragraph(localizedText))
                }

                // Special: variant feature - external link after first paragraph
                if (key == "feature_variant_detail" && index == 0) {
                    items.add(DetailItem.ExternalLink(
                        Tab1Texts.featureVariantDictLink,
                        R.drawable.ic_open_in_new,
                        "https://sutian.moe.edu.tw/zh-hant/siongkuantsuguan/"
                    ))
                }

                // Special: FAQ 1 - navigation to setup guide after first paragraph
                if (key == "faq_1_answer" && index == 0) {
                    items.add(DetailItem.NavigationLink(
                        Tab1Texts.goToSetupGuide,
                        R.drawable.keyboard_24,
                        "setup_guide"
                    ))
                }

                // Special: FAQ 2 - navigation to feedback after first paragraph
                if (key == "faq_2_answer" && index == 0) {
                    items.add(DetailItem.NavigationLink(
                        Tab1Texts.goToFeedback,
                        R.drawable.ic_email,
                        "feedback"
                    ))
                }

                // Special: FAQ 3 - screenshot after first paragraph
                if (key == "faq_3_answer" && index == 0) {
                    items.add(DetailItem.ImageCard(R.drawable.faq_tone_handling))
                }

                // Special: user dict - screenshot after second paragraph
                if (key == "feature_user_dict_detail" && index == 1) {
                    items.add(DetailItem.ImageCard(R.drawable.feature_userdict))
                }

                // Special: case switch - screenshots after each paragraph
                if (key == "feature_case_switch_detail") {
                    when (index) {
                        0 -> items.add(DetailItem.Slideshow(
                            listOf(R.drawable.case_shift_1, R.drawable.case_shift_2)
                        ))
                        1 -> items.add(DetailItem.ImageCard(R.drawable.case_lowercase))
                        2 -> items.add(DetailItem.ImageCard(R.drawable.case_capslock))
                    }
                }
            }
        } else {
            val localizedText = getLocalizedTextByKey(key)
            if (localizedText != null) {
                items.add(DetailItem.Paragraph(localizedText))
            }
        }
    }

    return items
}

private fun getLocalizedTextByKey(key: String): LocalizedText? {
    return when (key) {
        "feature_next_word" -> Tab1Texts.featureNextWord
        "feature_variant" -> Tab1Texts.featureVariant
        "feature_custom_font" -> Tab1Texts.featureCustomFont
        "feature_user_dict" -> Tab1Texts.featureUserDict
        "feature_case_switch" -> Tab1Texts.featureCaseSwitch
        "faq_1_question" -> Tab1Texts.faq1Question
        "faq_2_question" -> Tab1Texts.faq2Question
        "faq_3_question" -> Tab1Texts.faq3Question
        "contact_us" -> Tab1Texts.contactUs
        "feedback_email" -> Tab1Texts.emailContact
        "version_history" -> Tab1Texts.versionHistory
        else -> null
    }
}

private fun getLocalizedTextListByKey(key: String): List<LocalizedText>? {
    return when (key) {
        "feature_next_word_detail" -> Tab1Texts.featureNextWordParagraphs
        "feature_variant_detail" -> Tab1Texts.featureVariantParagraphs
        "feature_custom_font_detail" -> Tab1Texts.featureCustomFontParagraphs
        "feature_user_dict_detail" -> Tab1Texts.featureUserDictParagraphs
        "feature_case_switch_detail" -> Tab1Texts.featureCaseSwitchParagraphs
        "faq_1_answer" -> Tab1Texts.faq1Paragraphs
        "faq_2_answer" -> Tab1Texts.faq2Paragraphs
        "faq_3_answer" -> Tab1Texts.faq3Paragraphs
        else -> null
    }
}
