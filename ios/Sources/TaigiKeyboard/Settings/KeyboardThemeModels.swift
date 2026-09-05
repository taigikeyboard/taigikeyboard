import Foundation
import SwiftUI

// MARK: - Theme identity

enum ThemeId {
    /// The legacy free-pick buffer. Uncustomized users resolve to all-nil →
    /// KeyboardKit adaptive colors + Liquid Glass; customized users keep their
    /// `colorSettings` look without any migration.
    static let `default` = "default"

    /// Whether `id` is a user theme. User-theme ids are `UUID` strings; the
    /// `default` buffer and built-in ids are not. Distinguishes themes that own
    /// their appearance (incl. explicit shadow) from `default` / built-in themes
    /// that inherit KeyboardKit's standard look.
    static func isUserTheme(_ id: String) -> Bool {
        UUID(uuidString: id) != nil
    }
}

// MARK: - Built-in theme

/// A read-only, app-bundled theme: a named palette with light and/or dark
/// 6-role color variants, resolved against the system `colorScheme` at render
/// time. `light`/`dark` are concrete `KeyboardColorSettings` (every role set);
/// a `nil` variant (e.g. 暗眠山貓/Catppuccin, which ships dark-only by design)
/// falls back to the other variant.
struct BuiltInTheme: Equatable {
    let id: String
    /// i18n key for the display name, resolved at the picker call site via the
    /// `DisplayLanguageStore` so the name follows the user's chosen display
    /// language (mirrors `InputMode.displayNameKey` / `FontType.displayNameKey`).
    let displayNameKey: StringKey
    let light: KeyboardColorSettings?
    let dark: KeyboardColorSettings?

    /// Asset name for the card preview screenshot (sized to match the 齒盤佈局
    /// page's `layout_*_preview` assets). `nil` → fall back to the live color
    /// swatch.
    var previewImageName: String? = nil

    /// Optional key-outline width override. Built-in themes are colors-first, but
    /// a theme may carry this one appearance scalar so the resolver applies it on
    /// top of the factory sizes (used by the 框線 key-style family). `nil` keeps
    /// the factory `keyBorderWidth` (0 = no border).
    var keyBorderWidth: Double? = nil

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
/// `colors` stays OPTIONAL per role (reuses `KeyboardColorSettings`): a `nil`
/// role inherits KeyboardKit's adaptive color, preserving the "customize 2 of 6"
/// behavior. The defaults match `ThemeDefaults`.
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
}
