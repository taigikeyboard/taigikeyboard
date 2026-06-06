package com.siansiansu.taigikeyboard.ime.text.smartbar

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.SettingsTexts
import com.siansiansu.taigikeyboard.ui.components.SettingsIcons
import com.siansiansu.taigikeyboard.ui.components.SwitchRow
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
    var outputBoth by remember(refreshTrigger) { mutableStateOf(prefs.outputBothScripts) }
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

    val labelColor = MaterialTheme.colorScheme.onSurface
    val iconTint = MaterialTheme.colorScheme.onSurfaceVariant

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(top = 4.dp, bottom = 8.dp),
    ) {
        // General settings
        SwitchRow(
            label = SettingsTexts.outputBothScripts,
            checked = outputBoth,
            icon = SettingsIcons.outputBothScripts,
            iconTint = iconTint,
            onCheckedChange = {
                outputBoth = it
                prefs.outputBothScripts = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
        )
        SwitchRow(
            label = SettingsTexts.literalRomanCandidate,
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
            label = SettingsTexts.autoCapitalization,
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
            label = SettingsTexts.autoSpace,
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
            label = SettingsTexts.toolbarAutoCollapse,
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
            label = SettingsTexts.globeKey,
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
            label = SettingsTexts.soundFeedback,
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
            label = SettingsTexts.vibrationFeedback,
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
            label = SettingsTexts.doubleTapOO,
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
            label = SettingsTexts.doubleTapNN,
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
            label = SettingsTexts.tpsOrMapsToER,
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
                text = SettingsTexts.openApp,
                fontFamily = fontFamily,
            )
        }
    }
}
