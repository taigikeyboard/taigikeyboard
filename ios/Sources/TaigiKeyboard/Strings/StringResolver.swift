// Resolves i18n StringKeys for one display language via per-bundle .lproj override (Foundation-only).

import Foundation

/// Resolves a `StringKey` to a display string for one `DisplayLanguage`.
///
/// Each language's strings live in a compiled `.lproj` bundle (the String Catalog emits one per
/// `CFBundleLocalization`, incl. the private-use TL/POJ tags). `Bundle.main` is the running target's
/// bundle — the host `.app` or the keyboard `.appex` — each of which packages the same `.lproj` set.
///
/// Fallback is explicit and Xcode-behaviour-independent: a key the active `.lproj` lacks (a language
/// not yet authored) returns the sentinel, which routes to the always-present Hanji `.lproj`. Hanji is
/// authored for every key, so it is the guaranteed base.
struct StringResolver {
    /// The effective (concrete) display language. `DisplayLanguageStore` resolves `.system` to a real
    /// authored language via `effectiveLanguage(_:)` before constructing the resolver, so `.system` must
    /// never reach here — its `bcp47` is `nil` and the `language == .english` plural branch would miss.
    let language: DisplayLanguage
    private let activeBundle: Bundle?
    private let hanjiBundle: Bundle?

    // A value that cannot be a real localized string, so `localizedString(forKey:value:table:)`
    // returning it unambiguously means "this bundle has no entry for the key".
    private static let missSentinel = "\u{0}__i18n_miss__"

    init(_ language: DisplayLanguage) {
        // Fail fast on a boundary regression: `.system` has no authored strings, so it must be resolved
        // to an effective language before here. `assert` (DEBUG-only) mirrors Android's `error(...)` in
        // StringResolver.resolve, but degrades to the Hanji fallback in release rather than crashing the
        // keyboard extension (`.system.bcp47` is nil → activeBundle nil → hanjiDefault for every key).
        assert(language != .system, "StringResolver must be built from an effective language, never .system")
        self.language = language
        hanjiBundle = Self.lprojBundle(BCP47_HANJI)
        activeBundle = language.bcp47.flatMap(Self.lprojBundle)
    }

    func resolve(_ key: StringKey) -> String {
        #if DEBUG
        if language == .pseudo {
            return GeneratedPseudoStrings.lookup(key) ?? hanjiDefault(key)
        }
        #endif
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
    /// every language and the TL/POJ display languages have no OS plural locale at all, so one runtime
    /// arm-selector serves all five languages — a native String Catalog plural would be a second,
    /// English-only mechanism (plan R3-2). The active `language` drives which arm each count selects.
    func formatTemplate(_ template: String, _ args: CVarArg...) -> String {
        String(format: template, arguments: args)
    }

    private func hanjiDefault(_ key: StringKey) -> String {
        hanjiBundle?.localizedString(forKey: key.rawValue, value: key.rawValue, table: nil) ?? key.rawValue
    }

    // A `.lproj` is absent when a language has no authored values yet — a normal state that routes to
    // the Hanji fallback, not an error, so resolution is intentionally silent.
    private static func lprojBundle(_ bcp47: String) -> Bundle? {
        guard let path = Bundle.main.path(forResource: bcp47, ofType: "lproj") else { return nil }
        return Bundle(path: path)
    }
}
