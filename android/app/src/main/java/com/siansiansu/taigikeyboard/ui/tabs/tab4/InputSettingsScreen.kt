package com.siansiansu.taigikeyboard.ui.tabs.tab4

// Main settings screen (Tab4) — input mode, typing, keyboard, feedback, diagnostics, reset.

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
import com.siansiansu.taigikeyboard.content.FeatureContentLoader
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.CommonTexts
import com.siansiansu.taigikeyboard.localization.Tab4Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.ConfirmationDialog
import com.siansiansu.taigikeyboard.ui.components.ContentCopy
import com.siansiansu.taigikeyboard.ui.components.OpenInNew
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SettingsIcons
import com.siansiansu.taigikeyboard.ui.components.SwitchRow
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader
import kotlinx.coroutines.launch

private const val FEATURE_ID_HANLO_DESIGN = "hanloDesign"
private const val FEATURE_ID_CASE_SWITCH = "caseSwitch"
private const val DIAGNOSTIC_CLIP_LABEL = "Taigi Keyboard Diagnostic"
private const val DIAGNOSTIC_MIME_TYPE = "text/plain"
private const val DIAGNOSTIC_EMAIL = "info@taigikeyboard.tw"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun InputSettingsScreen(
    prefs: PrefHelper,
    onResetSettings: () -> Unit,
    resetCounter: Int,
) {
    val context = LocalContext.current
    var showResetDialog by remember { mutableStateOf(false) }
    var showInputModePicker by remember { mutableStateOf(false) }
    // Each state re-reads from prefs when resetCounter changes (after settings reset)
    var inputMode by remember(resetCounter) { mutableStateOf(prefs.inputMode) }
    var outputBoth by remember(resetCounter) { mutableStateOf(prefs.outputBothScripts) }
    var autoCap by remember(resetCounter) { mutableStateOf(prefs.autoCapitalizationEnabled) }
    var autoSpace by remember(resetCounter) { mutableStateOf(prefs.isAutoSpaceEnabled) }
    var doubleOO by remember(resetCounter) { mutableStateOf(prefs.enableDoubleTapOO) }
    var doubleNN by remember(resetCounter) { mutableStateOf(prefs.enableDoubleTapNN) }
    var toolbarAutoCollapse by remember(resetCounter) { mutableStateOf(prefs.isToolbarAutoCollapse) }
    var isGlobeKeyEnabled by remember(resetCounter) { mutableStateOf(prefs.isGlobeKeyEnabled) }
    var soundFeedback by remember(resetCounter) { mutableStateOf(prefs.isSoundFeedbackEnabled) }
    var vibrationFeedback by remember(resetCounter) { mutableStateOf(prefs.isVibrationFeedbackEnabled) }
    var tpsOrMapsToER by remember(resetCounter) { mutableStateOf(prefs.tpsOrMapsToER) }

    val features = remember { FeatureContentLoader.loadFeatures(context) }

    fun featureSummary(featureId: String): String? = features.firstOrNull { it.id == featureId }?.summary

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

                SectionHeader(Tab4Texts.typingSectionTitle)
                SettingsCard {
                    SwitchRow(
                        label = Tab4Texts.outputBothScripts,
                        checked = outputBoth,
                        infoText = featureSummary(FEATURE_ID_HANLO_DESIGN),
                        onCheckedChange = {
                            outputBoth = it
                            prefs.outputBothScripts = it
                        },
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = Tab4Texts.autoCapitalization,
                        checked = autoCap,
                        infoText = featureSummary(FEATURE_ID_CASE_SWITCH),
                        onCheckedChange = {
                            autoCap = it
                            prefs.autoCapitalizationEnabled = it
                        },
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = Tab4Texts.autoSpace,
                        checked = autoSpace,
                        infoText = featureSummary(FEATURE_ID_HANLO_DESIGN),
                        onCheckedChange = {
                            autoSpace = it
                            prefs.isAutoSpaceEnabled = it
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

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

                DiagnosticSection()

                Spacer(Modifier.height(24.dp))

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

@Composable
private fun DiagnosticSection() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
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
                        ClipData.newPlainText(DIAGNOSTIC_CLIP_LABEL, info.formatted()),
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
                            type = DIAGNOSTIC_MIME_TYPE
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
                    val uri = Uri.parse("mailto:$DIAGNOSTIC_EMAIL?subject=$subject&body=$body")
                    try {
                        context.startActivity(Intent(Intent.ACTION_SENDTO, uri))
                    } catch (_: Exception) {
                        Toast.makeText(context, Tab4Texts.noEmailApp, Toast.LENGTH_SHORT).show()
                    }
                }
            },
        )
    }
}
