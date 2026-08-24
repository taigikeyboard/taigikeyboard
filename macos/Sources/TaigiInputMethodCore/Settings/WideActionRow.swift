// A settings row that is one action across the whole width of the pane.

import SwiftUI

/// A form row whose action applies to everything above it, drawn as tinted text
/// across the full width rather than as a push button (USER 2026-08-24).
///
/// A push button borders a control that shares its row with something else — a
/// label on the left, a value it acts on. These rows have no second half: they
/// end a section that is already about one subject, so the border would be
/// drawing a boundary that is not there. What is left carries the meaning: the
/// tint says the row acts, and the width says it acts on all of it.
///
/// Still a `Button`, so it is read out as one and takes a click the way one
/// does; only its drawing is the pane's own.
struct WideActionRow: View {
    @Environment(DisplayLanguageStore.self) private var language

    let titleKey: StringKey
    /// `.destructive` colours the row red, the way the system marks an action
    /// that takes something away. Everything else reads as the accent.
    var role: ButtonRole?
    let action: () -> Void

    var body: some View {
        Button(role: role, action: action) {
            Text(language.string(titleKey))
                .frame(maxWidth: .infinity)
                .foregroundStyle(role == .destructive ? Color.red : Color.accentColor)
                // The label is the whole row, so the row is what takes the
                // click: a hit area left at the text's own width would leave
                // most of the row dead.
                .contentShape(.rect)
        }
        // The row draws its own tint, which a bordered style would override.
        .buttonStyle(.plain)
    }
}
