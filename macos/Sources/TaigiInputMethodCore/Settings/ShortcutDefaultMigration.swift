// Launch-time move of a shortcut default that changed between builds.

import Foundation
import KeyboardShortcuts

/// Moves installs still on a superseded shortcut default onto the current one.
///
/// `Name.init(initial:)` persists the default only into an install that has
/// never stored the action, so changing an `initial:` in code reaches fresh
/// installs and nobody else — an upgraded install keeps the chord the old
/// build wrote for it. This runs once per superseded default, guarded by a
/// flag: without one, a user who later deliberately records the OLD default
/// would be clobbered back on every launch.
///
/// A user who had deliberately recorded the old default is indistinguishable
/// from one who never touched the row, and is migrated too — the same
/// ambiguity `ShortcutConflicts.defaultsShadowedByRecordings` accepts, paid
/// once.
///
/// Runs before `ShortcutConflicts.resolveDefaultsShadowedByRecordings()`, so
/// a migrated-in default that collides with a chord the user recorded
/// elsewhere is resolved by the standing conflict policy.
@MainActor
enum ShortcutDefaultMigration {
    /// 切換 漢羅對調: ⌃⌘H → bare ` (USER 2026-08-21).
    private static let backtickFlagKey = "didMoveTranslateSwappedDefaultToBacktick"
    private static let supersededTranslateSwappedDefault =
        KeyboardShortcuts.Shortcut(.h, modifiers: [.control, .command])

    /// The shortcut store is injectable because the library reads and writes
    /// `UserDefaults.standard` with no suite injection: a test handing in its
    /// own `userDefaults` for the flag must not touch the real chords of
    /// whoever is running the tests.
    static func run(
        userDefaults: UserDefaults = .standard,
        shortcutFor: (ShortcutAction) -> KeyboardShortcuts.Shortcut? = {
            KeyboardShortcuts.getShortcut(for: $0.name)
        },
        setShortcut: (KeyboardShortcuts.Shortcut?, ShortcutAction) -> Void = {
            KeyboardShortcuts.setShortcut($0, for: $1.name)
        },
    ) {
        guard !userDefaults.bool(forKey: backtickFlagKey) else { return }
        // The flag is written whether or not the rewrite ran: an install
        // already off the old default has nothing to migrate, and must not be
        // revisited on a later launch either.
        userDefaults.set(true, forKey: backtickFlagKey)

        if shortcutFor(.toggleTranslateSwapped) == supersededTranslateSwappedDefault {
            setShortcut(
                ShortcutAction.toggleTranslateSwapped.defaultShortcut,
                .toggleTranslateSwapped,
            )
        }
    }
}
