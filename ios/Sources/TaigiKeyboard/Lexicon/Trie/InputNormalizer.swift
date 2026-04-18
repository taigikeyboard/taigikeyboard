import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

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
    private static var logger: LoggerBackend {
        LoggerFactory.make(category: "InputNormalizer")
    }

    // MARK: - Public API

    /// Normalize input to Trie query format (TL numeric tones)
    static func normalize(_ input: String, mode _: InputMode) -> String {
        guard !input.isEmpty else { return "" }

        let processedInput = TPSTables.containsTPS(input)
            ? TPSToTL.convert(input)
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
                logger.debug("[NORMALIZE] input='\(input)' -> '\(normalized)'")
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

        // Nasal + NFD + o͘→o (so "ho͘2" from the POJ picker normalizes to "hoo2"
        // BEFORE the digit-tone check below).
        let withOoConverted = TaigiUnicode.nfdPreprocessed(syllable)

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
