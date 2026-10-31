package com.siansiansu.taigikeyboard.ui.tabs.theme

import android.graphics.Bitmap
import android.net.Uri
import android.view.HapticFeedbackConstants
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.LocalStringResolver
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.settings.PrefHelper
import com.siansiansu.taigikeyboard.ime.theme.BuiltInTheme
import com.siansiansu.taigikeyboard.ime.theme.BuiltInThemes
import com.siansiansu.taigikeyboard.ime.theme.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.theme.ThemeAppearance
import com.siansiansu.taigikeyboard.ime.theme.ThemeBackground
import com.siansiansu.taigikeyboard.ime.theme.ThemeGradient
import com.siansiansu.taigikeyboard.ime.theme.ThemeImageBackground
import com.siansiansu.taigikeyboard.ime.theme.ThemeImageVariant
import com.siansiansu.taigikeyboard.ime.theme.ThemeKeyStyle
import com.siansiansu.taigikeyboard.ime.theme.UserTheme
import com.siansiansu.taigikeyboard.ime.theme.UserThemeSeed
import com.siansiansu.taigikeyboard.ime.theme.rememberThemePhoto
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.ColorRow
import com.siansiansu.taigikeyboard.ui.components.NameDialog
import com.siansiansu.taigikeyboard.ui.components.SegmentedChoiceRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SliderRow
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader
import kotlinx.coroutines.launch
import org.json.JSONObject

// User-theme editor: edits a single draft ThemeAppearance with a live keyboard
// preview pinned at the bottom — three sections, one per visual surface (USER
// 2026-09-19). The draft always carries concrete colors (a new theme starts from its
// baseline — a built-in's userThemeBase when customized from one, else USER_THEME_SEED — and
// an edited one was seeded at decode), so a user theme never follows light / dark and every
// color row has a value to reset to (the baseline's). The draft is local —
// nothing persists until Save, and Back discards. The name is entered in a Save-time
// dialog (no inline field), so the soft keyboard never squeezes the preview. A gradient's
// direction and a photo's position are set by dragging on the preview
// (GradientDirectionOverlay / PhotoPositionOverlay), not by form rows.
// Mirrors iOS ThemeEditorView.

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

private val PHOTO_THUMBNAIL_SIZE = 44.dp
private val PREVIEW_TOGGLE_MIN_HEIGHT = 32.dp

// Mirrors iOS ThemeEditorView previewToggle `.easeInOut(duration: 0.2)`.
private const val PREVIEW_TOGGLE_ANIMATION_MS = 200

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
    // Create only: the built-in a new theme is customized from (null = blank).
    base: BuiltInTheme?,
    canSaveNew: () -> Boolean,
    // Persists + auto-applies; returns false only when adding a NEW theme loses a
    // cap race (caller then surfaces the cap dialog). Editing always returns true.
    onSave: (name: String, appearance: ThemeAppearance, baseThemeId: String?) -> Boolean,
    onNavigateBack: () -> Unit,
) {
    // Persisted on the theme so later edits reset to the same base. Mirrors iOS
    // ThemeEditorViewModel.baseline / filledBaseline.
    val baseThemeId = editing?.baseThemeId ?: base?.id
    // What every reset restores: the built-in base, else the user-theme seed.
    val baseline = remember(baseThemeId) { BuiltInThemes.userThemeBaseline(baseThemeId) }
    // baseline with Filled keys (BuiltInThemes.userThemeFilledBaseline): the role-color reset
    // target and the first filledKeys.
    val filledBaseline = remember(baseThemeId) { BuiltInThemes.userThemeFilledBaseline(baseThemeId) }
    // The baseline's color as a solid background (ThemeBackground.leadColor).
    val baselineSolidColor = baseline.colors.background?.leadColor ?: UserThemeSeed.SOLID_COLOR
    var draft by rememberSaveable(stateSaver = appearanceSaver) {
        mutableStateOf(editing?.appearance ?: baseline)
    }
    val baseName = base?.let { stringRes(it.displayNameKey) }.orEmpty()
    var draftName by rememberSaveable { mutableStateOf(editing?.name ?: baseName) }
    var showNameDialog by rememberSaveable { mutableStateOf(false) }
    var showCapDialog by rememberSaveable { mutableStateOf(false) }
    var colorPickerTarget by remember { mutableStateOf<ColorPickerTarget?>(null) }
    // The pinned preview folds away so the settings get the whole screen to scroll in
    // (USER 2026-09-28). Mirrors iOS ThemeEditorView.isPreviewCollapsed.
    var isPreviewCollapsed by rememberSaveable { mutableStateOf(false) }

    val currentLayoutType by prefs
        .observeKeyboardLayoutType()
        .collectAsState(initial = prefs.keyboardLayoutType)

    val updateColors: ((KeyboardColorSettings) -> KeyboardColorSettings) -> Unit = { transform ->
        draft = draft.copy(colors = transform(draft.colors))
    }
    // Seeded drafts always carry a background; the seed is the last-resort read.
    val background = draft.colors.background ?: UserThemeSeed.BACKGROUND
    val setBackground: (ThemeBackground) -> Unit = { next -> updateColors { it.copy(background = next) } }
    // The Background segmented choice. Normally `background.kind`; it runs ahead of the background
    // while Photo is chosen but no photo has been picked yet, so the picker row shows without
    // the surface changing (the draft keeps its solid / gradient until a photo lands).
    // Mirrors iOS ThemeEditorViewModel.backgroundKind.
    var selectedKind by rememberSaveable { mutableStateOf(background.kind) }
    // The last Filled draft, so leaving Filled for Outlined / Borderless and coming back restores
    // its key fill, shadow and border. Recorded on leaving Filled; a theme opened see-through comes
    // back to filledBaseline's. Mirrors iOS ThemeEditorViewModel.filledKeys.
    var filledKeys by rememberSaveable(stateSaver = appearanceSaver) { mutableStateOf(filledBaseline) }
    val keyStyle = draft.keyStyle

    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val themeImageCache = remember(context) { CompositionRoot.shared(context).themeImages }
    val photoPicker =
        rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri: Uri? ->
            uri ?: return@rememberLauncherForActivityResult
            scope.launch {
                val file = themeImageCache.store.save(context.contentResolver, uri) ?: return@launch
                val dim = background.asImage?.dim ?: ThemeImageBackground.DEFAULT_DIM
                setBackground(ThemeBackground.Image(ThemeImageBackground(file, dim)))
            }
        }

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
                            contentDescription = L10n.commonBack,
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
                        Text(L10n.commonSave)
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
                // Background: one surface for keyboard + candidate bar.
                SectionHeader(L10n.themeBackgroundSection)
                SettingsCard {
                    Column(modifier = Modifier.padding(24.dp)) {
                        BackgroundKindRow(
                            kind = selectedKind,
                            onKindChange = { kind ->
                                // Switching keeps the current hue: solid -> gradient runs the solid
                                // color into a lighter tint of it; gradient -> solid keeps the first
                                // stop; leaving a photo lands on the baseline colour. Choosing Photo
                                // changes nothing until a photo is picked.
                                selectedKind = kind
                                if (kind == background.kind) return@BackgroundKindRow
                                val solidColor = background.leadColor ?: baselineSolidColor
                                when (kind) {
                                    ThemeBackground.Kind.SOLID -> setBackground(ThemeBackground.Solid(solidColor))
                                    ThemeBackground.Kind.GRADIENT -> setBackground(ThemeBackground.Gradient(ThemeGradient.seeded(solidColor)))
                                    ThemeBackground.Kind.IMAGE -> Unit
                                }
                            },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        when {
                            selectedKind == ThemeBackground.Kind.IMAGE -> {
                                val photo = background.asImage
                                PhotoRow(
                                    label = if (photo == null) L10n.themePhotoPick else L10n.themePhotoChange,
                                    bitmap = rememberThemePhoto(photo?.file, ThemeImageVariant.THUMBNAIL),
                                    onClick = { photoPicker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)) },
                                )
                                if (photo != null) {
                                    SettingsDivider(Modifier.padding(vertical = 8.dp))
                                    SliderRow(
                                        label = L10n.themePhotoDim,
                                        value = photo.dim,
                                        valueFrom = ThemeImageBackground.DIM_MIN,
                                        valueTo = ThemeImageBackground.DIM_MAX,
                                        stepSize = ThemeImageBackground.DIM_STEP,
                                        defaultValue = ThemeImageBackground.DEFAULT_DIM,
                                        onValueChange = { setBackground(ThemeBackground.Image(photo.copy(dim = it))) },
                                    )
                                }
                            }
                            background is ThemeBackground.Solid ->
                                RoleColorRow(
                                    labelKey = StringKey.THEME_COLOR_KEYBOARD_BACKGROUND,
                                    currentColor = background.color,
                                    // Solid reset stays Solid even when the baseline is a gradient.
                                    seedColor = baselineSolidColor,
                                    onColorChange = { setBackground(ThemeBackground.Solid(it)) },
                                    onPickerOpen = { colorPickerTarget = it },
                                )
                            background is ThemeBackground.Gradient -> {
                                val gradient = background.gradient
                                // The editor authors two-stop gradients; the two stops ARE the
                                // gradient, not overrides of a seed -> no reset icon.
                                // No Direction row: the pointer on the preview below is the direction control.
                                listOf(StringKey.THEME_GRADIENT_START_COLOR, StringKey.THEME_GRADIENT_END_COLOR)
                                    .forEachIndexed { index, labelKey ->
                                        if (index > 0) SettingsDivider(Modifier.padding(vertical = 8.dp))
                                        ColorSettingRow(
                                            labelKey = labelKey,
                                            currentColor = gradient.stops[index],
                                            onColorSelected = { picked ->
                                                val stops = gradient.stops.toMutableList().also { it[index] = picked }
                                                setBackground(ThemeBackground.Gradient(gradient.copy(stops = stops)))
                                            },
                                            onReset = null,
                                            onPickerOpen = { colorPickerTarget = it },
                                        )
                                    }
                            }
                        }
                    }
                }

                Spacer(Modifier.height(24.dp))

                // Keys: style, fill + text, then shape, then size. Outlined / Borderless keys are
                // see-through, so they hide the fill and shadow rows; Borderless also the border.
                SectionHeader(L10n.themeColorKeySection)
                SettingsCard {
                    Column(modifier = Modifier.padding(24.dp)) {
                        KeyStyleRow(
                            style = keyStyle,
                            onStyleChange = { style ->
                                if (style == keyStyle) return@KeyStyleRow
                                if (keyStyle == ThemeKeyStyle.CLASSIC) filledKeys = draft
                                draft = draft.withKeyStyle(style, filledKeys)
                            },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        if (!keyStyle.hasTransparentKeys) {
                            RoleColorRow(
                                labelKey = StringKey.THEME_COLOR_KEY_FILL,
                                currentColor = draft.colors.normalKeyFillColor,
                                seedColor = filledBaseline.colors.normalKeyFillColor ?: UserThemeSeed.KEY_FILL,
                                onColorChange = { v -> updateColors { it.withKeyFill(v) } },
                                onPickerOpen = { colorPickerTarget = it },
                            )
                            SettingsDivider(Modifier.padding(vertical = 8.dp))
                        }
                        RoleColorRow(
                            labelKey = StringKey.THEME_COLOR_KEY_TEXT,
                            currentColor = draft.colors.keyTextColor,
                            seedColor = filledBaseline.colors.keyTextColor ?: UserThemeSeed.KEY_TEXT,
                            onColorChange = { v -> updateColors { it.copy(keyTextColor = v) } },
                            onPickerOpen = { colorPickerTarget = it },
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
                        if (keyStyle != ThemeKeyStyle.CLEAN) {
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
                        }
                        if (!keyStyle.hasTransparentKeys) {
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
                    }
                }

                Spacer(Modifier.height(24.dp))

                // Candidates: text + highlight color, then size (the bar shares the background surface).
                SectionHeader(L10n.themeCandidateSection)
                SettingsCard {
                    Column(modifier = Modifier.padding(24.dp)) {
                        RoleColorRow(
                            labelKey = StringKey.THEME_COLOR_CANDIDATE_TEXT,
                            currentColor = draft.colors.candidateTextColor,
                            seedColor = filledBaseline.colors.candidateTextColor ?: UserThemeSeed.CANDIDATE_TEXT,
                            onColorChange = { v -> updateColors { it.copy(candidateTextColor = v) } },
                            onPickerOpen = { colorPickerTarget = it },
                        )
                        SettingsDivider(Modifier.padding(vertical = 8.dp))
                        // Shows the effective highlight (user pick, else derived from the gradient /
                        // key fill); reset returns to auto (null), not to a seed color.
                        ColorSettingRow(
                            labelKey = StringKey.THEME_COLOR_CANDIDATE_HIGHLIGHT,
                            currentColor = draft.colors.candidateTints?.first ?: UserThemeSeed.KEY_FILL,
                            onColorSelected = { v -> updateColors { it.copy(candidateHighlightColor = v) } },
                            onReset =
                                if (draft.colors.candidateHighlightColor != null) ({ updateColors { it.copy(candidateHighlightColor = null) } }) else null,
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
                        // Draft-only: resets the appearance to the baseline, keeps the name, persists
                        // nothing and never touches the applied theme until Save.
                        onClick = {
                            draft = baseline
                            selectedKind = (baseline.colors.background ?: UserThemeSeed.BACKGROUND).kind
                        },
                        textColor = MaterialTheme.colorScheme.error,
                    )
                }
            }

            HorizontalDivider()
            PreviewToggle(
                isCollapsed = isPreviewCollapsed,
                onToggle = { isPreviewCollapsed = !isPreviewCollapsed },
            )
            // While the background is a gradient the preview doubles as the direction
            // control (drag to set the angle); while it is a photo, as the position
            // control (drag to move, pinch to zoom the photo; a gesture hint shows
            // until the first touch).
            AnimatedVisibility(
                visible = !isPreviewCollapsed,
                enter = expandVertically(tween(PREVIEW_TOGGLE_ANIMATION_MS)),
                exit = shrinkVertically(tween(PREVIEW_TOGGLE_ANIMATION_MS)),
            ) {
                Box {
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
                    (background as? ThemeBackground.Gradient)?.let { gradientBackground ->
                        val gradient = gradientBackground.gradient
                        GradientDirectionOverlay(
                            label = L10n.themeGradientDirection,
                            angle = gradient.angle,
                            // Remembered per gradient so the overlay skips recomposition (and its
                            // redraw) when unrelated controls change.
                            onAngleChange = remember(gradient) { { angle: Float -> setBackground(ThemeBackground.Gradient(gradient.copy(angle = angle))) } },
                            modifier = Modifier.matchParentSize(),
                        )
                    }
                    (background as? ThemeBackground.Image)?.image?.let { photo ->
                        rememberThemePhoto(photo.file, ThemeImageVariant.FULL)?.let { bitmap ->
                            PhotoPositionOverlay(
                                label = L10n.themePhotoPosition,
                                moveHint = L10n.themePhotoHintMove,
                                zoomHint = L10n.themePhotoHintZoom,
                                imageWidth = bitmap.width,
                                imageHeight = bitmap.height,
                                photo = photo,
                                onPhotoChange = { setBackground(ThemeBackground.Image(it)) },
                                modifier = Modifier.matchParentSize(),
                            )
                        }
                    }
                }
            }
        }

        colorPickerTarget?.let { target ->
            ColorPickerDialog(
                title = stringRes(target.labelKey),
                currentColor = target.currentColor,
                onDismiss = { colorPickerTarget = null },
                onColorSelected = target.onColorSelected,
            )
        }

        if (showNameDialog) {
            NameDialog(
                title = L10n.themeNameHeader,
                placeholder = L10n.themeNamePlaceholder,
                initialName = draftName,
                onConfirm = { entered ->
                    showNameDialog = false
                    draftName = entered
                    val finalName = entered.trim().ifEmpty { stringResolver.resolve(StringKey.THEME_EDITOR_TITLE_NEW) }
                    if (!onSave(finalName, draft, baseThemeId)) showCapDialog = true
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

// Folds the pinned preview away (or back), freeing the screen for the settings list.
// Mirrors iOS ThemeEditorView.previewToggle.
@Composable
private fun PreviewToggle(
    isCollapsed: Boolean,
    onToggle: () -> Unit,
) {
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = PREVIEW_TOGGLE_MIN_HEIGHT)
                .background(MaterialTheme.colorScheme.surfaceContainer)
                .clickable(onClick = onToggle),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            imageVector = if (isCollapsed) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown,
            contentDescription = if (isCollapsed) L10n.themePreviewExpand else L10n.themePreviewCollapse,
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

// Solid / Gradient segmented choice for the background surface.
@Composable
private fun BackgroundKindRow(
    kind: ThemeBackground.Kind,
    onKindChange: (ThemeBackground.Kind) -> Unit,
) {
    val kinds = ThemeBackground.Kind.entries
    SegmentedChoiceRow(
        labels =
            kinds.map {
                when (it) {
                    ThemeBackground.Kind.SOLID -> L10n.themeBackgroundTypeSolid
                    ThemeBackground.Kind.GRADIENT -> L10n.themeBackgroundTypeGradient
                    ThemeBackground.Kind.IMAGE -> L10n.themeBackgroundTypePhoto
                }
            },
        selectedIndex = kinds.indexOf(kind),
        onSelect = { onKindChange(kinds[it]) },
    )
}

// The Filled / Outlined / Borderless key-style choice; labels reuse the built-in family names.
// Mirrors the iOS ThemeEditorView key-style Picker.
@Composable
private fun KeyStyleRow(
    style: ThemeKeyStyle,
    onStyleChange: (ThemeKeyStyle) -> Unit,
) {
    val styles = ThemeKeyStyle.entries
    SegmentedChoiceRow(
        labels =
            styles.map {
                when (it) {
                    ThemeKeyStyle.CLASSIC -> L10n.themeFamilyClassic
                    ThemeKeyStyle.FRAMED -> L10n.themeFamilyFramed
                    ThemeKeyStyle.CLEAN -> L10n.themeFamilyClean
                }
            },
        selectedIndex = styles.indexOf(style),
        onSelect = { onStyleChange(styles[it]) },
    )
}

// The photo picker row: a thumbnail of the current theme photo (or a placeholder while none
// is picked) beside the Choose Photo / Change Photo label; tapping the row opens the system photo
// picker (no storage permission needed). Mirrors iOS ThemePhotoRow.
@Composable
private fun PhotoRow(
    label: String,
    bitmap: Bitmap?,
    onClick: () -> Unit,
) {
    val view = LocalView.current
    // [bitmap] is the thumbnail decode (off the main thread), not the 1280 px photo.
    val thumbnail = remember(bitmap) { bitmap?.asImageBitmap() }
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = 48.dp)
                .clickable {
                    view.performHapticFeedback(HapticFeedbackConstants.CONTEXT_CLICK)
                    onClick()
                },
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier =
                Modifier
                    .size(PHOTO_THUMBNAIL_SIZE)
                    .clip(RoundedCornerShape(8.dp))
                    .background(MaterialTheme.colorScheme.surfaceContainerHighest),
        ) {
            if (thumbnail != null) {
                Image(
                    bitmap = thumbnail,
                    contentDescription = null,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.fillMaxSize(),
                )
            }
        }
        Spacer(Modifier.width(12.dp))
        Text(
            text = label,
            modifier = Modifier.weight(1f),
            color = MaterialTheme.colorScheme.onSurface,
            style = MaterialTheme.typography.bodyLarge,
        )
        Icon(
            painter = painterResource(R.drawable.ic_photo),
            contentDescription = null,
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.size(AppStyle.selectionIconSize),
        )
    }
}

// A seeded role color row: the reset icon shows while the value differs from [seedColor]
// and restores it through the same [onColorChange] (the single writer).
@Composable
private fun RoleColorRow(
    labelKey: StringKey,
    currentColor: Int?,
    seedColor: Int,
    onColorChange: (Int) -> Unit,
    onPickerOpen: (ColorPickerTarget) -> Unit,
) {
    val color = currentColor ?: seedColor
    ColorSettingRow(
        labelKey = labelKey,
        currentColor = color,
        onColorSelected = onColorChange,
        onReset = if (color != seedColor) ({ onColorChange(seedColor) }) else null,
        onPickerOpen = onPickerOpen,
    )
}

// Private to the theme editor (its sole consumer) — not hoisted to ui/components
// since no other screen color-edits anymore.
@Composable
private fun ColorSettingRow(
    labelKey: StringKey,
    currentColor: Int,
    onColorSelected: (Int) -> Unit,
    onReset: (() -> Unit)?,
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
                    onColorSelected = onColorSelected,
                ),
            )
        },
        onReset = onReset,
    )
}

private data class ColorPickerTarget(
    val labelKey: StringKey,
    val currentColor: Int,
    val onColorSelected: (Int) -> Unit,
)
