import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.
// POJ preprocessing toggles are passed in explicitly (ToneToggles) — the caller
// reads them from `EngineSettingsProvider.current`, keeping this file free of
// singletons / platform deps.

/// Tone converter
///
/// Coordinates POJ and TL tone conversion through TaigiPhonetics engine.
/// POJ preprocessing (oo→o͘, nn→ⁿ) runs first when the caller has the
/// matching toggles enabled.
enum ToneConverter {
    private static var logger: LoggerBackend {
        LoggerFactory.make(category: "ToneConverter")
    }

    /// Convert input to tone marks.
    /// - Parameters:
    ///   - input: Input string (may contain multiple hyphen-separated syllables)
    ///   - mode: Input mode (POJ/TL)
    ///   - toneToggles: POJ preprocessing toggles (ignored for non-POJ modes)
    static func convertToToneMarks(
        _ input: String,
        mode: InputMode,
        toneToggles: ToneToggles,
    ) -> String {
        let result: String
        switch mode {
        case .poj:
            let preprocessed = preprocessPojInput(input, toggles: toneToggles)
            result = TaigiPhonetics.convertToToneMarks(preprocessed, mode: .poj)
        case .tl:
            result = TaigiPhonetics.convertToToneMarks(input, mode: .tl)
        case .english, .tps:
            result = input
        }

        let adjusted = ToneUtilities.adjustNasalMarkerCase(result)

        if input != adjusted {
            logger.debug("[TONE] input='\(input)' mode=\(String(describing: mode)) -> '\(adjusted)'")
        }

        return adjusted
    }

    // MARK: - POJ Preprocessing (moved from POJToneConverter)

    /// Preprocess POJ input per supplied toggles (oo→o͘, nn→ⁿ).
    private static func preprocessPojInput(_ input: String, toggles: ToneToggles) -> String {
        var result = input

        if toggles.isDoubleTapOOEnabled {
            result = result.replacingOccurrences(of: "oo", with: "o͘")
            result = result.replacingOccurrences(of: "Oo", with: "O͘")
            result = result.replacingOccurrences(of: "OO", with: "O͘")
        }

        if toggles.isDoubleTapNNEnabled {
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
