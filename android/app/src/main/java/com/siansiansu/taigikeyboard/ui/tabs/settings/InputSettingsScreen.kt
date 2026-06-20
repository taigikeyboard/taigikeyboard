package com.siansiansu.taigikeyboard.ui.tabs.settings

// Main Settings tab screen — input mode, typing, keyboard, feedback, diagnostics, reset.

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.widget.Toast
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.core.net.toUri
import com.siansiansu.taigikeyboard.content.FeatureContentLoader
import com.siansiansu.taigikeyboard.i18n.I18nProbeBar
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.SettingsTexts
import com.siansiansu.taigikeyboard.localization.ThemeTexts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.ConfirmationDialog
import com.siansiansu.taigikeyboard.ui.components.ContentCopy
import com.siansiansu.taigikeyboard.ui.components.OpenInNew
import com.siansiansu.taigikeyboard.ui.components.SettingNavigationRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SettingsIcons
import com.siansiansu.taigikeyboard.ui.components.SwitchRow
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader

private const val FEATURE_ID_HANLO_DESIGN = "hanloDesign"
private const val FEATURE_ID_CASE_SWITCH = "caseSwitch"
private const val DIAGNOSTIC_CLIP_LABEL = "Taigi Keyboard Diagnostic"
private const val DIAGNOSTIC_MIME_TYPE = "text/plain"
private const val DIAGNOSTIC_EMAIL = "info@taigikeyboard.tw"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun InputSettingsScreen(
    prefs: PrefHelper,
    diagnosticViewModel: DiagnosticViewModel,
    onResetSettings: () -> Unit,
    resetCounter: Int,
) {
    val context = LocalContext.current
    var showResetDialog by remember { mutableStateOf(false) }
    var showInputModePicker by remember { mutableStateOf(false) }
    var showFontPicker by remember { mutableStateOf(false) }
    // Each state re-reads from prefs when resetCounter changes (after settings reset)
    var inputMode by remember(resetCounter) { mutableStateOf(prefs.inputMode) }
    var fontType by remember(resetCounter) { mutableStateOf(prefs.fontType) }
    var outputBoth by remember(resetCounter) { mutableStateOf(prefs.outputBothScripts) }
    var literalRomanCandidate by remember(resetCounter) { mutableStateOf(prefs.literalRomanCandidateEnabled) }
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
    } else if (showFontPicker) {
        FontPickerContent(
            fontType = fontType,
            onFontSelected = { selected ->
                fontType = selected
                prefs.fontType = selected
            },
            onNavigateBack = { showFontPicker = false },
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
                            text = SettingsTexts.tabTitle,
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
                // Debug-only i18n live-switch probe (no-op in release).
                I18nProbeBar(prefs)
                SettingsCard {
                    SettingNavigationRow(
                        label = SettingsTexts.inputMode,
                        value = inputModeDisplayName(inputMode),
                        onClick = { showInputModePicker = true },
                    )
                }

                Spacer(Modifier.height(24.dp))

                // Global keyboard font — its own card below 輸入模式 (font is a global
                // setting, not per-theme). Mirrors iOS SettingsTab font Section.
                SettingsCard {
                    SettingNavigationRow(
                        label = ThemeTexts.customFont,
                        value = fontDisplayName(fontType),
                        onClick = { showFontPicker = true },
                    )
                }

                Spacer(Modifier.height(24.dp))

                SectionHeader(SettingsTexts.typingSectionTitle)
                SettingsCard {
                    SwitchRow(
                        label = SettingsTexts.outputBothScripts,
                        checked = outputBoth,
                        infoText = featureSummary(FEATURE_ID_HANLO_DESIGN),
                        onCheckedChange = {
                            outputBoth = it
                            prefs.outputBothScripts = it
                        },
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = SettingsTexts.literalRomanCandidate,
                        checked = literalRomanCandidate,
                        infoText = SettingsTexts.literalRomanCandidateInfo,
                        onCheckedChange = {
                            literalRomanCandidate = it
                            prefs.literalRomanCandidateEnabled = it
                        },
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = SettingsTexts.autoCapitalization,
                        checked = autoCap,
                        infoText = featureSummary(FEATURE_ID_CASE_SWITCH),
                        onCheckedChange = {
                            autoCap = it
                            prefs.autoCapitalizationEnabled = it
                        },
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = SettingsTexts.autoSpace,
                        checked = autoSpace,
                        infoText = featureSummary(FEATURE_ID_HANLO_DESIGN),
                        onCheckedChange = {
                            autoSpace = it
                            prefs.isAutoSpaceEnabled = it
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

                SectionHeader(SettingsTexts.keyboardSectionTitle)
                SettingsCard {
                    SwitchRow(
                        label = SettingsTexts.toolbarAutoCollapse,
                        checked = toolbarAutoCollapse,
                        icon = SettingsIcons.toolbar,
                        infoText = SettingsTexts.toolbarAutoCollapseInfo,
                        onCheckedChange = {
                            toolbarAutoCollapse = it
                            prefs.isToolbarAutoCollapse = it
                        },
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = SettingsTexts.globeKey,
                        checked = isGlobeKeyEnabled,
                        icon = SettingsIcons.globe,
                        infoText = SettingsTexts.globeKeyInfo,
                        onCheckedChange = {
                            isGlobeKeyEnabled = it
                            prefs.isGlobeKeyEnabled = it
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

                SectionHeader(SettingsTexts.feedbackSectionTitle)
                SettingsCard {
                    SwitchRow(
                        label = SettingsTexts.soundFeedback,
                        checked = soundFeedback,
                        icon = SettingsIcons.sound,
                        onCheckedChange = {
                            soundFeedback = it
                            prefs.isSoundFeedbackEnabled = it
                        },
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = SettingsTexts.vibrationFeedback,
                        checked = vibrationFeedback,
                        icon = SettingsIcons.vibration,
                        onCheckedChange = {
                            vibrationFeedback = it
                            prefs.isVibrationFeedbackEnabled = it
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

                SectionHeader(SettingsTexts.pojSettingsSectionTitle)
                SettingsCard {
                    SwitchRow(
                        label = SettingsTexts.doubleTapOO,
                        checked = doubleOO,
                        onCheckedChange = {
                            doubleOO = it
                            prefs.enableDoubleTapOO = it
                        },
                    )
                    SettingsDivider()
                    SwitchRow(
                        label = SettingsTexts.doubleTapNN,
                        checked = doubleNN,
                        onCheckedChange = {
                            doubleNN = it
                            prefs.enableDoubleTapNN = it
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

                SectionHeader(SettingsTexts.tpsSettingsSectionTitle)
                SettingsCard {
                    SwitchRow(
                        label = SettingsTexts.tpsOrMapsToER,
                        checked = tpsOrMapsToER,
                        infoText = SettingsTexts.tpsOrMapsToERInfo,
                        onCheckedChange = {
                            tpsOrMapsToER = it
                            prefs.tpsOrMapsToER = it
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

                DiagnosticSection(viewModel = diagnosticViewModel)

                Spacer(Modifier.height(24.dp))

                SettingsCard {
                    ActionRow(
                        label = SettingsTexts.resetSettings,
                        onClick = { showResetDialog = true },
                        textColor = MaterialTheme.colorScheme.error,
                    )
                }
            }
        }

        if (showResetDialog) {
            ConfirmationDialog(
                title = SettingsTexts.resetSettings,
                message = SettingsTexts.resetSettingsMessage,
                confirmLabel = SettingsTexts.reset,
                dismissLabel = L10n.commonCancel,
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
private fun DiagnosticSection(viewModel: DiagnosticViewModel) {
    val context = LocalContext.current
    SectionHeader(SettingsTexts.diagnosticSectionTitle)
    SettingsCard {
        ActionRow(
            label = SettingsTexts.diagnosticCopy,
            icon = Icons.Outlined.ContentCopy,
            onClick = {
                val info = viewModel.gather()
                val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                clipboard.setPrimaryClip(
                    ClipData.newPlainText(DIAGNOSTIC_CLIP_LABEL, info.formatted()),
                )
                Toast.makeText(context, SettingsTexts.diagnosticCopied, Toast.LENGTH_SHORT).show()
            },
        )
        SettingsDivider()
        ActionRow(
            label = SettingsTexts.diagnosticShare,
            icon = Icons.AutoMirrored.Outlined.OpenInNew,
            textColor = MaterialTheme.colorScheme.primary,
            onClick = {
                val info = viewModel.gather()
                val sendIntent =
                    Intent().apply {
                        action = Intent.ACTION_SEND
                        putExtra(Intent.EXTRA_TEXT, info.formatted())
                        type = DIAGNOSTIC_MIME_TYPE
                    }
                context.startActivity(Intent.createChooser(sendIntent, null))
            },
        )
        SettingsDivider()
        ActionRow(
            label = SettingsTexts.diagnosticEmail,
            icon = Icons.AutoMirrored.Outlined.OpenInNew,
            textColor = MaterialTheme.colorScheme.primary,
            onClick = {
                val info = viewModel.gather()
                val subject = Uri.encode("台語齒盤 Bug 回報 (v${info.appVersion})")
                val body = Uri.encode(info.formatted())
                val uri = "mailto:$DIAGNOSTIC_EMAIL?subject=$subject&body=$body".toUri()
                try {
                    context.startActivity(Intent(Intent.ACTION_SENDTO, uri))
                } catch (_: Exception) {
                    Toast.makeText(context, SettingsTexts.noEmailApp, Toast.LENGTH_SHORT).show()
                }
            },
        )
    }
}
