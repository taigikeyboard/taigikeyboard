// The settings the engine renders under and their fresh-install defaults.

/// What the user types: one of the two romanizations, or TPS (方音符號) on
/// the Zhuyin key positions (`docs/architecture/desktop-tps-roadmap.md` D1).
/// Raw values are desktop-core's `InputMode` spellings, which the key path
/// reads from the settings snapshot.
enum InputMode: String, CaseIterable, Sendable {
    case tl
    case poj
    case tps

    /// The Input Script picker's label for this mode — also what a switch
    /// flashes, so the HUD names the mode in the words the pane uses.
    var displayNameKey: StringKey {
        switch self {
        case .tl: .settingsTlMode
        case .poj: .settingsPojMode
        case .tps: .settingsTpsMode
        }
    }
}

/// What asked for an input-mode change — desktop-core's `InputModeRequest`,
/// which answers it (`SettingsStore.switchInputMode(_:)`).
enum InputModeSwitch: Sendable, Equatable {
    /// The Input Script picker chose this mode.
    case pick(InputMode)
    /// Switch Romanization: TL ↔ POJ; from TPS, to the romanization not last
    /// used.
    case toggleRomanization
    /// Switch TPS: into TPS, or back to the romanization last used.
    case toggleTps
}

/// How the candidate window renders the `(Hanji, romanization)` pair: both scripts
/// side by side (the swap setting decides which leads), each script as its own
/// adjacent cell (Hanji with Romanization; desktop-core's
/// `composing/presentation.rs`), or the romanization alone.
///
/// Raw values are the storage contract every platform shares
/// (`docs/architecture/behavioral-invariants.md` §42) — the same
/// convention `isHanjiFirst` follows, so a future
/// settings transfer carries one vocabulary.
/// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SettingsModels.swift
/// `CandidateDisplayMode` and the Android `CandidateDisplayMode.storageValue`.
/// Drift changes which mode a transferred setting resolves to.
enum CandidateDisplayMode: String, CaseIterable, Sendable {
    case sideBySide
    case combined
    case romanOnly

    /// Whether the cell shows any Hanji — `false` only under `.romanOnly`.
    var showsHanji: Bool {
        self != .romanOnly
    }

    /// The mode after this one, in the order the Appearance picker lists them, and
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

    /// Whether the swap shortcut writes the stored swap — exactly where Hanji
    /// is on screen. Under `.combined` the cells are split per script, so the
    /// shortcut only picks the punctuation width (USER 2026-09-13: "Hanji with Romanization needs an
    /// isTranslateSwapped button"); `.romanOnly` leaves it inert and the stored
    /// swap waits for the way back.
    var allowsSwapToggle: Bool {
        showsHanji
    }
}

/// How the rendered romanization separates syllables (Syllable Separator,
/// `behavioral-invariants.md` §49): the dictionary hyphen (`tâi-uân`), a space
/// (`tâi uân`), or nothing (`tâiuân`). The engine rewrites the romanization
/// itself; desktop-core reads the stored raw value through its key-path
/// snapshot (`key_path.rs`).
/// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SettingsModels.swift
/// `SyllableSeparator`, the Android `SyllableSeparator.storageValue` and
/// desktop-core `SyllableSeparator::raw`. Drift changes which choice a stored
/// value resolves to.
enum SyllableSeparator: String, CaseIterable, Sendable {
    case hyphen
    case space
    /// Raw `none`; spelled out in Swift so it never reads as `Optional.none`.
    case noSeparator = "none"

    /// The picker row's label.
    var displayNameKey: StringKey {
        switch self {
        case .hyphen: .desktopTelexGuideHyphen
        case .space: .settingsSyllableSeparatorSpace
        case .noSeparator: .settingsSyllableSeparatorNone
        }
    }
}

/// What a fresh install composes under: the default table `SettingsStore.Keys`
/// reads its engine-setting defaults from. Desktop-core reads the settings
/// themselves, from the snapshot each request carries
/// (`DesktopCoreRuntime.settingsSnapshot`); `DesktopCoreRuntimeTests` pins its
/// defaults to these.
struct EngineSettings: Sendable {
    let inputMode: InputMode

    /// The STORED swap (`isTranslateSwapped`): whether a cell leads with — and
    /// commits — the Hanji. Its readers take it through the display mode
    /// (§42): desktop-core's `SettingsDocument::engine_settings` for the
    /// candidate lead, `effective_full_width_punctuation` for the width.
    /// CROSS-PLATFORM INVARIANT — defaults ON on every platform (§48): iOS
    /// `SharedSettings.isHanjiFirstKey`, Android `PrefHelper.storedIsHanjiFirst`,
    /// desktop-core `EngineSettings::DEFAULT`. Drift changes what a fresh
    /// install commits.
    let isHanjiFirst: Bool

    /// How the candidate window renders the pair. Sent to the engine as
    /// `AppConfig.candidate_display_mode`, which is what collapses
    /// same-romanization rows under `.romanOnly`
    /// (`engine/composing/src/requests.rs`, `engine/nextword/src/filter.rs`).
    /// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/EngineSettings.swift
    /// `candidateDisplayMode`, which defaults it to side-by-side. Drift changes
    /// what a fresh install's candidate cells show.
    let candidateDisplayMode: CandidateDisplayMode

    /// §34/S22 — when on, TL/POJ composing surfaces the preedit literal as the
    /// index-0 candidate so mixed-script writing commits the romanization in one keystroke, and
    /// Return on a fresh bar writes what was typed. The bridge inverts it into
    /// `FetchAtPos.literal_roman_candidate_disabled`.
    /// CROSS-PLATFORM INVARIANT — mirrors `isLiteralRomanCandidateEnabled` in
    /// ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift and
    /// `literalRomanCandidateEnabled` in android/…/ime/settings/PrefHelper.kt;
    /// every platform defaults it OFF (USER 2026-10-02).
    /// Drift changes which candidate leads the list on a fresh install.
    let isLiteralRomanCandidateEnabled: Bool

    /// Syllable Separator (`behavioral-invariants.md` §49) — sent as
    /// `AppConfig.syllable_separator` on every request.
    /// CROSS-PLATFORM INVARIANT — mirrors `syllableSeparator` in
    /// ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift and
    /// android/…/ime/settings/PrefHelper.kt, both `hyphen`.
    let syllableSeparator: SyllableSeparator

    /// Nasal mark in POJ capitals (`behavioral-invariants.md` §53) — the POJ nasal marker follows
    /// the case of the letters before it (`SIÂᴺ`); off, it is always `ⁿ`.
    /// Sent inverted as `AppConfig.force_lowercase_nasal_marker` on the base
    /// config.
    /// CROSS-PLATFORM INVARIANT — mirrors `isNasalMarkerUppercaseEnabled` in
    /// ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift and
    /// `nasalMarkerUppercaseEnabled` in android/…/ime/settings/PrefHelper.kt, all ON.
    let isNasalMarkerUppercaseEnabled: Bool

    /// Whether the user's own dictionary contributes candidates. Gates the
    /// lookup itself, not just the display: with it off nothing is read from
    /// `custom_dictionary.db` (`AppConfig.custom_dictionary_disabled`).
    /// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift:51,
    /// which defaults it ON.
    let isCustomDictEnabled: Bool

    /// Which bundled dictionaries the engine may draw candidates from. Reaches
    /// the engine as `AppConfig.dictionary_sources`, which it resolves into its
    /// source filter.
    let dictionarySources: DictionarySourceToggles

    /// What a fresh install types with. Every value matches the iOS and Android
    /// default for the same setting, so someone using two of the four platforms
    /// gets the same composition and the same candidate order out of the box.
    ///
    /// Hanji-first since 2026-09-18 (USER: "by default Hanji is output first, that is, Hanji is the
    /// title and romanization is the subtitle"): the stored swap is on.
    static let defaults = EngineSettings(
        inputMode: .tl,
        isHanjiFirst: true,
        candidateDisplayMode: .sideBySide,
        isLiteralRomanCandidateEnabled: false,
        syllableSeparator: .hyphen,
        isNasalMarkerUppercaseEnabled: true,
        isCustomDictEnabled: true,
        dictionarySources: .defaults,
    )
}
