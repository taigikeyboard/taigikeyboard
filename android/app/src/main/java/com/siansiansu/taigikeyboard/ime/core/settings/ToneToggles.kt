package com.siansiansu.taigikeyboard.ime.core.settings

/**
 * POJ preprocessing toggles the composing engine needs when deriving display text.
 *
 * Carried as an explicit value so `ComposingState` / `ToneConverter` can stay
 * Kotlin-stdlib-only. The wrapper reads the booleans from
 * `EngineSettingsProvider.current` at call time (live read — see
 * `EngineSettingsProvider`), then passes them through.
 *
 * Mirrors iOS `Settings/ToneToggles.swift`.
 */
data class ToneToggles(
    val isDoubleTapOOEnabled: Boolean,
    val isDoubleTapNNEnabled: Boolean,
)
