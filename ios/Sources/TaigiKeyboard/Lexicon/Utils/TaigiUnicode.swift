import Foundation

/// Taigi-specific Unicode preprocessing helpers.
///
/// Stays on the platform (not behind `RustEngineBridge`) because the
/// remaining callers — `ExternalLookupURLBuilder.normalizeSyllableToDigit`
/// (URL build) and the cold-start dedup path in `CandidateProcessor`
/// (`romanToBase` / `inputToBase` retained for the freq-DB-not-yet-
/// connected branch) — must also work in JVM unit-test contexts on
/// Android where the JNI native library is not loaded. Routing through
/// the FFI would make `RustEngineBridge.<init>` →
/// `System.loadLibrary("rust_taigi")` throw `UnsatisfiedLinkError`.
/// Keeping a pure Foundation implementation here preserves the test path.
///
/// CROSS-PLATFORM INVARIANT — semantics must match Android
/// `TaigiUnicode.kt::nfdPreprocessed` and Rust
/// `engine/ranking/src/nfd.rs::taigi_unicode_base_form` (Rust copy is
/// the production ranking path; this Foundation copy is the platform
/// pin). All three implementations are exact-equivalent preprocessing
/// for tone-mark / combining-character analysis.

// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.
enum TaigiUnicode {
    /// Apply Taigi-specific Unicode preprocessing:
    /// 1. POJ nasal markers `ⁿ` (U+207F) / `ᴺ` (U+1D3A) → `"nn"`
    /// 2. NFD decompose (combining marks become individually accessible)
    /// 3. Replace `\u{0358}` (POJ `o͘` combining mark) → `"o"` so the
    ///    decomposed `o\u{0358}` collapses to `"oo"`.
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
