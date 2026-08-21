// Draws the input-source menu: what each row does on the left, the key it answers to on the right.

import AppKit

/// One row of the input-source menu, before it is drawn.
///
/// A key equivalent here is not a display device: the menu is vended to the
/// system's text-input menu agent, which dispatches key equivalents even while
/// the menu is CLOSED — and it canonicalizes ANY key-equivalent string back to
/// a functional key for both display and matching, so whatever the key column
/// can draw, typing can trigger (probes 2026-08-21: ↩ U+21A9, ␠ U+2420 and
/// char+word-joiner all dispatched on the real key). Only a row that IS a
/// shortcut may claim one — the global actions, whose modifier-laden chords
/// are meant to fire anywhere. A composing key must not: its keys are mostly
/// bare (Return, Space, `[`), and a bare Return claimed here sent every
/// mid-composition Enter to the settings window instead of committing.
///
/// The rows that cannot claim one show no key at all (USER decision
/// 2026-08-21): every display-only channel is dropped by the agent
/// (attributedTitle, custom view, image, badge, subtitle), and key text
/// appended to the plain title was rejected as reading badly. Those rows lead
/// to the shortcut pane, where the keys are.
struct InputSourceMenuRow: Sendable {
    let label: String
    /// The key AppKit lays out for this row, empty when the row has no chord.
    let keyEquivalent: String
    let modifiers: NSEvent.ModifierFlags
    let action: Selector

    init(
        label: String,
        keyEquivalent: String = "",
        modifiers: NSEvent.ModifierFlags = [],
        action: Selector,
    ) {
        self.label = label
        self.keyEquivalent = keyEquivalent
        self.modifiers = modifiers
        self.action = action
    }
}

/// Turns rows into the menu `IMKInputController.menu()` hands back.
///
/// Nonisolated, like `IMKInputController.menu()` itself: it reads no actor state,
/// and `NSMenu` cannot cross a main-actor hop as a return value.
enum InputSourceMenuRenderer {
    /// The menu, with a rule between each pair of groups and none at either
    /// end — an empty group draws no rule at all.
    static func menu(_ groups: [[InputSourceMenuRow]]) -> NSMenu {
        let menu = NSMenu()
        // The rows set their own state, and automatic validation would
        // second-guess it (`references/MacishType/macos/MacishType/InputController.swift:27-29`).
        menu.autoenablesItems = false

        for (index, group) in groups.filter({ !$0.isEmpty }).enumerated() {
            if index > 0 {
                menu.addItem(.separator())
            }
            for row in group {
                menu.addItem(item(for: row))
            }
        }
        return menu
    }

    private static func item(for row: InputSourceMenuRow) -> NSMenuItem {
        let item = NSMenuItem(title: row.label, action: row.action, keyEquivalent: row.keyEquivalent)
        // Always assigned: an unset mask is ⌘, so a bare Space would draw — and
        // answer to — ⌘Space.
        item.keyEquivalentModifierMask = row.modifiers
        return item
    }
}
