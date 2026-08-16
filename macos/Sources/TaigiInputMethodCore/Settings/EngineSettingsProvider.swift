// Where a composing operation reads its settings from, one snapshot at a time.

import Foundation

/// Live read, not a stored snapshot. The composing session outlives any single
/// settings change — the user can switch TL↔POJ while an input session is open
/// — so the manager asks for the current values at the start of each operation
/// rather than capturing them once at construction.
///
/// Mirrors the iOS contract in `.claude/rules/ios-settings-injection.md` §2,
/// minus `addChangeListener`: macOS has nothing observing settings until the
/// settings window lands, and a listener registry with no listeners is a
/// registry that will be written twice.
protocol EngineSettingsProvider: AnyObject, Sendable {
    var current: EngineSettings { get }
}

/// Serves the shipped defaults. The `UserDefaults`-backed implementation
/// arrives with the settings window, which is what will define the key names;
/// defining them here first would pre-commit that schema to no UI.
final class DefaultEngineSettingsProvider: EngineSettingsProvider {
    var current: EngineSettings {
        .defaults
    }
}
