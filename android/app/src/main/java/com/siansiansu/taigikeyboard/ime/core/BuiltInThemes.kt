// 中文: 內建主題靜態表 — 三個「按鍵風格」family(經典 / 框線 / 簡潔),共用同一組 7 色(預設 + 5 light 漸層 + 1 dark 暗眠山貓)。對齊 iOS BuiltInThemes。
// 中文: 三 family 顏色完全相同,差別只在按鍵風格:經典 = 一般填色鍵;框線 = 透明鍵 + 邊框;簡潔 = 透明鍵無框。
// 中文: 透明鍵 = normalKeyFill/specialKeyFill 設成透明(ARGB alpha 0)→ 鍵盤背景(平塗/漸層)從鍵面透出 = 「鍵=背景」。
// 中文: 5 漸層 light-only(dark = null,深色模式仍顯 light);暗眠山貓 dark-only(light = null,淺色模式仍顯 dark);預設保持 adaptive(bg/text 留 null,render 端依 night-mode 解析)。
// 中文: id 為非 UUID 字串。經典維持既有 id(default sentinel / standardPink…);框線 = framed*、簡潔 = clean*。

package com.siansiansu.taigikeyboard.ime.core

import com.siansiansu.taigikeyboard.i18n.generated.StringKey

/**
 * One built-in, read-only theme: a named palette with optional light/dark 6-role
 * color variants resolved against night-mode at render time. A null variant falls
 * back to the other. A theme may also carry one appearance override
 * ([keyBorderWidth], used by the 框線 family). Mirrors iOS BuiltInTheme.
 */
data class BuiltInTheme(
    val id: String,
    // i18n key for the display name, resolved at the picker call site via the
    // active StringResolver so the name follows the user's chosen display language.
    // 中文: 顯示名稱的 i18n key,在 picker call site 用 StringResolver 解析,跟隨使用者選的顯示語言。
    val displayNameKey: StringKey,
    val light: KeyboardColorSettings?,
    val dark: KeyboardColorSettings?,
    val previewImageName: String? = null,
    val keyBorderWidth: Float? = null,
) {
    /** Picks the variant for [isDark], falling back to the other when absent. */
    fun colors(isDark: Boolean): KeyboardColorSettings =
        if (isDark) {
            dark ?: light ?: KeyboardColorSettings()
        } else {
            light ?: dark ?: KeyboardColorSettings()
        }
}

/**
 * One built-in family — a section header + its variant themes (one picker shelf).
 * The three families (經典 / 框線 / 簡潔) are a key-STYLE axis over one shared set
 * of 7 colors.
 */
data class BuiltInThemeFamily(
    val titleKey: StringKey,
    val themes: List<BuiltInTheme>,
)

/**
 * The app-bundled, read-only theme catalog. Three key-style families, each with
 * the same 7 colors (adaptive 預設 + 5 light-only gradients + 1 dark-only 暗眠山貓).
 * 經典 keeps filled keys; 框線 / 簡潔 make keys transparent (background shows
 * through), 框線 adding an outline. Mirrors iOS BuiltInThemes.
 */
object BuiltInThemes {
    // `by lazy` so the catalog builds on FIRST ACCESS, not during object init: a
    // plain `val` here would run `familyThemes` (which reads the `baseColors` val
    // declared below) before this object finishes constructing top-to-bottom, so
    // `baseColors` would still be null. iOS gets this for free (`static let` is lazy).
    val families: List<BuiltInThemeFamily> by lazy {
        listOf(
            BuiltInThemeFamily(StringKey.THEME_FAMILY_CLASSIC, familyThemes(KeyStyle.CLASSIC)),
            BuiltInThemeFamily(StringKey.THEME_FAMILY_FRAMED, familyThemes(KeyStyle.FRAMED)),
            BuiltInThemeFamily(StringKey.THEME_FAMILY_CLEAN, familyThemes(KeyStyle.CLEAN)),
        )
    }

    /** Flattened lookup roster — used by [ThemeResolver] to resolve a selected id. */
    val all: List<BuiltInTheme> by lazy { families.flatMap { it.themes } }

    /** Looks up a built-in by id; null when [id] is not a built-in. */
    fun theme(id: String): BuiltInTheme? = all.firstOrNull { it.id == id }

    /**
     * The per-family key-style axis. All three families share one set of colors;
     * only the key rendering differs. [idPrefix] keeps 經典 on the legacy
     * `standard*` ids.
     */
    private enum class KeyStyle(val idPrefix: String) {
        CLASSIC("standard"), // filled keys (white over a gradient, adaptive for 預設)
        FRAMED("framed"), // transparent keys + outline border
        CLEAN("clean"), // transparent keys, no border
        ;

        val hasTransparentKeys: Boolean get() = this != CLASSIC
        val isBordered: Boolean get() = this == FRAMED
    }

    /**
     * One of the 7 shared color identities. [gradient] == null is the adaptive 預設
     * head; the next 5 are soft light single-hue gradients (raw 0xRRGGBB pairs); the
     * last is the dark-only 暗眠山貓 (Catppuccin Mocha) gradient ([isDarkPalette]), which
     * lands in the `dark` variant slot with light text over a dark gradient.
     */
    private data class BaseColor(
        val key: String,
        val displayNameKey: StringKey,
        val gradient: Pair<Int, Int>?,
        // 中文: dark 主題:深漸層配 light 字 + 深中性鍵色,放 dark slot(light=null),不隨系統明暗變。
        val isDarkPalette: Boolean = false,
    )

    // 中文: 7 色順序:預設(adaptive)→ 櫻花 → 金煌 → 海風 → 翠青 → 藤紫(以上 light)→ 暗眠山貓(dark)。漸層 hex 為視覺估值。
    private val baseColors: List<BaseColor> =
        listOf(
            BaseColor("default", StringKey.THEME_PALETTE_DEFAULT, null),
            BaseColor("pink", StringKey.THEME_PALETTE_PINK, 0xE6C2D0 to 0xEADCE2),
            BaseColor("gold", StringKey.THEME_PALETTE_GOLD, 0xEAD9A6 to 0xECE4D2),
            BaseColor("blue", StringKey.THEME_PALETTE_BLUE, 0xBFD2EA to 0xDCE2EC),
            BaseColor("green", StringKey.THEME_PALETTE_GREEN, 0xC3D8C8 to 0xDCE5DD),
            BaseColor("purple", StringKey.THEME_PALETTE_PURPLE, 0xCDC4E4 to 0xDEDAEA),
            // 暗眠山貓: Catppuccin Mocha — 背景 Base→Mantle 漸層(比鍵深),鍵 Surface0,字 Text。
            BaseColor("catppuccin", StringKey.THEME_PALETTE_CATPPUCCIN, 0x1E1E2E to 0x181825, isDarkPalette = true),
        )

    // 中文: 漸層中性鍵色 + 鍵字色 — light 主題(經典白鍵 / ≈經典黑字)與 dark 主題(暗眠山貓:Catppuccin Mocha Surface0 鍵 / Text 字)各一組。
    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/BuiltInThemes.swift lightKeyFill/lightKeyText/darkKeyFill/darkKeyText.
    // Drift causes silent divergence (iOS/Android theme key colors differ).
    private const val LIGHT_KEY_FILL = 0xFFFFFF
    private const val LIGHT_KEY_TEXT = 0x1C1C1E
    private const val DARK_KEY_FILL = 0x313244 // Catppuccin Mocha Surface0
    private const val DARK_KEY_TEXT = 0xCDD6F4 // Catppuccin Mocha Text

    // 中文: 透明鍵填色(ARGB alpha 0)— 框線/簡潔 用,鍵盤背景從鍵面透出。
    private const val TRANSPARENT_KEY_FILL = 0x00000000

    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/BuiltInThemes.swift outlinedKeyBorderWidth.
    // Drift causes silent divergence. 框線 family 的鍵邊框寬度。
    private const val OUTLINED_KEY_BORDER_WIDTH = 1.0f

    /**
     * Builds the 7 themes for one key-style family. The 經典 head keeps the
     * [ThemeId.DEFAULT] sentinel (so reset shows it selected); framed / clean use
     * `framedDefault` / `cleanDefault` ids. A light theme builds into the `light`
     * slot (`dark` null); a dark theme (暗眠山貓) builds into the `dark` slot (`light`
     * null) — mirror-symmetric. Preview slots mirror the family id prefix; missing
     * assets fall back to a neutral placeholder until screenshots ship.
     */
    private fun familyThemes(style: KeyStyle): List<BuiltInTheme> =
        baseColors.map { base ->
            val isDefault = base.gradient == null
            val suffix = base.key.replaceFirstChar { it.uppercase() }
            val id =
                when {
                    !isDefault -> "${style.idPrefix}$suffix"
                    style == KeyStyle.CLASSIC -> ThemeId.DEFAULT
                    else -> "${style.idPrefix}Default"
                }
            val previewName =
                if (isDefault) "theme_${style.idPrefix}_preview" else "theme_${style.idPrefix}${suffix}_preview"
            val scheme = colorsFor(base, style)
            BuiltInTheme(
                id = id,
                displayNameKey = base.displayNameKey,
                light = if (base.isDarkPalette) null else scheme,
                dark = if (base.isDarkPalette) scheme else null,
                previewImageName = previewName,
                keyBorderWidth = if (style.isBordered) OUTLINED_KEY_BORDER_WIDTH else null,
            )
        }

    /**
     * Resolves the color palette for one (color, key-style) pair. 經典 預設 stays
     * fully adaptive (null); framed / clean 預設 carry only transparent key fills so
     * the adaptive background/text still show through and adapt to dark mode.
     */
    private fun colorsFor(base: BaseColor, style: KeyStyle): KeyboardColorSettings? {
        base.gradient?.let { (top, bottom) ->
            val keyText = if (base.isDarkPalette) DARK_KEY_TEXT else LIGHT_KEY_TEXT
            val neutralFill = if (base.isDarkPalette) DARK_KEY_FILL else LIGHT_KEY_FILL
            return gradientColors(top, bottom, keyText, neutralFill, style.hasTransparentKeys)
        }
        if (!style.hasTransparentKeys) return null // 經典 預設 = adaptive
        return KeyboardColorSettings(
            normalKeyFillColor = TRANSPARENT_KEY_FILL,
            specialKeyFillColor = TRANSPARENT_KEY_FILL,
        )
    }

    /**
     * One scheme variant for a gradient color: the 2-stop background gradient +
     * key/candidate text ([keyText]). Keys are either the neutral fill ([neutralFill],
     * 經典) or transparent so the gradient shows through (框線 / 簡潔). backgroundColor
     * and candidateBackgroundColor stay null so the gradient owns the background and
     * the candidate bar is transparent over it. light/dark themes pass their own
     * keyText/neutralFill.
     */
    private fun gradientColors(top: Int, bottom: Int, keyText: Int, neutralFill: Int, transparentKeys: Boolean): KeyboardColorSettings {
        val fill = if (transparentKeys) TRANSPARENT_KEY_FILL else argb(neutralFill)
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
