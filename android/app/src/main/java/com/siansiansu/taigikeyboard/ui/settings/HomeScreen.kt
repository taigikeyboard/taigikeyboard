package com.siansiansu.taigikeyboard.ui.settings

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
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab1Texts
import com.siansiansu.taigikeyboard.ui.components.NavigationRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider

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
    onFaqClick: (titleKey: String, contentKeys: Array<String>) -> Unit
) {
    val language by languageManager.currentLanguageFlow.collectAsState()

    val chevronRight = Icons.AutoMirrored.Filled.KeyboardArrowRight
    val externalLink = Icons.AutoMirrored.Outlined.OpenInNew
    val featureIconTint = Color(0xFFFF9800)

    Surface(
        modifier = Modifier.fillMaxSize(),
        color = MaterialTheme.colorScheme.surfaceContainer
    ) {
        Column(modifier = Modifier.fillMaxSize()) {
            // Page title (pinned)
            Text(
                text = languageManager.text(Tab1Texts.appHeaderTitle),
                fontSize = 34.sp,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface,
                modifier = Modifier
                    .padding(horizontal = 20.dp)
                    .padding(top = 80.dp)
            )

            Spacer(Modifier.height(24.dp))

            // Scrollable content
            Column(
                modifier = Modifier
                    .weight(1f)
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 20.dp)
                    .padding(bottom = 40.dp)
            ) {

            // Section 1: Setup keyboard
            SectionHeader(languageManager.text(Tab1Texts.setupKeyboard))
            Spacer(Modifier.height(8.dp))

            SettingsCard {
                NavigationRow(
                    icon = painterResource(R.drawable.keyboard_24),
                    label = languageManager.text(Tab1Texts.setupGuide),
                    trailingIcon = chevronRight,
                    onClick = onSetupGuide
                )
            }

            Spacer(Modifier.height(32.dp))

            // Section 2: Features
            SectionHeader(languageManager.text(Tab1Texts.newFeatures))
            Spacer(Modifier.height(8.dp))

            SettingsCard {
                NavigationRow(
                    icon = painterResource(R.drawable.lightbulb_24),
                    label = languageManager.text(Tab1Texts.featureNextWord),
                    trailingIcon = chevronRight,
                    iconTint = featureIconTint,
                    onClick = {
                        onFeatureClick("feature_next_word", "feature", arrayOf("feature_next_word_detail"))
                    }
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.switches_24),
                    label = languageManager.text(Tab1Texts.featureVariant),
                    trailingIcon = chevronRight,
                    iconTint = featureIconTint,
                    onClick = {
                        onFeatureClick("feature_variant", "feature", arrayOf("feature_variant_detail"))
                    }
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.dictionary_24),
                    label = languageManager.text(Tab1Texts.featureCustomFont),
                    trailingIcon = chevronRight,
                    iconTint = featureIconTint,
                    onClick = {
                        onFeatureClick("feature_custom_font", "feature", arrayOf("feature_custom_font_detail"))
                    }
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_person_storage),
                    label = languageManager.text(Tab1Texts.featureUserDict),
                    trailingIcon = chevronRight,
                    iconTint = featureIconTint,
                    onClick = {
                        onFeatureClick("feature_user_dict", "feature", arrayOf("feature_user_dict_detail"))
                    }
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_text_format),
                    label = languageManager.text(Tab1Texts.featureCaseSwitch),
                    trailingIcon = chevronRight,
                    iconTint = featureIconTint,
                    onClick = {
                        onFeatureClick("feature_case_switch", "feature", arrayOf("feature_case_switch_detail"))
                    }
                )
            }

            Spacer(Modifier.height(32.dp))

            // Section 3: Resources & links
            SettingsCard {
                NavigationRow(
                    icon = painterResource(R.drawable.ic_globe),
                    label = languageManager.text(Tab1Texts.userGuide),
                    trailingIcon = externalLink,
                    onClick = { onUrlClick("https://www.taigikeyboard.tw/") }
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_privacy),
                    label = languageManager.text(Tab1Texts.privacyPolicy),
                    trailingIcon = externalLink,
                    onClick = { onUrlClick("https://taigikeyboard.tw/privacypolicy.html") }
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_star),
                    label = languageManager.text(Tab1Texts.rateUs),
                    trailingIcon = externalLink,
                    onClick = { onUrlClick("https://play.google.com/store/apps/details?id=com.siansiansu.taigikeyboard") }
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_document),
                    label = languageManager.text(Tab1Texts.copyrightNotice),
                    trailingIcon = chevronRight,
                    onClick = onCopyright
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_star),
                    label = languageManager.text(Tab1Texts.contactUs),
                    trailingIcon = chevronRight,
                    onClick = onFeedback
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_history),
                    label = languageManager.text(Tab1Texts.versionHistory),
                    trailingIcon = chevronRight,
                    onClick = onVersionHistory
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))

                // Version info row (not clickable)
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(
                        painter = painterResource(R.drawable.ic_info),
                        contentDescription = null,
                        modifier = Modifier.size(24.dp),
                        tint = MaterialTheme.colorScheme.primary
                    )
                    Spacer(Modifier.width(12.dp))
                    Text(
                        text = languageManager.text(Tab1Texts.version),
                        modifier = Modifier.weight(1f),
                        fontSize = 16.sp,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                    Text(
                        text = versionName,
                        fontSize = 16.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            Spacer(Modifier.height(32.dp))

            // Section 4: FAQ
            SectionHeader(languageManager.text(Tab1Texts.faq))
            Spacer(Modifier.height(8.dp))

            SettingsCard {
                NavigationRow(
                    icon = painterResource(R.drawable.keyboard_24),
                    label = languageManager.text(Tab1Texts.faq1Question),
                    trailingIcon = chevronRight,
                    onClick = {
                        onFaqClick("faq_1_question", arrayOf("faq_1_answer"))
                    }
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_email),
                    label = languageManager.text(Tab1Texts.faq2Question),
                    trailingIcon = chevronRight,
                    onClick = {
                        onFaqClick("faq_2_question", arrayOf("faq_2_answer"))
                    }
                )
                SettingsDivider(Modifier.padding(horizontal = 16.dp))
                NavigationRow(
                    icon = painterResource(R.drawable.ic_numbers),
                    label = languageManager.text(Tab1Texts.faq3Question),
                    trailingIcon = chevronRight,
                    onClick = {
                        onFaqClick("faq_3_question", arrayOf("faq_3_answer"))
                    }
                )
            }
            }
        }
    }
}

@Composable
private fun SectionHeader(text: String) {
    Text(
        text = text,
        fontSize = 18.sp,
        color = MaterialTheme.colorScheme.onSurfaceVariant
    )
}
