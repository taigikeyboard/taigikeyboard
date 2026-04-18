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
    /// Toneless form — strips tone diacritics (via NFD), trailing digits,
    /// hyphens, and spaces. Used for toneless prefix search.
    static func generateNotone(_ roman: String) -> String {
        // POJ nasal markers ⁿ (U+207F) / ᴺ (U+1D3A) → nn
        let withNasalConverted = roman.lowercased()
            .replacingOccurrences(of: "\u{207F}", with: "nn")
            .replacingOccurrences(of: "\u{1D3A}", with: "nn")
        let decomposed = withNasalConverted.decomposedStringWithCanonicalMapping
        var result = ""
        for scalar in decomposed.unicodeScalars {
            if scalar.properties.generalCategory == .nonspacingMark { continue }
            if scalar.value >= 0x30 && scalar.value <= 0x39 { continue }
            if scalar == "-" || scalar == " " { continue }
            result.unicodeScalars.append(scalar)
        }
        return result.precomposedStringWithCanonicalMapping
    }

    /// Abbreviation form — first letter of each syllable (split by `-` or space),
    /// diacritics stripped. Empty string when fewer than two syllables.
    static func generateAbbrev(_ roman: String) -> String {
        let syllables = roman.lowercased()
            .components(separatedBy: CharacterSet(charactersIn: "- "))
            .filter { !$0.isEmpty }
        guard syllables.count >= 2 else { return "" }
        return syllables.map { syllable in
            let first = String(syllable.prefix(1))
            let decomposed = first.decomposedStringWithCanonicalMapping
            var bare = ""
            for scalar in decomposed.unicodeScalars {
                if scalar.properties.generalCategory != .nonspacingMark {
                    bare.unicodeScalars.append(scalar)
                }
            }
            return bare.precomposedStringWithCanonicalMapping
        }.joined()
    }

    /// Numeric-toned form for tone-aware search — diacritics converted to
    /// tone digits, hyphens removed. Example: `"gâu-tsá" → "gau5tsa2"`.
    static func generateRomanNum(_ roman: String) -> String {
        InputNormalizer.normalize(roman, mode: .tl)
    }
}
