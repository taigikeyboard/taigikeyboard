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
    /// One superseded default, and the flag that keeps its rewrite to a single
    /// launch.
    ///
    /// A flag PER migration, never one for the file: a shared flag would let
    /// whichever migration ran first mark the rest as done, so an install that
    /// upgraded across two of them would take only the earlier one.
    private struct Migration {
        let action: ShortcutAction
        let supersededDefault: KeyboardShortcuts.Shortcut
        let flagKey: String
    }

    /// Oldest first. Each is independent — an install may match none, one, or
    /// both — so they are checked in turn rather than short-circuited.
    private static let migrations: [Migration] = [
        // 切換 漢羅對調: ⌃⌘H → bare ` (USER 2026-08-21).
        Migration(
            action: .toggleTranslateSwapped,
            supersededDefault: KeyboardShortcuts.Shortcut(.h, modifiers: [.control, .command]),
            flagKey: "didMoveTranslateSwappedDefaultToBacktick",
        ),
        // 切換 台羅/白話字: ⌃⌘R → ⌃⌘C (USER 2026-08-25). R was the mnemonic
        // for "romanization", but this switch is reached for all day and is
        // muscle memory by the second day — what is left is how far the hand
        // travels, and ⌃, ⌘ and C are all on the bottom row while R is two
        // rows up with the pinky still anchored.
        Migration(
            action: .toggleRomanization,
            supersededDefault: KeyboardShortcuts.Shortcut(.r, modifiers: [.control, .command]),
            flagKey: "didMoveRomanizationDefaultToControlCommandC",
        ),
    ]

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
        for migration in migrations {
            guard !userDefaults.bool(forKey: migration.flagKey) else { continue }
            // The flag is written whether or not the rewrite ran: an install
            // already off the old default has nothing to migrate, and must not
            // be revisited on a later launch either.
            userDefaults.set(true, forKey: migration.flagKey)

            if shortcutFor(migration.action) == migration.supersededDefault {
                setShortcut(migration.action.defaultShortcut, migration.action)
            }
        }
    }
}
