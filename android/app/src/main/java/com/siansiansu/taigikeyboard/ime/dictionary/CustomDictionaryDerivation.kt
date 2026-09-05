// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.engine.CustomSearchKey
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode

/**
 * Pure derivation functions for custom-dictionary columns. Mirrors iOS
 * `Lexicon/Database/CustomDictionaryDerivation.swift`. After D9.4 every
 * entry point is a thin wrapper over [RustEngineBridge]; the previous
 * NFD walks + nasal-marker conversion + diacritic stripping moved into
 * `engine/phonetics/src/derivation.rs`.
 */
object CustomDictionaryDerivation {
    /** Toneless form used for toneless prefix search — Rust `Method::DeriveNotone`. */
    fun generateNotone(roman: String): String = RustEngineBridge.deriveNotone(roman)

    /**
     * Abbreviation: first letter of each syllable (split by hyphen + ASCII
     * whitespace `[ \t\n\x0B\f\r-]+`) with diacritics stripped. Returns
     * empty string for single-syllable input. Whitespace canonical matches
     * Android JVM `Regex("[\\s-]+")` exactly per Codex v3 §1.
     */
    fun generateAbbrev(roman: String): String = RustEngineBridge.deriveAbbrev(roman)

    /** Numeric-toned form for tone-aware search — same as `Method::NormalizeInput`. */
    fun generateRomanNum(roman: String): String = RustEngineBridge.normalizeInput(roman)

    /**
     * v3.6.1 R3 WRITE side — full {tl, poj, tps} × {num, notone, abbrev} (+ TPS
     * er/or variant) cross-mode search-key bundle for a stored custom-dict
     * roman. Materialized into the `custom_search_key` side table so a query in
     * ANY input mode finds the entry. Rust `Method::DeriveCustomSearchKeys`.
     * Mirrors iOS `CustomDictionaryDerivation.deriveCustomSearchKeys`.
     */
    fun deriveCustomSearchKeys(roman: String): List<CustomSearchKey> = RustEngineBridge.deriveCustomSearchKeys(roman)

    /**
     * v3.6.1 R3 READ side — single family-native key for the user's current raw
     * `input` + `mode`. The engine upgrades the family to TPS when the raw input
     * carries Bopomofo, so the caller passes its settings mode verbatim. `null`
     * for residue-only / empty input. Rust `Method::DeriveCustomQueryKey`.
     * Mirrors iOS `CustomDictionaryDerivation.deriveCustomQueryKey`.
     */
    fun deriveCustomQueryKey(
        input: String,
        mode: InputMode,
    ): CustomSearchKey? = RustEngineBridge.deriveCustomQueryKey(input, mode)
}
