// Static built-in theme table: three key-style families over one shared set of 7 colors.
// Transparent keys (Outlined / Borderless) set normalKeyFill/specialKeyFill to clear so the keyboard background
// shows through. The 5 gradients are light-only, Catppuccin is dark-only, Default stays adaptive (bg/text
// left nil so the renderer resolves them from the system colorScheme).

import Foundation

/// One built-in theme family — a section header (`title`) plus its variant
/// themes, shown as a horizontal shelf in the theme picker. The three families
/// (Filled / Outlined / Borderless) are a key-STYLE axis over one shared set of 7 colors.
struct BuiltInThemeFamily: Equatable {
    let titleKey: StringKey
    let themes: [BuiltInTheme]
}

/// The app-bundled, read-only theme catalog. Three key-style families, each with
/// the same 7 colors (adaptive Default + 5 light-only gradients + 1 dark-only Catppuccin).
/// Filled keeps filled keys; Outlined / Borderless make keys transparent (background shows
/// through), Outlined adding an outline.
enum BuiltInThemes {
    /// Shelf order shown to the user (after `Default`, before user themes).
    static let families: [BuiltInThemeFamily] = [
        BuiltInThemeFamily(titleKey: .themeFamilyClassic, themes: familyThemes(.classic)),
        BuiltInThemeFamily(titleKey: .themeFamilyFramed, themes: familyThemes(.framed)),
        BuiltInThemeFamily(titleKey: .themeFamilyClean, themes: familyThemes(.clean)),
    ]

    /// Flattened lookup roster — used by `ThemeResolver` / `SharedSettings` to
    /// resolve a selected id.
    static let all: [BuiltInTheme] = families.flatMap(\.themes)

    /// Looks up a built-in by id; `nil` when the id is not a built-in.
    static func theme(id: String) -> BuiltInTheme? {
        all.first { $0.id == id }
    }

    /// What the editor resets a user theme to: its built-in base when it was customized from one
    /// (`UserTheme.baseThemeId`), else the plain seed. An id no longer in the catalog reads as no base.
    // CROSS-PLATFORM INVARIANT — mirrors android BuiltInThemes.userThemeBaseline.
    static func userThemeBaseline(baseThemeId: String?) -> ThemeAppearance {
        baseThemeId.flatMap { theme(id: $0)?.userThemeBase } ?? .userThemeSeed
    }

    /// `userThemeBaseline` with Filled keys: the editor's key-color reset target and its first
    /// Filled draft. An Outlined / Borderless base takes its Filled twin's keys
    /// (`BuiltInTheme.filledThemeId`), so Catppuccin keeps its dark keys under its light text.
    // CROSS-PLATFORM INVARIANT — mirrors android BuiltInThemes.userThemeFilledBaseline.
    static func userThemeFilledBaseline(baseThemeId: String?) -> ThemeAppearance {
        let baseline = userThemeBaseline(baseThemeId: baseThemeId)
        guard baseline.keyStyle != .classic else { return baseline }
        let filledKeys = baseThemeId.flatMap { theme(id: $0)?.filledThemeId }.flatMap { theme(id: $0)?.userThemeBase }
        return baseline.withKeyStyle(.classic, filledKeys: filledKeys ?? .userThemeSeed)
    }

    // MARK: - Shared colors

    /// One of the 7 shared color identities. `gradient == nil` is the adaptive
    /// Default head; the next 5 are soft light single-hue gradients; the last is the
    /// dark-only Catppuccin (Catppuccin Mocha) gradient (`isDarkPalette`), which lands
    /// in the `dark` variant slot with light text over a dark gradient.
    private struct BaseColor {
        let key: String
        let displayNameKey: StringKey
        let gradient: (top: UInt32, bottom: UInt32)?
        /// Dark palette (e.g. Catppuccin): light text over a dark gradient,
        /// with a dark neutral key fill; builds into the `dark` variant slot
        /// (`light` nil) so it stays dark regardless of system scheme.
        var isDarkPalette: Bool = false
    }

    // Gradient hex values are visual approximations.
    private static let baseColors: [BaseColor] = [
        BaseColor(key: "default", displayNameKey: .themePaletteDefault, gradient: nil),
        BaseColor(key: "pink", displayNameKey: .themePalettePink, gradient: (0xE6C2D0, 0xEADCE2)),
        BaseColor(key: "gold", displayNameKey: .themePaletteGold, gradient: (0xEAD9A6, 0xECE4D2)),
        BaseColor(key: "blue", displayNameKey: .themePaletteBlue, gradient: (0xBFD2EA, 0xDCE2EC)),
        BaseColor(key: "green", displayNameKey: .themePaletteGreen, gradient: (0xC3D8C8, 0xDCE5DD)),
        BaseColor(key: "purple", displayNameKey: .themePalettePurple, gradient: (0xCDC4E4, 0xDEDAEA)),
        // Catppuccin = Catppuccin Mocha: Base→Mantle background (darker than keys), Surface0 keys, Text glyphs.
        BaseColor(key: "catppuccin", displayNameKey: .themePaletteCatppuccin, gradient: (0x1E1E2E, 0x181825), isDarkPalette: true),
    ]

    // Neutral key fill + text for gradient themes — a light pair and a dark (Catppuccin Mocha) pair.
    // CROSS-PLATFORM INVARIANT — mirrors android .../ime/theme/BuiltInThemes.kt LIGHT_KEY_FILL/LIGHT_KEY_TEXT/DARK_KEY_FILL/DARK_KEY_TEXT.
    // Drift causes silent divergence (iOS/Android theme key colors differ).
    private static let lightKeyFill: UInt32 = 0xFFFFFF
    private static let lightKeyText: UInt32 = 0x1C1C1E
    private static let darkKeyFill: UInt32 = 0x313244 // Catppuccin Mocha Surface0
    private static let darkKeyText: UInt32 = 0xCDD6F4 // Catppuccin Mocha Text

    // MARK: - Builders

    /// Builds the 7 themes for one key-style family. The Filled head keeps the
    /// `ThemeId.default` sentinel (so reset shows it selected); framed / clean use
    /// `framedDefault` / `cleanDefault` ids. A light theme builds into the `light`
    /// slot (`dark` nil); a dark theme (Catppuccin) builds into the `dark` slot
    /// (`light` nil) — mirror-symmetric. Preview slots mirror the family id prefix;
    /// missing assets fall back to a neutral placeholder until screenshots ship.
    private static func familyThemes(_ style: ThemeKeyStyle) -> [BuiltInTheme] {
        baseColors.map { base in
            let isDefault = base.gradient == nil
            let suffix = base.key.prefix(1).uppercased() + base.key.dropFirst()
            let id = themeId(isDefault: isDefault, suffix: suffix, style: style)
            let previewName = isDefault
                ? "theme_\(style.idPrefix)_preview"
                : "theme_\(style.idPrefix)\(suffix)_preview"
            let scheme = colors(for: base, style: style)
            return BuiltInTheme(
                id: id,
                displayNameKey: base.displayNameKey,
                light: base.isDarkPalette ? nil : scheme,
                dark: base.isDarkPalette ? scheme : nil,
                previewImageName: previewName,
                keyBorderWidth: style.isBordered ? ThemeKeyStyle.outlinedBorderWidth : nil,
                filledThemeId: themeId(isDefault: isDefault, suffix: suffix, style: .classic),
            )
        }
    }

    /// One theme's id: the Filled Default keeps the `ThemeId.default` sentinel; every other theme is
    /// the family prefix + the color suffix (`Default` for framed / clean Default).
    private static func themeId(isDefault: Bool, suffix: String, style: ThemeKeyStyle) -> String {
        isDefault
            ? (style == .classic ? ThemeId.default : "\(style.idPrefix)Default")
            : "\(style.idPrefix)\(suffix)"
    }

    /// Resolves the color palette for one (color, key-style) pair. Filled Default stays
    /// fully adaptive (`nil`); framed / clean Default carry only transparent key fills
    /// so the adaptive background/text still show through and adapt to dark mode.
    private static func colors(for base: BaseColor, style: ThemeKeyStyle) -> KeyboardColorSettings? {
        if let gradient = base.gradient {
            let keyText = base.isDarkPalette ? darkKeyText : lightKeyText
            let neutralFill = base.isDarkPalette ? darkKeyFill : lightKeyFill
            return gradientColors(top: gradient.top, bottom: gradient.bottom, keyText: keyText, neutralFill: neutralFill, transparentKeys: style.hasTransparentKeys)
        }
        guard style.hasTransparentKeys else { return nil } // Filled Default = adaptive
        var colors = KeyboardColorSettings()
        colors.normalKeyFillColor = CodableColor(.clear)
        colors.specialKeyFillColor = CodableColor(.clear)
        return colors
    }

    /// One scheme variant for a gradient color: the vertical 2-stop background
    /// gradient + key/candidate text (`keyText`). Keys are either the neutral fill
    /// (`neutralFill`, Filled) or transparent so the gradient shows through (Outlined / Borderless).
    /// The gradient owns the whole surface — the candidate bar is made transparent in
    /// `TaigiKeyboardView.candidateStyle`.
    private static func gradientColors(top: UInt32, bottom: UInt32, keyText: UInt32, neutralFill: UInt32, transparentKeys: Bool) -> KeyboardColorSettings {
        var colors = KeyboardColorSettings()
        colors.background = .gradient(ThemeGradient(stops: [CodableColor(hex: top), CodableColor(hex: bottom)]))
        colors.keyTextColor = CodableColor(hex: keyText)
        colors.candidateTextColor = CodableColor(hex: keyText)
        if transparentKeys {
            colors.normalKeyFillColor = CodableColor(.clear)
            colors.specialKeyFillColor = CodableColor(.clear)
        } else {
            let fill = CodableColor(hex: neutralFill)
            colors.normalKeyFillColor = fill
            colors.specialKeyFillColor = fill
        }
        return colors
    }
}

/// id prefix per key-style family — Filled keeps the legacy `standard*` ids.
private extension ThemeKeyStyle {
    var idPrefix: String {
        switch self {
        case .classic: "standard"
        case .framed: "framed"
        case .clean: "clean"
        }
    }
}
