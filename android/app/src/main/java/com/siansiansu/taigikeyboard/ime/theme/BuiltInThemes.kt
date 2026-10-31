// Built-in theme catalog — three key-style families over one shared 7-color palette (iOS parity).

package com.siansiansu.taigikeyboard.ime.theme

import com.siansiansu.taigikeyboard.i18n.generated.StringKey

/**
 * One built-in, read-only theme: a named palette with optional light/dark 6-role
 * color variants resolved against night-mode at render time. A null variant falls
 * back to the other. A theme may also carry one appearance override
 * ([keyBorderWidth], used by the Outlined family). Mirrors iOS BuiltInTheme.
 */
data class BuiltInTheme(
    val id: String,
    // i18n key for the display name, resolved at the picker call site via the
    // active StringResolver so the name follows the user's chosen display language.
    val displayNameKey: StringKey,
    val light: KeyboardColorSettings?,
    val dark: KeyboardColorSettings?,
    val previewImageName: String? = null,
    val keyBorderWidth: Float? = null,
    // The Filled-family theme of the same color (itself for a Filled theme): where an Outlined /
    // Borderless theme's Filled keys come from. Null only for a hand-built test entry.
    val filledThemeId: String? = null,
) {
    /** Picks the variant for [isDark], falling back to the other when absent. */
    fun colors(isDark: Boolean): KeyboardColorSettings =
        if (isDark) {
            dark ?: light ?: KeyboardColorSettings()
        } else {
            light ?: dark ?: KeyboardColorSettings()
        }

    /**
     * Whether this theme can seed a user theme: every theme but the adaptive Default heads — they
     * own no background, so a light copy of them is the plain seed (USER 2026-10-31: Default is
     * not a customize base). Mirrors iOS BuiltInTheme.isCustomizeBase.
     */
    val isCustomizeBase: Boolean
        get() = (light ?: dark)?.background != null

    /**
     * The draft a user theme customized from this built-in starts from and resets to: its one
     * concrete variant (light, or dark for Catppuccin) seeded for a user theme, plus its outline.
     * Null for the adaptive Default heads ([isCustomizeBase]).
     * CROSS-PLATFORM INVARIANT — mirrors iOS BuiltInTheme.userThemeBase.
     */
    val userThemeBase: ThemeAppearance?
        get() {
            if (!isCustomizeBase) return null
            val colors = light ?: dark ?: return null
            return ThemeAppearance.DEFAULT.copy(
                colors = colors.seededForUserTheme(),
                keyBorderWidth = keyBorderWidth ?: ThemeAppearance.DEFAULT_KEY_BORDER_WIDTH,
            )
        }
}

/**
 * One built-in family — a section header + its variant themes (one picker shelf).
 * The three families (Filled / Outlined / Borderless) are a key-STYLE axis over one shared set
 * of 7 colors.
 */
data class BuiltInThemeFamily(
    val titleKey: StringKey,
    val themes: List<BuiltInTheme>,
)

/**
 * The app-bundled, read-only theme catalog. Three key-style families, each with
 * the same 7 colors (adaptive Default + 5 light-only gradients + 1 dark-only Catppuccin).
 * Filled keeps filled keys; Outlined / Borderless make keys transparent (background shows
 * through), Outlined adding an outline. Mirrors iOS BuiltInThemes.
 */
object BuiltInThemes {
    // `by lazy` so the catalog builds on FIRST ACCESS, not during object init: a
    // plain `val` here would run `familyThemes` (which reads the `baseColors` val
    // declared below) before this object finishes constructing top-to-bottom, so
    // `baseColors` would still be null. iOS gets this for free (`static let` is lazy).
    val families: List<BuiltInThemeFamily> by lazy {
        listOf(
            BuiltInThemeFamily(StringKey.THEME_FAMILY_CLASSIC, familyThemes(ThemeKeyStyle.CLASSIC)),
            BuiltInThemeFamily(StringKey.THEME_FAMILY_FRAMED, familyThemes(ThemeKeyStyle.FRAMED)),
            BuiltInThemeFamily(StringKey.THEME_FAMILY_CLEAN, familyThemes(ThemeKeyStyle.CLEAN)),
        )
    }

    /** Flattened lookup roster — used by [ThemeResolver] to resolve a selected id. */
    val all: List<BuiltInTheme> by lazy { families.flatMap { it.themes } }

    /** Looks up a built-in by id; null when [id] is not a built-in. */
    fun theme(id: String): BuiltInTheme? = all.firstOrNull { it.id == id }

    /**
     * What the editor resets a user theme to: its built-in base when it was customized from one
     * ([UserTheme.baseThemeId]), else the plain seed. An id no longer in the catalog reads as no
     * base. CROSS-PLATFORM INVARIANT — mirrors iOS BuiltInThemes.userThemeBaseline.
     */
    fun userThemeBaseline(baseThemeId: String?): ThemeAppearance = baseThemeId?.let { theme(it)?.userThemeBase } ?: ThemeAppearance.USER_THEME_SEED

    /**
     * [userThemeBaseline] with Filled keys: the editor's key-color reset target and its first
     * Filled draft. An Outlined / Borderless base takes its Filled twin's keys
     * ([BuiltInTheme.filledThemeId]), so Catppuccin keeps its dark keys under its light text.
     * CROSS-PLATFORM INVARIANT — mirrors iOS BuiltInThemes.userThemeFilledBaseline.
     */
    fun userThemeFilledBaseline(baseThemeId: String?): ThemeAppearance {
        val baseline = userThemeBaseline(baseThemeId)
        if (baseline.keyStyle == ThemeKeyStyle.CLASSIC) return baseline
        val filledKeys = baseThemeId?.let { theme(it)?.filledThemeId }?.let { theme(it)?.userThemeBase }
        return baseline.withKeyStyle(ThemeKeyStyle.CLASSIC, filledKeys ?: ThemeAppearance.USER_THEME_SEED)
    }

    /** id prefix per key-style family — Filled keeps the legacy `standard*` ids. */
    private val ThemeKeyStyle.idPrefix: String
        get() =
            when (this) {
                ThemeKeyStyle.CLASSIC -> "standard"
                ThemeKeyStyle.FRAMED -> "framed"
                ThemeKeyStyle.CLEAN -> "clean"
            }

    /**
     * One of the 7 shared color identities. [gradient] == null is the adaptive Default
     * head; the next 5 are soft light single-hue gradients (raw 0xRRGGBB pairs); the
     * last is the dark-only Catppuccin (Catppuccin Mocha) gradient ([isDarkPalette]), which
     * lands in the `dark` variant slot with light text over a dark gradient.
     */
    private data class BaseColor(
        val key: String,
        val displayNameKey: StringKey,
        val gradient: Pair<Int, Int>?,
        val isDarkPalette: Boolean = false,
    )

    // Order matches the picker shelf; gradient hex values are visual approximations.
    private val baseColors: List<BaseColor> =
        listOf(
            BaseColor("default", StringKey.THEME_PALETTE_DEFAULT, null),
            BaseColor("pink", StringKey.THEME_PALETTE_PINK, 0xE6C2D0 to 0xEADCE2),
            BaseColor("gold", StringKey.THEME_PALETTE_GOLD, 0xEAD9A6 to 0xECE4D2),
            BaseColor("blue", StringKey.THEME_PALETTE_BLUE, 0xBFD2EA to 0xDCE2EC),
            BaseColor("green", StringKey.THEME_PALETTE_GREEN, 0xC3D8C8 to 0xDCE5DD),
            BaseColor("purple", StringKey.THEME_PALETTE_PURPLE, 0xCDC4E4 to 0xDEDAEA),
            // Catppuccin Mocha Base→Mantle background gradient, deliberately darker than the keys.
            BaseColor("catppuccin", StringKey.THEME_PALETTE_CATPPUCCIN, 0x1E1E2E to 0x181825, isDarkPalette = true),
        )

    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Theme/BuiltInThemes.swift lightKeyFill/lightKeyText/darkKeyFill/darkKeyText.
    // Drift causes silent divergence (iOS/Android theme key colors differ).
    private const val LIGHT_KEY_FILL = 0xFFFFFF
    private const val LIGHT_KEY_TEXT = 0x1C1C1E
    private const val DARK_KEY_FILL = 0x313244 // Catppuccin Mocha Surface0
    private const val DARK_KEY_TEXT = 0xCDD6F4 // Catppuccin Mocha Text

    /**
     * Builds the 7 themes for one key-style family. The Filled head keeps the
     * [ThemeId.DEFAULT] sentinel (so reset shows it selected); framed / clean use
     * `framedDefault` / `cleanDefault` ids. A light theme builds into the `light`
     * slot (`dark` null); a dark theme (Catppuccin) builds into the `dark` slot (`light`
     * null) — mirror-symmetric. Preview slots mirror the family id prefix; missing
     * assets fall back to a neutral placeholder until screenshots ship.
     */
    private fun familyThemes(style: ThemeKeyStyle): List<BuiltInTheme> =
        baseColors.map { base ->
            val isDefault = base.gradient == null
            val suffix = base.key.replaceFirstChar { it.uppercase() }
            val id = themeId(isDefault, suffix, style)
            val previewName =
                if (isDefault) "theme_${style.idPrefix}_preview" else "theme_${style.idPrefix}${suffix}_preview"
            val scheme = colorsFor(base, style)
            BuiltInTheme(
                id = id,
                displayNameKey = base.displayNameKey,
                light = if (base.isDarkPalette) null else scheme,
                dark = if (base.isDarkPalette) scheme else null,
                previewImageName = previewName,
                keyBorderWidth = if (style.isBordered) ThemeKeyStyle.OUTLINED_BORDER_WIDTH else null,
                filledThemeId = themeId(isDefault, suffix, ThemeKeyStyle.CLASSIC),
            )
        }

    /**
     * One theme's id: the Filled Default keeps the [ThemeId.DEFAULT] sentinel; every other theme is
     * the family prefix + the color suffix (`Default` for framed / clean Default).
     */
    private fun themeId(
        isDefault: Boolean,
        suffix: String,
        style: ThemeKeyStyle,
    ): String =
        when {
            !isDefault -> "${style.idPrefix}$suffix"
            style == ThemeKeyStyle.CLASSIC -> ThemeId.DEFAULT
            else -> "${style.idPrefix}Default"
        }

    /**
     * Resolves the color palette for one (color, key-style) pair. Filled Default stays
     * fully adaptive (null); framed / clean Default carry only transparent key fills so
     * the adaptive background/text still show through and adapt to dark mode.
     */
    private fun colorsFor(
        base: BaseColor,
        style: ThemeKeyStyle,
    ): KeyboardColorSettings? {
        base.gradient?.let { (top, bottom) ->
            val keyText = if (base.isDarkPalette) DARK_KEY_TEXT else LIGHT_KEY_TEXT
            val neutralFill = if (base.isDarkPalette) DARK_KEY_FILL else LIGHT_KEY_FILL
            return gradientColors(top, bottom, keyText, neutralFill, style.hasTransparentKeys)
        }
        if (!style.hasTransparentKeys) return null // Filled Default stays adaptive
        return KeyboardColorSettings(
            normalKeyFillColor = ThemeKeyStyle.TRANSPARENT_KEY_FILL,
            specialKeyFillColor = ThemeKeyStyle.TRANSPARENT_KEY_FILL,
        )
    }

    /**
     * One scheme variant for a gradient color: the vertical 2-stop background gradient
     * + key/candidate text ([keyText]). Keys are either the neutral fill ([neutralFill],
     * Filled) or transparent so the gradient shows through (Outlined / Borderless). The gradient owns
     * the whole surface — the candidate bar is transparent over it. light/dark themes
     * pass their own keyText/neutralFill.
     */
    private fun gradientColors(
        top: Int,
        bottom: Int,
        keyText: Int,
        neutralFill: Int,
        transparentKeys: Boolean,
    ): KeyboardColorSettings {
        val fill = if (transparentKeys) ThemeKeyStyle.TRANSPARENT_KEY_FILL else argb(neutralFill)
        return KeyboardColorSettings(
            background = ThemeBackground.Gradient(ThemeGradient(listOf(argb(top), argb(bottom)))),
            keyTextColor = argb(keyText),
            normalKeyFillColor = fill,
            specialKeyFillColor = fill,
            candidateTextColor = argb(keyText),
        )
    }

    /** 0xRRGGBB -> opaque ARGB Int (alpha forced 0xFF). */
    private fun argb(rgb: Int): Int = (0xFF shl 24) or (rgb and 0xFFFFFF)
}
