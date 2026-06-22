package com.siansiansu.taigikeyboard.ui.tabs.settings

// Sub-screen for selecting the app UI display language (orthogonal to the input mode).

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.i18n.DisplayLanguage
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.theme.AppStyle

// The picker/settings-row label for a [DisplayLanguage]: SYSTEM is a selection policy with no endonym,
// so it renders the i18n "Automatic" key (translated with the UI language); every authored language
// renders its endonym (own script, language-invariant). Shared by the picker rows and the settings-row
// trailing value so the SYSTEM special-case lives in one place.
@Composable
fun displayLanguageLabel(language: DisplayLanguage): String =
    if (language == DisplayLanguage.SYSTEM) L10n.settingsDisplayLanguageAutomatic else language.endonym

// Rows use selectable(role = RadioButton) so the selected state is announced by TalkBack — the
// checkmark is decorative (contentDescription = null) and must not be read twice. Labels come from
// displayLanguageLabel (endonyms for authored languages, the i18n Automatic key for SYSTEM).
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DisplayLanguageScreen(
    selectedLanguage: DisplayLanguage,
    onLanguageSelected: (DisplayLanguage) -> Unit,
    onBack: () -> Unit,
) {
    BackHandler(onBack = onBack)

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(L10n.settingsDisplayLanguage) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = L10n.commonBack,
                        )
                    }
                },
            )
        },
    ) { innerPadding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(innerPadding)
                    .padding(horizontal = 20.dp, vertical = 16.dp),
        ) {
            // selectableGroup() gives the radio rows proper group accessibility semantics (TalkBack
            // announces "n of m"), required when each row is a Role.RadioButton.
            SettingsCard(modifier = Modifier.selectableGroup()) {
                val languages = DisplayLanguage.selectableLanguages
                languages.forEachIndexed { index, language ->
                    val isSelected = language == selectedLanguage
                    Row(
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .heightIn(min = 48.dp)
                                .selectable(
                                    selected = isSelected,
                                    role = Role.RadioButton,
                                    onClick = { onLanguageSelected(language) },
                                )
                                .padding(horizontal = 20.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = displayLanguageLabel(language),
                            modifier = Modifier.weight(1f),
                            color = MaterialTheme.colorScheme.onSurface,
                            style = MaterialTheme.typography.bodyLarge,
                        )
                        if (isSelected) {
                            Icon(
                                imageVector = Icons.Default.Check,
                                contentDescription = null,
                                modifier = Modifier.size(AppStyle.selectionIconSize),
                                tint = MaterialTheme.colorScheme.primary,
                            )
                        }
                    }
                    if (index < languages.lastIndex) {
                        SettingsDivider()
                    }
                }
            }
        }
    }
}
