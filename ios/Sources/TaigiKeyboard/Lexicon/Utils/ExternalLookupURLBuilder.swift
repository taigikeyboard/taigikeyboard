// 中文: 把 TL 顯示形(帶調符)轉成數字調並組成外部辭典查詢 URL。

import Foundation

/// Builds external dictionary lookup URLs from Taiwanese TL forms.
///
/// Third-party dictionaries (Chhoe Taigi, MOE) expect *digit-toned* TL
/// syllables (e.g. `tai7-tsi3`), while our search results carry the
/// diacritic display form (`tāi-tsì`). This helper encapsulates that
/// conversion and percent-encoding so the lookup call sites stay trivial.
///
/// Platform-owned by design — URL conventions (which tones to drop, which
/// query parameters each dictionary takes, percent-encoding rules) are
/// not phonetics; they belong with the platform that knows about
/// `URLQueryAllowed` / `URLEncoder`. The phonetic prep step delegates to
/// `RustEngineBridge.nfdPreprocessForLookup` and tone-strip to
/// `RustEngineBridge.stripTone`. Mirror at
/// `android/.../ime/dictionary/ExternalLookupURLBuilder.kt`.
// 中文: URL 慣例屬於平台層(percent-encoding / query 參數),所以這裡刻意不搬到 Rust;
// 中文: 只有調符前處理與 strip-tone 走 RustEngineBridge。
enum ExternalLookupURLBuilder {
    /// Chhoe Taigi dictionary lookup URL for the given TL display form.
    // 中文: Chhoe Taigi 辭典查詢 URL。
    static func chhoeURL(forTL tl: String) -> URL? {
        guard let encoded = encodedTLDigit(tl) else { return nil }
        return URL(string: "https://chhoe.taigi.info/s?s=su&f=e&lmjf=ki&lmj=\(encoded)")
    }

    /// MOE Sutian dictionary lookup URL for the given TL display form.
    // 中文: 教育部臺語辭典(Sutian)查詢 URL。
    static func moeURL(forTL tl: String) -> URL? {
        guard let encoded = encodedTLDigit(tl) else { return nil }
        return URL(string: "https://sutian.moe.edu.tw/zh-hant/tshiau/?lui=tai_su&tsha=\(encoded)")
    }

    /// Convert TL display form (diacritics) to TL digit form.
    /// e.g. `"tāi-tsì"` → `"tai7-tsi3"`.
    // 中文: 把 TL 顯示形轉成數字調形,例:tāi-tsì → tai7-tsi3。
    static func toTLDigit(_ tl: String) -> String {
        let syllables = tl.lowercased().split(separator: "-", omittingEmptySubsequences: false)
        return syllables
            .map { normalizeSyllableToDigit(String($0)) }
            .joined(separator: "-")
    }

    private static func encodedTLDigit(_ tl: String) -> String? {
        let digit = toTLDigit(tl)
        guard !digit.isEmpty else { return nil }
        return digit.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed)
    }

    /// Normalize a single syllable from diacritics to digit tone.
    ///
    /// External dictionary URLs omit tones 1 (open) and 4 (checked), matching
    /// the query convention used by both Chhoe Taigi and MOE Sutian.
    // 中文: 單音節 diacritic → 數字調轉換。1 / 4 聲在 URL 中省略,
    // 中文: 與 Chhoe Taigi、MOE Sutian 的 query 慣例一致。
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

        // Diacritic path: apply Taigi preprocessing (nasal / o͘ normalization)
        // then reuse the shared tone-stripping helper so all call sites share
        // one implementation.
        let preprocessed = RustEngineBridge.nfdPreprocessForLookup(syllable)
        let stripped = RustEngineBridge.stripTone(preprocessed)
        let bare = stripped.bare
        let tone = stripped.tone

        // Tone 1 (open) and 4 (checked) are omitted in external dictionary URLs.
        if tone.isEmpty || tone == "1" || tone == "4" {
            return bare
        }
        return bare + tone
    }
}
