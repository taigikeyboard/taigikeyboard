package com.siansiansu.taigikeyboard.ui.tabs.layout

import android.content.Context
import android.view.ContextThemeWrapper
import android.view.inputmethod.EditorInfo
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.Subtype
import com.siansiansu.taigikeyboard.ime.core.ThemeAppearance
import com.siansiansu.taigikeyboard.ime.popup.KeyAnchor
import com.siansiansu.taigikeyboard.ime.popup.NoOpPopupHost
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyBounds
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyDimensionsInput
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyEventDispatcher
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyTouchCoordinator
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardAppearance
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardHeightFactor
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardLayout
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardLayoutData
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardLayoutSolver
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.siansiansu.taigikeyboard.ime.text.keyboard.isLandscape
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutManager
import com.siansiansu.taigikeyboard.ime.theme.getColorFromAttr
import com.siansiansu.taigikeyboard.localization.SettingsTexts
import com.siansiansu.taigikeyboard.typeface.TypefaceLoader

// Live keyboard preview panel with candidate bar for appearance settings

@Composable
fun KeyboardPreviewPanel(
    prefs: PrefHelper,
    layoutType: String,
    colorSettings: KeyboardColorSettings,
    candidateTextSizeScale: Float,
    keyHeightScale: Float,
    keyFontSizeScale: Float,
    keyCornerRadius: Float,
    keyBorderWidth: Float,
    fontType: String,
    // The theme editor passes the draft theme's shadow so the preview matches the
    // saved key look. The Layout-tab appearance editor omits it (it has no shadow
    // control) and stays flat.
    keyShadowIntensity: Float = ThemeAppearance.DEFAULT_KEY_SHADOW_INTENSITY,
) {
    Column(modifier = Modifier.fillMaxWidth()) {
        CandidatePreviewRow(
            colorSettings = colorSettings,
            candidateTextSizeScale = candidateTextSizeScale,
            fontType = fontType,
        )

        val context = LocalContext.current
        val resources = LocalResources.current
        // Force Taigi mode preview when user is in English mode — mirrors
        // `LayoutManager.fetchComputedLayoutForPreview`.
        val previewInputMode = if (prefs.inputMode == "english") "tl" else prefs.inputMode

        // Settings activity runs under SettingsTheme; the IME runtime runs under
        // KeyboardTheme. `KeyboardLayout` / `KeyContent` resolve key colors via
        // `LocalContext.current` + `getColorFromAttr(R.attr.key_*)` — under
        // SettingsTheme those attrs miss and resolve to 0 (transparent), which
        // hides the entire keyboard body. Provide a themed context to the
        // preview subtree so attr lookups land on KeyboardTheme.
        val themedContext = remember(context) {
            ContextThemeWrapper(context, R.style.KeyboardTheme)
        }

        val layoutData = remember(layoutType, previewInputMode, themedContext) {
            val layoutManager = LayoutManager(themedContext, prefs)
            KeyboardLayoutData.from(
                layoutManager.fetchComputedLayoutForPreview(
                    KeyboardMode.CHARACTERS,
                    Subtype.DEFAULT,
                ),
            )
        }

        val typeface = remember(fontType, context) {
            TypefaceLoader.getTypefaceByType(fontType, context)
        }

        val appearance = KeyboardAppearance(
            keyboardLayoutType = layoutType,
            inputMode = previewInputMode,
            caps = false,
            capsLock = false,
            isComposing = false,
            isTranslateSwapped = false,
            imeOptions = EditorInfo.IME_ACTION_NONE,
            confirmKeyLabel = SettingsTexts.confirmKeyLabel(previewInputMode, false),
            colorSettings = colorSettings,
            typeface = typeface,
            keyFontSizeScale = keyFontSizeScale,
            keyCornerRadius = keyCornerRadius,
            keyBorderWidth = keyBorderWidth,
            keyShadowIntensity = keyShadowIntensity,
            heightFactor = KeyboardHeightFactor.fromPreferenceString(prefs.heightFactor),
            keyHeightScale = keyHeightScale,
        )

        // Solve dimensions inline so `KeyboardLayout` can be invoked with
        // a non-null `keyDimensions` argument. `BoxWithConstraints` would
        // also work but the explicit solve keeps the preview composable
        // identical in shape to its production-side counterpart.
        val density = androidx.compose.ui.platform.LocalDensity.current
        androidx.compose.foundation.layout.BoxWithConstraints(modifier = Modifier.fillMaxWidth()) {
            val containerWidthPx = constraints.maxWidth
            if (containerWidthPx == 0) return@BoxWithConstraints
            val keyMarginH = with(density) {
                resources.getDimension(R.dimen.key_marginH).toInt()
            }
            val baseKeyHeight = resources.getDimension(R.dimen.key_height)
            val keyDimensions = KeyboardLayoutSolver.solveKeyDimensions(
                KeyDimensionsInput(
                    containerWidth = containerWidthPx,
                    keyMarginH = keyMarginH,
                    baseKeyHeight = baseKeyHeight,
                    isLandscape = isLandscape(),
                    heightFactor = appearance.heightFactor,
                    keyHeightScale = keyHeightScale,
                ),
            )
            val coordinator = remember { KeyTouchCoordinator(NoOpPopupHost, NoOpKeyEventDispatcher) }
            CompositionLocalProvider(LocalContext provides themedContext) {
                KeyboardLayout(
                    layoutData = layoutData,
                    keyDimensions = keyDimensions,
                    appearance = appearance,
                    keyVariation = KeyVariation.NORMAL,
                    coordinator = coordinator,
                    popupHost = NoOpPopupHost,
                    isPreview = true,
                )
            }
        }
    }
}

// Preview mode never fires pointer events into the coordinator — the
// `Modifier.pointerInteropFilter` branch in [KeyboardLayout] short-circuits
// to `coordinator.previewHitTest` and never calls back into the dispatcher.
// The stub satisfies the type contract; the methods are unreachable.
private object NoOpKeyEventDispatcher : KeyEventDispatcher {
    override fun dispatchKeyPress(data: KeyData) = Unit

    override fun showInputMethodPicker() = Unit

    override fun keyPressVibrate() = Unit

    override fun keyPressSound(data: KeyData) = Unit

    override val longPressDelayMs: Long = 0L

    override fun resolveAnchor(
        bounds: KeyBounds,
        keyboardWidth: Int,
        desiredKeyWidth: Int,
        desiredKeyHeight: Int,
    ): KeyAnchor =
        KeyAnchor(
            data = bounds.data,
            measuredWidth = bounds.visible.width,
            measuredHeight = bounds.visible.height,
            xInKeyboard = bounds.visible.left,
            keyboardWidth = keyboardWidth,
            xInWindow = 0,
            yInWindow = 0,
            computedLabel = "",
            popupCells = emptyList(),
            isLandscape = false,
            desiredKeyWidth = desiredKeyWidth,
            desiredKeyHeight = desiredKeyHeight,
        )
}

private data class SampleCandidate(
    val roman: String,
    val hanzi: String,
)

private val sampleCandidates =
    listOf(
        SampleCandidate("mī-tê", "麵茶"),
        SampleCandidate("kú-nî", "久年"),
        SampleCandidate("gîm-á", "砛仔"),
    )

private fun resolveKeyboardThemeColor(
    context: Context,
    attrId: Int,
): Color {
    val themed = ContextThemeWrapper(context, R.style.KeyboardTheme)
    return Color(getColorFromAttr(themed, attrId))
}

@Composable
private fun CandidatePreviewRow(
    colorSettings: KeyboardColorSettings,
    candidateTextSizeScale: Float,
    fontType: String,
) {
    val context = LocalContext.current
    val typeface =
        remember(fontType) {
            TypefaceLoader.getTypefaceByType(fontType, context)
        }
    val fontFamily = remember(typeface) { FontFamily(typeface) }

    // Key on isDarkTheme so colors refresh on light/dark mode changes
    val isDarkTheme = isSystemInDarkTheme()
    val defaultBgColor = remember(isDarkTheme) { resolveKeyboardThemeColor(context, R.attr.smartbar_bgColor) }
    val defaultTextColor =
        remember(isDarkTheme) { resolveKeyboardThemeColor(context, R.attr.smartbar_candidate_fgColor) }
    val subtitleColor =
        remember(isDarkTheme) { resolveKeyboardThemeColor(context, R.attr.smartbar_candidate_subtitle_fgColor) }
    val firstCandidateBgColor = remember(isDarkTheme) { resolveKeyboardThemeColor(context, R.attr.key_bgColor) }
    val iconTint = remember(isDarkTheme) { resolveKeyboardThemeColor(context, R.attr.smartbar_fgColor) }

    val bgColor = colorSettings.candidateBackgroundColor?.let { Color(it) } ?: defaultBgColor
    val textColor = colorSettings.candidateTextColor?.let { Color(it) } ?: defaultTextColor
    val effectiveSubtitleColor = colorSettings.candidateTextColor?.let { Color(it) } ?: subtitleColor

    val titleSizeSp = 19.sp * candidateTextSizeScale
    val subtitleSizeSp = titleSizeSp * 0.70f

    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(50.dp)
                .background(bgColor),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            painter = painterResource(R.drawable.ic_add),
            contentDescription = null,
            modifier =
                Modifier
                    .width(36.dp)
                    .padding(start = 2.dp)
                    .padding(6.dp),
            tint = iconTint,
        )

        Row(
            modifier = Modifier.weight(1f),
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            sampleCandidates.forEachIndexed { index, candidate ->
                // First candidate has keycap-color background — matches the production candidate
                // strip's first-candidate hint (key_bgColor); see behavioral-invariants.md §19.
                val itemBg = if (index == 0) firstCandidateBgColor else Color.Transparent
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    modifier =
                        Modifier
                            .background(itemBg, RoundedCornerShape(8.dp))
                            .padding(horizontal = 6.dp, vertical = 3.dp),
                ) {
                    Text(
                        text = candidate.roman,
                        fontSize = titleSizeSp,
                        color = textColor,
                        fontFamily = fontFamily,
                        maxLines = 1,
                        textAlign = TextAlign.Center,
                    )
                    Text(
                        text = candidate.hanzi,
                        fontSize = subtitleSizeSp,
                        color = effectiveSubtitleColor,
                        fontFamily = fontFamily,
                        maxLines = 1,
                        textAlign = TextAlign.Center,
                    )
                }
                if (index < sampleCandidates.size - 1) {
                    Spacer(Modifier.width(10.dp))
                }
            }
        }

        Box(
            modifier =
                Modifier
                    .width(1.dp)
                    .height(32.dp)
                    .background(iconTint.copy(alpha = 0.3f)),
        )

        Icon(
            painter = painterResource(R.drawable.ic_keyboard_arrow_down),
            contentDescription = null,
            modifier =
                Modifier
                    .width(48.dp)
                    .padding(end = 4.dp)
                    .padding(2.dp),
            tint = iconTint,
        )
    }
}
