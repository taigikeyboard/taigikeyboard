package com.siansiansu.taigikeyboard.ui.tabs.settings

// Sub-screen for the Syllable Separator (§49): hyphen, space, or none.

import androidx.compose.runtime.Composable
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.settings.SyllableSeparator
import com.siansiansu.taigikeyboard.ime.settings.syllableSeparatorOptions
import com.siansiansu.taigikeyboard.ui.components.SelectionListScreen

@Composable
fun syllableSeparatorDisplayName(separator: SyllableSeparator): String = stringRes(syllableSeparatorOptions.first { it.first == separator }.second)

@Composable
fun SyllableSeparatorScreen(
    selected: SyllableSeparator,
    onSelected: (SyllableSeparator) -> Unit,
    onBack: () -> Unit,
) {
    SelectionListScreen(
        title = L10n.settingsSyllableSeparator,
        options = syllableSeparatorOptions,
        selected = selected,
        onSelected = onSelected,
        onBack = onBack,
    )
}
