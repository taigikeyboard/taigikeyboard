// App UI display-language identity + per-language string-resolution metadata (i18n plan D2/D3).

import Foundation

// Hanji has no OS locale; its strings are the String Catalog's source language. This Taiwanese-Hanji
// BCP-47 names the `.lproj` bundle used for the explicit Hanji fallback. Mirrors the generated
// tools/i18n BCP47_HANJI + the Kotlin BCP47_HANJI.
let BCP47_HANJI = "nan-Hant-TW"

/// App UI display language — orthogonal to the keyboard input mode.
///
/// Hanji + English are authored and user-selectable (`productionLanguages`); every other language falls
/// back to Hanji until its authoring phase populates the catalog (P3a ja / P3b TL / P3c POJ) and it joins
/// `productionLanguages`. `.pseudo` is a DEBUG-only layout probe, offered only in debug builds.
///
/// `system` (Automatic) is deliberately absent — it is a locale-negotiation policy, not a string set,
/// deferred to a later round. The raw value IS the persisted tag.
enum DisplayLanguage: String, CaseIterable {
    case hanji
    case tailo
    case poj
    case japanese = "ja"
    case english = "en"
    case pseudo

    /// Persisted tag (`SharedSettings.displayLanguage`). Equals the raw value; mirrors Android's `.tag`.
    var tag: String { rawValue }

    /// The language's own name in its own script (endonym), shown in the picker regardless of the
    /// current UI language — the W3C-recommended convention, so a user can always find their language.
    /// Language-invariant, so it is NOT an i18n key. The endonym strings MUST match across platforms.
    /// CROSS-PLATFORM INVARIANT (INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER) — mirrors
    /// android .../i18n/DisplayLanguage.kt:58 `endonym`. Drift causes silent divergence.
    var endonym: String {
        switch self {
        case .hanji: "漢字"
        case .tailo: "Tâi-lô"
        case .poj: "Pe̍h-ōe-jī"
        case .japanese: "日本語"
        case .english: "English"
        case .pseudo: "PSEUDO · DEBUG"
        }
    }

    /// BCP-47 tag naming the compiled `.lproj` bundle that holds this language's strings. `nil` for
    /// `.pseudo`, which is a generated Swift map (not a CFBundleLocalization, so it has no `.lproj`).
    ///
    /// MIRROR: must equal `tools/i18n/i18n_lib.py` `LANG_TO_BCP47` — the codegen emits each catalog
    /// localization under this exact tag; drift silently breaks `.lproj` resolution.
    var bcp47: String? {
        switch self {
        case .hanji: BCP47_HANJI
        case .tailo: "nan-Latn-TW-x-tailo"
        case .poj: "nan-Latn-TW-x-poj"
        case .japanese: "ja"
        case .english: "en"
        case .pseudo: nil
        }
    }

    /// Default tag persisted before the user ever picks a language. Keeps the app on Hanji.
    static let defaultTag = "hanji"

    /// Authored, user-selectable production languages. Drives the Settings language picker and clamps
    /// `fromTag`. Grows by one entry as each language's authoring phase lands (P3a ja / P3b TL / P3c POJ).
    /// CROSS-PLATFORM INVARIANT (INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER) — mirrors
    /// android .../i18n/DisplayLanguage.kt:79 `productionLanguages`. Drift causes silent divergence.
    static let productionLanguages: [DisplayLanguage] = [.hanji, .english]

    /// What the picker offers: the production roster, plus the `.pseudo` layout probe in DEBUG only.
    static var selectableLanguages: [DisplayLanguage] {
        #if DEBUG
        productionLanguages + [.pseudo]
        #else
        productionLanguages
        #endif
    }

    /// Maps a persisted tag to a language, clamped to the currently-selectable set: an unknown tag or one
    /// whose language is not yet user-selectable (a leftover `.pseudo` in release, or a `ja`/`tl`/`poj`
    /// tag from a future build) resolves to `.hanji`, so the effective language always matches a picker
    /// option. The persisted tag itself is left untouched, so it restores once that language ships.
    static func fromTag(_ tag: String) -> DisplayLanguage {
        let match = DisplayLanguage(rawValue: tag) ?? .hanji
        return selectableLanguages.contains(match) ? match : .hanji
    }
}
