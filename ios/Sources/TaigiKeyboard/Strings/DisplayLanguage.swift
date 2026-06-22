// App UI display-language identity + per-language string-resolution metadata (i18n plan D2/D3).

import Foundation

// Hanji has no OS locale; its strings are the String Catalog's source language. This Taiwanese-Hanji
// BCP-47 names the `.lproj` bundle used for the explicit Hanji fallback. Mirrors the generated
// tools/i18n BCP47_HANJI + the Kotlin BCP47_HANJI.
let BCP47_HANJI = "nan-Hant-TW"

/// App UI display language — orthogonal to the keyboard input mode.
///
/// Hanji, English, and Japanese are authored and user-selectable (`productionLanguages`); the remaining
/// languages fall back to Hanji until their authoring phase populates the catalog (P3b TL / P3c POJ) and
/// they join `productionLanguages`. `.pseudo` is a DEBUG-only layout probe, offered only in debug builds.
///
/// `system` (Automatic) is a selection policy, not a string set: it has NO authored strings and never
/// reaches the resolver. The picker boundary maps it to a concrete language via `effectiveLanguage(_:)`
/// (driven by the device OS locale) before any string lookup. It IS a persisted selection — the user can
/// return to it, and the raw value `"system"` is the persisted tag.
enum DisplayLanguage: String, CaseIterable {
    case system
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
    /// `.system` has no endonym — it is a policy, not a language, so the picker special-cases it and
    /// labels it with the localized `settingsDisplayLanguageAutomatic` string instead.
    /// CROSS-PLATFORM INVARIANT (INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER) — mirrors
    /// android .../i18n/DisplayLanguage.kt `endonym`. Drift causes silent divergence.
    var endonym: String {
        switch self {
        case .hanji: "漢字"
        case .tailo: "Tâi-lô"
        case .poj: "Pe̍h-ōe-jī"
        case .japanese: "日本語"
        case .english: "English"
        case .pseudo: "PSEUDO · DEBUG"
        case .system: fatalError("system has no endonym; use settings.displayLanguageAutomatic")
        }
    }

    /// BCP-47 tag naming the compiled `.lproj` bundle that holds this language's strings. `nil` for
    /// `.pseudo`, which is a generated Swift map (not a CFBundleLocalization, so it has no `.lproj`),
    /// and `nil` for `.system`, which has no authored bundle: `nil` means "no authored lproj — system
    /// must be resolved to an effective language before the resolver; it must never be passed to
    /// `lprojBundle` directly".
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
        case .system: nil
        }
    }

    /// Default tag persisted before the user ever picks a language. Keeps the app on Hanji.
    static let defaultTag = "hanji"

    /// Authored, user-selectable production languages — the catalog roster, SEPARATE from
    /// `selectableLanguages` (which leads with `.system`). `.system` is a resolution policy with no
    /// authored strings, so it never appears here. Drives the per-language string catalog and clamps
    /// `fromTag`. Grows by one entry as each language's authoring phase lands (P3b TL / P3c POJ remain).
    /// CROSS-PLATFORM INVARIANT (INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER) — mirrors
    /// android .../i18n/DisplayLanguage.kt `productionLanguages`. Drift causes silent divergence.
    static let productionLanguages: [DisplayLanguage] = [.hanji, .english, .japanese]

    /// What the picker offers: `.system` (Automatic) first, then the production roster, plus DEBUG-only
    /// previews — `.tailo` (authored but not yet a production language; debug-selectable so a debug build
    /// can dogfood its strings + rendering before it joins `productionLanguages`) and the `.pseudo` layout
    /// probe. Release builds only ever offer `.system + productionLanguages`.
    /// CROSS-PLATFORM INVARIANT — mirrors android .../i18n/DisplayLanguage.kt `selectableLanguages`.
    static var selectableLanguages: [DisplayLanguage] {
        #if DEBUG
        [.system] + productionLanguages + [.tailo, .pseudo]
        #else
        [.system] + productionLanguages
        #endif
    }

    /// Resolves the Automatic policy to a concrete authored language from the device OS language subtag
    /// (lowercased ISO 639). Pure + injectable for tests — never reads `Locale` itself; the store passes
    /// the device subtag in. `ja*` → Japanese, `zh*` → Hanji, anything else (incl. absent) → English.
    /// CROSS-PLATFORM INVARIANT — mirrors android .../i18n/DisplayLanguage.kt `resolveAutomatic`.
    static func resolveAutomatic(_ deviceLanguageSubtag: String) -> DisplayLanguage {
        if deviceLanguageSubtag.hasPrefix("ja") { return .japanese }
        if deviceLanguageSubtag.hasPrefix("zh") { return .hanji }
        return .english
    }

    /// The concrete language this selection resolves to: `.system` defers to the device locale via
    /// `resolveAutomatic`; every explicit language resolves to itself. The resolver / `.lproj` lookup
    /// always run against this effective language, never against `.system`.
    /// CROSS-PLATFORM INVARIANT — mirrors android .../i18n/DisplayLanguage.kt `effectiveLanguage`.
    func effectiveLanguage(_ deviceLanguageSubtag: String) -> DisplayLanguage {
        self == .system ? DisplayLanguage.resolveAutomatic(deviceLanguageSubtag) : self
    }

    /// Maps a persisted tag to a language, clamped to the currently-selectable set: an unknown tag or one
    /// whose language is not user-selectable in this build (a `.pseudo`/`.tailo` preview in release, or an
    /// unauthored `poj` tag) resolves to `.hanji`, so the effective language always matches a picker
    /// option. `"system"` is selectable, so it round-trips to `.system`. The persisted tag itself is left
    /// untouched, so it restores once a clamped language ships.
    static func fromTag(_ tag: String) -> DisplayLanguage {
        let match = DisplayLanguage(rawValue: tag) ?? .hanji
        return selectableLanguages.contains(match) ? match : .hanji
    }
}
