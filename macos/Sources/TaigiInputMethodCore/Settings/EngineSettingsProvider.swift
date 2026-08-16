// Where a composing operation reads its settings from, one snapshot at a time.

import Foundation

/// Live read, not a stored snapshot. The composing session outlives any single
/// settings change — the user can switch TL↔POJ while an input session is open
/// — so the manager asks for the current values at the start of each operation
/// rather than capturing them once at construction.
///
/// Mirrors the iOS contract in `.claude/rules/ios-settings-injection.md` §2,
/// minus `addChangeListener`: nothing on macOS observes settings through this
/// protocol. The settings form observes `UserDefaults` itself through
/// `@AppStorage`, and everything else reads `current` when it needs it, so a
/// listener registry here would have no listeners to serve.
protocol EngineSettingsProvider: AnyObject, Sendable {
    var current: EngineSettings { get }
}
