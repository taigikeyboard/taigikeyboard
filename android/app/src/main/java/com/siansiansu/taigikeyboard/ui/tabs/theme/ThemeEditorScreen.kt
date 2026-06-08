package com.siansiansu.taigikeyboard.ui.tabs.theme

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.ThemeAppearance
import com.siansiansu.taigikeyboard.ime.core.UserTheme
import com.siansiansu.taigikeyboard.localization.ThemeTexts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.ColorRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SliderRow
import com.siansiansu.taigikeyboard.ui.tabs.layout.ColorPickerDialog
import com.siansiansu.taigikeyboard.ui.tabs.layout.KeyboardPreviewPanel
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader
import org.json.JSONObject

// User-theme editor: edits a single draft ThemeAppearance (6 colors + 5 size
// scalars + key shadow) with a live keyboard preview pinned at the bottom. The
// draft is local — nothing persists until Save, and Back discards. The name is
// entered in a Save-time dialog (no inline field), so the soft keyboard never
// squeezes the preview. Reuses ColorRow / SliderRow / ColorPickerDialog /
// KeyboardPreviewPanel. Mirrors iOS ThemeEditorView.

// Slider ranges (mirror the Layout-tab appearance editor; shadow is editor-only).
private const val SCALE_MIN = 0.85f
private const val SCALE_MAX = 1.15f
private const val SCALE_STEP = 0.01f
private const val CORNER_RADIUS_MAX = 15f
private const val CORNER_RADIUS_STEP = 0.5f
private const val BORDER_WIDTH_MAX = 3f
private const val BORDER_WIDTH_STEP = 0.5f
private const val SHADOW_MAX = 4f
private const val SHADOW_STEP = 1f

// Saver so the draft survives Activity recreation (rotation / process death).
private val appearanceSaver: Saver<ThemeAppearance, String> =
    Saver(
        save = { it.toJson().toString() },
        restore = { ThemeAppearance.fromJson(JSONObject(it)) },
    )

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ThemeEditorScreen(
    prefs: PrefHelper,
    editing: UserTheme?,
    canSaveNew: () -> Boolean,
    // Persists + auto-applies; returns false only when adding a NEW theme loses a
    // cap race (caller then surfaces the cap dialog). Editing always returns true.
    onSave: (name: String, appearance: ThemeAppearance) -> Boolean,
    onNavigateBack: () -> Unit,
) {
    var draft by rememberSaveable(stateSaver = appearanceSaver) {
        mutableStateOf(editing?.appearance ?: ThemeAppearance.DEFAULT)
    }
    var draftName by rememberSaveable { mutableStateOf(editing?.name ?: "") }
    var showNameDialog by rememberSaveable { mutableStateOf(false) }
    var showCapDialog by rememberSaveable { mutableStateOf(false) }
    var colorPickerTarget by remember { mutableStateOf<ColorPickerTarget?>(null) }

    val currentLayoutType by prefs
        .observeKeyboardLayoutType()
        .collectAsState(initial = prefs.keyboardLayoutType)

    val onColorsChanged: (KeyboardColorSettings) -> Unit = { draft = draft.copy(colors = it) }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = if (editing != null) ThemeTexts.editorTitleEdit else ThemeTexts.editorTitleNew,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = ThemeTexts.editorCancel,
                            tint = MaterialTheme.colorScheme.onSurface,
                        )
                    }
                },
                actions = {
                    TextButton(
                        onClick = {
                            // Cap-check NEW themes up front so the cap dialog and the name
                            // dialog never present back-to-back.
                            if (editing == null && !canSaveNew()) {
                                showCapDialog = true
                            } else {
                                showNameDialog = true
                            }
                        },
                    ) {
                        Text(ThemeTexts.editorSave)
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
                SectionHeader(ThemeTexts.keyboardSection)
                SettingsCard {
                    Column(modifier = Modifier.padding(24.dp)) {
                        ColorSettingRow(
                            label = ThemeTexts.colorKeyboardBackground,
                            currentColor = draft.colors.backgroundColor,
                            colors = draft.colors,
                            onUpdate = { c, v -> c.copy(backgroundColor = v) },
                            onColorsChanged = onColorsChanged,
                            onPickerOpen = { colorPickerTarget = it },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = ThemeTexts.keyHeight,
                            value = draft.keyHeightScale,
                            valueFrom = SCALE_MIN,
                            valueTo = SCALE_MAX,
                            stepSize = SCALE_STEP,
                            defaultValue = ThemeAppearance.DEFAULT_KEY_HEIGHT_SCALE,
                            onValueChange = { if (it != draft.keyHeightScale) draft = draft.copy(keyHeightScale = it) },
                        )
                    }
                }

                Spacer(Modifier.height(24.dp))

                SectionHeader(ThemeTexts.colorKeySection)
                SettingsCard {
                    Column(modifier = Modifier.padding(24.dp)) {
                        ColorSettingRow(
                            label = ThemeTexts.colorKeyText,
                            currentColor = draft.colors.keyTextColor,
                            colors = draft.colors,
                            onUpdate = { c, v -> c.copy(keyTextColor = v) },
                            onColorsChanged = onColorsChanged,
                            onPickerOpen = { colorPickerTarget = it },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        ColorSettingRow(
                            label = ThemeTexts.colorNormalKeyFill,
                            currentColor = draft.colors.normalKeyFillColor,
                            colors = draft.colors,
                            onUpdate = { c, v -> c.copy(normalKeyFillColor = v) },
                            onColorsChanged = onColorsChanged,
                            onPickerOpen = { colorPickerTarget = it },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        ColorSettingRow(
                            label = ThemeTexts.colorSpecialKeyFill,
                            currentColor = draft.colors.specialKeyFillColor,
                            colors = draft.colors,
                            onUpdate = { c, v -> c.copy(specialKeyFillColor = v) },
                            onColorsChanged = onColorsChanged,
                            onPickerOpen = { colorPickerTarget = it },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = ThemeTexts.keyFontSize,
                            value = draft.keyFontSizeScale,
                            valueFrom = SCALE_MIN,
                            valueTo = SCALE_MAX,
                            stepSize = SCALE_STEP,
                            defaultValue = ThemeAppearance.DEFAULT_KEY_FONT_SIZE_SCALE,
                            onValueChange = { if (it != draft.keyFontSizeScale) draft = draft.copy(keyFontSizeScale = it) },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = ThemeTexts.keyCornerRadius,
                            value = draft.keyCornerRadius,
                            valueFrom = 0f,
                            valueTo = CORNER_RADIUS_MAX,
                            stepSize = CORNER_RADIUS_STEP,
                            defaultValue = ThemeAppearance.DEFAULT_KEY_CORNER_RADIUS,
                            onValueChange = { if (it != draft.keyCornerRadius) draft = draft.copy(keyCornerRadius = it) },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = ThemeTexts.keyBorderWidth,
                            value = draft.keyBorderWidth,
                            valueFrom = 0f,
                            valueTo = BORDER_WIDTH_MAX,
                            stepSize = BORDER_WIDTH_STEP,
                            defaultValue = ThemeAppearance.DEFAULT_KEY_BORDER_WIDTH,
                            onValueChange = { if (it != draft.keyBorderWidth) draft = draft.copy(keyBorderWidth = it) },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = ThemeTexts.keyShadow,
                            value = draft.keyShadowIntensity,
                            valueFrom = 0f,
                            valueTo = SHADOW_MAX,
                            stepSize = SHADOW_STEP,
                            defaultValue = ThemeAppearance.DEFAULT_KEY_SHADOW_INTENSITY,
                            onValueChange = { if (it != draft.keyShadowIntensity) draft = draft.copy(keyShadowIntensity = it) },
                        )
                    }
                }

                Spacer(Modifier.height(24.dp))

                SectionHeader(ThemeTexts.candidateSection)
                SettingsCard {
                    Column(modifier = Modifier.padding(24.dp)) {
                        ColorSettingRow(
                            label = ThemeTexts.colorCandidateText,
                            currentColor = draft.colors.candidateTextColor,
                            colors = draft.colors,
                            onUpdate = { c, v -> c.copy(candidateTextColor = v) },
                            onColorsChanged = onColorsChanged,
                            onPickerOpen = { colorPickerTarget = it },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        ColorSettingRow(
                            label = ThemeTexts.colorCandidateBackground,
                            currentColor = draft.colors.candidateBackgroundColor,
                            colors = draft.colors,
                            onUpdate = { c, v -> c.copy(candidateBackgroundColor = v) },
                            onColorsChanged = onColorsChanged,
                            onPickerOpen = { colorPickerTarget = it },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = ThemeTexts.candidateTextSize,
                            value = draft.candidateTextSizeScale,
                            valueFrom = SCALE_MIN,
                            valueTo = SCALE_MAX,
                            stepSize = SCALE_STEP,
                            defaultValue = ThemeAppearance.DEFAULT_CANDIDATE_TEXT_SIZE_SCALE,
                            onValueChange = {
                                if (it != draft.candidateTextSizeScale) draft = draft.copy(candidateTextSizeScale = it)
                            },
                        )
                    }
                }

                Spacer(Modifier.height(24.dp))

                SettingsCard {
                    ActionRow(
                        label = ThemeTexts.editorResetAll,
                        // Draft-only: resets the appearance, keeps the name, persists nothing
                        // and never touches the applied theme until Save.
                        onClick = { draft = ThemeAppearance.DEFAULT },
                        textColor = MaterialTheme.colorScheme.error,
                    )
                }
            }

            HorizontalDivider()
            KeyboardPreviewPanel(
                prefs = prefs,
                layoutType = currentLayoutType,
                colorSettings = draft.colors,
                candidateTextSizeScale = draft.candidateTextSizeScale,
                keyHeightScale = draft.keyHeightScale,
                keyFontSizeScale = draft.keyFontSizeScale,
                keyCornerRadius = draft.keyCornerRadius,
                keyBorderWidth = draft.keyBorderWidth,
                fontType = prefs.fontType,
                keyShadowIntensity = draft.keyShadowIntensity,
            )
        }

        colorPickerTarget?.let { target ->
            ColorPickerDialog(
                title = target.label,
                currentColor = target.currentColor,
                onDismiss = { colorPickerTarget = null },
                onColorSelected = { target.onColorSelected(it) },
            )
        }

        if (showNameDialog) {
            ThemeNameDialog(
                initialName = draftName,
                onConfirm = { entered ->
                    showNameDialog = false
                    draftName = entered
                    val finalName = entered.trim().ifEmpty { ThemeTexts.defaultThemeName }
                    if (!onSave(finalName, draft)) showCapDialog = true
                },
                onDismiss = { showNameDialog = false },
            )
        }

        if (showCapDialog) {
            AlertDialog(
                onDismissRequest = { showCapDialog = false },
                title = { Text(ThemeTexts.capReachedTitle) },
                text = { Text(ThemeTexts.capReachedMessage) },
                confirmButton = {
                    TextButton(onClick = { showCapDialog = false }) { Text(ThemeTexts.capReachedOK) }
                },
            )
        }
    }
}

@Composable
private fun ThemeNameDialog(
    initialName: String,
    onConfirm: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    var name by rememberSaveable(initialName) { mutableStateOf(initialName) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(ThemeTexts.themeNameHeader) },
        text = {
            OutlinedTextField(
                value = name,
                onValueChange = { name = it },
                singleLine = true,
                placeholder = { Text(ThemeTexts.themeNamePlaceholder) },
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
            )
        },
        confirmButton = {
            TextButton(onClick = { onConfirm(name) }) { Text(ThemeTexts.editorSave) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(ThemeTexts.editorCancel) }
        },
    )
}

// P5: ColorSettingRow + ColorPickerTarget duplicate the private copies in
// AppearanceSettingsScreen.kt. They consolidate into ui/components when P5 deletes
// that screen — kept separate now to avoid a cross-package hoist mid-migration.
@Composable
private fun ColorSettingRow(
    label: String,
    currentColor: Int?,
    colors: KeyboardColorSettings,
    onUpdate: (KeyboardColorSettings, Int?) -> KeyboardColorSettings,
    onColorsChanged: (KeyboardColorSettings) -> Unit,
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
                    onColorSelected = { newColor -> onColorsChanged(onUpdate(colors, newColor)) },
                ),
            )
        },
        onReset = { onColorsChanged(onUpdate(colors, null)) },
    )
}

private data class ColorPickerTarget(
    val label: String,
    val currentColor: Int?,
    val onColorSelected: (Int?) -> Unit,
)
