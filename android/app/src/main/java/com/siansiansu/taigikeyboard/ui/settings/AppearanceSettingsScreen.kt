package com.siansiansu.taigikeyboard.ui.settings

import android.graphics.Bitmap
import android.view.ContextThemeWrapper
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.withStyle
import com.siansiansu.taigikeyboard.util.FontUtils
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.rememberModalBottomSheetState
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.Subtype
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardView
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutManager
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab2Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import kotlin.math.roundToInt

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
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
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
                        containerColor = MaterialTheme.colorScheme.surfaceContainerLow
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
                            fontSize = 16.sp,
                            color = MaterialTheme.colorScheme.onSurface
                        )
                        Text(
                            text = fontDisplayName(fontType, languageManager),
                            fontSize = 14.sp,
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
                    fontSize = 16.sp,
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
                    fontSize = 16.sp,
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
                    fontSize = 16.sp,
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

// Font picker sub-page (mimics iOS NavigationLink behavior)
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun FontPickerContent(
    languageManager: LanguageManager,
    fontType: String,
    onFontSelected: (String) -> Unit,
    onNavigateBack: () -> Unit
) {
    Scaffold(
        containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
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
                    containerColor = MaterialTheme.colorScheme.surfaceContainerLow
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
            fontSize = 16.sp,
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

private fun fontDisplayName(fontType: String, languageManager: LanguageManager): String {
    return when (fontType) {
        "system" -> languageManager.text(Tab2Texts.fontSystemDefault)
        "openHuninn" -> languageManager.text(Tab2Texts.fontOpenHuninn)
        "iansui" -> languageManager.text(Tab2Texts.fontIansui)
        else -> fontType
    }
}

private data class ColorPickerTarget(
    val label: String,
    val currentColor: Int?,
    val onColorSelected: (Int?) -> Unit
)

@Composable
private fun SliderRow(
    label: String,
    value: Float,
    valueFrom: Float,
    valueTo: Float,
    stepSize: Float,
    defaultValue: Float? = null,
    onValueChange: (Float) -> Unit
) {
    Column(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = label,
                fontSize = 16.sp,
                color = MaterialTheme.colorScheme.onSurface,
                modifier = Modifier.weight(1f)
            )
            if (defaultValue != null && value != defaultValue) {
                Icon(
                    imageVector = Icons.Default.Refresh,
                    contentDescription = "Reset",
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier
                        .size(20.dp)
                        .clickable { onValueChange(defaultValue) }
                )
            }
        }
        Slider(
            value = value.coerceIn(valueFrom, valueTo),
            onValueChange = onValueChange,
            valueRange = valueFrom..valueTo,
            steps = ((valueTo - valueFrom) / stepSize).toInt() - 1,
            modifier = Modifier.fillMaxWidth()
        )
    }
}

@Composable
private fun ColorRow(
    label: String,
    color: Int?,
    onColorClick: () -> Unit,
    onReset: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = label,
            modifier = Modifier.weight(1f),
            fontSize = 16.sp,
            color = MaterialTheme.colorScheme.onSurface
        )

        // Color swatch
        Box(
            modifier = Modifier
                .size(32.dp)
                .clip(CircleShape)
                .background(
                    if (color != null) Color(color) else Color.LightGray
                )
                .border(
                    width = if (color != null) 1.dp else 2.dp,
                    color = if (color != null) Color.Gray else Color.DarkGray,
                    shape = CircleShape
                )
                .clickable(onClick = onColorClick)
        )

        if (color != null) {
            Spacer(Modifier.width(4.dp))
            Icon(
                imageVector = Icons.Default.Refresh,
                contentDescription = "Reset",
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier
                    .size(20.dp)
                    .clickable(onClick = onReset)
            )
        }
    }
}

// ============= Color Picker Dialog (iOS-style tabs, rendering from flutter_colorpicker) =============

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ColorPickerDialog(
    title: String,
    currentColor: Int?,
    onDismiss: () -> Unit,
    onColorSelected: (Int?) -> Unit
) {
    val defaultColor = android.graphics.Color.rgb(213, 214, 221)
    val initialHsv = remember {
        floatArrayOf(0f, 0f, 0f).also { hsv ->
            android.graphics.Color.colorToHSV(currentColor ?: defaultColor, hsv)
        }
    }
    var hue by remember { mutableFloatStateOf(initialHsv[0]) }
    var saturation by remember { mutableFloatStateOf(initialHsv[1]) }
    var brightness by remember { mutableFloatStateOf(initialHsv[2]) }
    var hexInput by remember {
        mutableStateOf(String.format("#%06X", 0xFFFFFF and (currentColor ?: defaultColor)))
    }
    var selectedTab by remember { mutableIntStateOf(1) } // default: Spectrum

    fun currentColorInt(): Int =
        android.graphics.Color.HSVToColor(floatArrayOf(hue, saturation, brightness))

    fun updateFromColor(color: Int) {
        val hsv = floatArrayOf(0f, 0f, 0f)
        android.graphics.Color.colorToHSV(color, hsv)
        hue = hsv[0]; saturation = hsv[1]; brightness = hsv[2]
        hexInput = String.format("#%06X", 0xFFFFFF and color)
    }

    fun updateHex() {
        hexInput = String.format("#%06X", 0xFFFFFF and currentColorInt())
    }

    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        tonalElevation = 6.dp
    ) {
        Column(modifier = Modifier.padding(horizontal = 24.dp).padding(bottom = 24.dp)) {
            // Title row with close button
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = title,
                    fontSize = 18.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface,
                    modifier = Modifier.weight(1f)
                )
                Icon(
                    imageVector = Icons.Default.Close,
                    contentDescription = "Close",
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier
                        .size(24.dp)
                        .clickable(onClick = onDismiss)
                )
            }

            Spacer(Modifier.height(16.dp))

            // iOS-style segmented tabs
            val tabLabels = listOf("格仔", "光譜", "滑桿")
            SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                tabLabels.forEachIndexed { index, label ->
                    SegmentedButton(
                        selected = selectedTab == index,
                        onClick = { selectedTab = index },
                        shape = SegmentedButtonDefaults.itemShape(index, tabLabels.size),
                        icon = {}
                    ) { Text(label, fontSize = 13.sp) }
                }
            }

            Spacer(Modifier.height(16.dp))

            // Tab content (fixed height)
            Box(modifier = Modifier.fillMaxWidth().height(280.dp)) {
                when (selectedTab) {
                    0 -> ColorGridContent(
                        onColorSelected = { updateFromColor(it); onColorSelected(currentColorInt()) }
                    )
                    1 -> ColorSpectrumContent(
                        hue = hue,
                        saturation = saturation,
                        brightness = brightness,
                        onHsvChanged = { h, s, v ->
                            hue = h; saturation = s; brightness = v; updateHex()
                            onColorSelected(currentColorInt())
                        }
                    )
                    2 -> ColorSlidersContent(
                        colorInt = currentColorInt(),
                        onColorChanged = { updateFromColor(it); onColorSelected(currentColorInt()) }
                    )
                }
            }

            Spacer(Modifier.height(16.dp))

            // Preview swatch + hex input
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Box(
                    modifier = Modifier
                        .size(40.dp)
                        .clip(CircleShape)
                        .background(Color(currentColorInt()))
                        .border(1.dp, Color.Gray, CircleShape)
                )
                Spacer(Modifier.width(12.dp))
                OutlinedTextField(
                    value = hexInput,
                    onValueChange = { input ->
                        hexInput = input
                        try {
                            val hex = input.trim()
                            val parsed = android.graphics.Color.parseColor(
                                if (hex.startsWith("#")) hex else "#$hex"
                            )
                            val hsv = floatArrayOf(0f, 0f, 0f)
                            android.graphics.Color.colorToHSV(parsed, hsv)
                            hue = hsv[0]; saturation = hsv[1]; brightness = hsv[2]
                            onColorSelected(currentColorInt())
                        } catch (_: Exception) { }
                    },
                    label = { Text("#RRGGBB") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Ascii),
                    modifier = Modifier.weight(1f)
                )
            }
        }
    }
}

// ============= Tab 1: 隔線 (iOS-style preset color swatches, no gaps) =============

private val presetColors: List<Int> = buildList {
    // 12 hues: 淺藍 → 深藍 → 紫 → 粉紅 → 紅 → 黃 → 綠 (evenly spaced 285° arc)
    val hues = listOf(195f, 221f, 247f, 273f, 299f, 325f, 351f, 17f, 43f, 69f, 95f, 121f)
    // Row 1: Grayscale (12 swatches)
    for (i in 0..11) {
        val v = 255 - (i * 255 / 11)
        add(android.graphics.Color.rgb(v, v, v))
    }
    // Rows 2-9: 12 hues × 8 lightness levels (light → dark)
    val levels = listOf(
        1.00f to 0.40f, // very dark
        1.00f to 0.60f, // dark
        1.00f to 0.80f, // medium
        1.00f to 1.00f, // vivid
        0.75f to 1.00f, // bright
        0.50f to 1.00f, // medium light
        0.30f to 1.00f, // light pastel
        0.15f to 1.00f  // very light pastel
    )
    for ((s, v) in levels) {
        for (h in hues) {
            add(android.graphics.Color.HSVToColor(floatArrayOf(h, s, v)))
        }
    }
}

@Composable
private fun ColorGridContent(onColorSelected: (Int) -> Unit) {
    Column(modifier = Modifier.fillMaxSize()) {
        presetColors.chunked(12).forEach { row ->
            Row(modifier = Modifier.fillMaxWidth().weight(1f)) {
                row.forEach { color ->
                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .fillMaxSize()
                            .background(Color(color))
                            .clickable { onColorSelected(color) }
                    )
                }
            }
        }
    }
}

// ============= Tab 2: 光譜 (rotated 90° CCW: X=lightness, Y=hue) =============

@Composable
private fun ColorSpectrumContent(
    hue: Float,
    saturation: Float,
    brightness: Float,
    onHsvChanged: (h: Float, s: Float, v: Float) -> Unit
) {
    // Pre-render spectrum bitmap (rotated 90° CCW): X=lightness(white→black), Y=hue(360°→0°)
    val spectrumBitmap = remember {
        val w = 200; val h = 360
        val bitmap = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
        val hsv = floatArrayOf(0f, 0f, 0f)
        for (x in 0 until w) {
            val ratio = x.toFloat() / (w - 1)
            if (ratio <= 0.5f) { hsv[1] = ratio * 2f; hsv[2] = 1f }
            else { hsv[1] = 1f; hsv[2] = 1f - (ratio - 0.5f) * 2f }
            for (y in 0 until h) {
                hsv[0] = y.toFloat() / (h - 1) * 360f
                bitmap.setPixel(x, y, android.graphics.Color.HSVToColor(hsv))
            }
        }
        bitmap.asImageBitmap()
    }

    // Selector position from HSV (rotated: X=lightness, Y=hue inverted)
    val selectorNormX = if (brightness >= 0.99f) saturation * 0.5f
        else 0.5f + (1f - brightness) * 0.5f
    val selectorNormY = hue / 360f

    Canvas(
        modifier = Modifier
            .fillMaxSize()
            .clip(RoundedCornerShape(8.dp))
            .pointerInput(Unit) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    val update = { offset: Offset ->
                        val nx = (offset.x / size.width).coerceIn(0f, 1f)
                        val ny = (offset.y / size.height).coerceIn(0f, 1f)
                        val h = ny * 360f
                        val s: Float; val v: Float
                        if (nx <= 0.5f) { s = nx * 2f; v = 1f }
                        else { s = 1f; v = 1f - (nx - 0.5f) * 2f }
                        onHsvChanged(h, s, v)
                    }
                    update(down.position); down.consume()
                    do {
                        val event = awaitPointerEvent()
                        event.changes.forEach { it.consume(); update(it.position) }
                    } while (event.changes.any { it.pressed })
                }
            }
    ) {
        drawImage(
            image = spectrumBitmap,
            srcOffset = IntOffset.Zero,
            srcSize = IntSize(spectrumBitmap.width, spectrumBitmap.height),
            dstOffset = IntOffset.Zero,
            dstSize = IntSize(size.width.roundToInt(), size.height.roundToInt())
        )
        // Selector: white ring + black outline (visible on any background)
        val sx = selectorNormX * size.width
        val sy = selectorNormY * size.height
        drawCircle(Color.Black, 10.dp.toPx(), Offset(sx, sy), style = Stroke(1.5.dp.toPx()))
        drawCircle(Color.White, 8.5.dp.toPx(), Offset(sx, sy), style = Stroke(2.dp.toPx()))
    }
}

// ============= Tab 3: 滑桿 (ported from flutter_colorpicker TrackPainter + ThumbPainter) =============

@Composable
private fun ColorSlidersContent(
    colorInt: Int,
    onColorChanged: (Int) -> Unit
) {
    val r = (colorInt shr 16) and 0xFF
    val g = (colorInt shr 8) and 0xFF
    val b = colorInt and 0xFF

    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        // R slider — gradient: (0,G,B) → (255,G,B) (from TrackPainter.red)
        GradientColorSlider(
            label = "紅色",
            value = r / 255f,
            trackColors = listOf(
                Color(android.graphics.Color.rgb(0, g, b)),
                Color(android.graphics.Color.rgb(255, g, b))
            ),
            thumbColor = Color(colorInt),
            onValueChange = { onColorChanged(android.graphics.Color.rgb((it * 255).toInt(), g, b)) }
        )
        // G slider — gradient: (R,0,B) → (R,255,B) (from TrackPainter.green)
        GradientColorSlider(
            label = "綠色",
            value = g / 255f,
            trackColors = listOf(
                Color(android.graphics.Color.rgb(r, 0, b)),
                Color(android.graphics.Color.rgb(r, 255, b))
            ),
            thumbColor = Color(colorInt),
            onValueChange = { onColorChanged(android.graphics.Color.rgb(r, (it * 255).toInt(), b)) }
        )
        // B slider — gradient: (R,G,0) → (R,G,255) (from TrackPainter.blue)
        GradientColorSlider(
            label = "藍色",
            value = b / 255f,
            trackColors = listOf(
                Color(android.graphics.Color.rgb(r, g, 0)),
                Color(android.graphics.Color.rgb(r, g, 255))
            ),
            thumbColor = Color(colorInt),
            onValueChange = { onColorChanged(android.graphics.Color.rgb(r, g, (it * 255).toInt())) }
        )
    }
}

/** Custom gradient slider ported from flutter_colorpicker TrackPainter + ThumbPainter */
@Composable
private fun GradientColorSlider(
    label: String,
    value: Float,
    trackColors: List<Color>,
    thumbColor: Color,
    onValueChange: (Float) -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = label,
            fontSize = 15.sp,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.width(40.dp)
        )
        Canvas(
            modifier = Modifier
                .weight(1f)
                .height(36.dp)
                .pointerInput(Unit) {
                    awaitEachGesture {
                        val down = awaitFirstDown()
                        val update = { offset: Offset ->
                            onValueChange((offset.x / size.width).coerceIn(0f, 1f))
                        }
                        update(down.position); down.consume()
                        do {
                            val event = awaitPointerEvent()
                            event.changes.forEach { it.consume(); update(it.position) }
                        } while (event.changes.any { it.pressed })
                    }
                }
        ) {
            val trackHeight = 10.dp.toPx()
            val trackY = (size.height - trackHeight) / 2f
            val thumbRadius = 11.dp.toPx()

            // Gradient track (from TrackPainter)
            drawRoundRect(
                brush = Brush.horizontalGradient(trackColors),
                topLeft = Offset(0f, trackY),
                size = androidx.compose.ui.geometry.Size(size.width, trackHeight),
                cornerRadius = CornerRadius(trackHeight / 2f)
            )

            // Thumb (from ThumbPainter): shadow → white fill → color fill
            val thumbX = value * size.width
            val thumbCenter = Offset(thumbX, size.height / 2f)
            drawCircle(Color(0x30000000), thumbRadius + 2.dp.toPx(), thumbCenter)
            drawCircle(Color.White, thumbRadius, thumbCenter)
            drawCircle(thumbColor, thumbRadius * 0.7f, thumbCenter)
        }
        Text(
            text = "${(value * 255).toInt()}",
            fontSize = 14.sp,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.width(36.dp),
            textAlign = TextAlign.End
        )
    }
}

// ============= Keyboard Preview (using real KeyboardView with isPreviewMode) =============

@Composable
private fun KeyboardPreviewPanel(
    prefs: PrefHelper,
    previewKey: Int,
    layoutType: String,
    colorSettings: KeyboardColorSettings,
    candidateTextSizeScale: Float,
    fontType: String
) {
    Column(modifier = Modifier.fillMaxWidth()) {
        // Candidate bar preview (matches iOS KeyboardPreviewPanel sample suggestions)
        CandidatePreviewRow(
            colorSettings = colorSettings,
            candidateTextSizeScale = candidateTextSizeScale,
            fontType = fontType
        )

        // Use key() to force full AndroidView recreation when previewKey or layoutType changes.
        // This recomputes the layout from current prefs (layout type, height, colors, etc.)
        // matching iOS KeyboardPreviewPanel which re-evaluates on every state change.
        key(previewKey, layoutType) {
            AndroidView(
                factory = { ctx ->
                    val themedContext = ContextThemeWrapper(ctx, R.style.KeyboardTheme)
                    val layoutManager = LayoutManager(themedContext, prefs)
                    // Always show Taigi layout in preview (match iOS KeyboardPreviewPanel behavior)
                    // Use overrideInputMode to avoid showing English layout
                    val layout = layoutManager.fetchComputedLayoutForPreview(
                        KeyboardMode.CHARACTERS,
                        Subtype.DEFAULT
                    )
                    KeyboardView(themedContext).apply {
                        this.prefs = prefs
                        this.isPreviewMode = true
                        this.computedLayout = layout
                        // Filter keys by variation (hides duplicate TRANSLATE keys etc.)
                        updateVisibility()
                    }
                },
                modifier = Modifier.fillMaxWidth()
            )
        }
    }
}

// ============= Candidate Bar Preview (matches actual Smartbar styling) =============

private data class SampleCandidate(val roman: String, val hanzi: String)

private val sampleCandidates = listOf(
    SampleCandidate("mī-tê", "麵茶"),
    SampleCandidate("kú-nî", "久年"),
    SampleCandidate("gîm-á", "砛仔")
)

/** Resolve a color attribute from KeyboardTheme (single source of truth: themes.xml). */
private fun resolveKeyboardThemeColor(context: android.content.Context, attrId: Int): Color {
    val themed = android.view.ContextThemeWrapper(context, R.style.KeyboardTheme)
    val tv = android.util.TypedValue()
    themed.theme.resolveAttribute(attrId, tv, true)
    return Color(tv.data)
}

@Composable
private fun CandidatePreviewRow(
    colorSettings: KeyboardColorSettings,
    candidateTextSizeScale: Float,
    fontType: String
) {
    val context = LocalContext.current
    val typeface = remember(fontType) {
        FontUtils.getTypefaceByType(fontType, context)
    }
    val fontFamily = remember(typeface) { FontFamily(typeface) }

    // Resolve default colors from KeyboardTheme (auto light/dark via DayNight parent)
    val defaultBgColor = remember { resolveKeyboardThemeColor(context, R.attr.smartbar_bgColor) }
    val defaultTextColor = remember { resolveKeyboardThemeColor(context, R.attr.smartbar_candidate_fgColor) }
    val subtitleColor = remember { resolveKeyboardThemeColor(context, R.attr.smartbar_candidate_subtitle_fgColor) }
    val composingBgColor = remember { resolveKeyboardThemeColor(context, R.attr.semiTransparentColor) }
    // Icon tint: matches smartbar toolbar_toggle_button and expand_toggle_button tint
    val iconTint = remember { resolveKeyboardThemeColor(context, R.attr.smartbar_fgColor) }

    val bgColor = colorSettings.candidateBackgroundColor?.let { Color(it) } ?: defaultBgColor
    val textColor = colorSettings.candidateTextColor?.let { Color(it) } ?: defaultTextColor
    val effectiveSubtitleColor = colorSettings.candidateTextColor?.let { Color(it) } ?: subtitleColor

    // Text size: smartbarHeight(50dp) * 0.46 = 23sp, scaled by user preference
    val baseSizeSp = 23.sp * candidateTextSizeScale
    val subtitleSizeSp = baseSizeSp * 0.70f

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .height(50.dp)
            .background(bgColor),
        verticalAlignment = Alignment.CenterVertically
    ) {
        // "+" toolbar toggle button (left side, matches smartbar.xml toolbar_toggle_button)
        Icon(
            painter = painterResource(R.drawable.ic_add),
            contentDescription = null,
            modifier = Modifier
                .width(36.dp)
                .padding(start = 2.dp)
                .padding(6.dp),
            tint = iconTint
        )

        // Candidate items (fill remaining space)
        Row(
            modifier = Modifier.weight(1f),
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically
        ) {
            sampleCandidates.forEachIndexed { index, candidate ->
                // First candidate has composing background (matches candidate_composing_background)
                val itemBg = if (index == 0) composingBgColor else Color.Transparent
                Text(
                    text = buildAnnotatedString {
                        append(candidate.roman)
                        append(" ")
                        withStyle(
                            SpanStyle(
                                fontSize = subtitleSizeSp,
                                color = effectiveSubtitleColor
                            )
                        ) {
                            append(candidate.hanzi)
                        }
                    },
                    fontSize = baseSizeSp,
                    color = textColor,
                    fontFamily = fontFamily,
                    maxLines = 1,
                    textAlign = TextAlign.Center,
                    modifier = Modifier
                        .background(itemBg, RoundedCornerShape(8.dp))
                        .padding(horizontal = 6.dp, vertical = 3.dp)
                )
                // Add spacing between items (matching margin * 5 = 5dp each side)
                if (index < sampleCandidates.size - 1) {
                    Spacer(Modifier.width(10.dp))
                }
            }
        }

        // Vertical divider (matches smartbar.xml candidate_divider)
        Box(
            modifier = Modifier
                .width(1.dp)
                .height(32.dp)
                .background(iconTint.copy(alpha = 0.3f))
        )

        // Expand/collapse chevron (right side, matches smartbar.xml expand_toggle_button)
        Icon(
            painter = painterResource(R.drawable.ic_keyboard_arrow_down),
            contentDescription = null,
            modifier = Modifier
                .width(48.dp)
                .padding(end = 4.dp)
                .padding(2.dp),
            tint = iconTint
        )
    }
}
