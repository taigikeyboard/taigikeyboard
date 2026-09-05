import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Façade over the Rust engine's custom-dictionary search-key derivation.
///
/// Two families of keys:
/// - **Legacy** `notone` / `abbrev` / `roman_num` — still written to the
///   `custom_dictionary` columns for backcompat / rollback (write-only now;
///   the live query path is the side table).
/// - **Cross-mode** (v3.6.1 R3) — `searchKeys(for:)` produces the full
///   {family × form} bundle written to the `custom_search_key` side table at
///   write time; `queryKey(for:mode:)` produces the single family-native key
///   matched against it at read time. Both delegate to the engine so write
///   and read derivation stay byte-identical.
enum CustomDictionaryDerivation {
    /// Toneless form — Rust `Method::DeriveNotone` strips tone diacritics + digits
    /// + hyphens + spaces after lowercase + nasal-marker conversion.
    static func generateNotone(_ roman: String) -> String {
        RustEngineBridge.deriveNotone(roman)
    }

    /// Abbreviation form — Rust `Method::DeriveAbbrev` returns first char per
    /// syllable (split by ASCII whitespace + hyphen), diacritics stripped.
    /// Returns "" when fewer than 2 syllables. Whitespace canonical
    /// `[ \t\n\x0B\f\r-]+` matches Android JVM `Regex("[\\s-]+")` (Codex v3 §1).
    static func generateAbbrev(_ roman: String) -> String {
        RustEngineBridge.deriveAbbrev(roman)
    }

    /// Numeric-toned form for tone-aware search — same as `Method::NormalizeInput`.
    static func generateRomanNum(_ roman: String) -> String {
        RustEngineBridge.normalizeInput(roman)
    }

    /// Full cross-mode search-key bundle for a stored custom-dict roman
    /// (v3.6.1 R3 WRITE side). Materialized into the `custom_search_key` side
    /// table so a query in any input mode finds the entry. See
    /// `RustEngineBridge.deriveCustomSearchKeys`.
    static func searchKeys(for roman: String) -> [CustomSearchKey] {
        RustEngineBridge.deriveCustomSearchKeys(roman)
    }

    /// Single family-native query key for the current `input` + `mode`
    /// (v3.6.1 R3 READ side). The engine upgrades the effective family to TPS
    /// when the raw input carries Bopomofo, so callers pass their settings
    /// mode verbatim. `nil` for residue-only / empty input. See
    /// `RustEngineBridge.deriveCustomQueryKey`.
    static func queryKey(for input: String, mode: InputMode) -> CustomSearchKey? {
        RustEngineBridge.deriveCustomQueryKey(input, mode: mode)
    }
}
