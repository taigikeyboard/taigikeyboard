import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Pure-function derivation of search-key variants from a romanization string.
///
/// These keys back the indexed columns (`notone`, `abbrev`, `roman_num`) of
/// the custom-dictionary table. The same logic runs at write time (inside
/// `CustomDictionaryRepository.bindEntry` and the backfill migrator) and at
/// read time (when `LexiconService` / `DictionarySearchViewModel` build the
/// query key), so both sides must stay exactly in sync — hence a single
/// canonical implementation here rather than any callback into the service.
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

    /// Build the custom-dictionary prefix-search key for `roman`.
    ///
    /// Selects the column strategy by input shape:
    /// - tone-aware (contains a digit) → lowercase, strip `-` / spaces, match `roman_num`.
    /// - toneless → strip diacritics/digits/separators via `generateNotone`, match `notone`.
    static func searchPrefix(for roman: String) -> (key: String, isToneAware: Bool) {
        let isToneAware = roman.contains { $0.isNumber }
        let key: String = if isToneAware {
            roman.lowercased()
                .replacingOccurrences(of: "-", with: "")
                .replacingOccurrences(of: " ", with: "")
        } else {
            generateNotone(roman)
        }
        return (key, isToneAware)
    }

}
