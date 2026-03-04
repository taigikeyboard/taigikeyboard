package com.siansiansu.taigikeyboard.ui.settings

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.rememberVectorPainter
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab4Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SwitchRow
import androidx.compose.material3.Icon

@Composable
fun InputSettingsScreen(
    languageManager: LanguageManager,
    prefs: PrefHelper,
    onResetSettings: () -> Unit,
    onNavigateToDebug: () -> Unit,
    isDebugBuild: Boolean,
    resetCounter: Int
) {
    val language by languageManager.currentLanguageFlow.collectAsState()
    var showResetDialog by remember { mutableStateOf(false) }
    var showInputModePicker by remember { mutableStateOf(false) }

    // Force recomposition when resetCounter changes (after settings reset)
    val currentInputMode = remember(resetCounter) { prefs.inputMode }
    val currentOutputBoth = remember(resetCounter) { prefs.outputBothScripts }
    val currentAutoCap = remember(resetCounter) { prefs.autoCapitalizationEnabled }
    val currentAutoSpace = remember(resetCounter) { prefs.isAutoSpaceEnabled }
    val currentDoubleOO = remember(resetCounter) { prefs.enableDoubleTapOO }
    val currentDoubleNN = remember(resetCounter) { prefs.enableDoubleTapNN }

    var inputMode by remember(currentInputMode) { mutableStateOf(currentInputMode) }
    var outputBoth by remember(currentOutputBoth) { mutableStateOf(currentOutputBoth) }
    var autoCap by remember(currentAutoCap) { mutableStateOf(currentAutoCap) }
    var autoSpace by remember(currentAutoSpace) { mutableStateOf(currentAutoSpace) }
    var doubleOO by remember(currentDoubleOO) { mutableStateOf(currentDoubleOO) }
    var doubleNN by remember(currentDoubleNN) { mutableStateOf(currentDoubleNN) }

    if (showInputModePicker) {
        InputModeScreen(
            languageManager = languageManager,
            selectedMode = inputMode,
            onModeSelected = {
                inputMode = it
                prefs.inputMode = it
            },
            onBack = { showInputModePicker = false }
        )
    } else {
        Surface(
            modifier = Modifier.fillMaxSize(),
            color = MaterialTheme.colorScheme.surfaceContainerLow
        ) {
            Column(modifier = Modifier.fillMaxSize()) {
                // Page title (pinned)
                Text(
                    text = languageManager.text(Tab4Texts.tabTitle),
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

                // Input mode card - navigates to sub-page
                SettingsCard {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(min = 48.dp)
                            .clickable { showInputModePicker = true }
                            .padding(horizontal = 20.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = languageManager.text(Tab4Texts.inputMode),
                            modifier = Modifier.weight(1f),
                            fontSize = 16.sp,
                            color = MaterialTheme.colorScheme.onSurface
                        )
                        Text(
                            text = inputModeDisplayName(inputMode, languageManager),
                            fontSize = 16.sp,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Spacer(Modifier.width(8.dp))
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                            contentDescription = null,
                            modifier = Modifier.size(18.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }

                Spacer(Modifier.height(24.dp))

                // Settings switches card
                SettingsCard {
                    SwitchRow(
                        label = languageManager.text(Tab4Texts.outputBothScripts),
                        checked = outputBoth,
                        onCheckedChange = {
                            outputBoth = it
                            prefs.outputBothScripts = it
                        }
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = languageManager.text(Tab4Texts.autoCapitalization),
                        checked = autoCap,
                        onCheckedChange = {
                            autoCap = it
                            prefs.autoCapitalizationEnabled = it
                        }
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = languageManager.text(Tab4Texts.autoSpace),
                        checked = autoSpace,
                        onCheckedChange = {
                            autoSpace = it
                            prefs.isAutoSpaceEnabled = it
                        }
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = languageManager.text(Tab4Texts.doubleTapOO),
                        checked = doubleOO,
                        onCheckedChange = {
                            doubleOO = it
                            prefs.enableDoubleTapOO = it
                        }
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = languageManager.text(Tab4Texts.doubleTapNN),
                        checked = doubleNN,
                        onCheckedChange = {
                            doubleNN = it
                            prefs.enableDoubleTapNN = it
                        }
                    )
                }

                Spacer(Modifier.height(24.dp))

                // Reset settings card
                SettingsCard {
                    ActionRow(
                        label = languageManager.text(Tab4Texts.resetSettings),
                        onClick = { showResetDialog = true },
                        textColor = MaterialTheme.colorScheme.error
                    )
                }

                // Debug zone card (only visible in debug builds)
                if (isDebugBuild) {
                    Spacer(Modifier.height(24.dp))

                    SettingsCard {
                        ActionRow(
                            label = languageManager.text(Tab4Texts.debugMode),
                            onClick = onNavigateToDebug,
                            trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight
                        )
                    }
                }
                }
            }
        }

        if (showResetDialog) {
            AlertDialog(
                onDismissRequest = { showResetDialog = false },
                title = { Text(languageManager.text(Tab4Texts.resetSettings)) },
                text = { Text(languageManager.text(Tab4Texts.resetSettingsMessage)) },
                confirmButton = {
                    TextButton(onClick = {
                        showResetDialog = false
                        onResetSettings()
                    }) {
                        Text(languageManager.text(Tab4Texts.reset))
                    }
                },
                dismissButton = {
                    TextButton(onClick = { showResetDialog = false }) {
                        Text(languageManager.text(Tab4Texts.cancel))
                    }
                }
            )
        }
    }
}

private fun inputModeDisplayName(mode: String, languageManager: LanguageManager): String {
    return when (mode) {
        "poj" -> languageManager.text(Tab4Texts.pojMode)
        "tl" -> languageManager.text(Tab4Texts.tlMode)
        "english" -> languageManager.text(Tab4Texts.englishMode)
        else -> languageManager.text(Tab4Texts.tlMode)
    }
}
