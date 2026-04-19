// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

import java.text.Normalizer

/**
 * Taigi-specific Unicode preprocessing helpers.
 *
 * CROSS-PLATFORM INVARIANT — semantics must match iOS
 * `TaigiUnicode.swift` `nfdPreprocessed`. Both implementations are exact-equivalent
 * preprocessing for tone-mark / combining-character analysis.
 */
object TaigiUnicode {
    /**
     * Apply Taigi-specific Unicode preprocessing:
     * 1. POJ nasal markers `ⁿ` (U+207F) / `ᴺ` (U+1D3A) → `"nn"`
     * 2. NFD decompose (combining marks become individually accessible)
     * 3. Decomposed `o͘` combining mark (U+0358) → `"o"`
     *
     * Combining tone marks remain decomposed for the caller to extract.
     * Each caller does different things with them downstream — do NOT
     * merge that downstream logic into this utility.
     */
    fun nfdPreprocessed(input: String): String {
        val withNasal = input.replace("\u207F", "nn").replace("\u1D3A", "nn")
        val nfd = Normalizer.normalize(withNasal, Normalizer.Form.NFD)
        return nfd.replace("\u0358", "o")
    }
}
