// Launch-time removal of persisted state for settings retired from the macOS UI.

import Foundation
import KeyboardShortcuts

/// Clears what older builds persisted for settings this build no longer
/// exposes, so an upgraded install behaves like a fresh one. Idempotent, and
/// run every launch rather than behind a version flag: removing an absent key
/// is free, and re-clearing also catches a manual `defaults write` that would
/// otherwise resurrect hidden state.
///
/// The engine still reads the four recording/output settings keys — they are
/// cross-platform contract, and the composing carrier encodes them either way —
/// so a value stored by a build that HAD the toggle would silently outlive the
/// UI that set it. Removing the stored values returns each to its default:
/// 括號標注 and 顯示羅馬字候選 back off, 詞頻紀錄 and 詞關聯紀錄 back on.
@MainActor
enum RetiredSettingsCleanup {
    /// Raw `KeyboardShortcuts.Name`s of the retired hotkey actions, kept so a
    /// stored chord can be cleared: the library persists chords by raw name,
    /// an orphaned chord would spring back to life on any future action that
    /// reused the name, and `ShortcutConflicts` only sees the live roster.
    /// Declared with no `initial:`, or the construction below would seed the
    /// very chord this is here to clear.
    private static let retiredShortcutNames = [
        KeyboardShortcuts.Name("toggleBothScripts"),
        KeyboardShortcuts.Name("toggleLiteralRomanCandidate"),
        // 開啟設定, retired 2026-08-24 when every pane got a row of its own:
        // a chord for "whichever pane was last used" is a second key for what
        // 一般 does. Unlike the two above this one shipped, so real installs
        // hold ⌃⇧, — or whatever the user recorded over it.
        KeyboardShortcuts.Name("openSettings"),
    ]

    /// Sidebar panes removed from `SettingsPane`. Cleared explicitly rather
    /// than trusting `@AppStorage` to shrug off an unknown raw value — that
    /// fallback is framework behaviour this codebase has not pinned.
    ///
    /// An explicit tombstone list, NOT "any value that is not a live case": an
    /// unknown raw value is not necessarily a retired one, and a build that
    /// cleared everything it did not recognise would reset the selection of
    /// anyone who also runs a newer build against the same defaults.
    private static let retiredPaneRawValues: Set<String> = [
        "dictionarySearch",
        "frequencyData",
        "associationData",
        "backupRestore",
    ]

    /// Retired raw defaults names, grouped by the round that retired them.
    ///
    /// Hygiene, not a behaviour fix: nothing reads any of them any more, so a
    /// stored value already changes nothing. They are cleared to keep the
    /// domain honest, and because a later setting reusing one of these names
    /// would inherit a value nobody chose for it. None are migrated — each
    /// described a choice this build no longer offers.
    private static let retiredDefaultsNames = [
        // The first shape of the composing-key settings, which described what
        // a key does rather than which key does a job (`ComposingAction`).
        // The shape they replaced never shipped in a release, so the only
        // installs that can hold one are builds from main.
        "returnKeyBehavior",
        "spaceKeyBehavior",
        "bracketPagingBehavior",
        "tabCycleBehavior",
        // The 外觀 pane's two retired rows — the reasoning for each lives on
        // its type, `CandidateAccentColor` and `CandidateWindowStyle`.
        "candidateAccentColor",
        "candidateWindowStyle",
        // 全形標點 and Shift 切換英數, retired 2026-08-24: both are always on
        // now, so the stored value is inert either way and this only keeps
        // the domain honest.
        "fullWidthPunctuationEnabled",
        "shiftTogglesAlphanumericEnabled",
    ]

    /// Raw values of composing actions removed from the roster: 直接送出漢字 and
    /// 直接送出羅馬字, retired 2026-08-25 when the 漢羅對調 switch was left as the
    /// one place a user chooses which script a commit writes.
    ///
    /// Their chords are stored under `ComposingAction.settingsKeyName`, which
    /// only live cases can build — `resetComposingShortcuts` walks `allCases` —
    /// so a chord recorded on one of these rows is unreachable by every row the
    /// pane still draws. Kept as raw values and composed through the live
    /// derivation, so the namespace keeps one owner.
    private static let retiredComposingActionRawValues = [
        "commitHanji",
        "commitRomanization",
    ]

    static func run(userDefaults: UserDefaults = .standard) {
        userDefaults.removeObject(forKey: SettingsStore.Keys.isOutputBothScripts.name)
        userDefaults.removeObject(forKey: SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)
        // The 詞頻紀錄 / 詞關聯紀錄 toggles went with the panes that carried
        // them. Unlike the retired names below this changes BEHAVIOUR rather
        // than only tidying: `SettingsStore.current` still reads both keys, so
        // a `false` stored by a build that HAD the toggles would keep learning
        // switched off with nothing left to switch it back on. Clearing them
        // returns both to their `true` defaults.
        //
        // Launch-time, and the read is live, so this restores the default once
        // per launch rather than making the key unreachable — a deliberate
        // `defaults write` still takes effect for the rest of that session.
        // Closing that would mean not reading these from defaults at all, which
        // is a wider change than this round: the two keys beside them
        // (`isOutputBothScripts`, `isLiteralRomanCandidateEnabled`) are read
        // live on purpose — see `CandidateDocumentText`.
        userDefaults.removeObject(forKey: SettingsStore.Keys.isFrequencyRecordingEnabled.name)
        userDefaults.removeObject(forKey: SettingsStore.Keys.isAssociationRecordingEnabled.name)
        for name in retiredDefaultsNames {
            userDefaults.removeObject(forKey: name)
        }
        for rawValue in retiredComposingActionRawValues {
            userDefaults.removeObject(forKey: ComposingAction.settingsKeyName(rawValue: rawValue))
        }
        if let pane = userDefaults.string(forKey: SettingsStore.Keys.selectedSettingsPane.name),
           retiredPaneRawValues.contains(pane) {
            userDefaults.removeObject(forKey: SettingsStore.Keys.selectedSettingsPane.name)
        }
        for name in retiredShortcutNames {
            KeyboardShortcuts.setShortcut(nil, for: name)
        }
    }
}
