import Foundation

// NOTE: Not shared-core — `preprocessPojInput` reads `SharedSettings.shared`
// directly (oo↔o͘ / nn↔ⁿ toggles). Before extraction, the caller must inject
// the two booleans as parameters so this file becomes Foundation-pure.

private let toneLogger = DebugLogger(category: "ToneConverter")

/// Tone converter
///
/// Coordinates POJ and TL tone conversion through TaigiPhonetics engine.
/// POJ preprocessing (oo→o͘, nn→ⁿ) is handled here since it depends on SharedSettings.
enum ToneConverter {
    /// Convert input to tone marks
    /// - Parameters:
    ///   - input: Input string (may contain multiple hyphen-separated syllables)
    ///   - mode: Input mode (POJ/TL)
    /// - Returns: Converted string
    static func convertToToneMarks(_ input: String, mode: InputMode) -> String {
        let result: String
        switch mode {
        case .poj:
            let preprocessed = preprocessPojInput(input)
            result = TaigiPhonetics.convertToToneMarks(preprocessed, mode: .poj)
        case .tl:
            result = TaigiPhonetics.convertToToneMarks(input, mode: .tl)
        case .english, .tps:
            result = input
        }

        let adjusted = ToneUtilities.adjustNasalMarkerCase(result)

        if input != adjusted {
            toneLogger.debug("[TONE] input='\(input)' mode=\(String(describing: mode)) -> '\(adjusted)'")
        }

        return adjusted
    }

    // MARK: - POJ Preprocessing (moved from POJToneConverter)

    /// Preprocess POJ input based on user settings (oo→o͘, nn→ⁿ)
    private static func preprocessPojInput(_ input: String) -> String {
        var result = input
        let settings = SharedSettings.shared

        if settings.isDoubleTapOOEnabled {
            result = result.replacingOccurrences(of: "oo", with: "o͘")
            result = result.replacingOccurrences(of: "Oo", with: "O͘")
            result = result.replacingOccurrences(of: "OO", with: "O͘")
        }

        if settings.isDoubleTapNNEnabled {
            result = convertNasalDoubleN(result)
        }

        return result
    }

    private static let nasalVowels: Set<Character> = ["a", "e", "i", "o", "u", "A", "E", "I", "O", "U"]

    /// Convert "nn" sequences after vowels to nasal marker "ⁿ"
    private static func convertNasalDoubleN(_ input: String) -> String {
        var result = ""
        let chars = Array(input)
        var i = 0

        while i < chars.count {
            if i + 2 < chars.count,
               nasalVowels.contains(chars[i]),
               chars[i + 1].lowercased() == "n",
               chars[i + 2].lowercased() == "n"
            {
                result += String(chars[i]) + "ⁿ"
                i += 3
            } else {
                result += String(chars[i])
                i += 1
            }
        }

        return result
    }
}
