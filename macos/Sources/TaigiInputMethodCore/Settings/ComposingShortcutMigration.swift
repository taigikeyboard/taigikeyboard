// Folds the first shipped composing-key settings into the per-action chords.

import AppKit

/// Carries a user's composing-key choices across the change from "what does
/// this key do" to "which key does this action".
///
/// The first shape shipped five behaviour settings — `returnKeyBehavior`,
/// `spaceKeyBehavior`, `bracketPagingBehavior`, `tabCycleBehavior` and
/// `candidateSlotModifier`. Only the last of them survives as itself; the other
/// four described a key's job, which is now recorded the other way round.
///
/// Runs once, behind `composingShortcutSchema`, rather than every launch: a
/// re-derivation would keep overwriting whatever the user recorded afterwards
/// every time a downgraded build or an external tool wrote an old key back.
///
/// A setting is migrated only when it was actually STORED. `object(forKey:)`
/// rather than a typed reader, because a reader answers the default for an
/// absent key and there would be no way to tell "chose the default" from "never
/// opened the pane" — and the two want different outcomes here: a stored choice
/// is carried, an absent one gives way to the new Zhuyin-parity defaults.
@MainActor
enum ComposingShortcutMigration {
    /// The schema this build writes once it has folded the old keys in.
    static let currentSchema = 1

    private static let returnKeyName = "returnKeyBehavior"
    private static let spaceKeyName = "spaceKeyBehavior"
    private static let bracketPagingName = "bracketPagingBehavior"
    private static let tabCycleName = "tabCycleBehavior"

    private static let retiredKeyNames = [returnKeyName, spaceKeyName, bracketPagingName, tabCycleName]

    static func run(userDefaults: UserDefaults = .standard) {
        let schemaKey = SettingsStore.Keys.composingShortcutSchema.name
        guard userDefaults.integer(forKey: schemaKey) < currentSchema else { return }

        migrateReturnKey(userDefaults)
        migrateSpaceKey(userDefaults)
        migrateBracketPaging(userDefaults)
        migrateTabCycle(userDefaults)

        for name in retiredKeyNames {
            userDefaults.removeObject(forKey: name)
        }
        userDefaults.set(currentSchema, forKey: schemaKey)
    }

    /// `confirmHighlighted` was Return-commits-the-candidate with ⇧Return kept
    /// as the literal escape hatch — which is exactly the new default pair, so
    /// it needs nothing written. `commitLiteral` was Return-commits-the-literal,
    /// so that user gets Return on the literal and the candidate on ⇧Return:
    /// the same two keys, swapped, rather than one of them going quiet.
    private static func migrateReturnKey(_ userDefaults: UserDefaults) {
        guard userDefaults.object(forKey: returnKeyName) as? String == "commitLiteral" else {
            return
        }
        write(.commitLiteral, key: "\r", modifiers: [], to: userDefaults)
        write(.confirmHighlighted, key: "\r", modifiers: .shift, to: userDefaults)
    }

    /// `nextCandidate` is the new default. A user who kept Space on the
    /// candidate commit gets Space on 確定輸出, and the literal moves off
    /// ⇧Return only if the Return migration above did not already place it.
    private static func migrateSpaceKey(_ userDefaults: UserDefaults) {
        guard userDefaults.object(forKey: spaceKeyName) as? String == "confirmHighlighted" else {
            return
        }
        write(.confirmHighlighted, key: " ", modifiers: [], to: userDefaults)
    }

    /// Paging was a single on/off over both brackets, and off means both rows
    /// end up empty — Page Up and Page Down still page, as they always do.
    private static func migrateBracketPaging(_ userDefaults: UserDefaults) {
        guard userDefaults.object(forKey: bracketPagingName) as? String == "disabled" else {
            return
        }
        clear(.pageForward, to: userDefaults)
        clear(.pageBackward, to: userDefaults)
    }

    /// Tab cycling was off by default, so only the user who turned it ON has
    /// something to carry: Tab walks forward, ⇧Tab back.
    private static func migrateTabCycle(_ userDefaults: UserDefaults) {
        guard userDefaults.object(forKey: tabCycleName) as? String == "enabled" else { return }
        write(.nextCandidate, key: "\t", modifiers: [], to: userDefaults)
        write(.previousCandidate, key: "\u{19}", modifiers: [], to: userDefaults)
    }

    private static func write(
        _ action: ComposingAction,
        key: String,
        modifiers: NSEvent.ModifierFlags,
        to userDefaults: UserDefaults,
    ) {
        guard case let .success(chord) = ComposingKeyChord.make(key: key, modifiers: modifiers) else {
            return
        }
        userDefaults.set(chord.rawValue, forKey: action.settingsKeyName)
    }

    private static func clear(_ action: ComposingAction, to userDefaults: UserDefaults) {
        userDefaults.set(SettingsStore.Keys.clearedComposingChord, forKey: action.settingsKeyName)
    }
}
