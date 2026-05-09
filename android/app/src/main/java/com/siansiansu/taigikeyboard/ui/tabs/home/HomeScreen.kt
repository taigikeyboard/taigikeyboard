package com.siansiansu.taigikeyboard.ui.tabs.home

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
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.content.ContentType
import com.siansiansu.taigikeyboard.content.FeatureContent
import com.siansiansu.taigikeyboard.content.FeatureContentLoader
import com.siansiansu.taigikeyboard.localization.HomeTexts
import com.siansiansu.taigikeyboard.ui.components.NavigationRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.resolveDrawableResId
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader

private val chevronRight = Icons.AutoMirrored.Filled.KeyboardArrowRight

// Home tab screen: setup guide, features, links, FAQ sections
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(
    versionName: String,
    onSetupGuide: () -> Unit,
    onFeatureClick: (titleKey: String, contentType: String, contentKeys: Array<String>) -> Unit,
    onUrlClick: (String) -> Unit,
    onCopyright: () -> Unit,
    onAboutDeveloper: () -> Unit,
    onVersionHistory: () -> Unit,
    onFaqClick: (titleKey: String, contentKeys: Array<String>) -> Unit,
) {
    val featureIconTint = AppStyle.warningOrange()

    val scrollBehavior = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()

    Scaffold(
        modifier = Modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            LargeTopAppBar(
                title = {
                    Text(
                        text = HomeTexts.appHeaderTitle,
                        style = MaterialTheme.typography.headlineLarge,
                    )
                },
                expandedHeight = AppStyle.largeTopAppBarExpandedHeight,
                colors =
                    TopAppBarDefaults.topAppBarColors(
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
                    .padding(bottom = AppStyle.scrollContentBottomPadding),
        ) {
            SectionHeader(HomeTexts.setupKeyboard)
            Spacer(Modifier.height(8.dp))

            SettingsCard {
                NavigationRow(
                    icon = painterResource(R.drawable.keyboard_24),
                    label = HomeTexts.setupGuide,
                    trailingIcon = chevronRight,
                    onClick = onSetupGuide,
                )
            }

            Spacer(Modifier.height(AppStyle.sectionSpacing))

            val context = LocalContext.current
            val features = remember { FeatureContentLoader.loadFeatures(context) }
            val typingGuideFeatures = features.take(6)
            val settingsFeatures = features.drop(6)

            SectionHeader(HomeTexts.typingGuide)
            Spacer(Modifier.height(8.dp))

            SettingsCard {
                FeatureList(
                    features = typingGuideFeatures,
                    context = context,
                    onFeatureClick = onFeatureClick,
                    iconTint = featureIconTint,
                )
            }

            Spacer(Modifier.height(AppStyle.sectionSpacing))

            SectionHeader(HomeTexts.newFeatures)
            Spacer(Modifier.height(8.dp))

            SettingsCard {
                FeatureList(
                    features = settingsFeatures,
                    context = context,
                    onFeatureClick = onFeatureClick,
                )
            }

            Spacer(Modifier.height(AppStyle.sectionSpacing))

            // Section 4: Resources & links
            val linkBlue = MaterialTheme.colorScheme.primary
            SettingsCard {
                NavigationRow(
                    icon = painterResource(R.drawable.ic_open_in_new),
                    label = HomeTexts.userGuide,
                    labelColor = linkBlue,
                    onClick = { onUrlClick("https://www.taigikeyboard.tw/") },
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_open_in_new),
                    label = HomeTexts.privacyPolicy,
                    labelColor = linkBlue,
                    onClick = { onUrlClick("https://taigikeyboard.tw/privacypolicy.html") },
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_open_in_new),
                    label = HomeTexts.rateUs,
                    labelColor = linkBlue,
                    onClick = {
                        onUrlClick(
                            "https://play.google.com/store/apps/details?id=com.siansiansu.taigikeyboard",
                        )
                    },
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_document),
                    label = HomeTexts.copyrightNotice,
                    trailingIcon = chevronRight,
                    onClick = onCopyright,
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_info),
                    label = HomeTexts.aboutDeveloper,
                    trailingIcon = chevronRight,
                    onClick = onAboutDeveloper,
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_history),
                    label = HomeTexts.versionHistory,
                    trailingIcon = chevronRight,
                    onClick = onVersionHistory,
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))

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
                        text = HomeTexts.version,
                        modifier = Modifier.weight(1f),
                        color = MaterialTheme.colorScheme.onSurface,
                        style = MaterialTheme.typography.bodyLarge,
                    )
                    Text(
                        text = versionName,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        style = MaterialTheme.typography.labelLarge,
                    )
                }
            }

            Spacer(Modifier.height(AppStyle.sectionSpacing))

            SectionHeader(HomeTexts.faq)
            Spacer(Modifier.height(8.dp))

            val faqs = remember { FeatureContentLoader.loadFAQs(context) }

            SettingsCard {
                faqs.forEachIndexed { index, faq ->
                    NavigationRow(
                        icon = painterResource(resolveDrawableResId(context, faq.icon.android, R.drawable.keyboard_24)),
                        label = faq.title,
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

@Composable
private fun FeatureList(
    features: List<FeatureContent>,
    context: android.content.Context,
    onFeatureClick: (String, String, Array<String>) -> Unit,
    iconTint: Color = MaterialTheme.colorScheme.primary,
) {
    features.forEachIndexed { index, feature ->
        NavigationRow(
            icon = painterResource(resolveDrawableResId(context, feature.icon.android, R.drawable.lightbulb_24)),
            label = feature.title,
            trailingIcon = chevronRight,
            iconTint = iconTint,
            onClick = { onFeatureClick(feature.id, ContentType.FEATURE, arrayOf(feature.id)) },
        )
        if (index < features.lastIndex) {
            SettingsDivider(Modifier.padding(horizontal = 16.dp))
        }
    }
}
