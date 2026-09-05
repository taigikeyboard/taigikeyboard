// Single-key Composable — draws label / icon / background and applies the theme color.
// Position is laid out by KeyboardLayout's custom Layout block; this file only draws one cell.

package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.content.Context
import android.content.res.Resources
import android.graphics.Paint
import android.graphics.Typeface
import android.view.inputmethod.EditorInfo
import androidx.annotation.DrawableRes
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.isTpsGlyphWithPopup
import com.siansiansu.taigikeyboard.ime.text.key.KeyLabelCaseCache
import com.siansiansu.taigikeyboard.ime.text.key.KeyType
import com.siansiansu.taigikeyboard.ime.theme.getColorFromAttr
import java.util.Locale
import kotlin.math.min

/**
 * Per-key Compose renderer. Pure function of state — no side effects, no
 * MotionEvent handling (that lives in [KeyTouchCoordinator]). Direct port of
 * the legacy `KeyView.onDraw` + `updateKeyContent` + `applyAppearance`
 * rendering paths.
 *
 * Background uses Compose primitives (`Modifier.background` + `Modifier.border`
 * with `RoundedCornerShape`) instead of the legacy `StateListDrawable` so the
 * pressed state can swap colors per recomposition without invalidating a
 * Drawable across the View tree.
 *
 * Label and hint glyphs are drawn through `Canvas { drawIntoCanvas {
 * nativeCanvas.drawText(...) } }` so the legacy Paint typeface, alpha,
 * baseline math, and per-key text-size scaling carry over byte-for-byte.
 */
@Suppress("LongParameterList")
@Composable
internal fun KeyContent(
    data: KeyData,
    mode: KeyboardMode,
    keyboardLayoutType: String,
    inputMode: String,
    caps: Boolean,
    capsLock: Boolean,
    isComposing: Boolean,
    isTranslateSwapped: Boolean,
    imeOptions: Int,
    confirmKeyLabel: String,
    colors: KeyboardColorSettings,
    typeface: Typeface,
    fontSizeScale: Float,
    cornerRadiusDp: Float,
    borderWidthDp: Float,
    shadowIntensity: Float,
    isPreview: Boolean,
    pressed: Boolean,
    themeColors: ThemePalette,
) {
    val isFunctionKey = isFunctionKey(data)
    val isSpecial = isFunctionKey || data.code == KeyCode.ENTER

    val shape = remember(cornerRadiusDp) { RoundedCornerShape(cornerRadiusDp.dp) }
    val customFill = if (isSpecial) colors.specialKeyFillColor else colors.normalKeyFillColor

    // Background tint resolution mirrors `applyAppearance` +
    // `updateKeyPressedBackground` + `setBackgroundTintList`. Custom fill
    // replaces BOTH pressed and unpressed — matches legacy where a single
    // `ColorStateList.valueOf(...)` tints all selector states equally
    // (pressed feedback is intentionally lost for tinted keys).
    val backgroundArgb = resolveBackgroundColor(
        themeColors = themeColors,
        data = data,
        isFunctionKey = isFunctionKey,
        pressed = pressed,
        isTranslateSwapped = isTranslateSwapped,
        customFill = customFill,
    )

    // Drop shadow is drawn BEFORE the background fill so the key face renders on
    // top of its shadow (per Compose `dropShadow` ordering). Flat/legacy themes
    // pass intensity 0 -> no shadow node added, keeping the default path identical.
    val shadowSpec = remember(shadowIntensity) { keyShadowSpec(shadowIntensity) }
    val backgroundModifier = Modifier
        .fillMaxSize()
        .let { base ->
            if (shadowSpec == null) {
                base
            } else {
                base.dropShadow(shape) {
                    radius = shadowSpec.radiusDp.dp.toPx()
                    offset = Offset(0f, shadowSpec.offsetYDp.dp.toPx())
                    alpha = shadowSpec.alpha
                    color = Color.Black
                }
            }
        }
        .background(Color(backgroundArgb), shape)
    // Border follows the theme keyTextColor role first (falling back to the
    // adaptive keyFg), so the 框線 outline stays visible on a light-only gradient in
    // dark mode — same role-first rule the key glyph/label uses (mirrors iOS).
    val borderColor = colors.keyTextColor?.let { Color(it) } ?: Color(themeColors.keyFg)
    val borderedModifier = if (borderWidthDp > 0f) {
        backgroundModifier.border(borderWidthDp.dp, borderColor, shape)
    } else {
        backgroundModifier
    }

    Box(modifier = borderedModifier) {
        val resources = LocalResources.current
        val visual = remember(
            data.code,
            data.type,
            data.label,
            data.popup.size,
            mode,
            keyboardLayoutType,
            inputMode,
            caps,
            capsLock,
            isComposing,
            isTranslateSwapped,
            imeOptions,
            confirmKeyLabel,
            isPreview,
            themeColors,
        ) {
            resolveKeyVisual(
                data = data,
                mode = mode,
                inputMode = inputMode,
                caps = caps,
                capsLock = capsLock,
                isComposing = isComposing,
                isTranslateSwapped = isTranslateSwapped,
                imeOptions = imeOptions,
                confirmKeyLabel = confirmKeyLabel,
                isPreview = isPreview,
                themeColors = themeColors,
                resources = resources,
            )
        }

        when (visual) {
            is KeyVisual.Icon -> IconContent(visual, colors)
            is KeyVisual.Label -> LabelContent(
                label = visual.label,
                data = data,
                mode = mode,
                keyboardLayoutType = keyboardLayoutType,
                inputMode = inputMode,
                colors = colors,
                typeface = typeface,
                fontSizeScale = fontSizeScale,
                themeColors = themeColors,
            )
            KeyVisual.Empty -> Unit
        }
    }
}

@Composable
private fun IconContent(
    visual: KeyVisual.Icon,
    colors: KeyboardColorSettings,
) {
    // Legacy `KeyView.onDraw` icon sizing: aspect-ratio center crop to a
    // square of side = min(w,h), then inset 0.15 × height padding on all
    // four sides. Result: icon side length = min(w,h) - 2 × 0.15h.
    // Theme keyTextColor role wins for ALL icons (incl. enter). A light-only theme
    // sets it to a fixed dark color so the glyph stays readable in system dark mode;
    // the adaptive default theme leaves it null and falls back to the night-aware
    // `visual.tintArgb` (keyEnterFg for enter, keyFg otherwise).
    val effectiveTint = colors.keyTextColor?.let { Color(it) } ?: Color(visual.tintArgb)
    BoxWithConstraints(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center,
    ) {
        val density = LocalDensity.current
        val widthPx = with(density) { maxWidth.roundToPx() }
        val heightPx = with(density) { maxHeight.roundToPx() }
        val drawablePaddingPx = (heightPx * 0.15f).toInt()
        val sidePx = (min(widthPx, heightPx) - 2 * drawablePaddingPx).coerceAtLeast(0)
        val sideDp = with(density) { sidePx.toDp() }
        Image(
            painter = painterResource(id = visual.drawableRes),
            contentDescription = null,
            colorFilter = ColorFilter.tint(effectiveTint),
            modifier = Modifier.size(sideDp),
        )
    }
}

@Suppress("LongParameterList")
@Composable
private fun LabelContent(
    label: String,
    data: KeyData,
    mode: KeyboardMode,
    keyboardLayoutType: String,
    inputMode: String,
    colors: KeyboardColorSettings,
    typeface: Typeface,
    fontSizeScale: Float,
    themeColors: ThemePalette,
) {
    // keyTextColor role wins for all labels (incl. the enter confirm-text); null
    // (adaptive default theme) falls back to the night-aware enter/normal attr.
    val labelColorArgb = colors.keyTextColor ?: if (data.code == KeyCode.ENTER) {
        themeColors.keyEnterFg
    } else {
        themeColors.keyFg
    }
    val labelPaint = remember {
        Paint().apply {
            isAntiAlias = true
            textAlign = Paint.Align.CENTER
        }
    }
    val hintPaint = remember {
        Paint().apply {
            isAntiAlias = true
            textAlign = Paint.Align.CENTER
        }
    }

    Canvas(modifier = Modifier.fillMaxSize()) {
        val w = size.width
        val h = size.height
        if (w <= 0f || h <= 0f) return@Canvas

        val baseTextSize = h * 0.58f * fontSizeScale
        val perKeyTextSize = scaleTextSizeForKey(data, mode, label, baseTextSize, keyboardLayoutType)

        labelPaint.textSize = perKeyTextSize
        labelPaint.typeface = typeface
        labelPaint.color = labelColorArgb
        labelPaint.alpha = if (mode == KeyboardMode.CHARACTERS && data.code == KeyCode.SPACE) 120 else 255

        val centerX = w / 2.0f
        val centerY = h / 2.0f + (perKeyTextSize - labelPaint.descent()) / 2.0f

        val isTpsWithPopup = keyboardLayoutType == "tps" && data.isTpsGlyphWithPopup()

        drawIntoCanvas { compose ->
            val nativeCanvas = compose.nativeCanvas
            if (isTpsWithPopup) {
                // Main glyph at bottom (smaller); popup callouts on top.
                labelPaint.textSize = baseTextSize * 0.78f
                val topY = h * 0.28f
                val bottomY = h * 0.80f
                hintPaint.color = labelPaint.color
                hintPaint.alpha = 130
                hintPaint.typeface = labelPaint.typeface
                if (data.popup.size >= 2) {
                    hintPaint.textSize = baseTextSize * 0.48f
                    val padding = w * 0.12f
                    hintPaint.textAlign = Paint.Align.LEFT
                    nativeCanvas.drawText(data.popup[0].label, padding, topY, hintPaint)
                    hintPaint.textAlign = Paint.Align.RIGHT
                    nativeCanvas.drawText(data.popup[1].label, w - padding, topY, hintPaint)
                    hintPaint.textAlign = Paint.Align.CENTER
                } else {
                    hintPaint.textSize = baseTextSize * 0.52f
                    hintPaint.textAlign = Paint.Align.CENTER
                    nativeCanvas.drawText(data.popup[0].label, centerX, topY, hintPaint)
                }
                nativeCanvas.drawText(label, centerX, bottomY, labelPaint)
            } else if (label.contains('\n')) {
                // Two-line split: only the first 2 lines are honored.
                val lines = label.split('\n')
                nativeCanvas.drawText(lines[0], centerX, centerY * 0.70f, labelPaint)
                nativeCanvas.drawText(lines[1], centerX, centerY * 1.30f, labelPaint)
            } else {
                nativeCanvas.drawText(label, centerX, centerY, labelPaint)
            }

            // TPS punctuation hint (e.g., "。" hint above "，").
            if (keyboardLayoutType == "tps" &&
                mode == KeyboardMode.CHARACTERS &&
                data.type == KeyType.CHARACTER &&
                data.code != 0 &&
                data.popup.isNotEmpty()
            ) {
                hintPaint.textSize = baseTextSize * 0.52f
                hintPaint.color = labelPaint.color
                hintPaint.alpha = 130
                hintPaint.typeface = labelPaint.typeface
                hintPaint.textAlign = Paint.Align.CENTER
                nativeCanvas.drawText(data.popup[0].label, centerX, h * 0.28f, hintPaint)
            }

            // Tone diacritics + MOE1/MOE2 punctuation hints. Skipped for TPS.
            if (data.type == KeyType.CHARACTER && keyboardLayoutType != "tps") {
                val toneHint = toneHintForCode(data.code, inputMode)
                val moeHint = if (keyboardLayoutType == "moe1" || keyboardLayoutType == "moe2") {
                    moe1HintForCode(data.code)
                } else {
                    null
                }
                val hint = toneHint ?: moeHint
                if (hint != null && hint != " ") {
                    val isMoe1TextHint = moeHint != null
                    hintPaint.textSize = baseTextSize * if (isMoe1TextHint) 0.60f else 1.30f
                    hintPaint.color = labelPaint.color
                    hintPaint.alpha = if (isMoe1TextHint) 150 else 100
                    hintPaint.typeface = Typeface.DEFAULT
                    hintPaint.textAlign = Paint.Align.CENTER
                    val moe1HintFactor = if (data.code == 45) 0.35f else 0.40f
                    val hintY = h * if (isMoe1TextHint) moe1HintFactor else 0.66f
                    nativeCanvas.drawText(hint, centerX, hintY, hintPaint)
                }
            }
        }
    }
}

/**
 * Per-key text-size scaling rule — direct port of `KeyView.onDraw` switch.
 * Separated so the Canvas body stays focused on draw ordering.
 */
private fun scaleTextSizeForKey(
    data: KeyData,
    mode: KeyboardMode,
    label: String,
    baseTextSize: Float,
    keyboardLayoutType: String,
): Float =
    when {
        data.code == KeyCode.VIEW_SYMBOLS -> baseTextSize * 0.80f
        data.code == KeyCode.ENTER && label.isNotEmpty() -> baseTextSize * 0.85f
        data.code == KeyCode.VIEW_NUMERIC_ADVANCED ->
            if (label == "、") baseTextSize else baseTextSize * 0.55f
        data.code == KeyCode.VIEW_NUMERIC || data.code == KeyCode.SPACE -> baseTextSize * 0.55f
        data.type == KeyType.CHARACTER && keyboardLayoutType == "moe2" && label.length >= 3 ->
            baseTextSize * 0.75f
        else -> baseTextSize
    }

/** Sealed visual content — exactly one of icon, label, or empty per key. */
private sealed interface KeyVisual {
    data object Empty : KeyVisual

    data class Label(
        val label: String,
    ) : KeyVisual

    data class Icon(
        @DrawableRes val drawableRes: Int,
        /** Fallback tint when the theme leaves `keyTextColor` null — `keyEnterFg`
         *  for the enter key, `keyFg` otherwise. `keyTextColor` overrides both. */
        val tintArgb: Int,
    ) : KeyVisual
}

/** Theme attribute snapshot — resolved once per [LocalContext] change so the
 *  per-key remember keys don't all carry separate Int color params. */
internal data class ThemePalette(
    val keyFg: Int,
    val keyEnterFg: Int,
    val accent: Int,
    val keyBg: Int,
    val keyBgPressed: Int,
    val keyBgActive: Int,
    val keyFunctionBg: Int,
    val keyFunctionBgPressed: Int,
    val keyEnterBg: Int,
    val keyEnterBgPressed: Int,
) {
    companion object {
        fun from(context: Context): ThemePalette =
            ThemePalette(
                keyFg = getColorFromAttr(context, R.attr.key_fgColor),
                keyEnterFg = getColorFromAttr(context, R.attr.key_enter_fgColor),
                accent = getColorFromAttr(context, R.attr.colorAccent),
                keyBg = getColorFromAttr(context, R.attr.key_bgColor),
                keyBgPressed = getColorFromAttr(context, R.attr.key_bgColorPressed),
                keyBgActive = getColorFromAttr(context, R.attr.key_bgColorActive),
                keyFunctionBg = getColorFromAttr(context, R.attr.key_function_bgColor),
                keyFunctionBgPressed = getColorFromAttr(context, R.attr.key_function_bgColorPressed),
                keyEnterBg = getColorFromAttr(context, R.attr.key_enter_bgColor),
                keyEnterBgPressed = getColorFromAttr(context, R.attr.key_enter_bgColorPressed),
            )
    }
}

/**
 * Direct port of `KeyView.updateKeyContent`. Pure non-composable function so
 * it can be called from inside `remember { ... }` without composition-scope
 * gymnastics. All Context lookups (theme attrs, drawable / string resources)
 * are pre-resolved into [themeColors] and [resources].
 */
@Suppress("LongParameterList")
private fun resolveKeyVisual(
    data: KeyData,
    mode: KeyboardMode,
    inputMode: String,
    caps: Boolean,
    capsLock: Boolean,
    isComposing: Boolean,
    isTranslateSwapped: Boolean,
    imeOptions: Int,
    confirmKeyLabel: String,
    isPreview: Boolean,
    themeColors: ThemePalette,
    resources: Resources,
): KeyVisual {
    if ((data.type == KeyType.CHARACTER && data.code != KeyCode.SPACE) ||
        data.type == KeyType.NUMERIC
    ) {
        return KeyVisual.Label(computeKeyLetter(data, inputMode, caps, capsLock))
    }

    return when (data.code) {
        KeyCode.TRANSLATE -> KeyVisual.Icon(R.drawable.ic_translate, themeColors.keyFg)
        KeyCode.DELETE -> KeyVisual.Icon(R.drawable.ic_backspace, themeColors.keyFg)
        KeyCode.ENTER -> resolveEnterVisual(
            isPreview = isPreview,
            isComposing = isComposing,
            confirmKeyLabel = confirmKeyLabel,
            imeOptions = imeOptions,
            enterFg = themeColors.keyEnterFg,
        )
        KeyCode.LANGUAGE_SWITCH -> KeyVisual.Icon(R.drawable.ic_language, themeColors.keyFg)
        KeyCode.PHONE_PAUSE -> KeyVisual.Label(resources.getString(R.string.key__phone_pause))
        KeyCode.PHONE_WAIT -> KeyVisual.Label(resources.getString(R.string.key__phone_wait))
        KeyCode.SHIFT -> resolveShiftVisual(caps, capsLock, themeColors)
        KeyCode.SPACE -> resolveSpaceVisual(mode, inputMode, themeColors.keyFg)
        KeyCode.SWITCH_TO_MEDIA_CONTEXT ->
            KeyVisual.Icon(R.drawable.ic_sentiment_satisfied, themeColors.keyFg)
        KeyCode.SWITCH_TO_TEXT_CONTEXT,
        KeyCode.VIEW_CHARACTERS,
        -> KeyVisual.Label(resources.getString(R.string.key__view_characters))
        KeyCode.VIEW_NUMERIC -> KeyVisual.Label(resources.getString(R.string.key__view_numeric))
        KeyCode.VIEW_NUMERIC_ADVANCED -> {
            // In SYMBOLS mode with translate-swapped, this slot becomes "、".
            val label = if (isTranslateSwapped && mode == KeyboardMode.SYMBOLS) {
                "、"
            } else {
                resources.getString(R.string.key__view_numeric)
            }
            KeyVisual.Label(label)
        }
        KeyCode.VIEW_PHONE -> KeyVisual.Label(resources.getString(R.string.key__view_phone))
        KeyCode.VIEW_PHONE2 -> KeyVisual.Label(resources.getString(R.string.key__view_phone2))
        KeyCode.VIEW_SYMBOLS -> KeyVisual.Label(resources.getString(R.string.key__view_symbols))
        KeyCode.VIEW_SYMBOLS2 -> KeyVisual.Label(resources.getString(R.string.key__view_symbols2))
        else -> KeyVisual.Empty
    }
}

private fun resolveEnterVisual(
    isPreview: Boolean,
    isComposing: Boolean,
    confirmKeyLabel: String,
    imeOptions: Int,
    enterFg: Int,
): KeyVisual {
    if (isPreview) {
        return KeyVisual.Icon(R.drawable.ic_keyboard_return, enterFg)
    }
    if (isComposing) {
        return KeyVisual.Label(confirmKeyLabel)
    }
    val drawableRes = if (imeOptions and EditorInfo.IME_FLAG_NO_ENTER_ACTION > 0) {
        R.drawable.ic_keyboard_return
    } else {
        when (imeOptions and EditorInfo.IME_MASK_ACTION) {
            EditorInfo.IME_ACTION_DONE -> R.drawable.ic_done
            EditorInfo.IME_ACTION_GO,
            EditorInfo.IME_ACTION_NEXT,
            EditorInfo.IME_ACTION_PREVIOUS,
            -> R.drawable.ic_arrow_right_alt
            EditorInfo.IME_ACTION_NONE -> R.drawable.ic_keyboard_return
            EditorInfo.IME_ACTION_SEARCH -> R.drawable.ic_search
            EditorInfo.IME_ACTION_SEND -> R.drawable.ic_send
            else -> R.drawable.ic_arrow_right_alt
        }
    }
    return KeyVisual.Icon(drawableRes, enterFg)
}

private fun resolveShiftVisual(
    caps: Boolean,
    capsLock: Boolean,
    themeColors: ThemePalette,
): KeyVisual {
    val (drawable, tint) = when {
        caps && capsLock -> R.drawable.ic_keyboard_capslock to themeColors.accent
        caps -> R.drawable.ic_keyboard_capslock to themeColors.keyFg
        else -> R.drawable.ic_keyboard_arrow_up to themeColors.keyFg
    }
    return KeyVisual.Icon(drawable, tint)
}

private fun resolveSpaceVisual(
    mode: KeyboardMode,
    inputMode: String,
    keyFg: Int,
): KeyVisual =
    when (mode) {
        KeyboardMode.NUMERIC,
        KeyboardMode.NUMERIC_ADVANCED,
        KeyboardMode.PHONE,
        KeyboardMode.PHONE2,
        -> KeyVisual.Icon(R.drawable.ic_space_bar, keyFg)
        KeyboardMode.CHARACTERS -> {
            val label = when (inputMode) {
                "poj" -> "POJ"
                "tl" -> "TL"
                "tps" -> "TPS"
                "english" -> "EN"
                else -> null
            }
            if (label != null) KeyVisual.Label(label) else KeyVisual.Empty
        }
        else -> KeyVisual.Empty
    }

/**
 * Direct port of `KeyView.getComputedLetter`. Exposed as `internal` so the
 * popup-cell builder can resolve labels for popup KeyData without re-doing
 * the case + nasal + composing-letter rules.
 */
internal fun computeKeyLetter(
    data: KeyData,
    inputMode: String,
    caps: Boolean,
    capsLock: Boolean,
): String {
    if (data.code == KeyCode.URI_COMPONENT_TLD) {
        return if (caps) {
            data.label.uppercase(Locale.getDefault())
        } else {
            data.label.lowercase(Locale.getDefault())
        }
    }
    val baseLabel = if (data.label.isNotEmpty() && data.label != data.code.toChar().toString()) {
        data.label
    } else {
        data.code.toChar().toString()
    }
    if (baseLabel == "˙") return "·"
    if (baseLabel == "nn" && inputMode == "poj") {
        return if (caps) "ᴺ" else "ⁿ"
    }
    return KeyLabelCaseCache.getOrCompute(baseLabel, InputMode.fromPrefString(inputMode), caps, capsLock)
}

/** Resolved drop-shadow geometry for a key. */
internal data class KeyShadowSpec(
    val radiusDp: Float,
    val offsetYDp: Float,
    val alpha: Float,
)

/**
 * Maps a key-shadow intensity to drop-shadow geometry, or null for no shadow.
 * `0 = none`; `1..4` -> radius = intensity dp, offsetY = intensity/2 dp, fixed
 * alpha. Pure so the mapping is JVM-unit-testable. Mirrors iOS
 * `ButtonShadowStyle(size: keyShadowIntensity)`.
 */
internal fun keyShadowSpec(intensity: Float): KeyShadowSpec? =
    if (intensity > 0f) {
        KeyShadowSpec(radiusDp = intensity, offsetYDp = intensity / 2f, alpha = KEY_SHADOW_ALPHA)
    } else {
        null
    }

private const val KEY_SHADOW_ALPHA = 0.30f

/** Direct port of `KeyView.applyAppearance` "is this a function key?" check. */
private fun isFunctionKey(data: KeyData): Boolean =
    data.type == KeyType.MODIFIER ||
        data.type == KeyType.ENTER_EDITING ||
        data.code == KeyCode.DELETE ||
        data.code == KeyCode.SHIFT ||
        data.code == KeyCode.VIEW_NUMERIC ||
        data.code == KeyCode.VIEW_NUMERIC_ADVANCED ||
        data.code == KeyCode.VIEW_SYMBOLS ||
        data.code == KeyCode.VIEW_SYMBOLS2 ||
        data.code == KeyCode.VIEW_CHARACTERS

private fun resolveBackgroundColor(
    themeColors: ThemePalette,
    data: KeyData,
    isFunctionKey: Boolean,
    pressed: Boolean,
    isTranslateSwapped: Boolean,
    customFill: Int?,
): Int {
    // Translate key in swapped state always wins (legacy:
    // `setBackgroundTintColor(this, R.attr.key_bgColorActive)` fires after
    // `applyAppearance` finishes, overriding any custom fill).
    if (data.code == KeyCode.TRANSLATE && isTranslateSwapped) {
        return themeColors.keyBgActive
    }
    // Custom fill replaces both pressed and unpressed (legacy
    // `backgroundTintList = ColorStateList.valueOf(it)` drops pressed
    // visual feedback for tinted keys — preserved 1:1 here).
    if (customFill != null) return customFill

    return when {
        data.code == KeyCode.ENTER ->
            if (pressed) themeColors.keyEnterBgPressed else themeColors.keyEnterBg
        isFunctionKey ->
            if (pressed) themeColors.keyFunctionBgPressed else themeColors.keyFunctionBg
        else ->
            if (pressed) themeColors.keyBgPressed else themeColors.keyBg
    }
}

// Tone hint diacritics for number keys — direct port of legacy
// `KeyView.toneHints`. POJ + TL share all but tone 9.
private val toneHints = mapOf(
    49 to "ˊ", // 1 (no mark, drawn as space placeholder downstream)
    50 to "ˊ", // 2  ˊ MODIFIER LETTER ACUTE ACCENT
    51 to "ˋ", // 3  ˋ MODIFIER LETTER GRAVE ACCENT
    53 to "ˆ", // 5  ˆ MODIFIER LETTER CIRCUMFLEX ACCENT
    54 to "ˇ", // 6  ˇ CARON
    55 to "ˉ", // 7  ˉ MODIFIER LETTER MACRON
    56 to "ˈ", // 8  ˈ MODIFIER LETTER VERTICAL LINE
)

/** No tone marks for "0", "1", "4" — render space for layout consistency. */
private val noToneHintCodes = setOf(48, 49, 52)

internal fun toneHintForCode(
    code: Int,
    inputMode: String?,
): String? {
    if (inputMode == "english") return null
    if (code == 57) {
        // Tone 9: POJ uses breve (˘ U+02D8); TL uses double prime (ʺ U+02BA).
        return if (inputMode == "poj") "˘" else "ʺ"
    }
    if (noToneHintCodes.contains(code)) return " "
    return toneHints[code]
}

private val moe1Hints = mapOf(
    45 to "@",
    44 to ":;",
    46 to "!?",
)

internal fun moe1HintForCode(code: Int): String? = moe1Hints[code]
