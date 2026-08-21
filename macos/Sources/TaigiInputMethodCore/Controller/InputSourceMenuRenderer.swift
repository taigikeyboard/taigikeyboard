// Draws the input-source menu: what each row does on the left, the key it answers to on the right.

import AppKit

/// One row of the input-source menu, before it is drawn.
///
/// A key equivalent here is not a display device: the menu is vended to the
/// system's text-input menu agent, which dispatches key equivalents even while
/// the menu is CLOSED. Only a row that IS a shortcut may claim one — the global
/// actions, whose modifier-laden chords are meant to fire anywhere. A composing
/// key must not: its keys are mostly bare (Return, Space, `[`), and a bare
/// Return claimed here sent every mid-composition Enter to the settings window
/// instead of committing (real device, 2026-08-21).
///
/// The rows that cannot claim one print their key inside the plain title
/// (`keyTextInTitle`) instead. Right-aligned-and-dimmed layout is thereby lost
/// for them: an `attributedTitle` does not survive the trip to the agent — it
/// falls back to the plain title, which lays out as-is (dogfood 2026-08-21) —
/// so hand-spacing in the title is all that is left.
struct InputSourceMenuRow: Sendable {
    let label: String
    /// The key AppKit lays out for this row, empty when the row has no chord.
    let keyEquivalent: String
    let modifiers: NSEvent.ModifierFlags
    /// The key printed after the label instead, for the rows that must not
    /// claim a key equivalent — and for the slot row, whose key is a range of
    /// nine chords rather than a key AppKit could lay out.
    let keyTextInTitle: String
    let action: Selector

    init(
        label: String,
        keyEquivalent: String = "",
        modifiers: NSEvent.ModifierFlags = [],
        keyTextInTitle: String = "",
        action: Selector,
    ) {
        self.label = label
        self.keyEquivalent = keyEquivalent
        self.modifiers = modifiers
        self.keyTextInTitle = keyTextInTitle
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
        let item = NSMenuItem(title: title(row), action: row.action, keyEquivalent: row.keyEquivalent)
        // Always assigned: an unset mask is ⌘, so a bare Space would draw — and
        // answer to — ⌘Space.
        item.keyEquivalentModifierMask = row.modifiers
        return item
    }

    /// Two spaces rather than a tab: a plain menu title is laid out as-is, so
    /// the rows that cannot use a key equivalent are spaced by hand.
    private static func title(_ row: InputSourceMenuRow) -> String {
        row.keyTextInTitle.isEmpty ? row.label : "\(row.label)  \(row.keyTextInTitle)"
    }
}
