// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

package com.siansiansu.taigikeyboard.ime.core.settings

/**
 * The POJ marker options the engine reads when it renders text: the two
 * double-tap folds that compose `o͘` / `ⁿ` (`oo` / `nn`) and ⁿ becomes ᴺ in capitals, the case
 * rule of the nasal marker it composed (`SIÂᴺ` after a capital, or always `ⁿ`;
 * `behavioral-invariants.md` §53).
 *
 * Carried as an explicit value so `ComposingState` / `ToneConverter` can stay
 * Kotlin-stdlib-only. The wrapper reads the booleans from
 * `EngineSettingsProvider.current` at call time (live read — see
 * `EngineSettingsProvider`), then passes them through.
 *
 * Mirrors iOS `Settings/PojMarkerOptions.swift`.
 */
data class PojMarkerOptions(
    val isDoubleTapOOEnabled: Boolean,
    val isDoubleTapNNEnabled: Boolean,
    val isNasalMarkerUppercaseEnabled: Boolean,
)
