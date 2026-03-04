import Foundation
import OSLog

#if DEBUG
private let toneLogger = Logger(
    subsystem: LexiconConstants.Logging.subsystem,
    category: "ToneConverter"
)
#endif

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
        case .english:
            result = input
        }

        let adjusted = ToneUtilities.adjustNasalMarkerCase(result)

        #if DEBUG
        if input != adjusted {
            toneLogger.debug("[TONE] input='\(input, privacy: .public)' mode=\(String(describing: mode), privacy: .public) -> '\(adjusted, privacy: .public)'")
        }
        #endif

        return adjusted
    }

    // MARK: - POJ Preprocessing (moved from POJToneConverter)

    /// Preprocess POJ input based on user settings (oo→o͘, nn→ⁿ)
    private static func preprocessPojInput(_ input: String) -> String {
        var result = input
        let settings = SharedSettings.shared

        if settings.enableDoubleTapOO {
            result = result.replacingOccurrences(of: "oo", with: "o͘")
            result = result.replacingOccurrences(of: "Oo", with: "O͘")
            result = result.replacingOccurrences(of: "OO", with: "O͘")
        }

        if settings.enableDoubleTapNN {
            result = convertNasalDoubleN(result)
        }

        return result
    }

    /// Convert "nn" sequences after vowels to nasal marker "ⁿ"
    private static func convertNasalDoubleN(_ input: String) -> String {
        let vowels = "aeiouAEIOU"
        var result = ""
        let chars = Array(input)
        var i = 0

        while i < chars.count {
            if i + 2 < chars.count,
               vowels.contains(chars[i]),
               String(chars[i + 1]).lowercased() == "n",
               String(chars[i + 2]).lowercased() == "n"
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
