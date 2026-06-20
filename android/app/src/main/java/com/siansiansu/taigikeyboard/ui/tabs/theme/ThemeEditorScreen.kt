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
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.i18n.LocalStringResolver
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.ThemeAppearance
import com.siansiansu.taigikeyboard.ime.core.UserTheme
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

    // Resolver captured for the name-dialog callback (runs outside composition), so the
    // blank-name fallback resolves under the live display language at save time.
    val stringResolver by rememberUpdatedState(LocalStringResolver.current)

    Scaffold(
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = if (editing != null) L10n.themeEditorTitleEdit else L10n.themeEditorTitleNew,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = L10n.commonCancel,
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
                        Text(L10n.themeEditorSave)
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
                SectionHeader(L10n.themeKeyboardSection)
                SettingsCard {
                    Column(modifier = Modifier.padding(24.dp)) {
                        ColorSettingRow(
                            labelKey = StringKey.THEME_COLOR_KEYBOARD_BACKGROUND,
                            currentColor = draft.colors.backgroundColor,
                            colors = draft.colors,
                            onUpdate = { c, v -> c.copy(backgroundColor = v) },
                            onColorsChanged = onColorsChanged,
                            onPickerOpen = { colorPickerTarget = it },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = L10n.themeKeyHeight,
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

                SectionHeader(L10n.themeColorKeySection)
                SettingsCard {
                    Column(modifier = Modifier.padding(24.dp)) {
                        ColorSettingRow(
                            labelKey = StringKey.THEME_COLOR_KEY_TEXT,
                            currentColor = draft.colors.keyTextColor,
                            colors = draft.colors,
                            onUpdate = { c, v -> c.copy(keyTextColor = v) },
                            onColorsChanged = onColorsChanged,
                            onPickerOpen = { colorPickerTarget = it },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        ColorSettingRow(
                            labelKey = StringKey.THEME_COLOR_NORMAL_KEY_FILL,
                            currentColor = draft.colors.normalKeyFillColor,
                            colors = draft.colors,
                            onUpdate = { c, v -> c.copy(normalKeyFillColor = v) },
                            onColorsChanged = onColorsChanged,
                            onPickerOpen = { colorPickerTarget = it },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        ColorSettingRow(
                            labelKey = StringKey.THEME_COLOR_SPECIAL_KEY_FILL,
                            currentColor = draft.colors.specialKeyFillColor,
                            colors = draft.colors,
                            onUpdate = { c, v -> c.copy(specialKeyFillColor = v) },
                            onColorsChanged = onColorsChanged,
                            onPickerOpen = { colorPickerTarget = it },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = L10n.themeKeyFontSize,
                            value = draft.keyFontSizeScale,
                            valueFrom = SCALE_MIN,
                            valueTo = SCALE_MAX,
                            stepSize = SCALE_STEP,
                            defaultValue = ThemeAppearance.DEFAULT_KEY_FONT_SIZE_SCALE,
                            onValueChange = { if (it != draft.keyFontSizeScale) draft = draft.copy(keyFontSizeScale = it) },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = L10n.themeKeyCornerRadius,
                            value = draft.keyCornerRadius,
                            valueFrom = 0f,
                            valueTo = CORNER_RADIUS_MAX,
                            stepSize = CORNER_RADIUS_STEP,
                            defaultValue = ThemeAppearance.DEFAULT_KEY_CORNER_RADIUS,
                            onValueChange = { if (it != draft.keyCornerRadius) draft = draft.copy(keyCornerRadius = it) },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = L10n.themeKeyBorderWidth,
                            value = draft.keyBorderWidth,
                            valueFrom = 0f,
                            valueTo = BORDER_WIDTH_MAX,
                            stepSize = BORDER_WIDTH_STEP,
                            defaultValue = ThemeAppearance.DEFAULT_KEY_BORDER_WIDTH,
                            onValueChange = { if (it != draft.keyBorderWidth) draft = draft.copy(keyBorderWidth = it) },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = L10n.themeKeyShadow,
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

                SectionHeader(L10n.themeCandidateSection)
                SettingsCard {
                    Column(modifier = Modifier.padding(24.dp)) {
                        ColorSettingRow(
                            labelKey = StringKey.THEME_COLOR_CANDIDATE_TEXT,
                            currentColor = draft.colors.candidateTextColor,
                            colors = draft.colors,
                            onUpdate = { c, v -> c.copy(candidateTextColor = v) },
                            onColorsChanged = onColorsChanged,
                            onPickerOpen = { colorPickerTarget = it },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        ColorSettingRow(
                            labelKey = StringKey.THEME_COLOR_CANDIDATE_BACKGROUND,
                            currentColor = draft.colors.candidateBackgroundColor,
                            colors = draft.colors,
                            onUpdate = { c, v -> c.copy(candidateBackgroundColor = v) },
                            onColorsChanged = onColorsChanged,
                            onPickerOpen = { colorPickerTarget = it },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        SliderRow(
                            label = L10n.themeCandidateTextSize,
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
                        label = L10n.themeEditorResetAll,
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
                title = stringRes(target.labelKey),
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
                    val finalName = entered.trim().ifEmpty { stringResolver.resolve(StringKey.THEME_DEFAULT_NAME) }
                    if (!onSave(finalName, draft)) showCapDialog = true
                },
                onDismiss = { showNameDialog = false },
            )
        }

        if (showCapDialog) {
            AlertDialog(
                onDismissRequest = { showCapDialog = false },
                title = { Text(L10n.themeCapReachedTitle) },
                text = { Text(L10n.themeCapReachedMessage) },
                confirmButton = {
                    TextButton(onClick = { showCapDialog = false }) { Text(L10n.commonOk) }
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
        title = { Text(L10n.themeNameHeader) },
        text = {
            OutlinedTextField(
                value = name,
                onValueChange = { name = it },
                singleLine = true,
                placeholder = { Text(L10n.themeNamePlaceholder) },
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
            )
        },
        confirmButton = {
            TextButton(onClick = { onConfirm(name) }) { Text(L10n.themeEditorSave) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(L10n.commonCancel) }
        },
    )
}

// Private to the theme editor (its sole consumer) — not hoisted to ui/components
// since no other screen color-edits anymore.
@Composable
private fun ColorSettingRow(
    labelKey: StringKey,
    currentColor: Int?,
    colors: KeyboardColorSettings,
    onUpdate: (KeyboardColorSettings, Int?) -> KeyboardColorSettings,
    onColorsChanged: (KeyboardColorSettings) -> Unit,
    onPickerOpen: (ColorPickerTarget) -> Unit,
) {
    ColorRow(
        label = stringRes(labelKey),
        color = currentColor,
        onColorClick = {
            onPickerOpen(
                ColorPickerTarget(
                    // Store the key, not the resolved label, so the picker title live-switches
                    // with the display language instead of pinning the string captured at open.
                    labelKey = labelKey,
                    currentColor = currentColor,
                    onColorSelected = { newColor -> onColorsChanged(onUpdate(colors, newColor)) },
                ),
            )
        },
        onReset = { onColorsChanged(onUpdate(colors, null)) },
    )
}

private data class ColorPickerTarget(
    val labelKey: StringKey,
    val currentColor: Int?,
    val onColorSelected: (Int?) -> Unit,
)
