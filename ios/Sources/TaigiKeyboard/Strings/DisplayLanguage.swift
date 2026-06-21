// App UI display-language identity + per-language string-resolution metadata (i18n plan D2/D3).

import Foundation

// Hanji has no OS locale; its strings are the String Catalog's source language. This Taiwanese-Hanji
// BCP-47 names the `.lproj` bundle used for the explicit Hanji fallback. Mirrors the generated
// tools/i18n BCP47_HANJI + the Kotlin BCP47_HANJI.
let BCP47_HANJI = "nan-Hant-TW"

/// App UI display language — orthogonal to the keyboard input mode.
///
/// R2b is behaviour-frozen infra: only `.hanji` carries real strings; every other language falls back
/// to Hanji until its authoring phase populates the catalog (P2 en / P3a ja / P3b TL / P3c POJ).
/// `.pseudo` is a DEBUG-only layout probe, never offered in the production picker.
///
/// `system` (Automatic) is deliberately absent — it is a locale-negotiation policy, not a string set,
/// and is introduced with the picker in P2. The raw value IS the persisted tag.
enum DisplayLanguage: String, CaseIterable {
    case hanji
    case tailo
    case poj
    case japanese = "ja"
    case english = "en"
    case pseudo

    /// Persisted tag (`SharedSettings.displayLanguage`). Equals the raw value; mirrors Android's `.tag`.
    var tag: String { rawValue }

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

    /// Maps a persisted tag to a language. Unknown tags fall back to `.hanji` (anti-crash). A leftover
    /// "pseudo" tag from a DEBUG build resolves to `.hanji` in release so production never renders the
    /// layout-probe strings.
    static func fromTag(_ tag: String) -> DisplayLanguage {
        let match = DisplayLanguage(rawValue: tag) ?? .hanji
        #if DEBUG
        return match
        #else
        return match == .pseudo ? .hanji : match
        #endif
    }
}
