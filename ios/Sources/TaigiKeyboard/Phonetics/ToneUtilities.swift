import Foundation

/// Tone utilities
///
/// Provides tone letter case conversion for POJ/TL special characters.
enum ToneUtilities {

    /// Uppercase a tone letter. Swift's built-in uppercased() handles combining marks correctly.
    /// - Parameters:
    ///   - char: Character to convert
    ///   - mode: Input mode (POJ/TL)
    /// - Returns: Uppercased character
    static func uppercaseToneLetter(_ char: String, mode: InputMode) -> String {
        if char == "\u{207F}" { return "\u{1D3A}" }  // ⁿ → ᴺ
        return char.uppercased()
    }

    /// Lowercase a tone letter. Swift's built-in lowercased() handles combining marks correctly.
    /// - Parameters:
    ///   - char: Character to convert
    ///   - mode: Input mode (POJ/TL)
    /// - Returns: Lowercased character
    static func lowercaseToneLetter(_ char: String, mode: InputMode) -> String {
        if char == "\u{1D3A}" { return "\u{207F}" }  // ᴺ → ⁿ
        return char.lowercased()
    }

    /// Adjust nasal marker (ⁿ/ᴺ) case to match the preceding letter's case.
    /// Rule: ⁿ follows lowercase letters, ᴺ follows uppercase letters.
    ///
    /// Swift caveat: `.isUppercase` returns `true` for both ⁿ and ᴺ,
    /// so we use explicit codepoint checks to identify nasal markers.
    static func adjustNasalMarkerCase(_ text: String) -> String {
        let nasalLower: Character = "\u{207F}"  // ⁿ
        let nasalUpper: Character = "\u{1D3A}"  // ᴺ

        // Early exit: skip iteration if no nasal markers present
        guard text.contains(nasalLower) || text.contains(nasalUpper) else { return text }

        var result = ""
        var lastLetterIsUppercase = false

        for char in text {
            if char == nasalLower || char == nasalUpper {
                result.append(lastLetterIsUppercase ? nasalUpper : nasalLower)
            } else {
                if char.isLetter {
                    lastLetterIsUppercase = char.isUppercase
                }
                result.append(char)
            }
        }

        return result
    }

}
