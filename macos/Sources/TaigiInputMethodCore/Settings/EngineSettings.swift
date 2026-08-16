// The settings one composing operation reads. The provider that serves them
// lives in EngineSettingsProvider.swift.

import Foundation

/// The romanization the user types. macOS ships TL and POJ only — TPS is
/// deliberately out of scope for this platform (`docs/architecture/macos-roadmap.md`
/// § Goal), which is why this enum has no `tps` case to fall through.
enum InputMode: String, CaseIterable, Sendable {
    case tl
    case poj
}

/// Immutable snapshot of everything the engine needs to render a composition.
///
/// A snapshot rather than a set of getters because a single user intent can
/// issue several FFI calls (an `Append` is immediately followed by an
/// `EnterContinuous`), and those calls must agree: reading the settings twice
/// could straddle a change and render the two halves of one keystroke under
/// different rules.
struct EngineSettings: Equatable, Sendable {
    let inputMode: InputMode

    /// POJ preprocessing: double-tapped `oo` / `nn` fold to their diacritic
    /// forms. Sent to the engine as `AppConfig.oo_doubletap_enabled` /
    /// `nn_doubletap_enabled`.
    let isDoubleTapOOEnabled: Bool
    let isDoubleTapNNEnabled: Bool

    /// Word-boundary spacing inputs for the engine's `continuous_word_space`
    /// predicate (`docs/engine/continuous-input-ranking.md` §10.2). Both are
    /// `false` until PR5 ships the settings UI, but they are carried in the
    /// snapshot rather than hardcoded at the call sites because iOS's #380
    /// regression was exactly a call site that stopped passing the live pair.
    /// `isOutputBothScripts` is what separates hanji-first (no inter-segment
    /// space) from both-scripts (`hit (彼)` — space wanted); the swap flag is
    /// `true` for both, so one flag cannot express it.
    let isTranslateSwapped: Bool
    let isOutputBothScripts: Bool

    /// §34/S22 — when on, TL/POJ composing surfaces the preedit literal as the
    /// index-0 candidate so 漢羅 commits the romanization in one keystroke. The
    /// bridge inverts it into `FetchAtPos.literal_roman_candidate_disabled`.
    /// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift:50
    /// and android/…/ime/core/PrefHelper.kt:326, both of which default it OFF.
    /// Drift changes which candidate leads the list on a fresh install.
    let isLiteralRomanCandidateEnabled: Bool

    /// What a fresh install types with. Every value matches the iOS and Android
    /// default for the same setting, so someone using two of the three platforms
    /// gets the same composition and the same candidate order out of the box.
    static let defaults = EngineSettings(
        inputMode: .tl,
        isDoubleTapOOEnabled: true,
        isDoubleTapNNEnabled: true,
        isTranslateSwapped: false,
        isOutputBothScripts: false,
        isLiteralRomanCandidateEnabled: false,
    )
}
