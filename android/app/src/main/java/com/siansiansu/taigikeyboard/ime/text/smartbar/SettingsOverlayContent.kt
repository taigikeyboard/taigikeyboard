package com.siansiansu.taigikeyboard.ime.text.smartbar

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
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
import androidx.compose.material.icons.filled.ArrowDropDown
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.settings.CandidateDisplayMode
import com.siansiansu.taigikeyboard.ui.components.SettingsIcons
import com.siansiansu.taigikeyboard.ui.components.SwitchRow
import com.siansiansu.taigikeyboard.ui.tabs.settings.candidateDisplayModeDisplayName
import com.siansiansu.taigikeyboard.ui.tabs.settings.candidateDisplayModeOptions
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Compose content for the keyboard settings overlay.
 *
 * Renders the same settings as InputSettingsScreen (Settings tab) but styled
 * for the keyboard overlay context. Uses shared SwitchRow and SettingsIcons.
 */
@Composable
fun SettingsOverlayContent(
    prefs: PrefHelper,
    refreshTrigger: Int,
    onDismiss: () -> Unit,
    onOpenApp: () -> Unit,
) {
    val scope = rememberCoroutineScope()

    val fontFamily =
        remember(prefs.fontType) {
            when (prefs.fontType) {
                "openHuninn" -> FontFamily(Font(R.font.jf_openhuninn_2_1))
                "iansui" -> FontFamily(Font(R.font.iansui_regular))
                "genYoMin" -> FontFamily(Font(R.font.genyomin2tw_r))
                "genYoGothic" -> FontFamily(Font(R.font.genyogothic2tw_r))
                else -> FontFamily.Default
            }
        }

    // Toggle states — refreshTrigger as key ensures re-read from prefs on each show()
    var candidateDisplayMode by remember(refreshTrigger) { mutableStateOf(prefs.candidateDisplayMode) }
    // 括號標註 binds the STORED flag; it is only disabled (not cleared) while roman-only.
    var outputBoth by remember(refreshTrigger) { mutableStateOf(prefs.storedOutputBothScripts) }
    var literalRomanCandidate by remember(refreshTrigger) { mutableStateOf(prefs.literalRomanCandidateEnabled) }
    var autoCap by remember(refreshTrigger) { mutableStateOf(prefs.autoCapitalizationEnabled) }
    var autoSpace by remember(refreshTrigger) { mutableStateOf(prefs.isAutoSpaceEnabled) }
    var toolbarAutoCollapse by remember(refreshTrigger) { mutableStateOf(prefs.isToolbarAutoCollapse) }
    var isGlobeKeyEnabled by remember(refreshTrigger) { mutableStateOf(prefs.isGlobeKeyEnabled) }
    var soundFeedback by remember(refreshTrigger) { mutableStateOf(prefs.isSoundFeedbackEnabled) }
    var vibrationFeedback by remember(refreshTrigger) { mutableStateOf(prefs.isVibrationFeedbackEnabled) }
    var doubleOO by remember(refreshTrigger) { mutableStateOf(prefs.enableDoubleTapOO) }
    var doubleNN by remember(refreshTrigger) { mutableStateOf(prefs.enableDoubleTapNN) }
    var tpsOrER by remember(refreshTrigger) { mutableStateOf(prefs.tpsOrMapsToER) }

    fun autoDismissIfNeeded() {
        if (prefs.isToolbarAutoCollapse) {
            scope.launch {
                delay(300)
                onDismiss()
            }
        }
    }

    // Role-first overlay colors so a light-only gradient theme stays readable in system dark mode
    // (the keyboard theme owns these, not the M3 app palette). Matches symbol / layout overlays.
    val appearance = rememberKeyboardOverlayAppearance(prefs, refreshTrigger)
    val labelColor = appearance.foreground
    val iconTint = appearance.foreground

    // Panel sits below the smartbar; offset the gradient by it so the slice stays continuous.
    val topInsetPx = rememberSmartbarInsetPx()

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .keyboardOverlayBackdrop(appearance.gradientStops, appearance.solidBackground, topInsetPx)
                .verticalScroll(rememberScrollState())
                .padding(top = 4.dp, bottom = 8.dp),
    ) {
        // General settings
        CandidateDisplayModeRow(
            selected = candidateDisplayMode,
            onSelected = {
                if (it != candidateDisplayMode) {
                    candidateDisplayMode = it
                    // The IME reacts through PrefHelper.observeCandidateDisplayMode.
                    prefs.candidateDisplayMode = it
                    autoDismissIfNeeded()
                }
            },
            iconTint = iconTint,
            labelColor = labelColor,
            accent = appearance.accent,
            fontFamily = fontFamily,
        )
        SwitchRow(
            label = L10n.settingsOutputBothScripts,
            checked = outputBoth,
            icon = SettingsIcons.outputBothScripts,
            iconTint = iconTint,
            enabled = candidateDisplayMode.showsHanji,
            onCheckedChange = {
                outputBoth = it
                prefs.storedOutputBothScripts = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
        )
        SwitchRow(
            label = L10n.settingsLiteralRomanCandidate,
            checked = literalRomanCandidate,
            icon = SettingsIcons.literalRomanCandidate,
            iconTint = iconTint,
            onCheckedChange = {
                literalRomanCandidate = it
                prefs.literalRomanCandidateEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
        )
        SwitchRow(
            label = L10n.settingsAutoCapitalization,
            checked = autoCap,
            icon = SettingsIcons.autoCapitalization,
            iconTint = iconTint,
            onCheckedChange = {
                autoCap = it
                prefs.autoCapitalizationEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
        )
        SwitchRow(
            label = L10n.settingsAutoSpace,
            checked = autoSpace,
            icon = SettingsIcons.autoSpace,
            iconTint = iconTint,
            onCheckedChange = {
                autoSpace = it
                prefs.isAutoSpaceEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
        )
        SwitchRow(
            label = L10n.settingsToolbarAutoCollapse,
            checked = toolbarAutoCollapse,
            icon = SettingsIcons.toolbar,
            iconTint = iconTint,
            onCheckedChange = {
                toolbarAutoCollapse = it
                prefs.isToolbarAutoCollapse = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
        )
        SwitchRow(
            label = L10n.settingsGlobeKey,
            checked = isGlobeKeyEnabled,
            icon = SettingsIcons.globe,
            iconTint = iconTint,
            onCheckedChange = {
                isGlobeKeyEnabled = it
                prefs.isGlobeKeyEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
        )

        // Feedback settings
        SwitchRow(
            label = L10n.settingsSoundFeedback,
            checked = soundFeedback,
            icon = SettingsIcons.sound,
            iconTint = iconTint,
            onCheckedChange = {
                soundFeedback = it
                prefs.isSoundFeedbackEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
        )
        SwitchRow(
            label = L10n.settingsVibrationFeedback,
            checked = vibrationFeedback,
            icon = SettingsIcons.vibration,
            iconTint = iconTint,
            onCheckedChange = {
                vibrationFeedback = it
                prefs.isVibrationFeedbackEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
        )

        // POJ settings
        SwitchRow(
            label = L10n.settingsDoubleTapOO,
            checked = doubleOO,
            onCheckedChange = {
                doubleOO = it
                prefs.enableDoubleTapOO = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
        )
        SwitchRow(
            label = L10n.settingsDoubleTapNN,
            checked = doubleNN,
            onCheckedChange = {
                doubleNN = it
                prefs.enableDoubleTapNN = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
        )

        // TPS settings
        SwitchRow(
            label = L10n.settingsTpsOrMapsToER,
            checked = tpsOrER,
            onCheckedChange = {
                tpsOrER = it
                prefs.tpsOrMapsToER = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
        )

        Spacer(Modifier.height(16.dp))

        // Open App button
        TextButton(
            onClick = {
                onOpenApp()
                onDismiss()
            },
            modifier = Modifier.align(Alignment.CenterHorizontally),
        ) {
            Text(
                text = L10n.settingsOpenApp,
                color = appearance.accent,
                fontFamily = fontFamily,
            )
        }
    }
}

// 候選詞顯示 dropdown row — same icon / label / padding shape as the SwitchRow siblings; the trailing
// slot shows the current value + a drop-down arrow and opens a DropdownMenu of the three modes
// (three segments no longer fit beside the label at keyboard width with en / ja strings).
@Composable
private fun CandidateDisplayModeRow(
    selected: CandidateDisplayMode,
    onSelected: (CandidateDisplayMode) -> Unit,
    iconTint: Color,
    labelColor: Color,
    accent: Color,
    fontFamily: FontFamily,
) {
    var expanded by remember { mutableStateOf(false) }
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = 48.dp)
                .padding(horizontal = 20.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = SettingsIcons.candidateDisplayMode,
            contentDescription = null,
            modifier = Modifier.size(24.dp),
            tint = iconTint,
        )
        Spacer(Modifier.width(12.dp))
        Text(
            text = L10n.settingsCandidateDisplayMode,
            modifier = Modifier.weight(1f),
            color = labelColor,
            fontFamily = fontFamily,
            style = MaterialTheme.typography.bodyLarge,
        )
        Spacer(Modifier.width(12.dp))
        Box {
            Row(
                modifier = Modifier.clickable { expanded = true },
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = candidateDisplayModeDisplayName(selected),
                    color = labelColor,
                    fontFamily = fontFamily,
                    style = MaterialTheme.typography.bodyLarge,
                )
                Icon(
                    imageVector = Icons.Filled.ArrowDropDown,
                    contentDescription = null,
                    tint = labelColor,
                )
            }
            // The menu is an M3 surface popup, not part of the gradient backdrop — default item colours apply.
            DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                candidateDisplayModeOptions.forEach { (mode, labelKey) ->
                    DropdownMenuItem(
                        text = {
                            Text(
                                text = stringRes(labelKey),
                                fontFamily = fontFamily,
                                style = MaterialTheme.typography.bodyLarge,
                            )
                        },
                        // Always-present slot keeps the three labels left-aligned; only the current mode draws the check.
                        trailingIcon = {
                            if (mode == selected) {
                                Icon(
                                    imageVector = Icons.Filled.Check,
                                    contentDescription = null,
                                    modifier = Modifier.size(AppStyle.selectionIconSize),
                                    tint = accent,
                                )
                            }
                        },
                        onClick = {
                            expanded = false
                            onSelected(mode)
                        },
                    )
                }
            }
        }
    }
}
