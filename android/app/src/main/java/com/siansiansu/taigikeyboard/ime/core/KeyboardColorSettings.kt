// 中文: 自訂鍵盤色彩設定值型別 — 6 個欄位都可為 null(代表「沿用主題預設」)+ 背景垂直漸層。
// 中文: 以 JSON 格式持久化於 DataStore.colorSettings;對齊 iOS KeyboardColorSettings 結構。

package com.siansiansu.taigikeyboard.ime.core

import org.json.JSONArray
import org.json.JSONObject

/**
 * A vertical (top->bottom) keyboard-background gradient.
 *
 * [stops] are ARGB ints ordered top->bottom and need >=2 entries to render; the
 * render layer ignores a gradient with fewer than 2 stops and falls back to the
 * flat backgroundColor. Built-in gradient themes set this; flat themes leave it
 * null. Mirrors iOS ThemeGradient. Persisted as `{ "stops": [argb, ...] }`.
 */
data class ThemeGradient(val stops: List<Int>) {
    fun toJson(): JSONObject {
        val array = JSONArray()
        stops.forEach { array.put(it) }
        return JSONObject().put("stops", array)
    }

    companion object {
        fun fromJson(obj: JSONObject): ThemeGradient {
            val array = obj.optJSONArray("stops") ?: return ThemeGradient(emptyList())
            return ThemeGradient(List(array.length()) { array.getInt(it) })
        }
    }
}

/**
 * Custom keyboard color settings.
 *
 * Each color field is nullable -- null means "use default theme color".
 * Stored as JSON in DataStore via colorSettings preference.
 *
 * Matches iOS KeyboardColorSettings structure.
 */
data class KeyboardColorSettings(
    val backgroundColor: Int? = null,
    val keyTextColor: Int? = null,
    val normalKeyFillColor: Int? = null,
    val specialKeyFillColor: Int? = null,
    val candidateTextColor: Int? = null,
    val candidateBackgroundColor: Int? = null,
    val backgroundGradient: ThemeGradient? = null,
) {
    /**
     * Whether a renderable gradient is set (>=2 stops). Single source for the
     * render branch, the gradient-vs-flat decision, and the candidate-bar
     * transparency. Mirrors iOS KeyboardColorSettings.hasBackgroundGradient.
     */
    val hasBackgroundGradient: Boolean
        get() = (backgroundGradient?.stops?.size ?: 0) >= 2

    /**
     * Renderable gradient stops (top->bottom ARGB) when [hasBackgroundGradient],
     * else null. Single pure source for the View-layer GradientDrawable build so
     * the render seam never re-derives the gradient-vs-flat decision. JVM-testable.
     */
    fun gradientStops(): IntArray? =
        if (hasBackgroundGradient) backgroundGradient!!.stops.toIntArray() else null

    /** The JSON object form. [toJson] is the string serialization; nested users (e.g. [ThemeAppearance]) embed this directly. */
    fun toJsonObject(): JSONObject {
        val json = JSONObject()
        backgroundColor?.let { json.put("backgroundColor", it) }
        keyTextColor?.let { json.put("keyTextColor", it) }
        normalKeyFillColor?.let { json.put("normalKeyFillColor", it) }
        specialKeyFillColor?.let { json.put("specialKeyFillColor", it) }
        candidateTextColor?.let { json.put("candidateTextColor", it) }
        candidateBackgroundColor?.let { json.put("candidateBackgroundColor", it) }
        backgroundGradient?.let { json.put("backgroundGradient", it.toJson()) }
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

        fun fromJson(obj: JSONObject): KeyboardColorSettings =
            KeyboardColorSettings(
                backgroundColor = obj.optIntOrNull("backgroundColor"),
                keyTextColor = obj.optIntOrNull("keyTextColor"),
                normalKeyFillColor = obj.optIntOrNull("normalKeyFillColor"),
                specialKeyFillColor = obj.optIntOrNull("specialKeyFillColor"),
                candidateTextColor = obj.optIntOrNull("candidateTextColor"),
                candidateBackgroundColor = obj.optIntOrNull("candidateBackgroundColor"),
                backgroundGradient = obj.optJSONObject("backgroundGradient")?.let { ThemeGradient.fromJson(it) },
            )

        private fun JSONObject.optIntOrNull(key: String): Int? = if (has(key) && !isNull(key)) getInt(key) else null
    }
}

// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/KeyboardColorSettings.swift
// candidateHighlightLightenFactor / candidatePressedDeepenFactor. Drift causes silent divergence.
// Factors used to derive the candidate strip's first-candidate highlight + pressed tints from a
// gradient theme's top stop, so those states match the theme hue instead of a neutral keycap color.
// The highlight is LIGHTENED toward white (a light tint, lighter than the gradient bar so it stays
// visible); the pressed state is DEEPENED toward black. A flat/scaffold theme (no gradient) keeps
// the neutral attr-based fallback.
const val CANDIDATE_HIGHLIGHT_LIGHTEN_FACTOR = 0.5
const val CANDIDATE_PRESSED_DEEPEN_FACTOR = 0.65

/**
 * Returns an opaque ARGB color lightened toward white by [factor]: each 0-255 RGB component is
 * lifted by `c + (255 - c) * factor`, truncated toward zero (alpha forced 0xFF). Used to derive the
 * candidate first-candidate highlight — a light tint of a gradient theme's top stop.
 */
fun lightenedArgb(argb: Int, factor: Double): Int {
    fun lift(c: Int): Int = c + ((255 - c) * factor).toInt()
    val r = lift(argb shr 16 and 0xFF)
    val g = lift(argb shr 8 and 0xFF)
    val b = lift(argb and 0xFF)
    return (0xFF shl 24) or (r shl 16) or (g shl 8) or b
}

/**
 * Returns an opaque ARGB color deepened toward black by [factor]: each 0-255 RGB component is
 * multiplied and truncated toward zero (alpha forced 0xFF). Used to derive the candidate pressed
 * tint from a gradient theme's top stop.
 */
fun deepenedArgb(argb: Int, factor: Double): Int {
    val r = ((argb shr 16 and 0xFF) * factor).toInt()
    val g = ((argb shr 8 and 0xFF) * factor).toInt()
    val b = ((argb and 0xFF) * factor).toInt()
    return (0xFF shl 24) or (r shl 16) or (g shl 8) or b
}
