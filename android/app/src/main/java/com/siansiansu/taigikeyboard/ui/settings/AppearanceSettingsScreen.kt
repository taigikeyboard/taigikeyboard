package com.siansiansu.taigikeyboard.ui.settings

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
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab2Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.ColorRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SliderRow

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AppearanceSettingsScreen(
    languageManager: LanguageManager,
    prefs: PrefHelper,
    onNavigateBack: () -> Unit,
    onFontChanged: () -> Unit
) {
    val language by languageManager.currentLanguageFlow.collectAsState()

    // Font type state
    var fontType by remember { mutableStateOf(prefs.fontType) }

    // Font picker sub-page state
    var showFontPicker by remember { mutableStateOf(false) }

    // Slider states
    var keyHeight by remember { mutableFloatStateOf(prefs.keyHeightScale) }
    var keyFontSize by remember { mutableFloatStateOf(prefs.keyFontSizeScale) }
    var candidateTextSize by remember { mutableFloatStateOf(prefs.candidateTextSizeScale) }
    var cornerRadius by remember { mutableFloatStateOf(prefs.keyCornerRadius) }
    var borderWidth by remember { mutableFloatStateOf(prefs.keyBorderWidth) }

    // Color settings state
    var colorSettings by remember {
        mutableStateOf(KeyboardColorSettings.fromJson(prefs.colorSettings))
    }

    // Color picker dialog state
    var colorPickerTarget by remember { mutableStateOf<ColorPickerTarget?>(null) }

    // Preview refresh counter — increment to force KeyboardView recreation
    var previewKey by remember { mutableIntStateOf(0) }

    // Observe layout type so preview updates when user changes layout
    val currentLayoutType by prefs.observeKeyboardLayoutType()
        .collectAsState(initial = prefs.keyboardLayoutType)

    if (showFontPicker) {
        FontPickerContent(
            languageManager = languageManager,
            fontType = fontType,
            onFontSelected = { selected ->
                fontType = selected
                prefs.fontType = selected
                onFontChanged()
            },
            onNavigateBack = { showFontPicker = false }
        )
    } else {
        Scaffold(
            containerColor = MaterialTheme.colorScheme.surfaceContainer,
            topBar = {
                TopAppBar(
                    title = {
                        Text(
                            text = languageManager.text(Tab2Texts.appearanceSettings),
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
            ) {
            // Scrollable settings area
            Column(
                modifier = Modifier
                    .weight(1f)
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 20.dp)
                    .padding(top = 16.dp, bottom = 24.dp)
            ) {
                // Card 1: Font — navigation to sub-page
                SettingsCard {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(min = 48.dp)
                            .clickable { showFontPicker = true }
                            .padding(horizontal = 20.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = languageManager.text(Tab2Texts.customFont),
                            modifier = Modifier.weight(1f),
                            fontSize = AppStyle.bodyFontSize,
                            color = MaterialTheme.colorScheme.onSurface
                        )
                        Text(
                            text = fontDisplayName(fontType, languageManager),
                            fontSize = AppStyle.bodyFontSize,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Spacer(Modifier.width(4.dp))
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                            contentDescription = null,
                            modifier = Modifier.size(18.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }

                Spacer(Modifier.height(24.dp))

                // Card 2: Keyboard (齒盤介面)
                Text(
                    text = languageManager.text(Tab2Texts.keyboardSection),
                    fontSize = AppStyle.sectionHeaderFontSize,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, bottom = 6.dp)
                )
                SettingsCard {
                    Column(modifier = Modifier.padding(24.dp)) {
                        ColorRow(
                            label = languageManager.text(Tab2Texts.colorKeyboardBackground),
                            color = colorSettings.backgroundColor,
                            onColorClick = {
                                colorPickerTarget = ColorPickerTarget(
                                    label = languageManager.text(Tab2Texts.colorKeyboardBackground),
                                    currentColor = colorSettings.backgroundColor,
                                    onColorSelected = { newColor ->
                                        colorSettings = colorSettings.copy(backgroundColor = newColor)
                                        prefs.colorSettings = colorSettings.toJson()
                                    }
                                )
                            },
                            onReset = {
                                colorSettings = colorSettings.copy(backgroundColor = null)
                                prefs.colorSettings = colorSettings.toJson()
                                previewKey++
                            }
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = languageManager.text(Tab2Texts.keyHeight),
                            value = keyHeight,
                            valueFrom = 0.85f,
                            valueTo = 1.15f,
                            stepSize = 0.01f,
                            defaultValue = 1.0f,
                            onValueChange = { keyHeight = it; prefs.keyHeightScale = it; previewKey++ }
                        )
                    }
                }

                Spacer(Modifier.height(24.dp))

                // Card 3: Key (揤鈕介面)
                Text(
                    text = languageManager.text(Tab2Texts.colorKeySection),
                    fontSize = AppStyle.sectionHeaderFontSize,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, bottom = 6.dp)
                )
                SettingsCard {
                    Column(modifier = Modifier.padding(24.dp)) {
                        ColorRow(
                            label = languageManager.text(Tab2Texts.colorKeyText),
                            color = colorSettings.keyTextColor,
                            onColorClick = {
                                colorPickerTarget = ColorPickerTarget(
                                    label = languageManager.text(Tab2Texts.colorKeyText),
                                    currentColor = colorSettings.keyTextColor,
                                    onColorSelected = { newColor ->
                                        colorSettings = colorSettings.copy(keyTextColor = newColor)
                                        prefs.colorSettings = colorSettings.toJson()
                                    }
                                )
                            },
                            onReset = {
                                colorSettings = colorSettings.copy(keyTextColor = null)
                                prefs.colorSettings = colorSettings.toJson()
                                previewKey++
                            }
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        ColorRow(
                            label = languageManager.text(Tab2Texts.colorNormalKeyFill),
                            color = colorSettings.normalKeyFillColor,
                            onColorClick = {
                                colorPickerTarget = ColorPickerTarget(
                                    label = languageManager.text(Tab2Texts.colorNormalKeyFill),
                                    currentColor = colorSettings.normalKeyFillColor,
                                    onColorSelected = { newColor ->
                                        colorSettings = colorSettings.copy(normalKeyFillColor = newColor)
                                        prefs.colorSettings = colorSettings.toJson()
                                    }
                                )
                            },
                            onReset = {
                                colorSettings = colorSettings.copy(normalKeyFillColor = null)
                                prefs.colorSettings = colorSettings.toJson()
                                previewKey++
                            }
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        ColorRow(
                            label = languageManager.text(Tab2Texts.colorSpecialKeyFill),
                            color = colorSettings.specialKeyFillColor,
                            onColorClick = {
                                colorPickerTarget = ColorPickerTarget(
                                    label = languageManager.text(Tab2Texts.colorSpecialKeyFill),
                                    currentColor = colorSettings.specialKeyFillColor,
                                    onColorSelected = { newColor ->
                                        colorSettings = colorSettings.copy(specialKeyFillColor = newColor)
                                        prefs.colorSettings = colorSettings.toJson()
                                    }
                                )
                            },
                            onReset = {
                                colorSettings = colorSettings.copy(specialKeyFillColor = null)
                                prefs.colorSettings = colorSettings.toJson()
                                previewKey++
                            }
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = languageManager.text(Tab2Texts.keyFontSize),
                            value = keyFontSize,
                            valueFrom = 0.85f,
                            valueTo = 1.15f,
                            stepSize = 0.01f,
                            defaultValue = 1.0f,
                            onValueChange = { keyFontSize = it; prefs.keyFontSizeScale = it; previewKey++ }
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = languageManager.text(Tab2Texts.keyCornerRadius),
                            value = cornerRadius,
                            valueFrom = 0f,
                            valueTo = 15f,
                            stepSize = 0.5f,
                            defaultValue = 6.0f,
                            onValueChange = { cornerRadius = it; prefs.keyCornerRadius = it; previewKey++ }
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = languageManager.text(Tab2Texts.keyBorderWidth),
                            value = borderWidth,
                            valueFrom = 0f,
                            valueTo = 3f,
                            stepSize = 0.5f,
                            defaultValue = 0.0f,
                            onValueChange = { borderWidth = it; prefs.keyBorderWidth = it; previewKey++ }
                        )
                    }
                }

                Spacer(Modifier.height(24.dp))

                // Card 4: Candidate (候選詞介面)
                Text(
                    text = languageManager.text(Tab2Texts.candidateSection),
                    fontSize = AppStyle.sectionHeaderFontSize,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, bottom = 6.dp)
                )
                SettingsCard {
                    Column(modifier = Modifier.padding(24.dp)) {
                        ColorRow(
                            label = languageManager.text(Tab2Texts.colorCandidateText),
                            color = colorSettings.candidateTextColor,
                            onColorClick = {
                                colorPickerTarget = ColorPickerTarget(
                                    label = languageManager.text(Tab2Texts.colorCandidateText),
                                    currentColor = colorSettings.candidateTextColor,
                                    onColorSelected = { newColor ->
                                        colorSettings = colorSettings.copy(candidateTextColor = newColor)
                                        prefs.colorSettings = colorSettings.toJson()
                                    }
                                )
                            },
                            onReset = {
                                colorSettings = colorSettings.copy(candidateTextColor = null)
                                prefs.colorSettings = colorSettings.toJson()
                                previewKey++
                            }
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        ColorRow(
                            label = languageManager.text(Tab2Texts.colorCandidateBackground),
                            color = colorSettings.candidateBackgroundColor,
                            onColorClick = {
                                colorPickerTarget = ColorPickerTarget(
                                    label = languageManager.text(Tab2Texts.colorCandidateBackground),
                                    currentColor = colorSettings.candidateBackgroundColor,
                                    onColorSelected = { newColor ->
                                        colorSettings = colorSettings.copy(candidateBackgroundColor = newColor)
                                        prefs.colorSettings = colorSettings.toJson()
                                    }
                                )
                            },
                            onReset = {
                                colorSettings = colorSettings.copy(candidateBackgroundColor = null)
                                prefs.colorSettings = colorSettings.toJson()
                                previewKey++
                            }
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = languageManager.text(Tab2Texts.candidateTextSize),
                            value = candidateTextSize,
                            valueFrom = 0.85f,
                            valueTo = 1.15f,
                            stepSize = 0.01f,
                            defaultValue = 1.0f,
                            onValueChange = { candidateTextSize = it; prefs.candidateTextSizeScale = it; previewKey++ }
                        )
                    }
                }

                Spacer(Modifier.height(24.dp))

                // Card 5: Reset
                SettingsCard {
                    ActionRow(
                        label = languageManager.text(Tab2Texts.appearanceResetAll),
                        onClick = {
                            prefs.keyHeightScale = 1.0f
                            prefs.keyFontSizeScale = 1.0f
                            prefs.candidateTextSizeScale = 1.0f
                            prefs.keyCornerRadius = 6.0f
                            prefs.keyBorderWidth = 0.0f
                            prefs.colorSettings = "{}"
                            prefs.fontType = "openHuninn"
                            // Update all states
                            fontType = "openHuninn"
                            keyHeight = 1.0f
                            keyFontSize = 1.0f
                            candidateTextSize = 1.0f
                            cornerRadius = 6.0f
                            borderWidth = 0.0f
                            colorSettings = KeyboardColorSettings()
                            previewKey++
                            onFontChanged()
                        },
                        textColor = MaterialTheme.colorScheme.error
                    )
                }
            }

            // Keyboard preview anchored at bottom
            HorizontalDivider()
            KeyboardPreviewPanel(
                prefs = prefs,
                previewKey = previewKey,
                layoutType = currentLayoutType,
                colorSettings = colorSettings,
                candidateTextSizeScale = candidateTextSize,
                fontType = fontType
            )

            } // outer Column

            // Color picker dialog
            colorPickerTarget?.let { target ->
                ColorPickerDialog(
                    title = target.label,
                    currentColor = target.currentColor,
                    onDismiss = { colorPickerTarget = null },
                    onColorSelected = { newColor ->
                        target.onColorSelected(newColor)
                        previewKey++
                    }
                )
            }
        }
    }
}

private data class ColorPickerTarget(
    val label: String,
    val currentColor: Int?,
    val onColorSelected: (Int?) -> Unit
)
