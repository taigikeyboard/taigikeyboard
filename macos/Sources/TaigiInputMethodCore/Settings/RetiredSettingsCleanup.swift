// Launch-time removal of persisted state for settings retired from the macOS UI.

import Foundation

/// Clears what older builds persisted for settings this build no longer
/// exposes, so an upgraded install behaves like a fresh one. Idempotent, and
/// run every launch rather than behind a version flag: removing an absent key
/// is free, and re-clearing also catches a manual `defaults write` that would
/// otherwise resurrect hidden state.
///
/// Nothing reads a retired key any more, so every sweep here is hygiene: it
/// keeps the defaults domain honest and stops a later setting that reused a
/// name from inheriting a value nobody chose for it.
///
/// A tombstone belongs here only for a name a released macOS build (the first
/// was `macos-v3.6.5`) actually wrote. A setting retired before it ever
/// shipped left nothing on a user's Mac to clear.
@MainActor
enum RetiredSettingsCleanup {
    /// Retired raw defaults names, grouped by the round that retired them.
    ///
    /// Hygiene, not a behaviour fix: nothing reads any of them any more, so a
    /// stored value already changes nothing. They are cleared to keep the
    /// domain honest, and because a later setting reusing one of these names
    /// would inherit a value nobody chose for it. None are migrated — each
    /// described a choice this build no longer offers.
    private static let retiredDefaultsNames = [
        // Candidate Selection Keys, retired 2026-09-08: the slot key set is derived from
        // `toneInputScheme` now, and the ⇧ / ⌃ / ⌥ digit sets this key could
        // name no longer exist to migrate to.
        "candidateSlotModifier",
        // Association Records, retired 2026-09-25 when the engine dropped
        // `AppConfig.is_association_recording_enabled`: association recording
        // is always on, so a stored value (a `false` left by an older build or
        // a hand edit) is inert now.
        "associationRecordingEnabled",
        // Annotate in Brackets and Frequency Records, retired 2026-09-30: both
        // toggles shipped in v3.6.5, and nothing reads the keys any more — the
        // desktop never brackets, and the engine counts every pick
        // (`RecordUsage` tag 4 reserved). Mobile keeps its own Annotate in
        // Brackets key and UI.
        "outputBothScripts",
        "frequencyRecordingEnabled",
    ]

    /// The No Hyphens switch, retired 2026-10-06 when the Syllable Separator
    /// picker replaced it (`behavioral-invariants.md` §49). Unlike the names
    /// above it described a choice this build still offers, so a stored `true`
    /// carries over as `none` first — unless the picker already holds a choice.
    /// desktop-core twin: `SettingsDocument::from_json`.
    static let retiredHyphenlessRomanName = "hyphenlessRomanEnabled"

    static func run(userDefaults: UserDefaults = .standard) {
        carryOverHyphenlessRoman(userDefaults: userDefaults)
        for name in retiredDefaultsNames {
            userDefaults.removeObject(forKey: name)
        }
    }

    private static func carryOverHyphenlessRoman(userDefaults: UserDefaults) {
        guard let stored = userDefaults.object(forKey: retiredHyphenlessRomanName) else { return }
        let separatorName = SettingsStore.Keys.syllableSeparator.name
        if stored as? Bool == true, userDefaults.object(forKey: separatorName) == nil {
            userDefaults.set(SyllableSeparator.noSeparator.rawValue, forKey: separatorName)
        }
        userDefaults.removeObject(forKey: retiredHyphenlessRomanName)
    }
}
