import Foundation

/// Taigi-specific Unicode preprocessing helpers.
///
/// CROSS-PLATFORM INVARIANT — semantics must match Android
/// `TaigiUnicode.kt` `nfdPreprocessed`. Both implementations are exact-equivalent
/// preprocessing for tone-mark / combining-character analysis.

// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.
enum TaigiUnicode {
    /// Apply Taigi-specific Unicode preprocessing:
    /// 1. POJ nasal markers `ⁿ` (U+207F) / `ᴺ` (U+1D3A) → `"nn"`
    /// 2. NFD decompose (combining marks become individually accessible)
    /// 3. Decomposed `o͘` combining mark (U+0358) → `"o"`
    ///
    /// Combining tone marks remain decomposed for the caller to extract.
    /// Each caller does different things with them downstream — do NOT
    /// merge that downstream logic into this utility.
    static func nfdPreprocessed(_ input: String) -> String {
        input
            .replacingOccurrences(of: "\u{207F}", with: "nn")
            .replacingOccurrences(of: "\u{1D3A}", with: "nn")
            .decomposedStringWithCanonicalMapping
            .replacingOccurrences(of: "\u{0358}", with: "o")
    }
}
