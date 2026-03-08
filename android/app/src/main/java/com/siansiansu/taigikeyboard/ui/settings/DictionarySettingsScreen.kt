package com.siansiansu.taigikeyboard.ui.settings

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowForward
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab3Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SwitchRow

@Composable
fun DictionarySettingsScreen(
    languageManager: LanguageManager,
    prefs: PrefHelper,
    onClearCache: () -> Unit,
    onCustomDictionary: () -> Unit
) {
    // Observe language changes for reactive text updates
    val language by languageManager.currentLanguageFlow.collectAsState()
    var showClearDialog by remember { mutableStateOf(false) }

    Surface(
        modifier = Modifier.fillMaxSize(),
        color = MaterialTheme.colorScheme.surfaceContainerLow
    ) {
        Column(modifier = Modifier.fillMaxSize()) {
            // Page title (pinned)
            Text(
                text = languageManager.text(Tab3Texts.tabTitle),
                modifier = Modifier
                    .padding(horizontal = 20.dp)
                    .padding(top = 80.dp),
                fontSize = 34.sp,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface
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

            // Custom dictionary card (at top)
            SettingsCard {
                ActionRow(
                    label = languageManager.text(Tab3Texts.customDictionary),
                    onClick = onCustomDictionary,
                    trailingIcon = Icons.AutoMirrored.Filled.ArrowForward
                )
            }

            Spacer(Modifier.height(24.dp))

            // Dictionary switches card
            SettingsCard {
                DictionarySwitch(languageManager.text(Tab3Texts.moeDict), prefs.moeDictEnabled) {
                    prefs.moeDictEnabled = it
                }
                SettingsDivider()
                DictionarySwitch(languageManager.text(Tab3Texts.sttiDict), prefs.sttiDictEnabled) {
                    prefs.sttiDictEnabled = it
                }
                SettingsDivider()
                DictionarySwitch(languageManager.text(Tab3Texts.newwordDict), prefs.newwordDictEnabled) {
                    prefs.newwordDictEnabled = it
                }
                SettingsDivider()
                DictionarySwitch(languageManager.text(Tab3Texts.kunggeDict), prefs.kunggeDictEnabled) {
                    prefs.kunggeDictEnabled = it
                }
                SettingsDivider()
                DictionarySwitch(languageManager.text(Tab3Texts.iTaigiDict), prefs.itaigiDictEnabled) {
                    prefs.itaigiDictEnabled = it
                }
                SettingsDivider()
                DictionarySwitch(languageManager.text(Tab3Texts.taiwanJapanDict), prefs.taiwanJapanDictEnabled) {
                    prefs.taiwanJapanDictEnabled = it
                }
                SettingsDivider()
                DictionarySwitch(languageManager.text(Tab3Texts.taiHuaDict), prefs.taiHuaDictEnabled) {
                    prefs.taiHuaDictEnabled = it
                }
                SettingsDivider()
                DictionarySwitch(languageManager.text(Tab3Texts.taiwanPlantDict), prefs.taiwanPlantDictEnabled) {
                    prefs.taiwanPlantDictEnabled = it
                }
                SettingsDivider()
                DictionarySwitch(languageManager.text(Tab3Texts.khpooDict), prefs.khpooDictEnabled) {
                    prefs.khpooDictEnabled = it
                }
            }

            Spacer(Modifier.height(24.dp))

            // Variant dictionary card
            SettingsCard {
                SwitchRow(
                    label = languageManager.text(Tab3Texts.variantDictionary),
                    checked = prefs.variantEnabled,
                    onCheckedChange = { prefs.variantEnabled = it }
                )
            }

            Spacer(Modifier.height(24.dp))

            // Clear cache card
            SettingsCard {
                ActionRow(
                    label = languageManager.text(Tab3Texts.clearCache),
                    onClick = { showClearDialog = true },
                    textColor = MaterialTheme.colorScheme.error
                )
            }
            }
        }
    }

    if (showClearDialog) {
        AlertDialog(
            onDismissRequest = { showClearDialog = false },
            title = { Text(languageManager.text(Tab3Texts.clearCache)) },
            text = { Text(languageManager.text(Tab3Texts.clearCacheMessage)) },
            confirmButton = {
                TextButton(onClick = {
                    showClearDialog = false
                    onClearCache()
                }) {
                    Text(languageManager.text(Tab3Texts.clear))
                }
            },
            dismissButton = {
                TextButton(onClick = { showClearDialog = false }) {
                    Text(languageManager.text(Tab3Texts.cancel))
                }
            }
        )
    }
}

@Composable
private fun DictionarySwitch(
    label: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit
) {
    var isChecked by remember(checked) { mutableStateOf(checked) }
    SwitchRow(
        label = label,
        checked = isChecked,
        onCheckedChange = {
            isChecked = it
            onCheckedChange(it)
        }
    )
}
