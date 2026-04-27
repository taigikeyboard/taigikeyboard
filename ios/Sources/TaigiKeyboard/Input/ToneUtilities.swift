import Foundation

/// Tone-letter case-conversion helpers for the `ⁿ` (U+207F) / `ᴺ` (U+1D3A)
/// nasal-marker codepoint pair plus mode-aware upper/lower routing.
///
/// Pure Foundation logic — `Character.uppercased()` / `lowercased()` already
/// handle combining tone marks correctly; this enum only handles the
/// non-roundtripping nasal-marker case the standard library can't infer.
///
/// Restored 2026-04-27 after `D9.4 commit 9` over-deleted alongside the
/// genuine Phonetics modules. Lives next to `CaseTransformer.swift` (its
/// primary consumer) to reflect the corrected scope: case-transform helper,
/// not phonetics core.
enum ToneUtilities {
    /// Uppercase a tone letter. Swift's built-in `uppercased()` handles
    /// combining marks correctly; the only special case is the nasal marker.
    static func uppercaseToneLetter(_ char: String, mode _: InputMode) -> String {
        if char == "\u{207F}" { return "\u{1D3A}" } // ⁿ → ᴺ
        return char.uppercased()
    }

    /// Lowercase a tone letter. Mirror of `uppercaseToneLetter`.
    static func lowercaseToneLetter(_ char: String, mode _: InputMode) -> String {
        if char == "\u{1D3A}" { return "\u{207F}" } // ᴺ → ⁿ
        return char.lowercased()
    }

    /// Adjust nasal marker (`ⁿ`/`ᴺ`) case to match the preceding letter.
    /// Rule: `ⁿ` follows lowercase, `ᴺ` follows uppercase.
    ///
    /// Swift caveat: `Character.isUppercase` returns `true` for both `ⁿ`
    /// and `ᴺ`, so explicit codepoint checks are required.
    static func adjustNasalMarkerCase(_ text: String) -> String {
        let nasalLower: Character = "\u{207F}" // ⁿ
        let nasalUpper: Character = "\u{1D3A}" // ᴺ

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
