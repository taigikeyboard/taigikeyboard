// Where the six user settings are persisted, and where every reader of them —
// the engine, the settings form, the input-source menu — agrees on the keys.

import Foundation

/// The `UserDefaults` key of one setting, paired with the value used when the
/// user has never touched it.
///
/// The default is not written here: it is read from `EngineSettings.defaults`,
/// which is the domain model's own statement of what a fresh install types
/// with and is kept aligned with iOS and Android on purpose. A literal repeated
/// here would be a second place for that alignment to drift.
struct SettingsKey<Value: Sendable>: Sendable {
    let name: String
    let defaultValue: Value
}

/// Reads and writes the settings the composing engine is driven by.
///
/// Live-read by construction: `current` goes to `UserDefaults` on every access
/// and caches nothing, so a mode switched from the input-source menu applies to
/// the very next keystroke without anything having to be told about it
/// (`.claude/rules/ios-settings-injection.md` §3 — a snapshot taken at
/// construction is exactly what breaks mid-session TL↔POJ switching).
///
/// Instances cache nothing, so there is no shared one: the input-source menu
/// and the composition root each hold their own against the same defaults
/// domain, and a mode written through either is read by both.
///
/// `@unchecked Sendable` because `UserDefaults` carries no `Sendable`
/// annotation in Foundation while Apple documents the class itself as
/// thread-safe. Nothing else is stored here, so that one reference is the whole
/// of what the unchecked promise covers.
final class SettingsStore: EngineSettingsProvider, @unchecked Sendable {
    /// Key spellings are the iOS ones (`ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift:36-50`).
    /// macOS has its own defaults domain, so this is not shared storage and
    /// carries no binary-compatibility obligation — it is a deliberate
    /// cross-platform schema alignment, so that `defaults read` speaks the same
    /// vocabulary on both platforms and a future settings-transfer feature has
    /// one name per setting rather than two.
    enum Keys {
        static let inputMode = SettingsKey(
            name: "inputMode",
            defaultValue: EngineSettings.defaults.inputMode,
        )
        static let isDoubleTapOOEnabled = SettingsKey(
            name: "enableDoubleTapOO",
            defaultValue: EngineSettings.defaults.isDoubleTapOOEnabled,
        )
        static let isDoubleTapNNEnabled = SettingsKey(
            name: "enableDoubleTapNN",
            defaultValue: EngineSettings.defaults.isDoubleTapNNEnabled,
        )
        static let isTranslateSwapped = SettingsKey(
            name: "isTranslateSwapped",
            defaultValue: EngineSettings.defaults.isTranslateSwapped,
        )
        static let isOutputBothScripts = SettingsKey(
            name: "outputBothScripts",
            defaultValue: EngineSettings.defaults.isOutputBothScripts,
        )
        static let isLiteralRomanCandidateEnabled = SettingsKey(
            name: "literalRomanCandidateEnabled",
            defaultValue: EngineSettings.defaults.isLiteralRomanCandidateEnabled,
        )
        static let isFrequencyRecordingEnabled = SettingsKey(
            name: "frequencyRecordingEnabled",
            defaultValue: EngineSettings.defaults.isFrequencyRecordingEnabled,
        )
        static let isAssociationRecordingEnabled = SettingsKey(
            name: "associationRecordingEnabled",
            defaultValue: EngineSettings.defaults.isAssociationRecordingEnabled,
        )
    }

    private let userDefaults: UserDefaults

    /// The suite is injectable so a test can run against its own domain rather
    /// than the one the user's real settings live in.
    init(userDefaults: UserDefaults = .standard) {
        self.userDefaults = userDefaults
    }

    var current: EngineSettings {
        EngineSettings(
            inputMode: inputMode,
            isDoubleTapOOEnabled: bool(Keys.isDoubleTapOOEnabled),
            isDoubleTapNNEnabled: bool(Keys.isDoubleTapNNEnabled),
            isTranslateSwapped: bool(Keys.isTranslateSwapped),
            isOutputBothScripts: bool(Keys.isOutputBothScripts),
            isLiteralRomanCandidateEnabled: bool(Keys.isLiteralRomanCandidateEnabled),
            isFrequencyRecordingEnabled: bool(Keys.isFrequencyRecordingEnabled),
            isAssociationRecordingEnabled: bool(Keys.isAssociationRecordingEnabled),
        )
    }

    /// The romanization being typed. A stored value that names no mode — a
    /// hand-edited `defaults write`, or a mode a future version removes — reads
    /// as the default rather than as a mode the engine cannot render.
    var inputMode: InputMode {
        get {
            userDefaults.string(forKey: Keys.inputMode.name)
                .flatMap(InputMode.init(rawValue:))
                ?? Keys.inputMode.defaultValue
        }
        set { userDefaults.set(newValue.rawValue, forKey: Keys.inputMode.name) }
    }

    /// `object(forKey:)` rather than `bool(forKey:)`: the latter answers `false`
    /// for a key that was never written, which would silently turn every
    /// default-on setting off on a fresh install.
    private func bool(_ key: SettingsKey<Bool>) -> Bool {
        userDefaults.object(forKey: key.name) as? Bool ?? key.defaultValue
    }
}
