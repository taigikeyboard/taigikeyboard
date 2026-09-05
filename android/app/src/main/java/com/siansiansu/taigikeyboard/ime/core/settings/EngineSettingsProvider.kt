// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

package com.siansiansu.taigikeyboard.ime.core.settings

/**
 * Supplies the engine with an [EngineSettings] that reflects the current
 * state of the underlying store.
 *
 * [current] is expected to return a *live-reading* implementation — each
 * chain like `provider.current.inputMode` reads the most recent DataStore
 * cache on Android (or `UserDefaults` on iOS). This preserves the keyboard
 * extension's live-update behavior: the user changes a setting in the
 * Settings Activity, DataStore emits a new snapshot, and the next engine
 * query sees the new value without the composition root being rebuilt.
 *
 * Do NOT return a one-shot snapshot from [current]. If a caller needs
 * consistency across multiple reads, it should capture a local copy.
 *
 * Mirrors iOS `Settings/EngineSettingsProvider.swift`.
 */
interface EngineSettingsProvider {
    val current: EngineSettings
}
