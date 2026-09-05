// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

// POJ 雙擊預處理 toggles 值型別 —(oo→o͘、nn→ⁿ)。
// 由 wrapper 在呼叫 engine 時 live-read,對應 iOS Settings/ToneToggles.swift。

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
