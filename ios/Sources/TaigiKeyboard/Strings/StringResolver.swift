// Resolves i18n StringKeys through native bundles or generated product-language maps (Foundation-only).

import Foundation

/// Resolves a `StringKey` to a display string for one `DisplayLanguage`.
///
/// English/Japanese live in compiled `.lproj` bundles. Hanji/TL/POJ live in generated Swift maps so
/// the archive contains no unsupported Apple locale directory names. Missing values fall back to the
/// complete Hanji map.
struct StringResolver {
    /// The effective (concrete) display language. `DisplayLanguageStore` resolves `.system` to a real
    /// authored language via `effectiveLanguage(_:)` before constructing the resolver, so `.system` must
    /// never reach here — it has no strings and the `language == .english` plural branch would miss.
    let language: DisplayLanguage
    private let activeBundle: Bundle?

    // A value that cannot be a real localized string, so `localizedString(forKey:value:table:)`
    // returning it unambiguously means "this bundle has no entry for the key".
    private static let missSentinel = "\u{0}__i18n_miss__"

    init(_ language: DisplayLanguage) {
        // Fail fast on a boundary regression: `.system` has no authored strings, so it must be resolved
        // to an effective language before here. `assert` (DEBUG-only) mirrors Android's `error(...)` in
        // StringResolver.resolve, but degrades to the Hanji fallback in release rather than crashing the
        // keyboard extension (`.automatic` → activeBundle nil → hanjiDefault for every key).
        assert(language != .system, "StringResolver must be built from an effective language, never .system")
        self.language = language
        if case let .native(bcp47) = language.resolution {
            activeBundle = Self.lprojBundle(bcp47)
        } else {
            activeBundle = nil
        }
    }

    func resolve(_ key: StringKey) -> String {
        if case .generatedMap = language.resolution {
            return GeneratedTaigiStrings.lookup(language, key) ?? hanjiDefault(key)
        }
        let value = activeBundle?.localizedString(forKey: key.rawValue, value: Self.missSentinel, table: nil)
            ?? Self.missSentinel
        return value == Self.missSentinel ? hanjiDefault(key) : value
    }

    /// Resolves a format key's template under the active language and interpolates `args`. The single
    /// entry point for `String(format:)`; the generated typed accessors in StringResolverFormats.swift
    /// are the only callers, so a raw "%1$lld" template never reaches a call site.
    func format(_ key: StringKey, _ args: CVarArg...) -> String {
        String(format: resolve(key), arguments: args)
    }

    /// Interpolates a ready-made positional `template`. Backs the plural-aware generated accessors,
    /// which select each count's plural arm at runtime. The codegen emits a flat catalog string for
    /// every language and the generated display languages have no OS plural locale, so one runtime
    /// arm-selector serves all five languages — a native String Catalog plural would be a second,
    /// English-only mechanism (plan R3-2). The active `language` drives which arm each count selects.
    func formatTemplate(_ template: String, _ args: CVarArg...) -> String {
        String(format: template, arguments: args)
    }

    private func hanjiDefault(_ key: StringKey) -> String {
        GeneratedTaigiStrings.lookup(.hanji, key) ?? key.rawValue
    }

    // A `.lproj` is absent when a language has no authored values yet — a normal state that routes to
    // the Hanji fallback, not an error, so resolution is intentionally silent.
    private static func lprojBundle(_ bcp47: String) -> Bundle? {
        guard let path = Bundle.main.path(forResource: bcp47, ofType: "lproj") else { return nil }
        return Bundle(path: path)
    }
}
