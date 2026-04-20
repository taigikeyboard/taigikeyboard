// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.core.settings

/**
 * Read-only view of the settings the lexicon / input engine needs to make
 * decisions during candidate search, composing, and suggestion ranking.
 * Supplied to engine services via [EngineSettingsProvider].
 *
 * Kept Kotlin-stdlib-only (no android.*, no androidx.*, no
 * kotlinx.coroutines.*) so engine-layer files can depend on this interface
 * without pulling platform frameworks — matches iOS
 * `Settings/EngineSettings.swift`.
 *
 * All getters MUST be live-read (see [EngineSettingsProvider]). Concrete
 * implementations should not snapshot values in the initializer — each
 * property access re-reads the underlying store so user-settings updates
 * propagate without rebuilding the engine graph.
 *
 * ### Divergence from iOS (deferred)
 * - [inputMode] is a `String` on Android because the existing
 *   `ToneConverterModels.InputMode` enum has only POJ/TL and cannot
 *   represent `english` or `tps`. Engine-side call sites key on the raw
 *   string (`"tps"`, `"poj"`, `"tl"`). iOS uses the full `InputMode` enum.
 *   A future round unifies the shape.
 */
interface EngineSettings {
    /** Current input mode — `"poj"`, `"tl"`, `"tps"`, or `"english"`. */
    val inputMode: String

    /**
     * Auto-capitalization toggle consumed by candidate case-transformation.
     * Mirrors iOS `isAutoCap` (which itself bridges
     * KeyboardKit's `isAutocapitalizationEnabled`).
     */
    val isAutoCap: Boolean

    val isTranslateSwapped: Boolean
    val isAssociationRecordingEnabled: Boolean

    /**
     * POJ preprocessing toggles bundled as a live-read value so
     * `ComposingState` / `ToneConverter` can stay Kotlin-stdlib-pure.
     */
    val toneToggles: ToneToggles

    val isCustomDictEnabled: Boolean
    val isTpsOrMappedToER: Boolean

    // Dictionary toggles (match iOS names)
    val isMoeDictEnabled: Boolean
    val isNewwordDictEnabled: Boolean
    val isKunggeDictEnabled: Boolean
    val isITaigiDictEnabled: Boolean
    val isTaiwanJapanDictEnabled: Boolean
    val isTaiHuaDictEnabled: Boolean
    val isTaiwanPlantDictEnabled: Boolean
    val isSttiDictEnabled: Boolean
    val isKhpooDictEnabled: Boolean
    val isVariantEnabled: Boolean
    val isKhiinEnabled: Boolean
    val isLkkDictEnabled: Boolean
}
