import Foundation
import SwiftUI

// MARK: - Theme identity

enum ThemeId {
    /// The factory theme: all-nil → KeyboardKit adaptive colors + Liquid Glass.
    static let `default` = "default"

    /// Whether `id` is a user theme. User-theme ids are `UUID` strings; the
    /// `default` theme and built-in ids are not. Distinguishes themes that own
    /// their appearance (incl. explicit shadow) from `default` / built-in themes
    /// that inherit KeyboardKit's standard look.
    static func isUserTheme(_ id: String) -> Bool {
        UUID(uuidString: id) != nil
    }

    /// The appearance the keyboard renders `id` in regardless of the system: a user theme is a
    /// light theme (USER 2026-09-26), so no color may follow dark mode; nil = follow the system.
    // CROSS-PLATFORM INVARIANT — mirrors android TaigiKeyboard.syncForcedLight (user theme ⇒ NIGHT_NO).
    static func forcedColorScheme(for id: String) -> ColorScheme? {
        isUserTheme(id) ? .light : nil
    }
}

// MARK: - Built-in theme

/// A read-only, app-bundled theme: a named palette with light and/or dark
/// 6-role color variants, resolved against the system `colorScheme` at render
/// time. `light`/`dark` are concrete `KeyboardColorSettings` (every role set);
/// a `nil` variant (e.g. Catppuccin, which ships dark-only by design)
/// falls back to the other variant.
struct BuiltInTheme: Equatable {
    let id: String
    /// i18n key for the display name, resolved at the picker call site via the
    /// `DisplayLanguageStore` so the name follows the user's chosen display
    /// language (mirrors `InputMode.displayNameKey` / `FontType.displayNameKey`).
    let displayNameKey: StringKey
    let light: KeyboardColorSettings?
    let dark: KeyboardColorSettings?

    /// Asset name for the card preview screenshot (sized to match the Layout
    /// page's `layout_*_preview` assets). `nil` → fall back to the live color
    /// swatch.
    var previewImageName: String?

    /// Optional key-outline width override. Built-in themes are colors-first, but
    /// a theme may carry this one appearance scalar so the resolver applies it on
    /// top of the factory sizes (used by the Outlined key-style family). `nil` keeps
    /// the factory `keyBorderWidth` (0 = no border).
    var keyBorderWidth: Double?

    /// The Filled-family theme of the same color (itself for a Filled theme): where an Outlined /
    /// Borderless theme's Filled keys come from. `nil` only for a hand-built test entry.
    var filledThemeId: String?

    /// Picks the variant for `scheme`, falling back to the other variant when
    /// one is absent. `.default` (all-nil → KeyboardKit adaptive) is the final
    /// fallback only for a malformed entry with neither variant.
    func colors(for scheme: ColorScheme) -> KeyboardColorSettings {
        switch scheme {
        case .dark: dark ?? light ?? .default
        case .light: light ?? dark ?? .default
        @unknown default: light ?? dark ?? .default
        }
    }

    /// Whether this theme can seed a user theme: every theme but the adaptive Default heads — they
    /// own no background, so a light copy of them is the plain seed (USER 2026-10-31: Default is
    /// not a customize base).
    var isCustomizeBase: Bool {
        (light ?? dark)?.background != nil
    }

    /// The draft a user theme customized from this built-in starts from and resets to: its one
    /// concrete variant (light, or dark for Catppuccin) seeded for a user theme, plus its outline.
    /// `nil` for the adaptive Default heads (`isCustomizeBase`).
    // CROSS-PLATFORM INVARIANT — mirrors android BuiltInTheme.userThemeBase.
    var userThemeBase: ThemeAppearance? {
        guard isCustomizeBase, let colors = light ?? dark else { return nil }
        var appearance = ThemeAppearance.default
        appearance.colors = colors.seededForUserTheme()
        appearance.keyBorderWidth = keyBorderWidth ?? ThemeAppearance.default.keyBorderWidth
        return appearance
    }
}

// MARK: - Theme appearance bundle

/// The full set of appearance values a theme captures: 6-role colors plus the
/// key-shadow intensity and the five size scalars (key height / key font /
/// candidate font / corner radius / border width).
///
/// One bundle is the unit of (a) what a `UserTheme` stores, (b) what the
/// `ThemeResolver` returns, and (c) what the renderer reads — so the values are
/// never spread field-by-field across resolver / snapshot / editor.
///
/// Font is intentionally NOT part of a theme: it is a GLOBAL setting
/// (`SharedSettings.fontType`), so switching themes never changes the font.
///
/// `colors` stays OPTIONAL per role (reuses `KeyboardColorSettings`) for the
/// `default` theme and built-in themes, where a `nil` role inherits KeyboardKit's
/// adaptive color. User themes are always seeded (`userThemeSeed`) so they carry
/// no `nil` role and look the same in light and dark mode.
struct ThemeAppearance: Codable, Equatable {
    var colors: KeyboardColorSettings
    var keyShadowIntensity: Double
    var keyHeightScale: Double
    var keyFontSizeScale: Double
    var candidateTextSizeScale: Double
    var keyCornerRadius: Double
    var keyBorderWidth: Double

    /// Factory appearance — all-nil adaptive colors, flat shadow, unity scales,
    /// project-default corner radius / border. Used as the base for built-in
    /// themes (which only define colors) and as the missing-field fallback when
    /// decoding.
    static let `default` = ThemeAppearance(
        colors: .default,
        keyShadowIntensity: 0,
        keyHeightScale: 1,
        keyFontSizeScale: 1,
        candidateTextSizeScale: 1,
        keyCornerRadius: 6,
        keyBorderWidth: 0,
    )

    /// The draft a NEW user theme starts from: factory sizes + the concrete light
    /// palette (`UserThemeSeed.colors`). Also what Reset to Defaults restores.
    static let userThemeSeed: ThemeAppearance = {
        var appearance = ThemeAppearance.default
        appearance.colors = UserThemeSeed.colors
        return appearance
    }()

    /// The key style this appearance renders: see-through keys (a clear key fill) are Outlined
    /// with a border and Borderless without; any visible fill is Filled, border or not.
    var keyStyle: ThemeKeyStyle {
        guard colors.hasTransparentKeys else { return .classic }
        return keyBorderWidth > 0 ? .framed : .clean
    }

    /// This appearance switched to `style`. Filled restores the key fill, shadow and border of
    /// `filledKeys` (the editor's last Filled draft); Outlined / Borderless clear the key fill and
    /// shadow, Outlined drawing the built-in outline and Borderless none.
    // CROSS-PLATFORM INVARIANT — mirrors android .../ime/theme/ThemeAppearance.kt withKeyStyle.
    func withKeyStyle(_ style: ThemeKeyStyle, filledKeys: ThemeAppearance) -> ThemeAppearance {
        var next = self
        switch style {
        case .classic:
            next.colors.keyFillColor = filledKeys.colors.keyFillColor
            next.keyShadowIntensity = filledKeys.keyShadowIntensity
            next.keyBorderWidth = filledKeys.keyBorderWidth
        case .framed, .clean:
            next.colors.keyFillColor = CodableColor(.clear)
            next.keyShadowIntensity = 0
            next.keyBorderWidth = style.isBordered ? ThemeKeyStyle.outlinedBorderWidth : 0
        }
        return next
    }
}

// MARK: - Key style

/// The key-style axis shared by the built-in families and the user-theme editor: Filled keys,
/// Outlined (see-through keys + outline) or Borderless (see-through keys, no outline). Not
/// stored — a clear key fill plus the border width encode it (`ThemeAppearance.keyStyle`).
// CROSS-PLATFORM INVARIANT — mirrors android .../ime/theme/ThemeAppearance.kt ThemeKeyStyle.
enum ThemeKeyStyle: CaseIterable {
    case classic // filled keys
    case framed // transparent keys + outline border
    case clean // transparent keys, no border

    /// Keys are transparent (background shows through) for framed / clean.
    var hasTransparentKeys: Bool {
        self != .classic
    }

    /// Only the framed style draws the key outline.
    var isBordered: Bool {
        self == .framed
    }

    /// The outline width of the framed style.
    // CROSS-PLATFORM INVARIANT — mirrors android ThemeKeyStyle.OUTLINED_BORDER_WIDTH. Drift causes silent divergence.
    static let outlinedBorderWidth: Double = 1.0
}

// Decode lives in an extension so the struct keeps its synthesized memberwise init.
extension ThemeAppearance {
    /// Forward-compatible decode: any field absent in a stored theme falls back
    /// to the project default, so a future appearance field never strands themes
    /// written by an older build. (Encode + memberwise init are synthesized.)
    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let fallback = ThemeAppearance.default
        colors = try container.decodeIfPresent(KeyboardColorSettings.self, forKey: .colors) ?? fallback.colors
        keyShadowIntensity = try container.decodeIfPresent(Double.self, forKey: .keyShadowIntensity) ?? fallback.keyShadowIntensity
        keyHeightScale = try container.decodeIfPresent(Double.self, forKey: .keyHeightScale) ?? fallback.keyHeightScale
        keyFontSizeScale = try container.decodeIfPresent(Double.self, forKey: .keyFontSizeScale) ?? fallback.keyFontSizeScale
        candidateTextSizeScale = try container.decodeIfPresent(Double.self, forKey: .candidateTextSizeScale) ?? fallback.candidateTextSizeScale
        keyCornerRadius = try container.decodeIfPresent(Double.self, forKey: .keyCornerRadius) ?? fallback.keyCornerRadius
        keyBorderWidth = try container.decodeIfPresent(Double.self, forKey: .keyBorderWidth) ?? fallback.keyBorderWidth
        // Font is a global setting, not part of a theme; an old user_themes.json "fontType" key still
        // decodes because Codable ignores unknown keys.
    }
}

// MARK: - User-created theme

/// A user-created, named, persisted keyboard theme: identity + name + the full
/// `ThemeAppearance` bundle + timestamps.
///
/// No backward-compat decode is needed: the user-theme write path (CRUD) has
/// never shipped (PR-3) and no seeding/migration ever wrote `user_themes.json`
/// (verified — the only `userThemeStore` mutators are the unused CRUD wrappers),
/// so no legacy envelope can exist on any device. Schema growth is handled
/// forward by `ThemeAppearance`'s `decodeIfPresent` decoder.
struct UserTheme: Codable, Equatable, Identifiable {
    let id: UUID
    var name: String
    var appearance: ThemeAppearance
    var createdAt: Date
    var updatedAt: Date
    /// The built-in theme this one was customized from (its reset target), or `nil` for a theme
    /// started blank. Optional, so themes saved before this field decode with `nil`.
    var baseThemeId: String? = nil
}
