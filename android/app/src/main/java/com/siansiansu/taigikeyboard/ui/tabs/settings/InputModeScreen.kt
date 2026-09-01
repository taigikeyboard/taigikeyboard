package com.siansiansu.taigikeyboard.ui.tabs.settings

// Sub-screen for selecting the active input mode (POJ / TL / English / TPS).

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.theme.AppStyle

// Shared input-mode key→label-key pairs, used by InputModeScreen and InputSettingsScreen.
// Structural (no resolved strings) so it stays class-load safe; the label is resolved at render.
val inputModeOptions: List<Pair<String, StringKey>> =
    listOf(
        "poj" to StringKey.SETTINGS_POJ_MODE,
        "tl" to StringKey.SETTINGS_TL_MODE,
        "english" to StringKey.SETTINGS_ENGLISH_MODE,
        "tps" to StringKey.SETTINGS_TPS_MODE,
    )

@Composable
fun inputModeDisplayName(mode: String): String =
    stringRes(inputModeOptions.firstOrNull { it.first == mode }?.second ?: StringKey.SETTINGS_TL_MODE)

@Composable
fun InputModeScreen(
    selectedMode: String,
    onModeSelected: (String) -> Unit,
    onBack: () -> Unit,
) {
    SelectionListScreen(
        title = L10n.settingsInputMode,
        options = inputModeOptions,
        selected = selectedMode,
        onSelected = onModeSelected,
        onBack = onBack,
    )
}

// Single-choice sub-screen: one card of label rows, checkmark on the selected value.
// Shared by the input-mode and candidate-display-mode pickers.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun <T> SelectionListScreen(
    title: String,
    options: List<Pair<T, StringKey>>,
    selected: T,
    onSelected: (T) -> Unit,
    onBack: () -> Unit,
) {
    BackHandler(onBack = onBack)

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(title) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = L10n.commonBack,
                        )
                    }
                },
            )
        },
    ) { innerPadding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(innerPadding)
                    .padding(horizontal = 20.dp, vertical = 16.dp),
        ) {
            SettingsCard {
                options.forEachIndexed { index, (value, labelKey) ->
                    Row(
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .heightIn(min = 48.dp)
                                .clickable { onSelected(value) }
                                .padding(horizontal = 20.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = stringRes(labelKey),
                            modifier = Modifier.weight(1f),
                            color = MaterialTheme.colorScheme.onSurface,
                            style = MaterialTheme.typography.bodyLarge,
                        )
                        if (selected == value) {
                            Icon(
                                imageVector = Icons.Default.Check,
                                contentDescription = null,
                                modifier = Modifier.size(AppStyle.selectionIconSize),
                                tint = MaterialTheme.colorScheme.primary,
                            )
                        }
                    }
                    if (index < options.lastIndex) {
                        SettingsDivider()
                    }
                }
            }
        }
    }
}
