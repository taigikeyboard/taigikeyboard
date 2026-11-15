package com.siansiansu.taigikeyboard.ui.tabs.settings

// Sub-screen for Nasal mark in POJ capitals (§53): ᴺ or ⁿ, over the stored switch.

import androidx.compose.runtime.Composable
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.settings.nasalMarkerStyleOptions
import com.siansiansu.taigikeyboard.ui.components.SelectionListScreen

@Composable
fun nasalMarkerStyleDisplayName(isUppercase: Boolean): String = stringRes(nasalMarkerStyleOptions.first { it.first == isUppercase }.second)

@Composable
fun NasalMarkerStyleScreen(
    isUppercase: Boolean,
    onSelected: (Boolean) -> Unit,
    onBack: () -> Unit,
) {
    SelectionListScreen(
        title = L10n.settingsNasalMarkerUppercase,
        options = nasalMarkerStyleOptions,
        selected = isUppercase,
        onSelected = onSelected,
        onBack = onBack,
    )
}
