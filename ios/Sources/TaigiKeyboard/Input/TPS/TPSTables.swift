import Foundation

/// TPS (Taiwanese Phonetic Symbols, 方音符號) mapping tables and membership queries.
///
/// Pure data + detection. No parsing or conversion logic lives here —
/// see `TPSToTL` (TPS→TL), `TLToTPS` (TL→TPS), `TPSInputAdjuster` (key-level auto-adjust).
///
/// Reference: https://github.com/leechunhoe/Tailo-TPS-Converter
enum TPSTables {
    // MARK: - TPS → TL Tables

    /// Initials (聲母) TPS → TL.
    /// Compound initials (ㄐㄧ, ㄑㄧ, etc.) must come first for greedy matching.
    static let consonants: [(tps: String, tl: String)] = [
        // Compound initials (match first)
        ("ㄑㄧ", "tshi"),
        ("ㄐㄧ", "tsi"),
        ("ㄒㄧ", "si"),
        ("ㆢㄧ", "ji"),
        // Standalone palatalized initials (for nasalized-vowel combinations like ㄐㆪ)
        ("ㄐ", "ts"),
        ("ㄑ", "tsh"),
        ("ㄒ", "s"),
        ("ㆢ", "j"),
        // Single initials
        ("ㄅ", "p"),
        ("ㄆ", "ph"),
        ("ㄇ", "m"),
        ("ㆠ", "b"),
        ("ㄉ", "t"),
        ("ㄊ", "th"),
        ("ㄋ", "n"),
        ("ㄌ", "l"),
        ("ㄍ", "k"),
        ("ㄎ", "kh"),
        ("ㄫ", "ng"),
        ("ㆣ", "g"),
        ("ㄏ", "h"),
        ("ㄗ", "ts"),
        ("ㄘ", "tsh"),
        ("ㄙ", "s"),
        ("ㆡ", "j"),
    ]

    /// Finals (韻母) TPS → TL.
    /// Compound finals must come first for greedy matching.
    static let vowels: [(tps: String, tl: String)] = [
        // Nasalized finals (match first)
        ("ㆮ", "ainn"),
        ("ㆯ", "aunn"),
        ("ㆩ", "ann"),
        ("ㆥ", "enn"),
        ("ㆪ", "inn"),
        ("ㆳ", "inn"), // Vertical glyph variant of ㆪ (same symbol, duplicate Unicode encoding)
        ("ㆧ", "onn"),
        ("ㆫ", "unn"),
        // Compound finals
        ("ㄤ", "ang"),
        ("ㆲ", "ong"),
        ("ㆦ", "oo"),
        ("ㄝ", "ee"),
        ("ㄜ", "er"),
        ("ㆨ", "ir"),
        ("ㄞ", "ai"),
        ("ㄠ", "au"),
        ("ㆰ", "am"),
        ("ㆱ", "om"),
        ("ㄢ", "an"),
        ("ㆭ", "ng"),
        // Single finals
        ("ㄚ", "a"),
        ("ㆤ", "e"),
        ("ㄧ", "i"),
        ("ㄛ", "o"),
        ("ㄨ", "u"),
        ("ㄣ", "n"),
        ("ㄥ", "ng"),
        ("ㆬ", "m"),
    ]

    /// Entering-tone finals (入聲韻尾). Each affects both coda and tone number.
    /// Dot (˙) suffix marks tone 8; no dot marks tone 4.
    static let tones: [(tps: String, tl: String, tone: String)] = [
        ("ㆴ˙", "p", "8"),
        ("ㆵ˙", "t", "8"),
        ("ㆻ˙", "k", "8"),
        ("ㆷ˙", "h", "8"),
        ("ㆴ", "p", "4"),
        ("ㆵ", "t", "4"),
        ("ㆻ", "k", "4"),
        ("ㆷ", "h", "4"),
    ]

    /// Non-entering tone marks (TPS tone symbol → TL tone number).
    /// Tone 1 has no symbol.
    static let toneMarks: [(tps: String, tone: String)] = [
        ("ˋ", "2"),
        ("˪", "3"),
        ("ˊ", "5"),
        ("ˇ", "6"),
        ("˫", "7"),
        ("ˆ", "9"),
        ("˙", "8"), // Non-entering tone 8, U+02D9 DOT ABOVE
    ]

    /// Stop consonants excluded from vowel matching (already consumed as checked-tone finals).
    static let stopConsonants: Set<String> = ["p", "t", "k", "h"]

    /// Non-palatalized affricates that cannot be followed by ㄧ.
    /// [ts/tsh/s/j] + [i] must use palatalized compound initials ㄐㄧ/ㄑㄧ/ㄒㄧ/ㆢㄧ instead.
    static let nonPalatalizedAffricates: Set<String> = ["ㄗ", "ㄘ", "ㄙ", "ㆡ"]

    // MARK: - TL → TPS Tables

    /// Initials TL → TPS (longer keys sorted first for greedy matching).
    static let tlToTPSConsonants: [(tl: String, tps: String)] = [
        // Compound initials (match first)
        ("tshi", "ㄑㄧ"),
        ("tsi", "ㄐㄧ"),
        ("tsh", "ㄘ"),
        ("ts", "ㄗ"),
        ("ph", "ㄆ"),
        ("th", "ㄊ"),
        ("kh", "ㄎ"),
        ("ng", "ㄫ"),
        ("si", "ㄒㄧ"),
        ("ji", "ㆢㄧ"),
        // Single initials
        ("p", "ㄅ"),
        ("m", "ㄇ"),
        ("b", "ㆠ"),
        ("t", "ㄉ"),
        ("n", "ㄋ"),
        ("l", "ㄌ"),
        ("k", "ㄍ"),
        ("g", "ㆣ"),
        ("h", "ㄏ"),
        ("s", "ㄙ"),
        ("j", "ㆡ"),
    ]

    /// Finals TL → TPS (longer keys sorted first for greedy matching).
    static let tlToTPSVowels: [(tl: String, tps: String)] = [
        // Nasalized finals (match first)
        ("ainn", "ㆮ"),
        ("aunn", "ㆯ"),
        ("ann", "ㆩ"),
        ("enn", "ㆥ"),
        ("inn", "ㆪ"),
        ("onn", "ㆧ"),
        ("unn", "ㆫ"),
        // Compound finals
        ("ang", "ㄤ"),
        ("ong", "ㆲ"),
        ("oo", "ㆦ"),
        ("ee", "ㄝ"),
        ("er", "ㄜ"),
        ("ir", "ㆨ"),
        ("or", "ㄛ"),
        ("ai", "ㄞ"),
        ("au", "ㄠ"),
        ("am", "ㆰ"),
        ("om", "ㆱ"),
        ("an", "ㄢ"),
        ("ng", "ㆭ"),
        // Single finals
        ("a", "ㄚ"),
        ("e", "ㆤ"),
        ("i", "ㄧ"),
        ("o", "ㄛ"),
        ("u", "ㄨ"),
        ("m", "ㆬ"),
        ("n", "ㄣ"),
    ]

    /// Non-entering tones TL → TPS. Tones 1 and 4 have no symbol.
    static let tlToTPSTones: [(tl: String, tps: String)] = [
        ("2", "ˋ"),
        ("3", "˪"),
        ("5", "ˊ"),
        ("6", "ˇ"),
        ("7", "˫"),
        ("8", "\u{02D9}"),
        ("9", "ˆ"),
    ]

    /// Entering-tone finals TL → TPS.
    static let tlToTPSCheckedTones: [(tl: String, tps: String)] = [
        ("p8", "ㆴ˙"),
        ("t8", "ㆵ˙"),
        ("k8", "ㆻ˙"),
        ("h8", "ㆷ˙"),
        ("p4", "ㆴ"),
        ("t4", "ㆵ"),
        ("k4", "ㆻ"),
        ("h4", "ㆷ"),
    ]

    // MARK: - Membership Queries

    /// All TPS characters (initials + finals + entering-tone finals + tone marks).
    static let tpsCharacters: Set<Character> = {
        var chars = Set<Character>()
        for (tps, _) in consonants {
            chars.formUnion(tps)
        }
        for (tps, _) in vowels {
            chars.formUnion(tps)
        }
        for (tps, _, _) in tones {
            chars.formUnion(tps)
        }
        for (tps, _) in toneMarks {
            chars.formUnion(tps)
        }
        return chars
    }()

    /// Non-entering tone marks only (ˋ ˪ ˊ ˇ ˫ ˙ ˆ).
    /// Does NOT include entering-tone finals (ㆴ ㆵ ㆻ ㆷ).
    /// Used to detect whether a syllable already has an explicit tone specified.
    static let toneMarkCharacters: Set<Character> = {
        var chars = Set<Character>()
        for (tps, _) in toneMarks {
            chars.formUnion(tps)
        }
        return chars
    }()

    /// True if the string contains any TPS character.
    static func containsTPS(_ input: String) -> Bool {
        input.contains { tpsCharacters.contains($0) }
    }

    /// True if the character is a non-entering TPS tone mark (ˋ ˪ ˊ ˇ ˫ ˙ ˆ).
    static func isTPSToneMark(_ char: Character) -> Bool {
        toneMarkCharacters.contains(char)
    }
}
