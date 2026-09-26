// IME root Composable — applies KeyboardMaterialTheme, resolves navbar insets (via declarative
// WindowInsets, avoiding the project_ime_window_arch.md dismiss-bug path), solves per-key
// geometry, pushes keyHeightFactor to the smartbar, and mounts KeyboardLayout for the current mode
// (narrowed + docked with a side panel in one-handed mode).

package com.siansiansu.taigikeyboard.ime.text.keyboard

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.mandatorySystemGestures
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.systemGestures
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.layout.positionInParent
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.unit.Dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.DisplayLanguage
import com.siansiansu.taigikeyboard.i18n.ProvideDisplayLanguage
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.settings.OneHandedMode
import com.siansiansu.taigikeyboard.ime.popup.PopupHost
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.theme.KeyboardMaterialTheme
import com.siansiansu.taigikeyboard.ime.theme.getColorFromAttr
import kotlinx.coroutines.flow.StateFlow
import kotlin.math.roundToInt

/**
 * Top-level keyboard body Composable hosted by `TextInputManager` inside the
 * IME root ComposeView. Owns four responsibilities:
 *
 * 1. Apply [KeyboardMaterialTheme] — Material3 theme wrapper required for all
 *    Compose subtrees in this codebase (Phase A/B/C convention).
 * 2. Resolve the bottom inset padding that used to live in
 *    `TaigiKeyboard.onCreateInputView`'s `setOnApplyWindowInsetsListener`
 *    block. Pins `INVARIANT_keyboard_navbar_inset_padding_factor` and
 *    `INVARIANT_keyboard_navbar_dismiss_bug_stays_resolved` (the imperative
 *    inset-listener path was the root cause behind the
 *    `project_ime_window_arch.md` dismiss bug; routing insets declaratively
 *    through Compose `WindowInsets` retires that hazard class).
 * 3. Solve per-key [KeyDimensions] via [BoxWithConstraints] + report the
 *    derived `keyHeightFactor` back to the smartbar through
 *    [onHeightFactorChanged] — replaces the legacy
 *    `KeyboardView.onMeasure → smartbarView.setHeightFactor` side effect.
 * 4. Mount [KeyboardLayout] for the active mode, sourced from the immutable
 *    [KeyboardUiState] published by `TextInputManager`. `LayoutManager`
 *    pre-publishes a layout into [KeyboardUiState.layouts] before the
 *    `activeMode` flips, so the active layout is non-null at every render.
 */
@Composable
fun KeyboardImeRoot(
    uiStateFlow: StateFlow<KeyboardUiState>,
    coordinator: KeyTouchCoordinator,
    popupHost: PopupHost,
    onHeightFactorChanged: (Float) -> Unit,
    prefs: PrefHelper,
    onOneHandedModeSelected: (OneHandedMode) -> Unit,
) {
    val uiState by uiStateFlow.collectAsState()
    val oneHandedModeFlow = remember(prefs) { prefs.observeOneHandedMode() }
    val oneHandedMode by oneHandedModeFlow.collectAsState(initial = prefs.oneHandedMode)
    val activeLayout = uiState.layouts[uiState.activeMode]
    val appearance = uiState.appearance

    KeyboardMaterialTheme {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .padding(bottom = computeBottomInsetPadding()),
        ) {
            if (activeLayout != null && appearance != null) {
                OneHandedFrame(
                    mode = oneHandedMode,
                    appearance = appearance,
                    prefs = prefs,
                    coordinator = coordinator,
                    onModeSelected = onOneHandedModeSelected,
                ) { keyAreaModifier ->
                    KeyboardSurface(
                        layout = activeLayout,
                        appearance = appearance,
                        keyVariation = uiState.keyVariation,
                        coordinator = coordinator,
                        popupHost = popupHost,
                        onHeightFactorChanged = onHeightFactorChanged,
                        modifier = keyAreaModifier,
                    )
                }
            }
        }
    }
}

@Composable
private fun KeyboardSurface(
    layout: KeyboardLayoutData,
    appearance: KeyboardAppearance,
    keyVariation: KeyVariation,
    coordinator: KeyTouchCoordinator,
    popupHost: PopupHost,
    onHeightFactorChanged: (Float) -> Unit,
    modifier: Modifier = Modifier,
) {
    val resources = LocalResources.current
    val density = LocalDensity.current
    val isLandscape = isLandscape()
    val baseKeyHeight = remember(resources) { resources.getDimension(R.dimen.key_height) }
    val keyMarginH = remember(density, resources) {
        with(density) { resources.getDimension(R.dimen.key_marginH).toInt() }
    }

    BoxWithConstraints(modifier = modifier) {
        val containerWidth = constraints.maxWidth
        val keyDimensions = remember(
            containerWidth,
            keyMarginH,
            baseKeyHeight,
            isLandscape,
            appearance.heightFactor,
            appearance.keyHeightScale,
        ) {
            KeyboardLayoutSolver.solveKeyDimensions(
                KeyDimensionsInput(
                    containerWidth = containerWidth,
                    keyMarginH = keyMarginH,
                    baseKeyHeight = baseKeyHeight,
                    isLandscape = isLandscape,
                    heightFactor = appearance.heightFactor,
                    keyHeightScale = appearance.keyHeightScale,
                ),
            )
        }
        LaunchedEffect(keyDimensions.keyHeightFactor) {
            onHeightFactorChanged(keyDimensions.keyHeightFactor)
        }
        KeyboardLayout(
            layoutData = layout,
            keyDimensions = keyDimensions,
            appearance = appearance,
            keyVariation = keyVariation,
            coordinator = coordinator,
            popupHost = popupHost,
        )
    }
}

/**
 * Replicates the legacy inset math that ran inside the imperative
 * `setOnApplyWindowInsetsListener` block:
 * `bottom = max(navigationBars, mandatorySystemGestures, systemGestures) × 0.9f`.
 * The 0.9× factor is preserved verbatim from
 * `TaigiKeyboard.onCreateInputView::adjustedHeight = (navBarHeight * 0.90f)` —
 * pins `INVARIANT_keyboard_navbar_inset_padding_factor`.
 */
@Composable
private fun computeBottomInsetPadding(): Dp =
    with(LocalDensity.current) {
        val navBars = WindowInsets.navigationBars.getBottom(this)
        val mandatory = WindowInsets.mandatorySystemGestures.getBottom(this)
        val gestures = WindowInsets.systemGestures.getBottom(this)
        val maxPx = maxOf(navBars, mandatory, gestures)
        (maxPx * 0.9f).toDp()
    }

/**
 * Lays the key area out full width, or — one-handed — at [OneHandedMode.KEY_AREA_FRACTION]
 * docked to one edge with [OneHandedSidePanel] in the gap (mirrors iOS KeyboardKit docking).
 * The key area is sized by its own constraints, so [KeyboardLayoutSolver] solves keys for the
 * narrowed width; its x offset goes to [coordinator] for the popup window anchor.
 */
@Composable
private fun OneHandedFrame(
    mode: OneHandedMode,
    appearance: KeyboardAppearance,
    prefs: PrefHelper,
    coordinator: KeyTouchCoordinator,
    onModeSelected: (OneHandedMode) -> Unit,
    keyArea: @Composable (Modifier) -> Unit,
) {
    val keyAreaFraction = if (mode == OneHandedMode.OFF) 1f else OneHandedMode.KEY_AREA_FRACTION
    Box(modifier = Modifier.fillMaxWidth()) {
        keyArea(
            Modifier
                .fillMaxWidth(keyAreaFraction)
                .align(if (mode == OneHandedMode.RIGHT) Alignment.TopEnd else Alignment.TopStart)
                .onPlaced { coordinator.keyAreaOffsetX = it.positionInParent().x.roundToInt() },
        )
        if (mode != OneHandedMode.OFF) {
            val context = LocalContext.current
            // Same colours as the key body: a custom background is painted on the common parent,
            // otherwise the theme's keyboard background; icons follow the key text colour.
            val colors = appearance.colorSettings
            val gapBackground = remember(context, colors) {
                if (colors.background != null) Color.Transparent else Color(getColorFromAttr(context, R.attr.keyboard_bgColor))
            }
            val tint = remember(context, colors) {
                colors.keyTextColor?.let { Color(it) } ?: Color(getColorFromAttr(context, R.attr.key_fgColor))
            }
            val displayLanguageFlow = remember(prefs) { prefs.observeDisplayLanguage() }
            val displayLanguageTag by displayLanguageFlow.collectAsState(initial = prefs.displayLanguageTag)
            Box(modifier = Modifier.matchParentSize()) {
                ProvideDisplayLanguage(DisplayLanguage.fromTag(displayLanguageTag)) {
                    OneHandedSidePanel(
                        mode = mode,
                        tint = tint,
                        onModeSelected = onModeSelected,
                        modifier = Modifier
                            .fillMaxHeight()
                            .fillMaxWidth(1f - OneHandedMode.KEY_AREA_FRACTION)
                            .align(if (mode == OneHandedMode.LEFT) Alignment.CenterEnd else Alignment.CenterStart)
                            .background(gapBackground),
                    )
                }
            }
        }
    }
}
