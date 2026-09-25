// Persisted keyboard color settings (JSON in DataStore); a null role falls back to the platform theme attr.

package com.siansiansu.taigikeyboard.ime.core

import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.abs
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.sin

/** A point in the unit square of a painted surface (0..1 on both axes; y down). */
data class UnitPoint(
    val x: Float,
    val y: Float,
)

// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/KeyboardColorSettings.swift ThemeGradient
// (stops + angle, same degree convention and unit-point math). Drift causes silent divergence.

/**
 * A linear keyboard-background gradient: >=2 ARGB [stops] from start to end plus
 * the direction [angle] in degrees, CSS / Figma convention (0 = bottom->top,
 * 90 = left->right, 180 = top->bottom, clockwise). Built-in gradient themes use
 * the vertical [DEFAULT_ANGLE].
 *
 * ">=2 stops" is enforced at construction ([init] require, [fromJson] null), so
 * every gradient a render site sees is renderable. Decode is forward-compatible:
 * an `angle` absent from old JSON reads as [DEFAULT_ANGLE].
 */
data class ThemeGradient(
    val stops: List<Int>,
    val angle: Float = DEFAULT_ANGLE,
) {
    init {
        require(stops.size >= MINIMUM_STOPS) { "a gradient needs at least $MINIMUM_STOPS stops, got ${stops.size}" }
    }

    /**
     * Start / end points for [angle] in the unit square of the painted surface. The CSS
     * direction vector `(sin θ, -cos θ)` (y down) is normalised by its larger component so
     * the diagonal presets run corner to corner (135° = top-left -> bottom-right) and the
     * axis presets run edge to edge (180° = top-centre -> bottom-centre).
     */
    fun unitPoints(): Pair<UnitPoint, UnitPoint> {
        val (dx, dy) = direction(angle)
        val magnitude = max(abs(dx), abs(dy))
        val halfX = dx / magnitude / 2
        val halfY = dy / magnitude / 2
        return UnitPoint(0.5f - halfX, 0.5f - halfY) to UnitPoint(0.5f + halfX, 0.5f + halfY)
    }

    companion object {
        /** Vertical top->bottom, the direction every built-in gradient theme uses. */
        const val DEFAULT_ANGLE = 180f
        const val MINIMUM_STOPS = 2

        /**
         * Spacing of the eight preset directions (↑ → ↓ ← and the diagonals) the editor's
         * preview drag snaps onto. Mirrors iOS ThemeGradient.presetStep.
         */
        const val PRESET_STEP = 45f

        /**
         * The unit direction vector of [degrees] in screen coordinates (y down): `0` -> (0, -1),
         * `90` -> (1, 0). Shared by [unitPoints] and the editor's direction overlay.
         */
        fun direction(degrees: Float): SurfaceVector {
            val radians = Math.toRadians(degrees.toDouble())
            return SurfaceVector(sin(radians).toFloat(), -cos(radians).toFloat())
        }

        /**
         * Inverse of [direction]: the angle of a screen-space vector, in `-180..180` (callers
         * wrap it into `0 until 360` as they see fit).
         */
        fun degrees(
            dx: Float,
            dy: Float,
        ): Float = Math.toDegrees(atan2(dx.toDouble(), -dy.toDouble())).toFloat()

        /** How far the end stop is lifted toward white in [seeded]. */
        private const val SEED_LIGHTEN_FACTOR = 0.45

        /**
         * The first vertical gradient a user sees when switching a solid background to
         * Gradient: the solid color running into a lighter tint of itself.
         */
        fun seeded(solid: Int): ThemeGradient = ThemeGradient(listOf(solid, lightenedArgb(solid, SEED_LIGHTEN_FACTOR)))

        /** Decodes `{ "stops": [...], "angle"?: n }`; null when fewer than [MINIMUM_STOPS] stops. */
        fun fromJson(obj: JSONObject): ThemeGradient? {
            val array = obj.optJSONArray("stops") ?: return null
            if (array.length() < MINIMUM_STOPS) return null
            val angle = obj.optDouble("angle", DEFAULT_ANGLE.toDouble()).toFloat()
            return ThemeGradient(List(array.length()) { array.getInt(it) }, angle)
        }
    }
}

/** A direction in surface space (x right, y down). */
data class SurfaceVector(
    val dx: Float,
    val dy: Float,
)

/** A rectangle in surface pixels (top-left origin). */
data class SurfaceRect(
    val left: Float,
    val top: Float,
    val width: Float,
    val height: Float,
)

// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/KeyboardColorSettings.swift ThemeImageBackground
// (same JSON fields, SATURATION, dim range, DEFAULT_DIM, DEFAULT_FOCUS). Drift causes silent divergence.

/**
 * A photo as the keyboard surface: [file] is the JPEG's name inside the app-private
 * `ThemeImageStore` directory (written by the settings app, read by the IME), [dim] the
 * opacity of the tone overlay laid over the desaturated photo so keys stay readable
 * (USER 2026-09-19: "the photo saturation must not be too eye-catching"). The overlay is white when the key text is dark
 * and black otherwise. [focusX] / [focusY] say which part of the aspect-filled photo stays in
 * view on each axis: 0 = its left / top edge, 1 = its right / bottom edge, 0.5 = centred (the
 * default). An alignment, not a focal point, so the crop never exposes a gap and the same
 * values fit every keyboard aspect (portrait, landscape, tablet).
 */
data class ThemeImageBackground(
    val file: String,
    /** Opacity of the tone overlay, within [DIM_MIN]..[DIM_MAX] (the editor slider range). */
    val dim: Float = DEFAULT_DIM,
    val focusX: Float = DEFAULT_FOCUS,
    val focusY: Float = DEFAULT_FOCUS,
) {
    init {
        require(file.isNotEmpty()) { "a photo background needs a file name" }
        require(dim in DIM_MIN..DIM_MAX) { "dim $dim outside $DIM_MIN..$DIM_MAX" }
        require(focusX in 0f..1f && focusY in 0f..1f) { "focus ($focusX, $focusY) outside 0..1" }
    }

    companion object {
        /** Saturation multiplier applied to every photo (1 = untouched). */
        const val SATURATION = 0.7f
        const val DIM_MIN = 0f
        const val DIM_MAX = 0.8f
        const val DIM_STEP = 0.05f
        const val DEFAULT_DIM = 0.35f
        const val DEFAULT_FOCUS = 0.5f

        /**
         * The rectangle that scales an `imageWidth`×`imageHeight` photo to cover [bounds]
         * (aspect fill), aligned on each axis by `focusX` / `focusY` (see [ThemeImageBackground.focusX])
         * — the photo's drawn frame over the whole keyboard, from which a panel shows its slice.
         */
        fun coverRect(
            imageWidth: Float,
            imageHeight: Float,
            bounds: SurfaceRect,
            focusX: Float,
            focusY: Float,
        ): SurfaceRect {
            if (imageWidth <= 0f || imageHeight <= 0f) return bounds
            val scale = max(bounds.width / imageWidth, bounds.height / imageHeight)
            val width = imageWidth * scale
            val height = imageHeight * scale
            return SurfaceRect(
                left = bounds.left + (bounds.width - width) * focusX,
                top = bounds.top + (bounds.height - height) * focusY,
                width = width,
                height = height,
            )
        }

        /** Decodes `{ "file": …, "dim"?: n, "focusX"?: n, "focusY"?: n }`; null when the file name is empty. */
        fun fromJson(obj: JSONObject): ThemeImageBackground? {
            val file = obj.optString("file")
            if (file.isEmpty()) return null
            return ThemeImageBackground(
                file = file,
                dim = obj.optDouble("dim", DEFAULT_DIM.toDouble()).toFloat().coerceIn(DIM_MIN, DIM_MAX),
                focusX = obj.optDouble("focusX", DEFAULT_FOCUS.toDouble()).toFloat().coerceIn(0f, 1f),
                focusY = obj.optDouble("focusY", DEFAULT_FOCUS.toDouble()).toFloat().coerceIn(0f, 1f),
            )
        }
    }
}

// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/KeyboardColorSettings.swift ThemeBackground
// (same `type` discriminator and field names; iOS stores the colour as an RGBA object). Drift causes silent divergence.

/**
 * What paints the keyboard surface — one field, mutually exclusive cases. The
 * candidate bar is the same surface: a solid background colours both, a gradient
 * or photo paints once behind both (the bar goes transparent). A null
 * [KeyboardColorSettings.background] means "adaptive" (`?keyboard_bgColor`) and
 * is reserved for the Filled Default head. Rendering lives in `Modifier.themeBackground`
 * (Compose) and `KeyboardThemeSurfaceController` (View).
 *
 * JSON: `{"type":"solid","color":argb}` / `{"type":"gradient","stops":[…],"angle":180}` /
 * `{"type":"image","file":"<uuid>.jpg","dim":0.35,"focusX":0.5,"focusY":0.5}`.
 */
sealed class ThemeBackground {
    data class Solid(
        val color: Int,
    ) : ThemeBackground()

    data class Gradient(
        val gradient: ThemeGradient,
    ) : ThemeBackground()

    data class Image(
        val image: ThemeImageBackground,
    ) : ThemeBackground()

    /** The JSON discriminator, also the editor's Solid / Gradient / Photo segmented choice. */
    enum class Kind(
        val jsonValue: String,
    ) {
        SOLID("solid"),
        GRADIENT("gradient"),
        IMAGE("image"),
    }

    val kind: Kind
        get() =
            when (this) {
                is Solid -> Kind.SOLID
                is Gradient -> Kind.GRADIENT
                is Image -> Kind.IMAGE
            }

    val asGradient: ThemeGradient?
        get() = (this as? Gradient)?.gradient

    val asImage: ThemeImageBackground?
        get() = (this as? Image)?.image

    fun toJson(): JSONObject =
        JSONObject().put("type", kind.jsonValue).apply {
            when (this@ThemeBackground) {
                is Solid -> put("color", color)
                is Gradient -> put("stops", JSONArray(gradient.stops)).put("angle", gradient.angle.toDouble())
                is Image ->
                    put("file", image.file)
                        .put("dim", image.dim.toDouble())
                        .put("focusX", image.focusX.toDouble())
                        .put("focusY", image.focusY.toDouble())
            }
        }

    companion object {
        /** Decodes one background; null for an unknown `type` (a newer build), a non-renderable gradient or an empty photo file. */
        fun fromJson(obj: JSONObject): ThemeBackground? =
            when (obj.optString("type")) {
                Kind.SOLID.jsonValue -> obj.optIntOrNull("color")?.let(::Solid)
                Kind.GRADIENT.jsonValue -> ThemeGradient.fromJson(obj)?.let(::Gradient)
                Kind.IMAGE.jsonValue -> ThemeImageBackground.fromJson(obj)?.let(::Image)
                else -> null
            }
    }
}

/**
 * A custom keyboard surface together with the tone its photo overlay takes — resolved once
 * from [KeyboardColorSettings.surface] so no render site can pair a background with the
 * wrong tone.
 */
data class ThemeSurface(
    val background: ThemeBackground,
    /** Whether a photo's dim overlay is white (dark key text) rather than black. */
    val dimsTowardWhite: Boolean,
)

/**
 * Custom keyboard color settings.
 *
 * Each role is nullable -- null means "use the platform theme attr color".
 * Stored as JSON in DataStore via colorSettings preference.
 *
 * Matches iOS KeyboardColorSettings structure.
 */
data class KeyboardColorSettings(
    /** The keyboard + candidate-bar surface. null = adaptive (`?keyboard_bgColor`). */
    val background: ThemeBackground? = null,
    val keyTextColor: Int? = null,
    val normalKeyFillColor: Int? = null,
    val specialKeyFillColor: Int? = null,
    val candidateTextColor: Int? = null,
) {
    /**
     * The background gradient, or null for a solid / photo / adaptive background. Single
     * source for the candidate-tint derivation and the built-in theme tests.
     */
    val backgroundGradient: ThemeGradient?
        get() = background?.asGradient

    /**
     * The custom surface to paint, or null for the adaptive default. The photo tone
     * overlay is white when the key text is dark and black otherwise (the seed's black
     * text is the fallback, so an unset role reads as "light").
     */
    val surface: ThemeSurface?
        get() = background?.let { ThemeSurface(it, isDarkArgb(keyTextColor ?: UserThemeSeed.KEY_TEXT)) }

    /**
     * Fills every null role from [UserThemeSeed]. Applied when a user theme is decoded
     * ([UserTheme.fromJson]), so themes saved before the seed existed become
     * scheme-invariant without a migration write.
     */
    fun seededForUserTheme(): KeyboardColorSettings =
        KeyboardColorSettings(
            background = background ?: UserThemeSeed.BACKGROUND,
            keyTextColor = keyTextColor ?: UserThemeSeed.KEY_TEXT,
            normalKeyFillColor = normalKeyFillColor ?: UserThemeSeed.NORMAL_KEY_FILL,
            specialKeyFillColor = specialKeyFillColor ?: UserThemeSeed.SPECIAL_KEY_FILL,
            candidateTextColor = candidateTextColor ?: UserThemeSeed.CANDIDATE_TEXT,
        )

    /** The JSON object form. [toJson] is the string serialization; nested users (e.g. [ThemeAppearance]) embed this directly. */
    fun toJsonObject(): JSONObject {
        val json = JSONObject()
        background?.let { json.put("background", it.toJson()) }
        keyTextColor?.let { json.put("keyTextColor", it) }
        normalKeyFillColor?.let { json.put("normalKeyFillColor", it) }
        specialKeyFillColor?.let { json.put("specialKeyFillColor", it) }
        candidateTextColor?.let { json.put("candidateTextColor", it) }
        return json
    }

    fun toJson(): String = toJsonObject().toString()

    companion object {
        fun fromJson(json: String): KeyboardColorSettings {
            if (json.isBlank() || json == "{}") return KeyboardColorSettings()
            return try {
                fromJson(JSONObject(json))
            } catch (e: Exception) {
                KeyboardColorSettings()
            }
        }

        /**
         * `background` replaced three older keys. Decoding still reads them so a theme
         * written by an older build keeps its look: `backgroundGradient` (>=2 stops) ->
         * [ThemeBackground.Gradient] at the vertical default angle, else `backgroundColor`
         * -> [ThemeBackground.Solid]. `candidateBackgroundColor` is dropped — the candidate
         * bar is the keyboard surface now (USER 2026-09-19). Encoding writes only `background`.
         */
        fun fromJson(obj: JSONObject): KeyboardColorSettings =
            KeyboardColorSettings(
                background =
                    obj.optJSONObject("background")?.let { ThemeBackground.fromJson(it) }
                        ?: obj.optJSONObject("backgroundGradient")?.let { ThemeGradient.fromJson(it) }?.let { ThemeBackground.Gradient(it) }
                        ?: obj.optIntOrNull("backgroundColor")?.let { ThemeBackground.Solid(it) },
                keyTextColor = obj.optIntOrNull("keyTextColor"),
                normalKeyFillColor = obj.optIntOrNull("normalKeyFillColor"),
                specialKeyFillColor = obj.optIntOrNull("specialKeyFillColor"),
                candidateTextColor = obj.optIntOrNull("candidateTextColor"),
            )
    }
}

private fun JSONObject.optIntOrNull(key: String): Int? = if (has(key) && !isNull(key)) getInt(key) else null

/**
 * Perceived luminance below mid-grey (Rec. 601 weighting). Mirrors iOS `CodableColor.isDark`;
 * used to pick a photo's tone overlay from the key-text colour.
 */
fun isDarkArgb(argb: Int): Boolean {
    val r = (argb shr 16 and 0xFF) / 255.0
    val g = (argb shr 8 and 0xFF) / 255.0
    val b = (argb and 0xFF) / 255.0
    return 0.299 * r + 0.587 * g + 0.114 * b < 0.5
}

// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/KeyboardColorSettings.swift UserThemeSeed.
// Drift = a new custom theme starts from different colors per platform.

/**
 * The concrete light palette every user theme starts from, so a user theme never
 * carries a null (scheme-following) role and renders identically in light and dark
 * mode (USER 2026-09-19). Background is the light keyboard grey; the special key
 * fill is the iOS light dark-button grey.
 */
object UserThemeSeed {
    const val SOLID_COLOR = 0xFFD4D5DD.toInt()
    val BACKGROUND: ThemeBackground = ThemeBackground.Solid(SOLID_COLOR)
    const val KEY_TEXT = 0xFF000000.toInt()
    const val NORMAL_KEY_FILL = 0xFFFFFFFF.toInt()
    const val SPECIAL_KEY_FILL = 0xFFABB1BA.toInt()
    const val CANDIDATE_TEXT = 0xFF000000.toInt()

    val colors =
        KeyboardColorSettings(
            background = BACKGROUND,
            keyTextColor = KEY_TEXT,
            normalKeyFillColor = NORMAL_KEY_FILL,
            specialKeyFillColor = SPECIAL_KEY_FILL,
            candidateTextColor = CANDIDATE_TEXT,
        )
}

// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/KeyboardColorSettings.swift
// candidateHighlightLightenFactor / candidatePressedDeepenFactor. Drift causes silent divergence.
// Factors used to derive the candidate strip's first-candidate highlight + pressed tints from a
// gradient theme's first stop, so those states match the theme hue instead of a neutral keycap color.
// The highlight is LIGHTENED toward white (a light tint, lighter than the gradient bar so it stays
// visible); the pressed state is DEEPENED toward black. A flat/scaffold theme (no gradient) keeps
// the neutral attr-based fallback.
const val CANDIDATE_HIGHLIGHT_LIGHTEN_FACTOR = 0.5
const val CANDIDATE_PRESSED_DEEPEN_FACTOR = 0.65

/**
 * Returns an opaque ARGB color lightened toward white by [factor]: each 0-255 RGB component is
 * lifted by `c + (255 - c) * factor`, truncated toward zero (alpha forced 0xFF). Used to derive the
 * candidate first-candidate highlight — a light tint of a gradient theme's first stop.
 */
fun lightenedArgb(
    argb: Int,
    factor: Double,
): Int {
    fun lift(c: Int): Int = c + ((255 - c) * factor).toInt()
    val r = lift(argb shr 16 and 0xFF)
    val g = lift(argb shr 8 and 0xFF)
    val b = lift(argb and 0xFF)
    return (0xFF shl 24) or (r shl 16) or (g shl 8) or b
}

/**
 * Returns an opaque ARGB color deepened toward black by [factor]: each 0-255 RGB component is
 * multiplied and truncated toward zero (alpha forced 0xFF). Used to derive the candidate pressed
 * tint from a gradient theme's first stop.
 */
fun deepenedArgb(
    argb: Int,
    factor: Double,
): Int {
    val r = ((argb shr 16 and 0xFF) * factor).toInt()
    val g = ((argb shr 8 and 0xFF) * factor).toInt()
    val b = ((argb and 0xFF) * factor).toInt()
    return (0xFF shl 24) or (r shl 16) or (g shl 8) or b
}
