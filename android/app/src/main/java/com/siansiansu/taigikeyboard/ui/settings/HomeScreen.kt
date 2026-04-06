package com.siansiansu.taigikeyboard.ui.settings

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab1Texts
import com.siansiansu.taigikeyboard.model.FeatureContentLoader
import com.siansiansu.taigikeyboard.ui.components.NavigationRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(
    languageManager: LanguageManager,
    versionName: String,
    onSetupGuide: () -> Unit,
    onFeatureClick: (titleKey: String, contentType: String, contentKeys: Array<String>) -> Unit,
    onUrlClick: (String) -> Unit,
    onCopyright: () -> Unit,
    onFeedback: () -> Unit,
    onVersionHistory: () -> Unit,
    onFaqClick: (titleKey: String, contentKeys: Array<String>) -> Unit,
) {
    val language by languageManager.currentLanguageFlow.collectAsState()

    val chevronRight = Icons.AutoMirrored.Filled.KeyboardArrowRight
    val externalLink = Icons.AutoMirrored.Outlined.OpenInNew
    val featureIconTint = AppStyle.warningOrange()

    val scrollBehavior = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()

    Scaffold(
        modifier = Modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            LargeTopAppBar(
                title = {
                    Text(
                        text = languageManager.text(Tab1Texts.appHeaderTitle),
                        fontSize = AppStyle.pageTitleFontSize,
                    )
                },
                expandedHeight = AppStyle.largeTopAppBarExpandedHeight,
                colors =
                    TopAppBarDefaults.largeTopAppBarColors(
                        containerColor = MaterialTheme.colorScheme.surfaceContainer,
                        scrolledContainerColor = MaterialTheme.colorScheme.surfaceContainer,
                    ),
                scrollBehavior = scrollBehavior,
            )
        },
    ) { innerPadding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(innerPadding)
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 20.dp)
                    .padding(bottom = 40.dp),
        ) {
            // Section 1: Setup keyboard
            SectionHeader(languageManager.text(Tab1Texts.setupKeyboard))
            Spacer(Modifier.height(8.dp))

            SettingsCard {
                NavigationRow(
                    icon = painterResource(R.drawable.keyboard_24),
                    label = languageManager.text(Tab1Texts.setupGuide),
                    trailingIcon = chevronRight,
                    onClick = onSetupGuide,
                )
            }

            Spacer(Modifier.height(32.dp))

            val context = LocalContext.current
            val features = remember { FeatureContentLoader.loadFeatures(context) }
            val typingGuideFeatures = features.take(6)
            val settingsFeatures = features.drop(6)

            // Section 2: Typing guide (first 6 features)
            SectionHeader(languageManager.text(Tab1Texts.typingGuide))
            Spacer(Modifier.height(8.dp))

            SettingsCard {
                typingGuideFeatures.forEachIndexed { index, feature ->
                    val iconResId =
                        context.resources.getIdentifier(
                            feature.icon.android,
                            "drawable",
                            context.packageName,
                        )
                    NavigationRow(
                        icon = painterResource(if (iconResId != 0) iconResId else R.drawable.lightbulb_24),
                        label = languageManager.text(feature.title),
                        trailingIcon = chevronRight,
                        iconTint = featureIconTint,
                        onClick = {
                            onFeatureClick(feature.id, "feature", arrayOf(feature.id))
                        },
                    )
                    if (index < typingGuideFeatures.lastIndex) {
                        SettingsDivider(Modifier.padding(horizontal = 16.dp))
                    }
                }
            }

            Spacer(Modifier.height(32.dp))

            // Section 3: Features & settings (remaining features)
            SectionHeader(languageManager.text(Tab1Texts.newFeatures))
            Spacer(Modifier.height(8.dp))

            SettingsCard {
                settingsFeatures.forEachIndexed { index, feature ->
                    val iconResId =
                        context.resources.getIdentifier(
                            feature.icon.android,
                            "drawable",
                            context.packageName,
                        )
                    NavigationRow(
                        icon = painterResource(if (iconResId != 0) iconResId else R.drawable.lightbulb_24),
                        label = languageManager.text(feature.title),
                        trailingIcon = chevronRight,
                        onClick = {
                            onFeatureClick(feature.id, "feature", arrayOf(feature.id))
                        },
                    )
                    if (index < settingsFeatures.lastIndex) {
                        SettingsDivider(Modifier.padding(horizontal = 16.dp))
                    }
                }
            }

            Spacer(Modifier.height(32.dp))

            // Section 3: Resources & links
            val linkBlue = MaterialTheme.colorScheme.primary
            SettingsCard {
                NavigationRow(
                    icon = painterResource(R.drawable.ic_open_in_new),
                    label = languageManager.text(Tab1Texts.userGuide),
                    labelColor = linkBlue,
                    onClick = { onUrlClick("https://www.taigikeyboard.tw/") },
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_open_in_new),
                    label = languageManager.text(Tab1Texts.privacyPolicy),
                    labelColor = linkBlue,
                    onClick = { onUrlClick("https://taigikeyboard.tw/privacypolicy.html") },
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_open_in_new),
                    label = languageManager.text(Tab1Texts.rateUs),
                    labelColor = linkBlue,
                    onClick = { onUrlClick("https://play.google.com/store/apps/details?id=com.siansiansu.taigikeyboard") },
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_document),
                    label = languageManager.text(Tab1Texts.copyrightNotice),
                    trailingIcon = chevronRight,
                    onClick = onCopyright,
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_heart),
                    label = languageManager.text(Tab1Texts.contactUs),
                    trailingIcon = chevronRight,
                    onClick = onFeedback,
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_history),
                    label = languageManager.text(Tab1Texts.versionHistory),
                    trailingIcon = chevronRight,
                    onClick = onVersionHistory,
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))

                // Version info row (not clickable)
                Row(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        painter = painterResource(R.drawable.ic_info),
                        contentDescription = null,
                        modifier = Modifier.size(24.dp),
                        tint = MaterialTheme.colorScheme.primary,
                    )
                    Spacer(Modifier.width(12.dp))
                    Text(
                        text = languageManager.text(Tab1Texts.version),
                        modifier = Modifier.weight(1f),
                        fontSize = AppStyle.bodyFontSize,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Text(
                        text = versionName,
                        fontSize = AppStyle.captionFontSize,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            Spacer(Modifier.height(32.dp))

            // Section 4: FAQ (data-driven from tab1-faq.json)
            SectionHeader(languageManager.text(Tab1Texts.faq))
            Spacer(Modifier.height(8.dp))

            val faqs = remember { FeatureContentLoader.loadFAQs(context) }

            SettingsCard {
                faqs.forEachIndexed { index, faq ->
                    val iconResId =
                        context.resources.getIdentifier(
                            faq.icon.android,
                            "drawable",
                            context.packageName,
                        )
                    NavigationRow(
                        icon = painterResource(if (iconResId != 0) iconResId else R.drawable.keyboard_24),
                        label = languageManager.text(faq.title),
                        trailingIcon = chevronRight,
                        onClick = {
                            onFaqClick(faq.id, arrayOf(faq.id))
                        },
                    )
                    if (index < faqs.lastIndex) {
                        SettingsDivider(Modifier.padding(horizontal = 16.dp))
                    }
                }
            }
        }
    }
}
