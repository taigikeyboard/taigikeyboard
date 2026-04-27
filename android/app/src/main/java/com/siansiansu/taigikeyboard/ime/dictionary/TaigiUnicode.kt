// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

import java.text.Normalizer

/**
 * Taigi-specific Unicode preprocessing helpers.
 *
 * Stays on the platform (not behind `RustEngineBridge`) because the only
 * callers — `CandidateProcessor.romanToBase` (per-candidate scoring) and
 * `ExternalLookupURLBuilder.normalizeSyllableToDigit` (URL build) — must
 * also work in JVM unit-test contexts (`src/test/`) where the JNI native
 * library is not loaded. Routing through the FFI would make
 * `RustEngineBridge.<init>` → `System.loadLibrary("rust_taigi")` throw
 * `UnsatisfiedLinkError`. Keeping a pure Kotlin implementation here
 * preserves the test path.
 *
 * CROSS-PLATFORM INVARIANT — semantics must match iOS
 * `TaigiUnicode.swift` `nfdPreprocessed`. Both implementations are
 * exact-equivalent preprocessing for tone-mark / combining-character
 * analysis.
 */
object TaigiUnicode {
    /**
     * Apply Taigi-specific Unicode preprocessing:
     * 1. POJ nasal markers `ⁿ` (U+207F) / `ᴺ` (U+1D3A) → `"nn"`
     * 2. NFD decompose (combining marks become individually accessible)
     * 3. Replace `\u{0358}` (POJ `o͘` combining mark) → `"o"` so the
     *    decomposed `o\u{0358}` collapses to `"oo"`.
     *
     * Combining tone marks remain decomposed for the caller to extract.
     * Each caller does different things with them downstream — do NOT
     * merge that downstream logic into this utility.
     */
    fun nfdPreprocessed(input: String): String {
        val withNasal = input.replace("ⁿ", "nn").replace("ᴺ", "nn")
        val nfd = Normalizer.normalize(withNasal, Normalizer.Form.NFD)
        return nfd.replace("͘", "o")
    }
}
