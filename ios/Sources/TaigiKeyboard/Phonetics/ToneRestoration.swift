import Foundation

/// Tone restoration
///
/// Restores tone-marked characters to their base form for backspace operations.
/// Uses NFD decomposition instead of lookup tables.

// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.
enum ToneRestoration {

    /// Attempt to restore tone marks in text
    /// - Parameters:
    ///   - text: Text to restore
    ///   - mode: Input mode (POJ/TL)
    /// - Returns: Restored text, or nil if no tone marks found
    static func restore(_ text: String, mode: InputMode) -> String? {
        guard !text.isEmpty else { return nil }

        // NFD decompose to expose combining marks
        let nfd = text.decomposedStringWithCanonicalMapping
        var scalars = Array(nfd.unicodeScalars)

        // Scan from end, find last combining tone mark
        for i in stride(from: scalars.count - 1, through: 0, by: -1) {
            if TaigiPhonetics.combiningToToneNum[scalars[i]] != nil {
                // Remove the combining mark
                scalars.remove(at: i)
                let restored = String(String.UnicodeScalarView(scalars))
                return restored.precomposedStringWithCanonicalMapping
            }
        }

        return nil
    }
}
