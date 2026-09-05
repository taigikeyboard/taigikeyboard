// Static built-in theme table: three key-style families over one shared set of 7 colors.
// Transparent keys (框線 / 簡潔) set normalKeyFill/specialKeyFill to clear so the keyboard background
// shows through. The 5 gradients are light-only, 暗眠山貓 is dark-only, 預設 stays adaptive (bg/text
// left nil so the renderer resolves them from the system colorScheme).

import Foundation

/// One built-in theme family — a section header (`title`) plus its variant
/// themes, shown as a horizontal shelf in the theme picker. The three families
/// (經典 / 框線 / 簡潔) are a key-STYLE axis over one shared set of 7 colors.
struct BuiltInThemeFamily: Equatable {
    let titleKey: StringKey
    let themes: [BuiltInTheme]
}

/// The app-bundled, read-only theme catalog. Three key-style families, each with
/// the same 7 colors (adaptive 預設 + 5 light-only gradients + 1 dark-only 暗眠山貓).
/// 經典 keeps filled keys; 框線 / 簡潔 make keys transparent (background shows
/// through), 框線 adding an outline.
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

    // MARK: - Key style

    /// The per-family key-style axis. All three families share one set of colors;
    /// only the key rendering differs.
    private enum KeyStyle {
        case classic // filled keys (white over a gradient, adaptive for 預設)
        case framed // transparent keys + outline border
        case clean // transparent keys, no border

        /// Keys are transparent (background shows through) for framed / clean.
        var hasTransparentKeys: Bool { self != .classic }
        /// Only the framed family draws the key outline.
        var isBordered: Bool { self == .framed }
        /// id prefix per family — 經典 keeps the legacy `standard*` ids.
        var idPrefix: String {
            switch self {
            case .classic: "standard"
            case .framed: "framed"
            case .clean: "clean"
            }
        }
    }

    // MARK: - Shared colors

    /// One of the 7 shared color identities. `gradient == nil` is the adaptive
    /// 預設 head; the next 5 are soft light single-hue gradients; the last is the
    /// dark-only 暗眠山貓 (Catppuccin Mocha) gradient (`isDarkPalette`), which lands
    /// in the `dark` variant slot with light text over a dark gradient.
    private struct BaseColor {
        let key: String
        let displayNameKey: StringKey
        let gradient: (top: UInt32, bottom: UInt32)?
        /// Dark palette (e.g. 暗眠山貓/Catppuccin): light text over a dark gradient,
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
        // 暗眠山貓 = Catppuccin Mocha: Base→Mantle background (darker than keys), Surface0 keys, Text glyphs.
        BaseColor(key: "catppuccin", displayNameKey: .themePaletteCatppuccin, gradient: (0x1E1E2E, 0x181825), isDarkPalette: true),
    ]

    // Neutral key fill + text for gradient themes — a light pair and a dark (Catppuccin Mocha) pair.
    // CROSS-PLATFORM INVARIANT — mirrors android .../ime/core/BuiltInThemes.kt LIGHT_KEY_FILL/LIGHT_KEY_TEXT/DARK_KEY_FILL/DARK_KEY_TEXT.
    // Drift causes silent divergence (iOS/Android theme key colors differ).
    private static let lightKeyFill: UInt32 = 0xFFFFFF
    private static let lightKeyText: UInt32 = 0x1C1C1E
    private static let darkKeyFill: UInt32 = 0x313244 // Catppuccin Mocha Surface0
    private static let darkKeyText: UInt32 = 0xCDD6F4 // Catppuccin Mocha Text

    // CROSS-PLATFORM INVARIANT — mirrors android .../ime/core/BuiltInThemes.kt OUTLINED_KEY_BORDER_WIDTH.
    // Drift causes silent divergence.
    private static let outlinedKeyBorderWidth: Double = 1.0

    // MARK: - Builders

    /// Builds the 7 themes for one key-style family. The 經典 head keeps the
    /// `ThemeId.default` sentinel (so reset shows it selected); framed / clean use
    /// `framedDefault` / `cleanDefault` ids. A light theme builds into the `light`
    /// slot (`dark` nil); a dark theme (暗眠山貓) builds into the `dark` slot
    /// (`light` nil) — mirror-symmetric. Preview slots mirror the family id prefix;
    /// missing assets fall back to a neutral placeholder until screenshots ship.
    private static func familyThemes(_ style: KeyStyle) -> [BuiltInTheme] {
        baseColors.map { base in
            let isDefault = base.gradient == nil
            let suffix = base.key.prefix(1).uppercased() + base.key.dropFirst()
            let id = isDefault
                ? (style == .classic ? ThemeId.default : "\(style.idPrefix)Default")
                : "\(style.idPrefix)\(suffix)"
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
                keyBorderWidth: style.isBordered ? outlinedKeyBorderWidth : nil,
            )
        }
    }

    /// Resolves the color palette for one (color, key-style) pair. 經典 預設 stays
    /// fully adaptive (`nil`); framed / clean 預設 carry only transparent key fills
    /// so the adaptive background/text still show through and adapt to dark mode.
    private static func colors(for base: BaseColor, style: KeyStyle) -> KeyboardColorSettings? {
        if let gradient = base.gradient {
            let keyText = base.isDarkPalette ? darkKeyText : lightKeyText
            let neutralFill = base.isDarkPalette ? darkKeyFill : lightKeyFill
            return gradientColors(top: gradient.top, bottom: gradient.bottom, keyText: keyText, neutralFill: neutralFill, transparentKeys: style.hasTransparentKeys)
        }
        guard style.hasTransparentKeys else { return nil } // 經典 預設 = adaptive
        var colors = KeyboardColorSettings()
        colors.normalKeyFillColor = CodableColor(.clear)
        colors.specialKeyFillColor = CodableColor(.clear)
        return colors
    }

    /// One scheme variant for a gradient color: the 2-stop background gradient +
    /// key/candidate text (`keyText`). Keys are either the neutral fill (`neutralFill`,
    /// 經典) or transparent so the gradient shows through (框線 / 簡潔). `backgroundColor`
    /// and `candidateBackgroundColor` stay nil — the gradient owns the background
    /// and the candidate bar is made transparent in `TaigiKeyboardView.candidateStyle`.
    private static func gradientColors(top: UInt32, bottom: UInt32, keyText: UInt32, neutralFill: UInt32, transparentKeys: Bool) -> KeyboardColorSettings {
        var colors = KeyboardColorSettings()
        colors.backgroundGradient = ThemeGradient(stops: [CodableColor(hex: top), CodableColor(hex: bottom)])
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
