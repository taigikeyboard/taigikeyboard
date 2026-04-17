import Foundation

/// Builds external dictionary lookup URLs from Taiwanese TL forms.
///
/// Third-party dictionaries (Chhoe Taigi, MOE) expect *digit-toned* TL
/// syllables (e.g. `tai7-tsi3`), while our search results carry the
/// diacritic display form (`tāi-tsì`). This helper encapsulates that
/// conversion and percent-encoding so the lookup call sites stay trivial.
enum ExternalLookupURLBuilder {
    /// Chhoe Taigi dictionary lookup URL for the given TL display form.
    static func chhoeURL(forTL tl: String) -> URL? {
        guard let encoded = encodedTLDigit(tl) else { return nil }
        return URL(string: "https://chhoe.taigi.info/s?s=su&f=e&lmjf=ki&lmj=\(encoded)")
    }

    /// MOE Sutian dictionary lookup URL for the given TL display form.
    static func moeURL(forTL tl: String) -> URL? {
        guard let encoded = encodedTLDigit(tl) else { return nil }
        return URL(string: "https://sutian.moe.edu.tw/zh-hant/tshiau/?lui=tai_su&tsha=\(encoded)")
    }

    /// Convert TL display form (diacritics) to TL digit form.
    /// e.g. `"tāi-tsì"` → `"tai7-tsi3"`.
    static func toTLDigit(_ tl: String) -> String {
        let syllables = tl.lowercased().split(separator: "-", omittingEmptySubsequences: false)
        return syllables
            .map { normalizeSyllableToDigit(String($0)) }
            .joined(separator: "-")
    }

    // MARK: - Private

    private static func encodedTLDigit(_ tl: String) -> String? {
        let digit = toTLDigit(tl)
        guard !digit.isEmpty else { return nil }
        return digit.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed)
    }

    /// Normalize a single syllable from diacritics to digit tone.
    ///
    /// External dictionary URLs omit tones 1 (open) and 4 (checked), matching
    /// the query convention used by both Chhoe Taigi and MOE Sutian.
    private static func normalizeSyllableToDigit(_ syllable: String) -> String {
        guard !syllable.isEmpty else { return "" }

        // Quick path: already-digit-toned input keeps the digit (or strips
        // tone 1 / 4 for external dictionary URL semantics). Note: we
        // intentionally check the digit on the *raw* input — only the
        // nasal-marker substitution matters for the digit-strip branch,
        // and the full preprocessing happens below for the diacritic path.
        let withNasalConverted = syllable
            .replacingOccurrences(of: "\u{207F}", with: "nn")
            .replacingOccurrences(of: "\u{1D3A}", with: "nn")
        if let lastChar = withNasalConverted.last, lastChar.isNumber {
            let tone = String(lastChar)
            if tone == "1" || tone == "4" {
                return String(withNasalConverted.dropLast())
            }
            return withNasalConverted
        }

        let withOoConverted = TaigiUnicode.nfdPreprocessed(syllable)

        var toneNumber = ""
        var withoutTone = ""

        for scalar in withOoConverted.unicodeScalars {
            if let tone = TaigiPhonetics.combiningToToneNum[scalar] {
                toneNumber = tone
            } else {
                withoutTone.append(String(scalar))
            }
        }

        // Tone 1 (open) and 4 (checked) are omitted in external dictionary URLs.
        if toneNumber.isEmpty || toneNumber == "1" || toneNumber == "4" {
            return withoutTone
        }
        return withoutTone + toneNumber
    }
}
