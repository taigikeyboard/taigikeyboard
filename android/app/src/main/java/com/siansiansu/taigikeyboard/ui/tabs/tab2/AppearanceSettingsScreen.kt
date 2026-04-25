package com.siansiansu.taigikeyboard.ui.tabs.tab2

import androidx.compose.foundation.clickable
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
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.Tab2Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.ColorRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SliderRow
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader

// Appearance settings screen: font, color, and slider customization with live keyboard preview

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AppearanceSettingsScreen(
    prefs: PrefHelper,
    onNavigateBack: () -> Unit,
    onFontChanged: () -> Unit,
) {
    var fontType by remember { mutableStateOf(prefs.fontType) }
    var showFontPicker by remember { mutableStateOf(false) }
    var keyHeight by remember { mutableFloatStateOf(prefs.keyHeightScale) }
    var keyFontSize by remember { mutableFloatStateOf(prefs.keyFontSizeScale) }
    var candidateTextSize by remember { mutableFloatStateOf(prefs.candidateTextSizeScale) }
    var cornerRadius by remember { mutableFloatStateOf(prefs.keyCornerRadius) }
    var borderWidth by remember { mutableFloatStateOf(prefs.keyBorderWidth) }
    var colorSettings by remember {
        mutableStateOf(KeyboardColorSettings.fromJson(prefs.colorSettings))
    }
    var colorPickerTarget by remember { mutableStateOf<ColorPickerTarget?>(null) }

    val currentLayoutType by prefs
        .observeKeyboardLayoutType()
        .collectAsState(initial = prefs.keyboardLayoutType)

    val onColorSettingsChanged: (KeyboardColorSettings) -> Unit = { updated ->
        colorSettings = updated
        prefs.colorSettings = updated.toJson()
    }
    val onPickerOpen: (ColorPickerTarget) -> Unit = { colorPickerTarget = it }

    if (showFontPicker) {
        FontPickerContent(
            fontType = fontType,
            onFontSelected = { selected ->
                fontType = selected
                prefs.fontType = selected
                onFontChanged()
            },
            onNavigateBack = { showFontPicker = false },
        )
    } else {
        Scaffold(
            containerColor = MaterialTheme.colorScheme.surfaceContainer,
            topBar = {
                TopAppBar(
                    title = {
                        Text(
                            text = Tab2Texts.appearanceSettings,
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
                        .padding(innerPadding),
            ) {
                Column(
                    modifier =
                        Modifier
                            .weight(1f)
                            .verticalScroll(rememberScrollState())
                            .padding(horizontal = 20.dp)
                            .padding(top = 16.dp, bottom = 24.dp),
                ) {
                    SettingsCard {
                        Row(
                            modifier =
                                Modifier
                                    .fillMaxWidth()
                                    .heightIn(min = 48.dp)
                                    .clickable { showFontPicker = true }
                                    .padding(horizontal = 20.dp, vertical = 12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                text = Tab2Texts.customFont,
                                modifier = Modifier.weight(1f),
                                color = MaterialTheme.colorScheme.onSurface,
                                style = MaterialTheme.typography.bodyLarge,
                            )
                            Text(
                                text = fontDisplayName(fontType),
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                style = MaterialTheme.typography.bodyLarge,
                            )
                            Spacer(Modifier.width(4.dp))
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                                contentDescription = null,
                                modifier = Modifier.size(AppStyle.trailingChevronSize),
                                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }

                    Spacer(Modifier.height(24.dp))

                    SectionHeader(Tab2Texts.keyboardSection)
                    SettingsCard {
                        Column(modifier = Modifier.padding(24.dp)) {
                            ColorSettingRow(
                                label = Tab2Texts.colorKeyboardBackground,
                                currentColor = colorSettings.backgroundColor,
                                colorSettings = colorSettings,
                                onUpdate = { s, c -> s.copy(backgroundColor = c) },
                                onColorSettingsChanged = onColorSettingsChanged,
                                onPickerOpen = onPickerOpen,
                            )
                            SettingsDivider(Modifier.padding(vertical = 8.dp))
                            SliderRow(
                                label = Tab2Texts.keyHeight,
                                value = keyHeight,
                                valueFrom = 0.85f,
                                valueTo = 1.15f,
                                stepSize = 0.01f,
                                defaultValue = PrefHelper.DEFAULT_KEY_HEIGHT_SCALE,
                                onValueChange = {
                                    if (it != keyHeight) {
                                        keyHeight = it
                                        prefs.keyHeightScale = it
                                    }
                                },
                            )
                        }
                    }

                    Spacer(Modifier.height(24.dp))

                    SectionHeader(Tab2Texts.colorKeySection)
                    SettingsCard {
                        Column(modifier = Modifier.padding(24.dp)) {
                            ColorSettingRow(
                                label = Tab2Texts.colorKeyText,
                                currentColor = colorSettings.keyTextColor,
                                colorSettings = colorSettings,
                                onUpdate = { s, c -> s.copy(keyTextColor = c) },
                                onColorSettingsChanged = onColorSettingsChanged,
                                onPickerOpen = onPickerOpen,
                            )
                            SettingsDivider(Modifier.padding(vertical = 8.dp))
                            ColorSettingRow(
                                label = Tab2Texts.colorNormalKeyFill,
                                currentColor = colorSettings.normalKeyFillColor,
                                colorSettings = colorSettings,
                                onUpdate = { s, c -> s.copy(normalKeyFillColor = c) },
                                onColorSettingsChanged = onColorSettingsChanged,
                                onPickerOpen = onPickerOpen,
                            )
                            SettingsDivider(Modifier.padding(vertical = 8.dp))
                            ColorSettingRow(
                                label = Tab2Texts.colorSpecialKeyFill,
                                currentColor = colorSettings.specialKeyFillColor,
                                colorSettings = colorSettings,
                                onUpdate = { s, c -> s.copy(specialKeyFillColor = c) },
                                onColorSettingsChanged = onColorSettingsChanged,
                                onPickerOpen = onPickerOpen,
                            )
                            SettingsDivider(Modifier.padding(vertical = 8.dp))
                            SliderRow(
                                label = Tab2Texts.keyFontSize,
                                value = keyFontSize,
                                valueFrom = 0.85f,
                                valueTo = 1.15f,
                                stepSize = 0.01f,
                                defaultValue = PrefHelper.DEFAULT_KEY_FONT_SIZE_SCALE,
                                onValueChange = {
                                    if (it != keyFontSize) {
                                        keyFontSize = it
                                        prefs.keyFontSizeScale = it
                                    }
                                },
                            )
                            SettingsDivider(Modifier.padding(vertical = 8.dp))
                            SliderRow(
                                label = Tab2Texts.keyCornerRadius,
                                value = cornerRadius,
                                valueFrom = 0f,
                                valueTo = 15f,
                                stepSize = 0.5f,
                                defaultValue = PrefHelper.DEFAULT_KEY_CORNER_RADIUS,
                                onValueChange = {
                                    if (it != cornerRadius) {
                                        cornerRadius = it
                                        prefs.keyCornerRadius = it
                                    }
                                },
                            )
                            SettingsDivider(Modifier.padding(vertical = 8.dp))
                            SliderRow(
                                label = Tab2Texts.keyBorderWidth,
                                value = borderWidth,
                                valueFrom = 0f,
                                valueTo = 3f,
                                stepSize = 0.5f,
                                defaultValue = PrefHelper.DEFAULT_KEY_BORDER_WIDTH,
                                onValueChange = {
                                    if (it != borderWidth) {
                                        borderWidth = it
                                        prefs.keyBorderWidth = it
                                    }
                                },
                            )
                        }
                    }

                    Spacer(Modifier.height(24.dp))

                    SectionHeader(Tab2Texts.candidateSection)
                    SettingsCard {
                        Column(modifier = Modifier.padding(24.dp)) {
                            ColorSettingRow(
                                label = Tab2Texts.colorCandidateText,
                                currentColor = colorSettings.candidateTextColor,
                                colorSettings = colorSettings,
                                onUpdate = { s, c -> s.copy(candidateTextColor = c) },
                                onColorSettingsChanged = onColorSettingsChanged,
                                onPickerOpen = onPickerOpen,
                            )
                            SettingsDivider(Modifier.padding(vertical = 8.dp))
                            ColorSettingRow(
                                label = Tab2Texts.colorCandidateBackground,
                                currentColor = colorSettings.candidateBackgroundColor,
                                colorSettings = colorSettings,
                                onUpdate = { s, c -> s.copy(candidateBackgroundColor = c) },
                                onColorSettingsChanged = onColorSettingsChanged,
                                onPickerOpen = onPickerOpen,
                            )
                            SettingsDivider(Modifier.padding(vertical = 8.dp))
                            SliderRow(
                                label = Tab2Texts.candidateTextSize,
                                value = candidateTextSize,
                                valueFrom = 0.85f,
                                valueTo = 1.15f,
                                stepSize = 0.01f,
                                defaultValue = PrefHelper.DEFAULT_CANDIDATE_TEXT_SIZE_SCALE,
                                onValueChange = {
                                    if (it != candidateTextSize) {
                                        candidateTextSize = it
                                        prefs.candidateTextSizeScale = it
                                    }
                                },
                            )
                        }
                    }

                    Spacer(Modifier.height(24.dp))

                    SettingsCard {
                        ActionRow(
                            label = Tab2Texts.appearanceResetAll,
                            onClick = {
                                prefs.keyHeightScale = PrefHelper.DEFAULT_KEY_HEIGHT_SCALE
                                prefs.keyFontSizeScale = PrefHelper.DEFAULT_KEY_FONT_SIZE_SCALE
                                prefs.candidateTextSizeScale = PrefHelper.DEFAULT_CANDIDATE_TEXT_SIZE_SCALE
                                prefs.keyCornerRadius = PrefHelper.DEFAULT_KEY_CORNER_RADIUS
                                prefs.keyBorderWidth = PrefHelper.DEFAULT_KEY_BORDER_WIDTH
                                colorSettings = KeyboardColorSettings()
                                prefs.colorSettings = colorSettings.toJson()
                                prefs.fontType = PrefHelper.DEFAULT_FONT_TYPE
                                fontType = PrefHelper.DEFAULT_FONT_TYPE
                                keyHeight = PrefHelper.DEFAULT_KEY_HEIGHT_SCALE
                                keyFontSize = PrefHelper.DEFAULT_KEY_FONT_SIZE_SCALE
                                candidateTextSize = PrefHelper.DEFAULT_CANDIDATE_TEXT_SIZE_SCALE
                                cornerRadius = PrefHelper.DEFAULT_KEY_CORNER_RADIUS
                                borderWidth = PrefHelper.DEFAULT_KEY_BORDER_WIDTH
                                onFontChanged()
                            },
                            textColor = MaterialTheme.colorScheme.error,
                        )
                    }
                }

                HorizontalDivider()
                KeyboardPreviewPanel(
                    prefs = prefs,
                    layoutType = currentLayoutType,
                    colorSettings = colorSettings,
                    candidateTextSizeScale = candidateTextSize,
                    keyHeightScale = keyHeight,
                    keyFontSizeScale = keyFontSize,
                    keyCornerRadius = cornerRadius,
                    keyBorderWidth = borderWidth,
                    fontType = fontType,
                )
            }

            colorPickerTarget?.let { target ->
                ColorPickerDialog(
                    title = target.label,
                    currentColor = target.currentColor,
                    onDismiss = { colorPickerTarget = null },
                    onColorSelected = { newColor ->
                        target.onColorSelected(newColor)
                    },
                )
            }
        }
    }
}

@Composable
private fun ColorSettingRow(
    label: String,
    currentColor: Int?,
    colorSettings: KeyboardColorSettings,
    onUpdate: (KeyboardColorSettings, Int?) -> KeyboardColorSettings,
    onColorSettingsChanged: (KeyboardColorSettings) -> Unit,
    onPickerOpen: (ColorPickerTarget) -> Unit,
) {
    ColorRow(
        label = label,
        color = currentColor,
        onColorClick = {
            onPickerOpen(
                ColorPickerTarget(
                    label = label,
                    currentColor = currentColor,
                    onColorSelected = { newColor ->
                        onColorSettingsChanged(onUpdate(colorSettings, newColor))
                    },
                ),
            )
        },
        onReset = {
            onColorSettingsChanged(onUpdate(colorSettings, null))
        },
    )
}

private data class ColorPickerTarget(
    val label: String,
    val currentColor: Int?,
    val onColorSelected: (Int?) -> Unit,
)
