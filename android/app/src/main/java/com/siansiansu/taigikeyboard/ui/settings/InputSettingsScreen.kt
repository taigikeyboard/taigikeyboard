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
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.Share
import androidx.compose.material.icons.outlined.Email
import androidx.compose.material.icons.outlined.OpenInNew
import com.siansiansu.taigikeyboard.ui.components.SettingsIcons
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.widget.Toast
import kotlinx.coroutines.launch
import com.siansiansu.taigikeyboard.diagnostics.DiagnosticService
import com.siansiansu.taigikeyboard.localization.DiagnosticTexts
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab4Texts
import com.siansiansu.taigikeyboard.model.FeatureContentLoader
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.SettingInfoButton
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SwitchRow
import androidx.compose.material3.Icon

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun InputSettingsScreen(
    languageManager: LanguageManager,
    prefs: PrefHelper,
    onResetSettings: () -> Unit,
    resetCounter: Int
) {
    val language by languageManager.currentLanguageFlow.collectAsState()
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
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
    val currentToolbarAutoCollapse = remember(resetCounter) { prefs.isToolbarAutoCollapse }
    var toolbarAutoCollapse by remember(currentToolbarAutoCollapse) { mutableStateOf(currentToolbarAutoCollapse) }
    val currentGlobeKey = remember(resetCounter) { prefs.isGlobeKeyEnabled }
    var isGlobeKeyEnabled by remember(currentGlobeKey) { mutableStateOf(currentGlobeKey) }
    val currentSoundFeedback = remember(resetCounter) { prefs.isSoundFeedbackEnabled }
    var soundFeedback by remember(currentSoundFeedback) { mutableStateOf(currentSoundFeedback) }
    val currentVibrationFeedback = remember(resetCounter) { prefs.isVibrationFeedbackEnabled }
    var vibrationFeedback by remember(currentVibrationFeedback) { mutableStateOf(currentVibrationFeedback) }
    val currentTpsOrMapsToER = remember(resetCounter) { prefs.tpsOrMapsToER }
    var tpsOrMapsToER by remember(currentTpsOrMapsToER) { mutableStateOf(currentTpsOrMapsToER) }

    val features = remember { FeatureContentLoader.loadFeatures(context) }
    fun featureSummary(id: String): String? =
        features.firstOrNull { it.id == id }?.summary?.let { languageManager.text(it) }

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
        val scrollBehavior = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()

        Scaffold(
            modifier = Modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
            containerColor = MaterialTheme.colorScheme.surfaceContainer,
            topBar = {
                LargeTopAppBar(
                    title = {
                        Text(
                            text = languageManager.text(Tab4Texts.tabTitle),
                            fontSize = AppStyle.pageTitleFontSize
                        )
                    },
                    expandedHeight = AppStyle.largeTopAppBarExpandedHeight,
                    colors = TopAppBarDefaults.largeTopAppBarColors(
                        containerColor = MaterialTheme.colorScheme.surfaceContainer,
                        scrolledContainerColor = MaterialTheme.colorScheme.surfaceContainer
                    ),
                    scrollBehavior = scrollBehavior
                )
            }
        ) { innerPadding ->
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding)
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
                            fontSize = AppStyle.bodyFontSize,
                            color = MaterialTheme.colorScheme.onSurface
                        )
                        Text(
                            text = inputModeDisplayName(inputMode, languageManager),
                            fontSize = AppStyle.bodyFontSize,
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

                // 拍字設定
                Text(
                    text = languageManager.text(Tab4Texts.typingSectionTitle),
                    fontSize = AppStyle.sectionHeaderFontSize,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, bottom = 6.dp)
                )
                SettingsCard {
                    SwitchRow(
                        label = languageManager.text(Tab4Texts.outputBothScripts),
                        checked = outputBoth,
                        infoText = featureSummary("hanloDesign"),
                        onCheckedChange = {
                            outputBoth = it
                            prefs.outputBothScripts = it
                        }
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = languageManager.text(Tab4Texts.autoCapitalization),
                        checked = autoCap,
                        infoText = featureSummary("caseSwitch"),
                        onCheckedChange = {
                            autoCap = it
                            prefs.autoCapitalizationEnabled = it
                        }
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = languageManager.text(Tab4Texts.autoSpace),
                        checked = autoSpace,
                        infoText = featureSummary("hanloDesign"),
                        onCheckedChange = {
                            autoSpace = it
                            prefs.isAutoSpaceEnabled = it
                        }
                    )
                }

                Spacer(Modifier.height(24.dp))

                // 齒盤設定
                Text(
                    text = languageManager.text(Tab4Texts.keyboardSectionTitle),
                    fontSize = AppStyle.sectionHeaderFontSize,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, bottom = 6.dp)
                )
                SettingsCard {
                    SwitchRow(
                        label = languageManager.text(Tab4Texts.toolbarAutoCollapse),
                        checked = toolbarAutoCollapse,
                        icon = SettingsIcons.toolbar,
                        infoText = languageManager.text(Tab4Texts.toolbarAutoCollapseInfo),
                        onCheckedChange = {
                            toolbarAutoCollapse = it
                            prefs.isToolbarAutoCollapse = it
                        }
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = languageManager.text(Tab4Texts.globeKey),
                        checked = isGlobeKeyEnabled,
                        icon = SettingsIcons.globe,
                        infoText = languageManager.text(Tab4Texts.globeKeyInfo),
                        onCheckedChange = {
                            isGlobeKeyEnabled = it
                            prefs.isGlobeKeyEnabled = it
                        }
                    )
                }

                Spacer(Modifier.height(24.dp))

                // Feedback settings card
                Text(
                    text = languageManager.text(Tab4Texts.feedbackSectionTitle),
                    fontSize = AppStyle.sectionHeaderFontSize,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, bottom = 6.dp)
                )
                SettingsCard {
                    SwitchRow(
                        label = languageManager.text(Tab4Texts.soundFeedback),
                        checked = soundFeedback,
                        icon = SettingsIcons.sound,
                        onCheckedChange = {
                            soundFeedback = it
                            prefs.isSoundFeedbackEnabled = it
                        }
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = languageManager.text(Tab4Texts.vibrationFeedback),
                        checked = vibrationFeedback,
                        icon = SettingsIcons.vibration,
                        onCheckedChange = {
                            vibrationFeedback = it
                            prefs.isVibrationFeedbackEnabled = it
                        }
                    )
                }

                Spacer(Modifier.height(24.dp))

                // POJ settings card
                Text(
                    text = languageManager.text(Tab4Texts.pojSettingsSectionTitle),
                    fontSize = AppStyle.sectionHeaderFontSize,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, bottom = 6.dp)
                )
                SettingsCard {
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

                // TPS settings card
                Text(
                    text = languageManager.text(Tab4Texts.tpsSettingsSectionTitle),
                    fontSize = AppStyle.sectionHeaderFontSize,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, bottom = 6.dp)
                )
                SettingsCard {
                    SwitchRow(
                        label = languageManager.text(Tab4Texts.tpsOrMapsToER),
                        checked = tpsOrMapsToER,
                        infoText = languageManager.text(Tab4Texts.tpsOrMapsToERInfo),
                        onCheckedChange = {
                            tpsOrMapsToER = it
                            prefs.tpsOrMapsToER = it
                        }
                    )
                }

                Spacer(Modifier.height(24.dp))

                // Diagnostic info card
                Text(
                    text = languageManager.text(DiagnosticTexts.sectionTitle),
                    fontSize = AppStyle.sectionHeaderFontSize,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, bottom = 6.dp)
                )
                SettingsCard {
                    ActionRow(
                        label = languageManager.text(DiagnosticTexts.copy),
                        icon = Icons.Outlined.ContentCopy,
                        onClick = {
                            scope.launch {
                                val info = DiagnosticService.gather(context)
                                val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                                clipboard.setPrimaryClip(
                                    ClipData.newPlainText("Taigi Keyboard Diagnostic", info.formatted())
                                )
                                Toast.makeText(context, languageManager.text(DiagnosticTexts.copied), Toast.LENGTH_SHORT).show()
                            }
                        }
                    )
                    SettingsDivider()
                    ActionRow(
                        label = languageManager.text(DiagnosticTexts.share),
                        icon = Icons.Outlined.OpenInNew,
                        textColor = MaterialTheme.colorScheme.primary,
                        onClick = {
                            scope.launch {
                                val info = DiagnosticService.gather(context)
                                val sendIntent = Intent().apply {
                                    action = Intent.ACTION_SEND
                                    putExtra(Intent.EXTRA_TEXT, info.formatted())
                                    type = "text/plain"
                                }
                                context.startActivity(Intent.createChooser(sendIntent, null))
                            }
                        }
                    )
                    SettingsDivider()
                    ActionRow(
                        label = languageManager.text(DiagnosticTexts.email),
                        icon = Icons.Outlined.OpenInNew,
                        textColor = MaterialTheme.colorScheme.primary,
                        onClick = {
                            scope.launch {
                                val info = DiagnosticService.gather(context)
                                val subject = Uri.encode("台語齒盤 Bug 回報 (v${info.appVersion})")
                                val body = Uri.encode(info.formatted())
                                val uri = Uri.parse("mailto:info@taigikeyboard.tw?subject=$subject&body=$body")
                                try {
                                    context.startActivity(Intent(Intent.ACTION_SENDTO, uri))
                                } catch (_: Exception) {
                                    Toast.makeText(context, "No email app found", Toast.LENGTH_SHORT).show()
                                }
                            }
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
        "tps" -> languageManager.text(Tab4Texts.tpsMode)
        else -> languageManager.text(Tab4Texts.tlMode)
    }
}
