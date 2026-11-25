package com.siansiansu.taigikeyboard.ime.text.overlays

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
import androidx.compose.material3.SwitchDefaults
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
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.settings.PrefHelper
import com.siansiansu.taigikeyboard.ime.settings.candidateDisplayModeOptions
import com.siansiansu.taigikeyboard.ime.settings.nasalMarkerStyleOptions
import com.siansiansu.taigikeyboard.ime.settings.syllableSeparatorOptions
import com.siansiansu.taigikeyboard.ime.text.keyboard.computeBottomInsetPadding
import com.siansiansu.taigikeyboard.ime.theme.themeBackground
import com.siansiansu.taigikeyboard.ui.components.SwitchRow
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Compose content for the keyboard settings overlay.
 *
 * Renders the Settings tab's (InputSettingsScreen) keyboard settings in its section order —
 * Typing / Keyboard / POJ / TPS / Feedback — styled for the keyboard overlay. Uses shared SwitchRow.
 */
@Composable
fun SettingsOverlayContent(
    prefs: PrefHelper,
    refreshTrigger: Int,
    onDismiss: () -> Unit,
    onOpenApp: () -> Unit,
    /** Navigation-bar inset the keyboard body pads by; the overlay covers that band too. */
    bottomInset: Dp = computeBottomInsetPadding(),
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
    // Annotate in Brackets binds the STORED flag; it is only disabled (not cleared) while roman-only.
    var outputBoth by remember(refreshTrigger) { mutableStateOf(prefs.storedOutputBothScripts) }
    var khinsiannInHanji by remember(refreshTrigger) { mutableStateOf(prefs.isKhinsiannInHanjiEnabled) }
    var hanLoMixed by remember(refreshTrigger) { mutableStateOf(prefs.isHanLoMixedEnabled) }
    var literalRomanCandidate by remember(refreshTrigger) { mutableStateOf(prefs.literalRomanCandidateEnabled) }
    var autoCap by remember(refreshTrigger) { mutableStateOf(prefs.autoCapitalizationEnabled) }
    var autoSpace by remember(refreshTrigger) { mutableStateOf(prefs.isAutoSpaceEnabled) }
    var toolbarAutoCollapse by remember(refreshTrigger) { mutableStateOf(prefs.isToolbarAutoCollapse) }
    var isGlobeKeyEnabled by remember(refreshTrigger) { mutableStateOf(prefs.isGlobeKeyEnabled) }
    var soundFeedback by remember(refreshTrigger) { mutableStateOf(prefs.isSoundFeedbackEnabled) }
    var vibrationFeedback by remember(refreshTrigger) { mutableStateOf(prefs.isVibrationFeedbackEnabled) }
    var doubleOO by remember(refreshTrigger) { mutableStateOf(prefs.enableDoubleTapOO) }
    var doubleNN by remember(refreshTrigger) { mutableStateOf(prefs.enableDoubleTapNN) }
    var nasalMarkerUppercase by remember(refreshTrigger) { mutableStateOf(prefs.isNasalMarkerUppercaseEnabled) }
    var syllableSeparator by remember(refreshTrigger) { mutableStateOf(prefs.syllableSeparator) }
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
    // Unchecked track = the theme key fill (checked keeps the M3 accent, a fixed state color).
    // Divergence: iOS keeps the native Toggle — SwiftUI exposes no off-track color (ui-style-guide.md).
    val switchColors = appearance.keyFill?.let { SwitchDefaults.colors(uncheckedTrackColor = it) }

    // Panel sits below the smartbar; offset the gradient by it so the slice stays continuous.
    val topInsetPx = rememberSmartbarInsetPx()

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .themeBackground(appearance.surface, appearance.solidBackground, topInsetPx)
                // The surface runs under the navigation bar like the keyboard's; the rows stop above it.
                .padding(bottom = bottomInset)
                .verticalScroll(rememberScrollState())
                .padding(bottom = 8.dp),
    ) {
        SectionHeader(L10n.settingsTypingSectionTitle, labelColor)
        DropdownRow(
            label = L10n.settingsCandidateDisplayMode,
            selected = candidateDisplayMode,
            options = candidateDisplayModeOptions,
            onSelected = {
                candidateDisplayMode = it
                // The IME reacts through PrefHelper.observeCandidateDisplayMode.
                prefs.candidateDisplayMode = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            accent = appearance.accent,
            fontFamily = fontFamily,
        )
        SwitchRow(
            label = L10n.settingsLiteralRomanCandidate,
            checked = literalRomanCandidate,
            onCheckedChange = {
                literalRomanCandidate = it
                prefs.literalRomanCandidateEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
            switchColors = switchColors,
        )
        SwitchRow(
            label = L10n.settingsOutputBothScripts,
            checked = outputBoth,
            enabled = candidateDisplayMode.showsHanji,
            onCheckedChange = {
                outputBoth = it
                prefs.storedOutputBothScripts = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
            switchColors = switchColors,
        )
        SwitchRow(
            label = L10n.settingsKhinsiannInHanji,
            checked = khinsiannInHanji,
            enabled = candidateDisplayMode.showsHanji,
            onCheckedChange = {
                khinsiannInHanji = it
                prefs.isKhinsiannInHanjiEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
            switchColors = switchColors,
        )
        // Romanization candidates: the engine ignores it under Romanization Only, side by side
        // led by the romanization and on TPS, so it greys there, as on the desktops
        // (`SettingsDocument::allows_han_lo_mixed`).
        SwitchRow(
            label = L10n.settingsHanLoMixed,
            checked = hanLoMixed,
            enabled =
                candidateDisplayMode.allowsHanLoMixed(prefs.storedIsHanjiFirst) &&
                    !prefs.isTpsLayout,
            onCheckedChange = {
                hanLoMixed = it
                prefs.isHanLoMixedEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
            switchColors = switchColors,
        )
        DropdownRow(
            label = L10n.settingsSyllableSeparator,
            selected = syllableSeparator,
            options = syllableSeparatorOptions,
            onSelected = {
                syllableSeparator = it
                prefs.syllableSeparator = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            accent = appearance.accent,
            fontFamily = fontFamily,
        )
        SwitchRow(
            label = L10n.settingsAutoCapitalization,
            checked = autoCap,
            onCheckedChange = {
                autoCap = it
                prefs.autoCapitalizationEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
            switchColors = switchColors,
        )
        SwitchRow(
            label = L10n.settingsAutoSpace,
            checked = autoSpace,
            onCheckedChange = {
                autoSpace = it
                prefs.isAutoSpaceEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
            switchColors = switchColors,
        )

        SectionHeader(L10n.settingsKeyboardSectionTitle, labelColor)
        SwitchRow(
            label = L10n.settingsToolbarAutoCollapse,
            checked = toolbarAutoCollapse,
            onCheckedChange = {
                toolbarAutoCollapse = it
                prefs.isToolbarAutoCollapse = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
            switchColors = switchColors,
        )
        SwitchRow(
            label = L10n.settingsGlobeKey,
            checked = isGlobeKeyEnabled,
            onCheckedChange = {
                isGlobeKeyEnabled = it
                prefs.isGlobeKeyEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
            switchColors = switchColors,
        )

        SectionHeader(L10n.settingsPojMode, labelColor)
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
            switchColors = switchColors,
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
            switchColors = switchColors,
        )
        DropdownRow(
            label = L10n.settingsNasalMarkerUppercase,
            selected = nasalMarkerUppercase,
            options = nasalMarkerStyleOptions,
            onSelected = {
                nasalMarkerUppercase = it
                prefs.isNasalMarkerUppercaseEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            accent = appearance.accent,
            fontFamily = fontFamily,
        )

        SectionHeader(L10n.settingsTpsMode, labelColor)
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
            switchColors = switchColors,
        )

        SectionHeader(L10n.settingsFeedbackSectionTitle, labelColor)
        SwitchRow(
            label = L10n.settingsSoundFeedback,
            checked = soundFeedback,
            onCheckedChange = {
                soundFeedback = it
                prefs.isSoundFeedbackEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
            switchColors = switchColors,
        )
        SwitchRow(
            label = L10n.settingsVibrationFeedback,
            checked = vibrationFeedback,
            onCheckedChange = {
                vibrationFeedback = it
                prefs.isVibrationFeedbackEnabled = it
                autoDismissIfNeeded()
            },
            labelColor = labelColor,
            fontFamily = fontFamily,
            switchColors = switchColors,
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

// Section label with the row inset (SwitchRow's 20 dp), above each group of rows.
@Composable
private fun SectionHeader(
    text: String,
    color: Color,
) {
    KeyboardOverlaySectionHeader(
        text = text,
        color = color,
        modifier = Modifier.padding(start = 20.dp, top = 12.dp, bottom = 2.dp),
    )
}

// Dropdown row — same label / padding shape as the SwitchRow siblings; the trailing slot shows the
// current value + a drop-down arrow and opens a DropdownMenu of the options (the choices do not fit as
// segments beside the label at keyboard width with en / ja strings).
@Composable
private fun <T> DropdownRow(
    label: String,
    selected: T,
    options: List<Pair<T, StringKey>>,
    onSelected: (T) -> Unit,
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
        Text(
            text = label,
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
                    text = stringRes(options.first { it.first == selected }.second),
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
                options.forEach { (value, labelKey) ->
                    DropdownMenuItem(
                        text = {
                            Text(
                                text = stringRes(labelKey),
                                fontFamily = fontFamily,
                                style = MaterialTheme.typography.bodyLarge,
                            )
                        },
                        // Always-present slot keeps the labels left-aligned; only the current value draws the check.
                        trailingIcon = {
                            if (value == selected) {
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
                            if (value != selected) onSelected(value)
                        },
                    )
                }
            }
        }
    }
}
