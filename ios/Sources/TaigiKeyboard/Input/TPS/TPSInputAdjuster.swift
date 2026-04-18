import Foundation

/// Key-level TPS input adjustments applied at keystroke time.
///
/// TPS has several context-sensitive keystrokes that a single key cannot represent:
/// - Dual-form nasal/stop consonants (initial vs. final position)
/// - Palatalization auto-correct after affricates
/// - Syllabic nasal auto-correct after tone marks
/// - ㆮ/ㆯ disambiguation after ㄧ
///
/// Each function is a pure query over the raw-input buffer — no side effects.
/// The caller (`CharacterInputPipeline`) orchestrates application order and
/// issues `replaceLastCharacter` calls when needed.

// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.
enum TPSInputAdjuster {
    // MARK: - Palatalization (ㄗ/ㄘ/ㄙ/ㆡ + ㄧ/ㆪ)

    /// Non-palatalized → palatalized affricate.
    private static let palatalizationMap: [Character: String] = [
        "ㄗ": "ㄐ",
        "ㄘ": "ㄑ",
        "ㄙ": "ㄒ",
        "ㆡ": "ㆢ",
    ]

    /// Characters that trigger palatalization of the preceding affricate.
    private static let palatalizationTriggers: Set<Character> = ["ㄧ", "ㆪ"]

    /// When user types ㄧ or ㆪ after a non-palatalized affricate (ㄗ/ㄘ/ㄙ/ㆡ),
    /// the affricate should auto-correct to its palatalized form (ㄐ/ㄑ/ㄒ/ㆢ).
    ///
    /// - Returns: Palatalized replacement for the last char of rawInput, or nil if no replacement needed.
    static func palatalizationReplacement(forIncoming char: String, lastRawChar: Character?) -> String? {
        guard let last = lastRawChar,
              let firstChar = char.first,
              palatalizationTriggers.contains(firstChar)
        else {
            return nil
        }
        return palatalizationMap[last]
    }

    // MARK: - Syllabic nasal (ㄇ/ㄫ + tone mark)

    /// When user types a tone mark after a bare ㄇ or ㄫ at syllable start,
    /// the consonant should retroactively correct to its syllabic form:
    /// ㄇ → ㆬ (syllabic m), ㄫ → ㆭ (syllabic ng).
    ///
    /// - Returns: Syllabic replacement for the last char of rawInput, or nil if no replacement needed.
    static func syllabicNasalReplacement(forIncoming char: String, lastRawChar: Character?) -> String? {
        guard let last = lastRawChar,
              let firstChar = char.first,
              TPSTables.isTPSToneMark(firstChar)
        else {
            return nil
        }
        switch last {
        case "ㄇ": return "ㆬ"
        case "ㄫ": return "ㆭ"
        default: return nil
        }
    }

    // MARK: - Nasal/stop positional auto-select

    /// Keys whose glyph depends on syllable position (initial vs. final).
    private static let dualFormKeys: Set<Character> = [
        "ㄇ", "ㄋ", "ㄫ", // nasals
        "ㄅ", "ㄉ", "ㄍ", "ㄏ", // stops (entering-tone codas)
    ]

    /// Characters that indicate the start of a new syllable
    /// (tone marks, checked-tone finals, nasal finals).
    private static let syllableBoundaryChars: Set<Character> = {
        var chars = TPSTables.toneMarkCharacters
        chars.formUnion(["ㆴ", "ㆵ", "ㆻ", "ㆷ"]) // Checked-tone finals
        chars.formUnion(["ㆬ", "ㄣ", "ㆭ", "ㄥ"]) // Nasal finals
        return chars
    }()

    /// Returns context-adjusted TPS character for keys with dual initial/final forms.
    ///
    /// At syllable start (empty, after tone mark, after checked-tone final, after space) → initial form.
    /// Not at syllable start → final form:
    ///   Nasals: ㄇ→ㆬ, ㄋ→ㄣ, ㄫ→ㄥ (after ㄧ) / ㆭ (otherwise).
    ///   Stops:  ㄅ→ㆴ, ㄉ→ㆵ, ㄍ→ㆻ, ㄏ→ㆷ (entering-tone coda, tone 4 default).
    ///
    /// Only called when `inputMode == .tps`.
    static func adjustInitialKey(_ char: String, afterRawInput raw: String) -> String {
        guard let firstChar = char.first, dualFormKeys.contains(firstChar) else {
            return char
        }

        // At syllable start → keep initial form.
        if raw.isEmpty { return char }
        let lastChar = raw.last!
        if lastChar == " " || syllableBoundaryChars.contains(lastChar) {
            return char
        }

        // Not at syllable start → final form.
        switch char {
        case "ㄇ": return "ㆬ"
        case "ㄋ": return "ㄣ"
        case "ㄅ": return "ㆴ"
        case "ㄉ": return "ㆵ"
        case "ㄍ": return "ㆻ"
        case "ㄏ": return "ㆷ"
        // ㄫ: after ㄧ → ㄥ (ing), otherwise → ㆭ.
        case "ㄫ": return lastChar == "ㄧ" ? "ㄥ" : "ㆭ"
        default: return char
        }
    }

    // MARK: - ㆮ/ㆯ after ㄧ

    /// Auto-correct ㆮ (ainn) → ㆯ (aunn) when preceded by ㄧ.
    /// "iainn" is not a valid Taiwanese final; only "iaunn" exists.
    ///
    /// Only called when `inputMode == .tps`.
    static func adjustNasalizedVowelKey(_ char: String, afterRawInput raw: String) -> String {
        guard char == "ㆮ", let lastChar = raw.last, lastChar == "ㄧ" else {
            return char
        }
        return "ㆯ"
    }
}
