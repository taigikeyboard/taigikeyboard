import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Syllable parser: strip tones, normalize spelling, split initial/final, parse syllable.
///
/// Ported from `references/taigi-converter/src/phonetics.js`.
enum SyllableParser {
    /// Strip tone mark from text, returning (bare NFC text, tone number string).
    /// Recognizes both combining diacritics and trailing digit.
    static func stripToneMark(_ text: String) -> (bare: String, tone: String) {
        let decomposed = text.decomposedStringWithCanonicalMapping

        if let mark = decomposed.unicodeScalars.first(where: PhoneticsTables.combiningScalars.contains) {
            let toneNum = PhoneticsTables.combiningToToneNum[mark] ?? ""
            let bare = decomposed.unicodeScalars.filter { $0 != mark }
            let bareStr = String(String.UnicodeScalarView(bare))
            return (bareStr.precomposedStringWithCanonicalMapping, toneNum)
        }

        // No combining mark — check trailing digit
        if let last = text.last, let digit = Int(String(last)), (1 ... 9).contains(digit) {
            let bare = String(text.dropLast())
            return (bare.precomposedStringWithCanonicalMapping, String(digit))
        }

        return (text.precomposedStringWithCanonicalMapping, "")
    }

    /// POJ→TL spelling substitutions applied by `normalizeToTL`. Order matters.
    private static let pojToTLSubstitutions: [(from: String, to: String)] = [
        ("ch", "ts"),
        ("ou", "oo"),
        ("o\u{0358}", "oo"),
        ("\u{207F}", "nn"),
        ("\u{1D3A}", "nn"),
        ("oa", "ua"),
        ("oe", "ue"),
        ("eng", "ing"),
        ("ek", "ik"),
        ("oonn", "onn"),
    ]

    /// Normalize text to TL spelling (lowercase). Replaces POJ conventions with TL equivalents.
    static func normalizeToTL(_ text: String) -> String {
        pojToTLSubstitutions.reduce(text) { acc, rule in
            acc.replacingOccurrences(of: rule.from, with: rule.to)
        }
    }

    /// Check if a final ends in a stop consonant (p, t, k, h), ignoring trailing nn.
    static func isStopTone(_ final: String) -> Bool {
        let cleaned = final.lowercased().replacingOccurrences(of: "nn", with: "")
        return cleaned.last.map { "ptkh".contains($0) } ?? false
    }

    /// Split bare TL text into (initial, final) by iterating prefixes.
    static func splitInitialFinal(_ text: String) -> (initial: String, final: String)? {
        for i in 0 ... text.count {
            let initial = String(text.prefix(i))
            if PhoneticsTables.tlInitials.contains(initial) {
                let final = String(text.dropFirst(i))
                if PhoneticsTables.tlFinals.contains(final) {
                    return (initial, final)
                }
            }
        }
        return nil
    }

    /// Parse a syllable (with tone marks or trailing digit) into (initial, final, tone).
    /// Returns nil if the syllable cannot be parsed.
    static func parseSyllable(_ text: String) -> (initial: String, final: String, tone: String)? {
        let (bare, tone) = stripToneMark(text)
        let normalized = normalizeToTL(bare.lowercased())
        guard let (initial, final) = splitInitialFinal(normalized) else { return nil }
        let finalTone = tone.isEmpty ? (isStopTone(final) ? "4" : "1") : tone
        return (initial, final, finalTone)
    }
}
