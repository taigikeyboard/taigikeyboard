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

/// How the candidate window renders the `(漢字, 羅馬字)` pair: both scripts
/// side by side (the swap setting decides which leads), each script as its own
/// adjacent cell (漢羅合用, `PresentedCandidate`), or the romanization alone.
///
/// Raw values are the storage contract every platform shares
/// (`docs/reports/2026-08-30-hanlo-together-mode-research.md` §12) — the same
/// convention `isTranslateSwapped` / `outputBothScripts` follow, so a future
/// settings transfer carries one vocabulary.
/// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SettingsModels.swift
/// `CandidateDisplayMode` and the Android `CandidateDisplayMode.storageValue`.
/// Drift changes which mode a transferred setting resolves to.
enum CandidateDisplayMode: String, CaseIterable, Sendable {
    case sideBySide
    case combined
    case romanOnly

    /// Whether the cell shows any Hanji — `false` only under `.romanOnly`.
    var showsHanji: Bool { self != .romanOnly }

    /// The mode after this one, in the order the 外觀 picker lists them, and
    /// round again from the end — what the cycle shortcut steps through, so
    /// the key and the picker agree on what "next" is.
    var next: CandidateDisplayMode {
        let all = Self.allCases
        let index = all.firstIndex(of: self) ?? all.startIndex
        return all[(index + 1) % all.count]
    }

    /// The picker row's label for this mode — also what the cycle shortcut
    /// flashes, so the HUD names the mode in the words the pane uses.
    var displayNameKey: StringKey {
        switch self {
        case .sideBySide: .settingsCandidateDisplayModeSideBySide
        case .combined: .settingsCandidateDisplayModeCombined
        case .romanOnly: .settingsCandidateDisplayModeRomanOnly
        }
    }

    /// Only side-by-side has a lead script the swap shortcut can flip; the
    /// other two fix it, so the shortcut is inert and the stored swap waits
    /// for the way back.
    var allowsSwapToggle: Bool { self == .sideBySide }

    /// Effective swap for a stored flag. `.combined` leads with — and commits —
    /// the Hanji: forcing the pair on is a compatibility projection of that,
    /// so every reader of the pair (auto-space, full-width punctuation, the
    /// nextword gates) behaves as today's hanji-first mode
    /// (`behavioral-invariants.md` §42). `.romanOnly` has no Hanji to lead with.
    /// CROSS-PLATFORM INVARIANT — mirrors ios `SettingsModels.swift`
    /// `CandidateDisplayMode.effectiveTranslateSwapped`, android
    /// `CandidateDisplayMode.kt`, windows `engine_settings.rs`. Drift causes
    /// silent divergence.
    func effectiveTranslateSwapped(stored: Bool) -> Bool {
        self == .combined || (stored && showsHanji)
    }

    /// Effective 括號標註 for a stored flag — off only where there is no Hanji
    /// to bracket; `.combined` keeps it (`漢字 (羅馬字)`).
    func effectiveOutputBothScripts(stored: Bool) -> Bool {
        stored && showsHanji
    }
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

    /// Word-boundary spacing inputs for the engine's `continuous_word_space`
    /// predicate (`docs/engine/continuous-input-ranking.md` §10.2). Both are
    /// `false` until PR5 ships the settings UI, but they are carried in the
    /// snapshot rather than hardcoded at the call sites because iOS's #380
    /// regression was exactly a call site that stopped passing the live pair.
    /// `isOutputBothScripts` is what separates hanji-first (no inter-segment
    /// space) from both-scripts (`hit (彼)` — space wanted); the swap flag is
    /// `true` for both, so one flag cannot express it.
    ///
    /// EFFECTIVE, not stored: under `candidateDisplayMode == .romanOnly` both
    /// read `false` whatever the user has stored, because a mode that shows
    /// and commits only romanization has no Hanji to lead with or to bracket.
    /// The stored values live on in `UserDefaults`
    /// (`SettingsStore.storedIsTranslateSwapped` / `storedIsOutputBothScripts`)
    /// and come back the moment the mode returns to `.sideBySide`. Under
    /// `.combined` the swap reads `true` whatever is stored — the Hanji cell
    /// comes first and is the `.primary` commit, the romanization cell beside
    /// it the `.alternate` one (`PresentedCandidate`) — while the bracket
    /// setting is read as stored (`SettingsStore.current` has the why). Every
    /// reader of "swap" — engine `AppConfig`, cell, document text, auto-space,
    /// full-width punctuation — reads THIS pair, never the stored one.
    /// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/EngineSettings.swift
    /// `isTranslateSwapped` / `isOutputBothScripts` (derived the same way) and
    /// the Windows `document.rs engine_settings()`. Drift changes what a
    /// romanization-only install commits.
    let isTranslateSwapped: Bool
    let isOutputBothScripts: Bool

    /// Whether the candidate window shows both scripts or the romanization
    /// alone. Sent to the engine as `AppConfig.candidate_display_mode`, which
    /// is what collapses same-romanization rows under `.romanOnly`
    /// (`engine/composing/src/dispatch.rs`, `engine/nextword/src/filter.rs`);
    /// on this side it selects the cell arm and derives the pair above.
    /// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/EngineSettings.swift
    /// `candidateDisplayMode`, which defaults it to side-by-side. Drift changes
    /// what a fresh install's candidate cells show.
    let candidateDisplayMode: CandidateDisplayMode

    /// §34/S22 — when on, TL/POJ composing surfaces the preedit literal as the
    /// index-0 candidate so 漢羅 commits the romanization in one keystroke. The
    /// bridge inverts it into `FetchAtPos.literal_roman_candidate_disabled`.
    /// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift:50
    /// and android/…/ime/core/PrefHelper.kt:326, both of which default it OFF.
    /// Drift changes which candidate leads the list on a fresh install.
    let isLiteralRomanCandidateEnabled: Bool

    /// Whether committing a candidate counts towards its ranking next time.
    /// Read on the write path only — the boost itself is always applied to
    /// whatever counts have been learned, so turning this off freezes the
    /// learned ranking rather than discarding it.
    /// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift:48,
    /// which defaults it ON. Drift changes whether a fresh install learns.
    let isFrequencyRecordingEnabled: Bool

    /// Whether a commit records which word followed which. Sent to the engine as
    /// `AppConfig.is_association_recording_enabled`, which is what gates the
    /// `RecordAssociation` effects in `engine/nextword/src/decide.rs:130`.
    ///
    /// The gate is on emitting the effect, not on tracking the context: the
    /// engine still remembers the last committed word while this is off, so a
    /// word committed with it off can become the predecessor of one committed
    /// within ten seconds of switching it back on. That is the engine's
    /// behaviour on all three platforms, not something macOS introduces here.
    /// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift:49,
    /// which defaults it ON.
    let isAssociationRecordingEnabled: Bool

    /// Whether the user's own dictionary contributes candidates. Gates the
    /// lookup itself, not just the display: with it off nothing is read from
    /// `custom_dictionary.db` and `FetchAtPos.custom_entries` goes out empty.
    /// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift:51,
    /// which defaults it ON.
    let isCustomDictEnabled: Bool

    /// Which bundled dictionaries the engine may draw candidates from. Reaches
    /// the engine as `FetchAtPos.enabled_sources_bitmask` after the
    /// `compute_filters` op resolves it (`RustEngineBridge+Lexicon.swift`).
    let dictionarySources: DictionarySourceToggles

    /// What a fresh install types with. Every value matches the iOS and Android
    /// default for the same setting, so someone using two of the three platforms
    /// gets the same composition and the same candidate order out of the box.
    static let defaults = EngineSettings(
        inputMode: .tl,
        isTranslateSwapped: false,
        isOutputBothScripts: false,
        candidateDisplayMode: .sideBySide,
        isLiteralRomanCandidateEnabled: false,
        isFrequencyRecordingEnabled: true,
        isAssociationRecordingEnabled: true,
        isCustomDictEnabled: true,
        dictionarySources: .defaults,
    )
}
