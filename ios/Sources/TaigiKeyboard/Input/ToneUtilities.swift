import Foundation

/// Per-character case mapping for POJ tone letters used by `CaseTransformer`,
/// plus the string-level `adjustNasalMarkerCase` used by
/// `SuggestionCaseTransformer`.
///
/// Why this lives on the platform (instead of going through `RustEngineBridge`):
/// `SuggestionCaseTransformer.transformWord` runs once per candidate during
/// JVM unit tests where the JNI native library is not loaded — routing every
/// candidate through the FFI would make `RustEngineBridge.<init>` (which
/// calls `System.loadLibrary("rust_taigi")`) throw `UnsatisfiedLinkError`.
/// `Method::NormalizeTone` still applies the same logic in-band for the
/// composing-display path, so the engine remains source-of-truth there.
enum ToneUtilities {
    static func uppercaseToneLetter(_ char: String, mode _: InputMode) -> String {
        if char == "\u{207F}" { return "\u{1D3A}" }
        return char.uppercased()
    }

    static func lowercaseToneLetter(_ char: String, mode _: InputMode) -> String {
        if char == "\u{1D3A}" { return "\u{207F}" }
        return char.lowercased()
    }

    /// Adjust nasal marker (`ⁿ`/`ᴺ`) case to match the preceding letter.
    /// Mirrors Rust `engine/phonetics/src/case_adjust.rs::adjust_nasal_marker_case`.
    /// CROSS-PLATFORM INVARIANT — drift causes silent divergence with
    /// `Method::NormalizeTone`'s in-band post-process.
    static func adjustNasalMarkerCase(_ text: String) -> String {
        let nasalLower: Character = "\u{207F}"
        let nasalUpper: Character = "\u{1D3A}"
        guard text.contains(nasalLower) || text.contains(nasalUpper) else { return text }

        var result = ""
        var lastLetterIsUppercase = false
        for char in text {
            if char == nasalLower || char == nasalUpper {
                result.append(lastLetterIsUppercase ? nasalUpper : nasalLower)
            } else {
                if char.isLetter { lastLetterIsUppercase = char.isUppercase }
                result.append(char)
            }
        }
        return result
    }
}
