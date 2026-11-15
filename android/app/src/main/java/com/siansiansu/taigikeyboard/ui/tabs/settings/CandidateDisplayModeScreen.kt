package com.siansiansu.taigikeyboard.ui.tabs.settings

// Sub-screen for selecting the candidate display mode (Hanji–Romanization Pairing / Romanization Only / Hanji with Romanization).

import androidx.compose.runtime.Composable
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.settings.CandidateDisplayMode
import com.siansiansu.taigikeyboard.ime.settings.candidateDisplayModeOptions
import com.siansiansu.taigikeyboard.ui.components.SelectionListScreen

@Composable
fun candidateDisplayModeDisplayName(mode: CandidateDisplayMode): String = stringRes(candidateDisplayModeOptions.first { it.first == mode }.second)

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
