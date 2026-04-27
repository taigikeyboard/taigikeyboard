// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.engine.RustEngineBridge

/**
 * Pure derivation functions for custom-dictionary columns. Mirrors iOS
 * `Lexicon/Database/CustomDictionaryDerivation.swift`. After D9.4 every
 * entry point is a thin wrapper over [RustEngineBridge]; the previous
 * NFD walks + nasal-marker conversion + diacritic stripping moved into
 * `engine/phonetics/src/derivation.rs`.
 */
object CustomDictionaryDerivation {
    /** Toneless form used for toneless prefix search — Rust `OP_DERIVE_NOTONE`. */
    fun generateNotone(roman: String): String = RustEngineBridge.deriveNotone(roman)

    /**
     * Abbreviation: first letter of each syllable (split by hyphen + ASCII
     * whitespace `[ \t\n\x0B\f\r-]+`) with diacritics stripped. Returns
     * empty string for single-syllable input. Whitespace canonical matches
     * Android JVM `Regex("[\\s-]+")` exactly per Codex v3 §1.
     */
    fun generateAbbrev(roman: String): String = RustEngineBridge.deriveAbbrev(roman)

    /** Numeric-toned form for tone-aware search — same as `OP_NORMALIZE_INPUT`. */
    fun generateRomanNum(roman: String): String = RustEngineBridge.normalizeInput(roman)
}
