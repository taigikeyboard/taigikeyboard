import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// High-level phonetics converters.
///
/// Orchestrates `SyllableParser`, `TLFormatter`, and `POJFormatter` to perform:
/// - tone-digit to tone-mark conversion (keyboard composition)
/// - POJ↔TL display text round-trip (cross-script rendering)
enum PhoneticsConverter {
    /// Convert a single syllable (with trailing tone digit) to tone-marked form.
    /// Keyboard-specific: tones 1/4 keep the trailing digit (e.g. "gua1" stays "gua1").
    static func convertSyllable(_ syllable: String, mode: InputMode) -> String {
        guard let last = syllable.last,
              let tone = Int(String(last)),
              (1 ... 9).contains(tone)
        else {
            return syllable
        }

        let baseForm = String(syllable.dropLast())
        guard !baseForm.isEmpty else { return syllable }

        // Tones 1 and 4: keep trailing digit for composing display
        if tone == 1 || tone == 4 {
            return syllable
        }

        // Parse the base form to get initial/final
        let normalized = SyllableParser.normalizeToTL(baseForm.lowercased())
        guard let (initial, final) = SyllableParser.splitInitialFinal(normalized) else {
            return syllable
        }

        let toneStr = String(tone)

        // Assemble with tone marks, preserving original case
        let assembled: String
        switch mode {
        case .poj:
            assembled = POJFormatter.toPOJ(initial: initial, final: final, tone: toneStr)
        case .tl:
            assembled = TLFormatter.toTL(initial: initial, final: final, tone: toneStr)
        case .english, .tps:
            return syllable
        }

        // Restore case: if original starts uppercase, capitalize result
        if let firstOrig = baseForm.first, firstOrig.isUppercase {
            return assembled.prefix(1).uppercased() + assembled.dropFirst()
        }
        return assembled
    }

    /// Convert hyphen-separated input to tone marks.
    /// Splits by "-", converts each syllable, rejoins with "-".
    static func convertToToneMarks(_ input: String, mode: InputMode) -> String {
        let syllables = input.components(separatedBy: "-")
        let converted = syllables.map { syllable in
            syllable.isEmpty ? "" : convertSyllable(syllable, mode: mode)
        }
        return converted.joined(separator: "-")
    }

    /// Convert POJ display text (with diacritics) to TL display text.
    /// Splits by "-" and " " (word boundary); for each syllable: strip tone → normalizeToTL → toTL.
    /// Preserves original separators (space = word boundary, hyphen = syllable boundary).
    static func pojDisplayToTLDisplay(_ text: String) -> String {
        convertDisplay(text, formatter: { initial, final, tone in
            TLFormatter.toTL(initial: initial, final: final, tone: tone)
        })
    }

    /// Convert TL display text (with diacritics) to POJ display text.
    static func tlDisplayToPOJDisplay(_ text: String) -> String {
        convertDisplay(text, formatter: { initial, final, tone in
            POJFormatter.toPOJ(initial: initial, final: final, tone: tone)
        })
    }

    private static func convertDisplay(
        _ text: String,
        formatter: (_ initial: String, _ final: String, _ tone: String) -> String,
    ) -> String {
        guard !text.isEmpty else { return "" }

        // Split while preserving separators (space and hyphen)
        var tokens: [(text: String, separator: String)] = []
        var current = ""
        for char in text {
            if char == "-" || char == " " {
                tokens.append((text: current, separator: String(char)))
                current = ""
            } else {
                current.append(char)
            }
        }
        tokens.append((text: current, separator: ""))

        var result = ""
        for (i, token) in tokens.enumerated() {
            let s = token.text
            if s.isEmpty {
                // Preserve separator (e.g., "--" for 輕聲)
                if i < tokens.count - 1 || !token.separator.isEmpty {
                    result += token.separator
                }
                continue
            }

            let (bare, toneNum) = SyllableParser.stripToneMark(s)
            let converted: String
            if bare.isEmpty {
                converted = s
            } else {
                let normalized = SyllableParser.normalizeToTL(bare.lowercased())
                if let (initial, final) = SyllableParser.splitInitialFinal(normalized) {
                    let tone = toneNum.isEmpty ? (SyllableParser.isStopTone(final) ? "4" : "1") : toneNum
                    let assembled = formatter(initial, final, tone)
                    if let first = s.first, first.isUppercase {
                        converted = assembled.prefix(1).uppercased() + assembled.dropFirst()
                    } else {
                        converted = assembled
                    }
                } else {
                    converted = s
                }
            }

            result += converted
            if !token.separator.isEmpty {
                result += token.separator
            }
        }
        return result
    }
}
