import Foundation
import OSLog

#if DEBUG
    private let normalizerLogger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "InputNormalizer",
    )
#endif

/// Input normalizer
///
/// Converts user input to numeric tone format (mode-native spelling):
/// - TPS (ㄉㄧㄠˊ) → tiau5
/// - POJ diacritics (hó) → ho2 (stays POJ)
/// - TL diacritics (hóo) → hoo2
/// - POJ numeric (ho2) → ho2 (stays POJ)
/// - TL numeric (hoo2) → hoo2
///
/// Output retains the input mode's spelling. The trie prefix (tl:/poj:)
/// is added at the query boundary, not here.
///
/// Pipeline: TPS conversion → split syllables → per-syllable (diacritics→digits) → join
enum InputNormalizer {
    // MARK: - Public API

    /// Normalize input to Trie query format (TL numeric tones)
    static func normalize(_ input: String, mode _: InputMode) -> String {
        guard !input.isEmpty else { return "" }

        let processedInput = TPSConverter.containsTPS(input)
            ? TPSConverter.toTL(input)
            : input

        let lowercased = processedInput.lowercased()
        let shouldAddDefaultTones = hasToneMarks(lowercased)

        let syllables = lowercased.split(omittingEmptySubsequences: false) { $0 == "-" || $0 == " " }
        let result = syllables.map { syllable in
            normalizeSyllable(String(syllable), addDefaultTone: shouldAddDefaultTones)
        }

        let normalized = result.joined()

        #if DEBUG
            if input != normalized {
                normalizerLogger.debug("[NORMALIZE] input='\(input, privacy: .public)' -> '\(normalized, privacy: .public)'")
            }
        #endif

        return normalized
    }

    /// Check if input contains tone mark diacritics
    static func hasToneMarks(_ input: String) -> Bool {
        let nfd = input.decomposedStringWithCanonicalMapping
        return nfd.unicodeScalars.contains { TaigiPhonetics.combiningToToneNum[$0] != nil }
    }

    // MARK: - Private Methods

    private static let checkedEndings: Set<Character> = ["p", "t", "k", "h"]

    /// Normalize a single syllable: strip diacritics → numeric tone
    private static func normalizeSyllable(_ syllable: String, addDefaultTone: Bool) -> String {
        guard !syllable.isEmpty else { return "" }

        let withNasalConverted = syllable
            .replacingOccurrences(of: "\u{207F}", with: "nn")
            .replacingOccurrences(of: "\u{1D3A}", with: "nn")

        // NFD + convert o͘ (U+0358) → oo before digit check,
        // so "ho͘2" from the POJ keyboard picker normalizes to "hoo2"
        let nfd = withNasalConverted.decomposedStringWithCanonicalMapping
        let withOoConverted = nfd.replacingOccurrences(of: "\u{0358}", with: "o")

        if let lastChar = withOoConverted.last, lastChar.isNumber {
            return withOoConverted
        }

        var toneNumber = ""
        var withoutTone = ""

        for scalar in withOoConverted.unicodeScalars {
            if let tone = TaigiPhonetics.combiningToToneNum[scalar] {
                toneNumber = tone
            } else {
                withoutTone.append(String(scalar))
            }
        }

        if addDefaultTone, toneNumber.isEmpty, let lastChar = withoutTone.last {
            if checkedEndings.contains(lastChar) {
                toneNumber = "4"
            } else {
                toneNumber = "1"
            }
        }

        return withoutTone + toneNumber
    }
}
