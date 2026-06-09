// 中文: 內建主題靜態表 — 三個「按鍵風格」family(經典 / 框線 / 簡潔),共用同一組 6 色(預設 + 5 漸層)。對齊 iOS BuiltInThemes。
// 中文: 三 family 顏色完全相同,差別只在按鍵風格:經典 = 一般填色鍵;框線 = 透明鍵 + 邊框;簡潔 = 透明鍵無框。
// 中文: 透明鍵 = normalKeyFill/specialKeyFill 設成透明(ARGB alpha 0)→ 鍵盤背景(平塗/漸層)從鍵面透出 = 「鍵=背景」。
// 中文: 漸層 + 框線/簡潔 皆 light-only(dark = null);預設保持 adaptive(bg/text 留 null,render 端依 night-mode 解析)。
// 中文: id 為非 UUID 字串。經典維持既有 id(default sentinel / standardPink…);框線 = framed*、簡潔 = clean*。

package com.siansiansu.taigikeyboard.ime.core

/**
 * One built-in, read-only theme: a named palette with optional light/dark 6-role
 * color variants resolved against night-mode at render time. A null variant falls
 * back to the other. A theme may also carry one appearance override
 * ([keyBorderWidth], used by the 框線 family). Mirrors iOS BuiltInTheme.
 */
data class BuiltInTheme(
    val id: String,
    val displayName: String,
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
 * of 6 colors.
 */
data class BuiltInThemeFamily(
    val title: String,
    val themes: List<BuiltInTheme>,
)

/**
 * The app-bundled, read-only theme catalog. Three key-style families, each with
 * the same 6 colors (adaptive 預設 + 5 light-only gradients). 經典 keeps filled
 * keys; 框線 / 簡潔 make keys transparent (background shows through), 框線 adding an
 * outline. Mirrors iOS BuiltInThemes.
 */
object BuiltInThemes {
    // `by lazy` so the catalog builds on FIRST ACCESS, not during object init: a
    // plain `val` here would run `familyThemes` (which reads the `baseColors` val
    // declared below) before this object finishes constructing top-to-bottom, so
    // `baseColors` would still be null. iOS gets this for free (`static let` is lazy).
    val families: List<BuiltInThemeFamily> by lazy {
        listOf(
            BuiltInThemeFamily("經典", familyThemes(KeyStyle.CLASSIC)),
            BuiltInThemeFamily("框線", familyThemes(KeyStyle.FRAMED)),
            BuiltInThemeFamily("簡潔", familyThemes(KeyStyle.CLEAN)),
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
     * One of the 6 shared color identities. [gradient] == null is the adaptive 預設
     * head; the other 5 are soft single-hue gradients (raw 0xRRGGBB pairs).
     */
    private data class BaseColor(
        val key: String,
        val displayName: String,
        val gradient: Pair<Int, Int>?,
    )

    // 中文: 6 色順序:預設(adaptive)→ 櫻花 → 金煌 → 海風 → 翠青 → 藤紫。漸層 hex 為視覺估值。
    private val baseColors: List<BaseColor> =
        listOf(
            BaseColor("default", "預設", null),
            BaseColor("pink", "櫻花", 0xE6C2D0 to 0xEADCE2),
            BaseColor("gold", "金煌", 0xEAD9A6 to 0xECE4D2),
            BaseColor("blue", "海風", 0xBFD2EA to 0xDCE2EC),
            BaseColor("green", "翠青", 0xC3D8C8 to 0xDCE5DD),
            BaseColor("purple", "藤紫", 0xCDC4E4 to 0xDEDAEA),
        )

    // 中文: 漸層中性鍵色(經典白鍵)+ 鍵字色(≈經典黑);light-only,只需 light 鍵色。
    private const val LIGHT_KEY_FILL = 0xFFFFFF
    private const val LIGHT_KEY_TEXT = 0x1C1C1E

    // 中文: 透明鍵填色(ARGB alpha 0)— 框線/簡潔 用,鍵盤背景從鍵面透出。
    private const val TRANSPARENT_KEY_FILL = 0x00000000

    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/BuiltInThemes.swift outlinedKeyBorderWidth.
    // Drift causes silent divergence. 框線 family 的鍵邊框寬度。
    private const val OUTLINED_KEY_BORDER_WIDTH = 1.0f

    /**
     * Builds the 6 themes for one key-style family. The 經典 head keeps the
     * [ThemeId.DEFAULT] sentinel (so reset shows it selected); framed / clean use
     * `framedDefault` / `cleanDefault` ids. Preview slots mirror the family id
     * prefix; only 經典's assets ship today (framed / clean → neutral placeholder).
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
            BuiltInTheme(
                id = id,
                displayName = base.displayName,
                light = colorsFor(base, style),
                dark = null,
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
            return gradientColors(top, bottom, style.hasTransparentKeys)
        }
        if (!style.hasTransparentKeys) return null // 經典 預設 = adaptive
        return KeyboardColorSettings(
            normalKeyFillColor = TRANSPARENT_KEY_FILL,
            specialKeyFillColor = TRANSPARENT_KEY_FILL,
        )
    }

    /**
     * One scheme variant for a gradient color: the 2-stop background gradient +
     * key/candidate text. Keys are either the neutral white fill (經典) or
     * transparent so the gradient shows through (框線 / 簡潔). backgroundColor and
     * candidateBackgroundColor stay null so the gradient owns the background and the
     * candidate bar is transparent over it.
     */
    private fun gradientColors(top: Int, bottom: Int, transparentKeys: Boolean): KeyboardColorSettings {
        val fill = if (transparentKeys) TRANSPARENT_KEY_FILL else argb(LIGHT_KEY_FILL)
        return KeyboardColorSettings(
            keyTextColor = argb(LIGHT_KEY_TEXT),
            normalKeyFillColor = fill,
            specialKeyFillColor = fill,
            candidateTextColor = argb(LIGHT_KEY_TEXT),
            backgroundGradient = ThemeGradient(listOf(argb(top), argb(bottom))),
        )
    }

    /** 0xRRGGBB -> opaque ARGB Int (alpha forced 0xFF). */
    private fun argb(rgb: Int): Int = (0xFF shl 24) or (rgb and 0xFFFFFF)
}
