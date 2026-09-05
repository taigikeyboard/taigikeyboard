package com.siansiansu.taigikeyboard.ime.core

import org.json.JSONObject

/**
 * The full set of appearance values a theme captures: 6-role colors plus the
 * key-shadow intensity and the five size scalars (key height / key font /
 * candidate font / corner radius / border width).
 *
 * Font is intentionally NOT part of a theme: it is a GLOBAL setting
 * (PrefHelper.fontType), so switching themes never changes the font.
 *
 * [colors] stays nullable per role (reuses [KeyboardColorSettings]): a null role
 * inherits the platform adaptive color. The model OWNS the size defaults (the
 * [PrefHelper] appearance keys reference them); mirrors iOS ThemeDefaults <-
 * ThemeAppearance.default. Mirrors iOS ThemeAppearance.
 */
data class ThemeAppearance(
    val colors: KeyboardColorSettings = KeyboardColorSettings(),
    val keyShadowIntensity: Float = DEFAULT_KEY_SHADOW_INTENSITY,
    val keyHeightScale: Float = DEFAULT_KEY_HEIGHT_SCALE,
    val keyFontSizeScale: Float = DEFAULT_KEY_FONT_SIZE_SCALE,
    val candidateTextSizeScale: Float = DEFAULT_CANDIDATE_TEXT_SIZE_SCALE,
    val keyCornerRadius: Float = DEFAULT_KEY_CORNER_RADIUS,
    val keyBorderWidth: Float = DEFAULT_KEY_BORDER_WIDTH,
) {
    fun toJson(): JSONObject =
        JSONObject().apply {
            put("colors", colors.toJsonObject())
            put("keyShadowIntensity", keyShadowIntensity.toDouble())
            put("keyHeightScale", keyHeightScale.toDouble())
            put("keyFontSizeScale", keyFontSizeScale.toDouble())
            put("candidateTextSizeScale", candidateTextSizeScale.toDouble())
            put("keyCornerRadius", keyCornerRadius.toDouble())
            put("keyBorderWidth", keyBorderWidth.toDouble())
        }

    companion object {
        // Appearance defaults — single source of truth for theme sizes AND the PrefHelper
        // settings keys (which reference these). Mirrors iOS ThemeDefaults <- ThemeAppearance.default.
        const val DEFAULT_KEY_SHADOW_INTENSITY = 0.0f
        const val DEFAULT_KEY_HEIGHT_SCALE = 1.0f
        const val DEFAULT_KEY_FONT_SIZE_SCALE = 1.0f
        const val DEFAULT_CANDIDATE_TEXT_SIZE_SCALE = 1.0f
        const val DEFAULT_KEY_CORNER_RADIUS = 6.0f
        const val DEFAULT_KEY_BORDER_WIDTH = 0.0f

        /** Factory appearance — adaptive colors, flat shadow, project-default sizes. */
        val DEFAULT = ThemeAppearance()

        /**
         * Forward-compatible decode: any field absent in stored JSON falls back
         * to the project default, so a future appearance field never strands a
         * theme written by an older build. Mirrors iOS decodeIfPresent.
         */
        fun fromJson(obj: JSONObject): ThemeAppearance {
            val fallback = DEFAULT
            val colors = obj.optJSONObject("colors")?.let { KeyboardColorSettings.fromJson(it) } ?: fallback.colors
            return ThemeAppearance(
                colors = colors,
                keyShadowIntensity = obj.optFloatOrNull("keyShadowIntensity") ?: fallback.keyShadowIntensity,
                keyHeightScale = obj.optFloatOrNull("keyHeightScale") ?: fallback.keyHeightScale,
                keyFontSizeScale = obj.optFloatOrNull("keyFontSizeScale") ?: fallback.keyFontSizeScale,
                candidateTextSizeScale = obj.optFloatOrNull("candidateTextSizeScale") ?: fallback.candidateTextSizeScale,
                keyCornerRadius = obj.optFloatOrNull("keyCornerRadius") ?: fallback.keyCornerRadius,
                keyBorderWidth = obj.optFloatOrNull("keyBorderWidth") ?: fallback.keyBorderWidth,
            )
        }

        private fun JSONObject.optFloatOrNull(key: String): Float? =
            if (has(key) && !isNull(key)) getDouble(key).toFloat() else null
    }
}
