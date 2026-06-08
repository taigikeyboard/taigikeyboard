// 中文: 內建主題靜態表 — 依 family(經典/Swifty/Minimal)分區。經典含 default + 5 個 light-only 漸層。
// 中文: 漸層主題只帶 light 配色(dark = null)→ 深色模式刻意維持 light 觀感(USER:這些主題色不隨深色變)。
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
                            displayName = "預設",
                            light = null,
                            dark = null,
                            previewImageName = "theme_standard_preview",
                        ),
                        // Soft single-hue gradient themes matching the KeyboardKit standard
                        // look: a gentle tint at the TOP fading DOWN to a pale version of the
                        // SAME hue. White keys ride on top. Hex are visual estimates; fine-tune
                        // on device. Shelf order: 櫻花 → 金煌 → 海風 → 翠青 → 藤紫 (after 預設 head).
                        // Light-only by design (dark == null): these themes keep their light
                        // palette in dark mode, so colors(isDark = true) falls back to light.
                        gradientTheme(
                            "standardPink", "櫻花",
                            top = 0xE6C2D0, bottom = 0xEADCE2,
                        ),
                        gradientTheme(
                            "standardGold", "金煌",
                            top = 0xEAD9A6, bottom = 0xECE4D2,
                        ),
                        gradientTheme(
                            "standardBlue", "海風",
                            top = 0xBFD2EA, bottom = 0xDCE2EC,
                        ),
                        gradientTheme(
                            "standardGreen", "翠青",
                            top = 0xC3D8C8, bottom = 0xDCE5DD,
                        ),
                        gradientTheme(
                            "standardPurple", "藤紫",
                            top = 0xCDC4E4, bottom = 0xDEDAEA,
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

    // 中文: 漸層主題中性鍵色 — 功能鍵與字母鍵同色(白),對齊 iOS Liquid Glass 預設白功能鍵。
    // 中文: 漸層主題 light-only,深色模式維持 light 觀感,故只需 light 鍵色。
    private const val LIGHT_KEY_FILL = 0xFFFFFF
    private const val LIGHT_KEY_TEXT = 0x1C1C1E

    /** A colorless scaffold theme: a name + a card screenshot slot, no palette. */
    private fun scaffold(id: String, displayName: String): BuiltInTheme =
        BuiltInTheme(
            id = id,
            displayName = displayName,
            light = null,
            dark = null,
            previewImageName = "theme_${id}_preview",
        )

    /**
     * A soft single-hue gradient theme — top->bottom gradient over neutral white
     * keys. Light-only (dark == null): in dark mode colors(isDark = true) falls back
     * to this light palette, so the theme keeps its light look (USER request — these
     * themes don't darken with the system).
     */
    private fun gradientTheme(
        id: String,
        displayName: String,
        top: Int,
        bottom: Int,
    ): BuiltInTheme =
        BuiltInTheme(
            id = id,
            displayName = displayName,
            light = softGradientColors(top, bottom, LIGHT_KEY_FILL, LIGHT_KEY_TEXT),
            dark = null,
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
