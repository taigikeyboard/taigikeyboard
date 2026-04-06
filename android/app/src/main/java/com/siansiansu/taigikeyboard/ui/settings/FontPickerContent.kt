package com.siansiansu.taigikeyboard.ui.settings

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.clickable
import androidx.compose.ui.graphics.Color
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
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
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab2Texts
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider

// Font picker sub-page (mimics iOS NavigationLink behavior)
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FontPickerContent(
    languageManager: LanguageManager,
    fontType: String,
    onFontSelected: (String) -> Unit,
    onNavigateBack: () -> Unit
) {
    Scaffold(
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = languageManager.text(Tab2Texts.customFont),
                        color = MaterialTheme.colorScheme.onSurface
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Back",
                            tint = MaterialTheme.colorScheme.onSurface
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surfaceContainer
                )
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .padding(horizontal = 20.dp)
                .padding(top = 16.dp)
        ) {
            SettingsCard {
                Column {
                    FontPickerRow(
                        label = languageManager.text(Tab2Texts.fontSystemDefault),
                        isSelected = fontType == "system",
                        onClick = { onFontSelected("system") }
                    )
                    SettingsDivider()
                    FontPickerRow(
                        label = languageManager.text(Tab2Texts.fontOpenHuninn),
                        isSelected = fontType == "openHuninn",
                        onClick = { onFontSelected("openHuninn") }
                    )
                    SettingsDivider()
                    FontPickerRow(
                        label = languageManager.text(Tab2Texts.fontIansui),
                        isSelected = fontType == "iansui",
                        onClick = { onFontSelected("iansui") }
                    )
                }
            }
        }
    }
}

@Composable
private fun FontPickerRow(
    label: String,
    isSelected: Boolean,
    onClick: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .clickable(onClick = onClick)
            .padding(horizontal = 20.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = label,
            modifier = Modifier.weight(1f),
            fontSize = AppStyle.bodyFontSize,
            color = MaterialTheme.colorScheme.onSurface
        )
        if (isSelected) {
            Icon(
                imageVector = Icons.Filled.Check,
                contentDescription = null,
                modifier = Modifier.size(20.dp),
                tint = MaterialTheme.colorScheme.primary
            )
        }
    }
}

internal fun fontDisplayName(fontType: String, languageManager: LanguageManager): String {
    return when (fontType) {
        "system" -> languageManager.text(Tab2Texts.fontSystemDefault)
        "openHuninn" -> languageManager.text(Tab2Texts.fontOpenHuninn)
        "iansui" -> languageManager.text(Tab2Texts.fontIansui)
        else -> fontType
    }
}
