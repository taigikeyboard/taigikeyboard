package com.siansiansu.taigikeyboard.ui.tabs.tab2

import androidx.compose.foundation.clickable
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
import com.siansiansu.taigikeyboard.localization.Tab2Texts
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.util.FontUtils.FontType

// Font selection sub-page for choosing keyboard typeface

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FontPickerContent(
    fontType: String,
    onFontSelected: (String) -> Unit,
    onNavigateBack: () -> Unit,
) {
    Scaffold(
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = Tab2Texts.customFont,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Back",
                            tint = MaterialTheme.colorScheme.onSurface,
                        )
                    }
                },
                colors =
                    TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.surfaceContainer,
                    ),
            )
        },
    ) { innerPadding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(innerPadding)
                    .padding(horizontal = 20.dp)
                    .padding(top = 16.dp),
        ) {
            SettingsCard {
                Column {
                    FontPickerRow(
                        label = Tab2Texts.fontSystemDefault,
                        isSelected = fontType == FontType.SYSTEM.value,
                        onClick = { onFontSelected(FontType.SYSTEM.value) },
                    )
                    SettingsDivider()
                    FontPickerRow(
                        label = Tab2Texts.fontOpenHuninn,
                        isSelected = fontType == FontType.OPEN_HUNINN.value,
                        onClick = { onFontSelected(FontType.OPEN_HUNINN.value) },
                    )
                    SettingsDivider()
                    FontPickerRow(
                        label = Tab2Texts.fontIansui,
                        isSelected = fontType == FontType.IANSUI.value,
                        onClick = { onFontSelected(FontType.IANSUI.value) },
                    )
                    SettingsDivider()
                    FontPickerRow(
                        label = Tab2Texts.fontGenYoMin,
                        isSelected = fontType == FontType.GEN_YO_MIN.value,
                        onClick = { onFontSelected(FontType.GEN_YO_MIN.value) },
                    )
                    SettingsDivider()
                    FontPickerRow(
                        label = Tab2Texts.fontGenYoGothic,
                        isSelected = fontType == FontType.GEN_YO_GOTHIC.value,
                        onClick = { onFontSelected(FontType.GEN_YO_GOTHIC.value) },
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
    onClick: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = 48.dp)
                .clickable(onClick = onClick)
                .padding(horizontal = 20.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            modifier = Modifier.weight(1f),
            color = MaterialTheme.colorScheme.onSurface,
            style = MaterialTheme.typography.bodyLarge,
        )
        if (isSelected) {
            Icon(
                imageVector = Icons.Filled.Check,
                contentDescription = null,
                modifier = Modifier.size(AppStyle.selectionIconSize),
                tint = MaterialTheme.colorScheme.primary,
            )
        }
    }
}

internal fun fontDisplayName(fontType: String): String =
    when (fontType) {
        FontType.SYSTEM.value -> Tab2Texts.fontSystemDefault
        FontType.OPEN_HUNINN.value -> Tab2Texts.fontOpenHuninn
        FontType.IANSUI.value -> Tab2Texts.fontIansui
        FontType.GEN_YO_MIN.value -> Tab2Texts.fontGenYoMin
        FontType.GEN_YO_GOTHIC.value -> Tab2Texts.fontGenYoGothic
        else -> fontType
    }
