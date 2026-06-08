// 中文: 內建主題靜態表 — 依 family(經典/Swifty/Minimal)分區。經典含 default + 海風/翠青/藤紫 漸層。
// 中文: Swifty/Minimal scaffold(無配色,light/dark = null)。id 為非 UUID、非 "default" 字串。對齊 iOS BuiltInThemes。

package com.siansiansu.taigikeyboard.ime.core

/**
 * One built-in, read-only theme: a named palette with optional light/dark 6-role
 * color variants resolved against night-mode at render time. A null variant
 * falls back to the other; both null (scaffold) degrades to adaptive defaults.
 * Mirrors iOS BuiltInTheme.
 */
data class BuiltInTheme(
    val id: String,
    val displayName: String,
    val light: KeyboardColorSettings?,
    val dark: KeyboardColorSettings?,
    val previewImageName: String? = null,
) {
    /** Picks the variant for [isDark], falling back to the other when absent. */
    fun colors(isDark: Boolean): KeyboardColorSettings =
        if (isDark) {
            dark ?: light ?: KeyboardColorSettings()
        } else {
            light ?: dark ?: KeyboardColorSettings()
        }
}

/** One built-in family — a section header + its variant themes (one picker shelf). */
data class BuiltInThemeFamily(
    val title: String,
    val themes: List<BuiltInTheme>,
)

/**
 * The app-bundled, read-only theme catalog, grouped by family. The Standard
 * (`經典`) family Blue/Green/Purple carry soft single-hue background gradients;
 * Swifty/Minimal are still scaffold (no palette). Mirrors iOS BuiltInThemes.
 */
object BuiltInThemes {
    val families: List<BuiltInThemeFamily> =
        listOf(
            BuiltInThemeFamily(
                title = "經典",
                themes =
                    listOf(
                        // The Standard head IS the app default (adaptive). Its id is the
                        // ThemeId.DEFAULT sentinel so selecting it = reset-to-default, and
                        // it shows selected whenever no other theme is chosen.
                        BuiltInTheme(
                            id = ThemeId.DEFAULT,
                            displayName = "經典",
                            light = null,
                            dark = null,
                            previewImageName = "theme_standard_preview",
                        ),
                        // Soft single-hue gradient themes matching the KeyboardKit standard
                        // look: a gentle tint at the TOP fading DOWN to a pale version of the
                        // SAME hue. White keys ride on top. Hex are visual estimates from the
                        // reference shots; fine-tune on device.
                        gradientTheme(
                            "standardBlue", "海風",
                            lightTop = 0xBFD2EA, lightBottom = 0xDCE2EC,
                            darkTop = 0x323E58, darkBottom = 0x262E40,
                        ),
                        gradientTheme(
                            "standardGreen", "翠青",
                            lightTop = 0xC3D8C8, lightBottom = 0xDCE5DD,
                            darkTop = 0x324235, darkBottom = 0x28342A,
                        ),
                        gradientTheme(
                            "standardPurple", "藤紫",
                            lightTop = 0xCDC4E4, lightBottom = 0xDEDAEA,
                            darkTop = 0x3A3252, darkBottom = 0x2C2640,
                        ),
                    ),
            ),
            BuiltInThemeFamily(
                title = "Swifty",
                themes =
                    listOf(
                        scaffold("swifty", "Swifty"),
                        scaffold("swiftyBlue", "Swifty Blue"),
                        scaffold("swiftyGreen", "Swifty Green"),
                        scaffold("swiftyPurple", "Swifty Purple"),
                    ),
            ),
            BuiltInThemeFamily(
                title = "Minimal",
                themes =
                    listOf(
                        scaffold("minimal", "Minimal"),
                        scaffold("minimalBlue", "Blue"),
                        scaffold("minimalGreen", "Green"),
                        scaffold("minimalSunset", "Sunset"),
                    ),
            ),
        )

    /** Flattened lookup roster — used by [ThemeResolver] to resolve a selected id. */
    val all: List<BuiltInTheme> = families.flatMap { it.themes }

    /** Looks up a built-in by id; null when [id] is not a built-in. */
    fun theme(id: String): BuiltInTheme? = all.firstOrNull { it.id == id }

    // 中文: 漸層主題中性鍵色 — 功能鍵與字母鍵同色(光面白 / 暗面 soft dark),對齊 iOS Liquid Glass 預設白功能鍵。
    private const val LIGHT_KEY_FILL = 0xFFFFFF
    private const val LIGHT_KEY_TEXT = 0x1C1C1E
    private const val DARK_KEY_FILL = 0x3A3A3C
    private const val DARK_KEY_TEXT = 0xFFFFFF

    /** A colorless scaffold theme: a name + a card screenshot slot, no palette. */
    private fun scaffold(id: String, displayName: String): BuiltInTheme =
        BuiltInTheme(
            id = id,
            displayName = displayName,
            light = null,
            dark = null,
            previewImageName = "theme_${id}_preview",
        )

    /** A soft single-hue gradient theme — top->bottom gradient over neutral keys. */
    private fun gradientTheme(
        id: String,
        displayName: String,
        lightTop: Int,
        lightBottom: Int,
        darkTop: Int,
        darkBottom: Int,
    ): BuiltInTheme =
        BuiltInTheme(
            id = id,
            displayName = displayName,
            light = softGradientColors(lightTop, lightBottom, LIGHT_KEY_FILL, LIGHT_KEY_TEXT),
            dark = softGradientColors(darkTop, darkBottom, DARK_KEY_FILL, DARK_KEY_TEXT),
            previewImageName = "theme_${id}_preview",
        )

    /**
     * One scheme variant for a gradient theme: the 2-stop background gradient + a
     * single neutral key fill (used for BOTH normal and special keys, so function
     * keys read like the neutral default) + key/candidate text color.
     * backgroundColor and candidateBackgroundColor stay null so the gradient owns
     * the background and the candidate bar is transparent over it.
     */
    private fun softGradientColors(top: Int, bottom: Int, keyFill: Int, keyText: Int): KeyboardColorSettings {
        val fill = argb(keyFill)
        return KeyboardColorSettings(
            keyTextColor = argb(keyText),
            normalKeyFillColor = fill,
            specialKeyFillColor = fill,
            candidateTextColor = argb(keyText),
            backgroundGradient = ThemeGradient(listOf(argb(top), argb(bottom))),
        )
    }

    /** 0xRRGGBB -> opaque ARGB Int (alpha forced 0xFF). */
    private fun argb(rgb: Int): Int = (0xFF shl 24) or (rgb and 0xFFFFFF)
}
