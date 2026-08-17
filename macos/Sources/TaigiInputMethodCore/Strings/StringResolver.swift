// Resolves i18n StringKeys to display text for one effective display language.

import Foundation

/// Resolves a `StringKey` to a display string for one `DisplayLanguage`.
///
/// Every production language is a generated map here: the package ships no string catalog and no
/// `.lproj` bundles, so English and Japanese are looked up exactly the way Hanji/TL/POJ are. A value
/// missing for the active language falls back to the complete Hanji map, and a key missing from that
/// too renders its own name rather than an empty string — a visible key beats a blank control.
struct StringResolver {
    /// The effective (concrete) display language. `DisplayLanguageStore` resolves `.system` to a real
    /// authored language via `effectiveLanguage(_:)` before constructing the resolver, so `.system` must
    /// never reach here — it has no strings and the `language == .english` plural branch would miss.
    let language: DisplayLanguage

    init(_ language: DisplayLanguage) {
        // Fail fast on a boundary regression, mirroring iOS StringResolver.swift:26: `assert` is
        // DEBUG-only, so release degrades to the Hanji fallback (`.system` → lookup nil → hanjiDefault)
        // rather than crashing the input method out from under whatever the user is typing into.
        assert(language != .system, "StringResolver must be built from an effective language, never .system")
        self.language = language
    }

    func resolve(_ key: StringKey) -> String {
        GeneratedStrings.lookup(language, key) ?? hanjiDefault(key)
    }

    /// Resolves a format key's template under the active language and interpolates `args`. The single
    /// entry point for `String(format:)`; the generated typed accessors in StringResolverFormats.swift
    /// are the only callers, so a raw "%1$lld" template never reaches a call site.
    func format(_ key: StringKey, _ args: CVarArg...) -> String {
        String(format: resolve(key), arguments: args)
    }

    /// Interpolates a ready-made positional `template`. Backs the plural-aware generated accessors,
    /// which select each count's plural arm at runtime — no display language here has an OS plural
    /// locale, so one runtime arm-selector serves all five.
    func formatTemplate(_ template: String, _ args: CVarArg...) -> String {
        String(format: template, arguments: args)
    }

    private func hanjiDefault(_ key: StringKey) -> String {
        GeneratedStrings.lookup(.hanji, key) ?? key.rawValue
    }
}
