import Foundation

/// Thin facade over the split TPS modules, kept to avoid churning ~180 existing
/// test call sites. Production code should prefer the underlying modules directly:
///
/// - `TPSTables` — mapping tables + membership queries (`containsTPS`, `isTPSToneMark`)
/// - `TPSToTL` — TPS → TL parser (`convert`, `convertMultiSyllable`)
/// - `TLToTPS` — TL → TPS parser (`convert`, `convertFromDisplay`)
/// - `TPSInputAdjuster` — key-level auto-adjust (positional, palatalization, syllabic nasal)
///
/// TODO: migrate `TPSConverterTests` to the new modules and delete this file.
enum TPSConverter {
    // MARK: - Detection (→ TPSTables)

    static func containsTPS(_ input: String) -> Bool {
        TPSTables.containsTPS(input)
    }

    static func isTPSToneMark(_ char: Character) -> Bool {
        TPSTables.isTPSToneMark(char)
    }

    // MARK: - TPS → TL (→ TPSToTL)

    static func toTL(_ tps: String) -> String {
        TPSToTL.convert(tps)
    }

    static func toTLMultiSyllable(_ tps: String) -> String {
        TPSToTL.convertMultiSyllable(tps)
    }

    // MARK: - TL → TPS (→ TLToTPS)

    static func toTPS(_ tl: String, orMapsToER: Bool = false) -> String {
        TLToTPS.convert(tl, orMapsToER: orMapsToER)
    }

    static func toTPSFromDisplay(_ displayRoman: String, orMapsToER: Bool = false) -> String {
        TLToTPS.convertFromDisplay(displayRoman, orMapsToER: orMapsToER)
    }

    // MARK: - Key-level adjustments (→ TPSInputAdjuster)

    static func palatalizationReplacement(forIncoming char: String, lastRawChar: Character?) -> String? {
        TPSInputAdjuster.palatalizationReplacement(forIncoming: char, lastRawChar: lastRawChar)
    }

    static func syllabicNasalReplacement(forIncoming char: String, lastRawChar: Character?) -> String? {
        TPSInputAdjuster.syllabicNasalReplacement(forIncoming: char, lastRawChar: lastRawChar)
    }

    static func adjustTPSInitialKey(_ char: String, afterRawInput raw: String) -> String {
        TPSInputAdjuster.adjustInitialKey(char, afterRawInput: raw)
    }

    static func adjustTPSNasalizedVowelKey(_ char: String, afterRawInput raw: String) -> String {
        TPSInputAdjuster.adjustNasalizedVowelKey(char, afterRawInput: raw)
    }
}
