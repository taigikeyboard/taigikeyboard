package com.siansiansu.taigikeyboard.ui.tabs.settings

// Sub-screen for selecting the candidate display mode (漢羅對應 / 羅馬字 / 漢羅濫).

import androidx.compose.runtime.Composable
import com.siansiansu.taigikeyboard.ui.components.SelectionListScreen
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.core.settings.CandidateDisplayMode

// Shared mode→label-key pairs, used by CandidateDisplayModeScreen, InputSettingsScreen and the
// in-keyboard SettingsOverlayContent. Structural (no resolved strings); resolved at render.
val candidateDisplayModeOptions: List<Pair<CandidateDisplayMode, StringKey>> =
    listOf(
        CandidateDisplayMode.SIDE_BY_SIDE to StringKey.SETTINGS_CANDIDATE_DISPLAY_MODE_SIDE_BY_SIDE,
        CandidateDisplayMode.COMBINED to StringKey.SETTINGS_CANDIDATE_DISPLAY_MODE_COMBINED,
        CandidateDisplayMode.ROMAN_ONLY to StringKey.SETTINGS_CANDIDATE_DISPLAY_MODE_ROMAN_ONLY,
    )

@Composable
fun candidateDisplayModeDisplayName(mode: CandidateDisplayMode): String =
    stringRes(candidateDisplayModeOptions.first { it.first == mode }.second)

@Composable
fun CandidateDisplayModeScreen(
    selectedMode: CandidateDisplayMode,
    onModeSelected: (CandidateDisplayMode) -> Unit,
    onBack: () -> Unit,
) {
    SelectionListScreen(
        title = L10n.settingsCandidateDisplayMode,
        options = candidateDisplayModeOptions,
        selected = selectedMode,
        onSelected = onModeSelected,
        onBack = onBack,
    )
}
