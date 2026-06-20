package com.siansiansu.taigikeyboard.ui.tabs.home

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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.content.ContentType
import com.siansiansu.taigikeyboard.content.FeatureContentLoader
import com.siansiansu.taigikeyboard.i18n.LocalStringResolver
import com.siansiansu.taigikeyboard.ui.components.OpenInNew
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import kotlinx.coroutines.delay

// Home tab detail screen: renders content items in a scrollable list with top app bar
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DetailScreen(
    titleKey: String,
    contentType: String,
    contentKeys: Array<String>,
    fontFamily: FontFamily,
    onNavigationAction: (String) -> Unit,
    onExternalUrl: (String) -> Unit,
    onNavigateBack: () -> Unit,
) {
    val context = LocalContext.current
    val resolver = LocalStringResolver.current

    // Load from JSON for feature/faq; fall back to key-based resolution for other types
    val contentItem =
        remember(contentType, contentKeys) {
            if (contentKeys.isEmpty()) {
                null
            } else {
                when (contentType) {
                    ContentType.FEATURE -> FeatureContentLoader.loadFeatures(context).find { it.id == contentKeys[0] }
                    ContentType.FAQ -> FeatureContentLoader.loadFAQs(context).find { it.id == contentKeys[0] }
                    else -> null
                }
            }
        }

    val title =
        remember(titleKey, contentItem, resolver) {
            contentItem?.title
                ?: getTextByKey(resolver, titleKey)
                ?: ""
        }

    val items =
        remember(contentType, contentKeys, contentItem, resolver) {
            if (contentItem != null) {
                buildContentItems(contentItem, context)
            } else {
                buildDetailItems(resolver, contentType, contentKeys)
            }
        }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = title,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Back",
                            tint = MaterialTheme.colorScheme.onSurface,
                        )
                    }
                },
                colors =
                    TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.surfaceContainer,
                    ),
            )
        },
    ) { innerPadding ->
        LazyColumn(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(innerPadding),
            contentPadding =
                PaddingValues(
                    start = 20.dp,
                    end = 20.dp,
                    top = 16.dp,
                    bottom = AppStyle.scrollContentBottomPadding,
                ),
        ) {
            itemsIndexed(items) { index, item ->
                if (index > 0) {
                    Spacer(Modifier.height(12.dp))
                }
                DetailItemContent(
                    item = item,
                    fontFamily = fontFamily,
                    onNavigationAction = onNavigationAction,
                    onExternalUrl = onExternalUrl,
                )
            }
        }
    }
}

@Composable
private fun DetailItemContent(
    item: DetailItem,
    fontFamily: FontFamily,
    onNavigationAction: (String) -> Unit,
    onExternalUrl: (String) -> Unit,
) {
    when (item) {
        is DetailItem.Paragraph -> {
            ParagraphCard(
                text = item.text,
                fontFamily = fontFamily,
            )
        }

        is DetailItem.ImageCard -> {
            ImageCardContent(item.imageResId)
        }

        is DetailItem.Slideshow -> {
            SlideshowCard(
                imageResIds = item.imageResIds,
                intervalMs = item.intervalMs,
            )
        }

        is DetailItem.NavigationLink -> {
            LinkCard(
                text = item.text,
                iconResId = item.iconResId,
                trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                fontFamily = fontFamily,
                onClick = { onNavigationAction(item.action) },
            )
        }

        is DetailItem.ExternalLink -> {
            LinkCard(
                text = item.text,
                iconResId = item.iconResId,
                trailingIcon = Icons.AutoMirrored.Outlined.OpenInNew,
                trailingIconSize = AppStyle.smallIconSize,
                fontFamily = fontFamily,
                onClick = { onExternalUrl(item.url) },
            )
        }

        is DetailItem.VersionCard -> {
            VersionEntryCard(
                version = item.version,
                date = item.date,
                changes = item.changes,
                fontFamily = fontFamily,
            )
        }
    }
}

@Composable
private fun ParagraphCard(
    text: String,
    fontFamily: FontFamily,
) {
    SettingsCard {
        Text(
            text = text,
            modifier = Modifier.padding(16.dp),
            fontFamily = fontFamily,
            lineHeight = 24.sp,
            color = MaterialTheme.colorScheme.onSurface,
            style = MaterialTheme.typography.bodyLarge,
        )
    }
}

@Composable
private fun ImageCardContent(
    @DrawableRes imageResId: Int,
) {
    SettingsCard {
        Image(
            painter = painterResource(imageResId),
            contentDescription = null,
            modifier = Modifier.fillMaxWidth(),
            contentScale = ContentScale.FillWidth,
        )
    }
}

@Composable
private fun SlideshowCard(
    imageResIds: List<Int>,
    intervalMs: Long,
) {
    var currentIndex by remember(imageResIds) { mutableIntStateOf(0) }

    if (imageResIds.size > 1) {
        LaunchedEffect(imageResIds, intervalMs) {
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
            contentScale = ContentScale.FillWidth,
        )
    }
}

@Composable
private fun LinkCard(
    text: String,
    @DrawableRes iconResId: Int,
    trailingIcon: ImageVector,
    fontFamily: FontFamily,
    onClick: () -> Unit,
    trailingIconSize: Dp = AppStyle.trailingChevronSize,
) {
    SettingsCard {
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .clickable(onClick = onClick)
                    .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                painter = painterResource(iconResId),
                contentDescription = null,
                modifier = Modifier.size(24.dp),
                tint = MaterialTheme.colorScheme.primary,
            )
            Spacer(Modifier.width(12.dp))
            Text(
                text = text,
                modifier = Modifier.weight(1f),
                fontFamily = fontFamily,
                color = MaterialTheme.colorScheme.onSurface,
                style = MaterialTheme.typography.bodyLarge,
            )
            Icon(
                imageVector = trailingIcon,
                contentDescription = null,
                modifier = Modifier.size(trailingIconSize),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun VersionEntryCard(
    version: String,
    date: String,
    changes: List<String>,
    fontFamily: FontFamily,
) {
    SettingsCard {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = "v$version",
                    modifier = Modifier.weight(1f),
                    fontWeight = FontWeight.Bold,
                    fontFamily = fontFamily,
                    color = MaterialTheme.colorScheme.primary,
                    style = MaterialTheme.typography.bodyLarge,
                )
                Text(
                    text = date,
                    fontFamily = fontFamily,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    style = MaterialTheme.typography.labelLarge,
                )
            }

            Spacer(Modifier.height(12.dp))

            changes.forEach { change ->
                Row(modifier = Modifier.padding(bottom = 8.dp)) {
                    Text(
                        text = "\u2022",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        style = MaterialTheme.typography.bodyLarge,
                    )
                    Spacer(Modifier.width(8.dp))
                    Text(
                        text = change,
                        fontFamily = fontFamily,
                        color = MaterialTheme.colorScheme.onSurface,
                        style = MaterialTheme.typography.bodyLarge,
                    )
                }
            }
        }
    }
}
