// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

// 中文: Lexicon / 輸入引擎用的唯讀設定介面 — 由 EngineSettingsProvider 提供。
// 中文: 純 Kotlin stdlib(無 android.* / androidx.* / coroutines),engine 層可直接依賴。
// 中文: 所有 getter 必須 live-read,具體實作不可在初始化時快照。對應 iOS Settings/EngineSettings.swift。

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
 * - [inputMode] is a `String` on Android because the platform's
 *   `InputMode` enum has only POJ/TL/ENGLISH and cannot represent `tps`.
 *   Engine-side call sites key on the raw string (`"tps"`, `"poj"`, `"tl"`).
 *   iOS uses the full `InputMode` enum (4 cases). A future round unifies
 *   the shape.
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

    /**
     * Output both hanji + roman ("both-scripts"). The continuous-input
     * §10.2 word-boundary-spacing predicate needs this to tell
     * hanji-first (no inter-segment space) from both-scripts (`hit (彼)`
     * — space wanted); [isTranslateSwapped] is `true` for both.
     */
    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/EngineSettings.swift:isOutputBothScripts.
    // Drift causes silent divergence (hanji-first spurious word-boundary spaces).
    val isOutputBothScripts: Boolean

    val isAssociationRecordingEnabled: Boolean

    /**
     * Literal-roman candidate toggle (§34/S22). When on (default), TL/POJ
     * composing surfaces the preedit literal (`derived_display`) as the
     * index-0 candidate so 漢羅 mixing commits the romanization in one tap.
     * `ComposingManager` inverts it into
     * `FetchAtPos.literalRomanCandidateDisabled`. Gates ONLY that forced
     * prepend, not naturally-produced roman candidates.
     */
    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/EngineSettings.swift:isLiteralRomanCandidateEnabled.
    // Drift causes silent divergence (one platform shows the §34 candidate, the other does not).
    val isLiteralRomanCandidateEnabled: Boolean

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
    val isDevDictEnabled: Boolean

    // kautian subcollection toggles (nested under the MOE/kautian master). Order
    // mirrors config.yaml dialect_columns; the bridge packs these into the
    // KautianSubcollToggles proto and Rust compute_filters owns the subtag bit
    // layout. Absent sub-message ⇒ engine keeps legacy all-on (DD5).
    // 中文: kautian subcollection 子開關 — 10 腔調 + 姓名附錄。對應 iOS Settings/EngineSettings.swift。
    val isKautianAccentLukangEnabled: Boolean
    val isKautianAccentSansiaEnabled: Boolean
    val isKautianAccentTaipakEnabled: Boolean
    val isKautianAccentGilanEnabled: Boolean
    val isKautianAccentTainanEnabled: Boolean
    val isKautianAccentKaohsiungEnabled: Boolean
    val isKautianAccentKinmenEnabled: Boolean
    val isKautianAccentMakungEnabled: Boolean
    val isKautianAccentSintikEnabled: Boolean
    val isKautianAccentTaichungEnabled: Boolean
    val isKautianNameAppendixEnabled: Boolean
}
