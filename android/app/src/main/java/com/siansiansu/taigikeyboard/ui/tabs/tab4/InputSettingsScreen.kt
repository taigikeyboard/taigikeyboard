package com.siansiansu.taigikeyboard.ui.tabs.tab4

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.widget.Toast
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.CommonTexts
import com.siansiansu.taigikeyboard.localization.Tab4Texts
import com.siansiansu.taigikeyboard.model.FeatureContentLoader
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.ConfirmationDialog
import com.siansiansu.taigikeyboard.ui.components.ContentCopy
import com.siansiansu.taigikeyboard.ui.components.OpenInNew
import com.siansiansu.taigikeyboard.ui.components.SettingInfoButton
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SettingsIcons
import com.siansiansu.taigikeyboard.ui.components.SwitchRow
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun InputSettingsScreen(
    prefs: PrefHelper,
    onResetSettings: () -> Unit,
    resetCounter: Int,
) {
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

    fun featureSummary(id: String): String? = features.firstOrNull { it.id == id }?.summary

    if (showInputModePicker) {
        InputModeScreen(
            selectedMode = inputMode,
            onModeSelected = {
                inputMode = it
                prefs.inputMode = it
            },
            onBack = { showInputModePicker = false },
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
                            text = Tab4Texts.tabTitle,
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
                // Input mode card - navigates to sub-page
                SettingsCard {
                    Row(
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .heightIn(min = 48.dp)
                                .clickable { showInputModePicker = true }
                                .padding(horizontal = 20.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = Tab4Texts.inputMode,
                            modifier = Modifier.weight(1f),
                            color = MaterialTheme.colorScheme.onSurface,
                            style = MaterialTheme.typography.bodyLarge,
                        )
                        Text(
                            text = inputModeDisplayName(inputMode),
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            style = MaterialTheme.typography.bodyLarge,
                        )
                        Spacer(Modifier.width(8.dp))
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                            contentDescription = null,
                            modifier = Modifier.size(AppStyle.trailingChevronSize),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }

                Spacer(Modifier.height(24.dp))

                // Typing settings
                SectionHeader(Tab4Texts.typingSectionTitle)
                SettingsCard {
                    SwitchRow(
                        label = Tab4Texts.outputBothScripts,
                        checked = outputBoth,
                        infoText = featureSummary("hanloDesign"),
                        onCheckedChange = {
                            outputBoth = it
                            prefs.outputBothScripts = it
                        },
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = Tab4Texts.autoCapitalization,
                        checked = autoCap,
                        infoText = featureSummary("caseSwitch"),
                        onCheckedChange = {
                            autoCap = it
                            prefs.autoCapitalizationEnabled = it
                        },
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = Tab4Texts.autoSpace,
                        checked = autoSpace,
                        infoText = featureSummary("hanloDesign"),
                        onCheckedChange = {
                            autoSpace = it
                            prefs.isAutoSpaceEnabled = it
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

                // Keyboard settings
                SectionHeader(Tab4Texts.keyboardSectionTitle)
                SettingsCard {
                    SwitchRow(
                        label = Tab4Texts.toolbarAutoCollapse,
                        checked = toolbarAutoCollapse,
                        icon = SettingsIcons.toolbar,
                        infoText = Tab4Texts.toolbarAutoCollapseInfo,
                        onCheckedChange = {
                            toolbarAutoCollapse = it
                            prefs.isToolbarAutoCollapse = it
                        },
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = Tab4Texts.globeKey,
                        checked = isGlobeKeyEnabled,
                        icon = SettingsIcons.globe,
                        infoText = Tab4Texts.globeKeyInfo,
                        onCheckedChange = {
                            isGlobeKeyEnabled = it
                            prefs.isGlobeKeyEnabled = it
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

                // Feedback settings card
                SectionHeader(Tab4Texts.feedbackSectionTitle)
                SettingsCard {
                    SwitchRow(
                        label = Tab4Texts.soundFeedback,
                        checked = soundFeedback,
                        icon = SettingsIcons.sound,
                        onCheckedChange = {
                            soundFeedback = it
                            prefs.isSoundFeedbackEnabled = it
                        },
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = Tab4Texts.vibrationFeedback,
                        checked = vibrationFeedback,
                        icon = SettingsIcons.vibration,
                        onCheckedChange = {
                            vibrationFeedback = it
                            prefs.isVibrationFeedbackEnabled = it
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

                // POJ settings card
                SectionHeader(Tab4Texts.pojSettingsSectionTitle)
                SettingsCard {
                    SwitchRow(
                        label = Tab4Texts.doubleTapOO,
                        checked = doubleOO,
                        onCheckedChange = {
                            doubleOO = it
                            prefs.enableDoubleTapOO = it
                        },
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = Tab4Texts.doubleTapNN,
                        checked = doubleNN,
                        onCheckedChange = {
                            doubleNN = it
                            prefs.enableDoubleTapNN = it
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

                // TPS settings card
                SectionHeader(Tab4Texts.tpsSettingsSectionTitle)
                SettingsCard {
                    SwitchRow(
                        label = Tab4Texts.tpsOrMapsToER,
                        checked = tpsOrMapsToER,
                        infoText = Tab4Texts.tpsOrMapsToERInfo,
                        onCheckedChange = {
                            tpsOrMapsToER = it
                            prefs.tpsOrMapsToER = it
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

                // Diagnostic info card
                SectionHeader(Tab4Texts.diagnosticSectionTitle)
                SettingsCard {
                    ActionRow(
                        label = Tab4Texts.diagnosticCopy,
                        icon = Icons.Outlined.ContentCopy,
                        onClick = {
                            scope.launch {
                                val info = DiagnosticService.gather(context)
                                val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                                clipboard.setPrimaryClip(
                                    ClipData.newPlainText("Taigi Keyboard Diagnostic", info.formatted()),
                                )
                                Toast.makeText(context, Tab4Texts.diagnosticCopied, Toast.LENGTH_SHORT).show()
                            }
                        },
                    )
                    SettingsDivider()
                    ActionRow(
                        label = Tab4Texts.diagnosticShare,
                        icon = Icons.AutoMirrored.Outlined.OpenInNew,
                        textColor = MaterialTheme.colorScheme.primary,
                        onClick = {
                            scope.launch {
                                val info = DiagnosticService.gather(context)
                                val sendIntent =
                                    Intent().apply {
                                        action = Intent.ACTION_SEND
                                        putExtra(Intent.EXTRA_TEXT, info.formatted())
                                        type = "text/plain"
                                    }
                                context.startActivity(Intent.createChooser(sendIntent, null))
                            }
                        },
                    )
                    SettingsDivider()
                    ActionRow(
                        label = Tab4Texts.diagnosticEmail,
                        icon = Icons.AutoMirrored.Outlined.OpenInNew,
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
                                    Toast.makeText(context, Tab4Texts.noEmailApp, Toast.LENGTH_SHORT).show()
                                }
                            }
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

                // Reset settings card
                SettingsCard {
                    ActionRow(
                        label = Tab4Texts.resetSettings,
                        onClick = { showResetDialog = true },
                        textColor = MaterialTheme.colorScheme.error,
                    )
                }
            }
        }

        if (showResetDialog) {
            ConfirmationDialog(
                title = Tab4Texts.resetSettings,
                message = Tab4Texts.resetSettingsMessage,
                confirmLabel = Tab4Texts.reset,
                dismissLabel = CommonTexts.cancel,
                onConfirm = {
                    showResetDialog = false
                    onResetSettings()
                },
                onDismiss = { showResetDialog = false },
            )
        }
    }
}

private fun inputModeDisplayName(mode: String): String =
    when (mode) {
        "poj" -> Tab4Texts.pojMode
        "tl" -> Tab4Texts.tlMode
        "english" -> Tab4Texts.englishMode
        "tps" -> Tab4Texts.tpsMode
        else -> Tab4Texts.tlMode
    }
