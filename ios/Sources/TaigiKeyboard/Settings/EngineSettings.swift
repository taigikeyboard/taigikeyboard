import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Read-only view of the settings the lexicon / input engine needs to
/// make decisions during candidate search, composing, and suggestion
/// ranking. Supplied to engine services via `EngineSettingsProvider`.
///
/// Kept Foundation-only (no UIKit, KeyboardKit, SwiftUI) so engine-layer
/// files can depend on this protocol without pulling platform frameworks.
protocol EngineSettings {
    var inputMode: InputMode { get }
    // Sourced from KeyboardKit `isAutocapitalizationEnabled`.
    var isAutoCap: Bool { get }
    var isTranslateSwapped: Bool { get }
    /// Output both hanji + roman ("both-scripts"). The continuous-input
    /// §10.2 word-boundary-spacing predicate needs this to tell
    /// hanji-first (no space) from both-scripts (`hit (彼)` — space
    /// wanted); `isTranslateSwapped` is `true` for both.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/core/settings/EngineSettings.kt:isOutputBothScripts.
    // Drift causes silent divergence (hanji-first spurious word-boundary spaces).
    var isOutputBothScripts: Bool { get }

    /// Candidate cell rendering mode (漢羅對應 / 羅馬字). Under `.romanOnly`
    /// the two flags above read `false` regardless of their stored values —
    /// they are derived, never overwritten — so auto-space, the commit
    /// formatter, and the engine `AppConfig` all take the roman-first arms.
    /// The bridge forwards this as `AppConfig.candidate_display_mode` so the
    /// engine collapses same-roman rows for display.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/core/settings/EngineSettings.kt:candidateDisplayMode.
    // Drift causes silent divergence (one platform still shows hanji / swaps scripts under 羅馬字).
    var candidateDisplayMode: CandidateDisplayMode { get }
    var isAssociationRecordingEnabled: Bool { get }

    /// Literal-roman candidate toggle (§34/S22). When on (default), TL/POJ
    /// composing surfaces the preedit literal (`derived_display`) as the
    /// index-0 candidate so 漢羅 mixing commits the romanization in one tap.
    /// The bridge inverts this into `FetchAtPos.literal_roman_candidate_disabled`.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/core/settings/EngineSettings.kt:isLiteralRomanCandidateEnabled.
    // Drift causes silent divergence (one platform shows the §34 candidate, the other does not).
    var isLiteralRomanCandidateEnabled: Bool { get }

    /// POJ preprocessing toggles bundled as a live-read value so
    /// `ComposingState` / `ToneConverter` can stay Foundation-pure.
    var toneToggles: ToneToggles { get }

    var isCustomDictEnabled: Bool { get }
    // TPS: "or" maps to ㄜ when true, ㄛ when false.
    var isTpsOrMappedToER: Bool { get }

    var isMoeDictEnabled: Bool { get }
    var isNewwordDictEnabled: Bool { get }
    var isKunggeDictEnabled: Bool { get }
    var isITaigiDictEnabled: Bool { get }
    var isTaiwanJapanDictEnabled: Bool { get }
    var isTaiHuaDictEnabled: Bool { get }
    var isTaiwanPlantDictEnabled: Bool { get }
    var isSttiDictEnabled: Bool { get }
    var isKhpooDictEnabled: Bool { get }
    var isVariantEnabled: Bool { get }
    var isKhiinEnabled: Bool { get }
    var isLkkDictEnabled: Bool { get }
    var isDevDictEnabled: Bool { get }

    // kautian subcollection toggles (nested under the kautian master). Order
    // mirrors config.yaml dialect_columns; the bridge packs these into the
    // KautianSubcollToggles proto and `compute_filters` owns the subtag bit
    // layout. Absent gate ⇒ engine keeps legacy all-on (DD5).
    var isKautianAccentLukangEnabled: Bool { get }
    var isKautianAccentSansiaEnabled: Bool { get }
    var isKautianAccentTaipakEnabled: Bool { get }
    var isKautianAccentGilanEnabled: Bool { get }
    var isKautianAccentTainanEnabled: Bool { get }
    var isKautianAccentKaohsiungEnabled: Bool { get }
    var isKautianAccentKinmenEnabled: Bool { get }
    var isKautianAccentMakungEnabled: Bool { get }
    var isKautianAccentSintikEnabled: Bool { get }
    var isKautianAccentTaichungEnabled: Bool { get }
    var isKautianNameAppendixEnabled: Bool { get }
}
