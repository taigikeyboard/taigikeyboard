// The 快捷鍵 pane: every key the user can put an action on, in one list.

import KeyboardShortcuts
import SwiftUI

/// The 快捷鍵 pane of the settings window.
///
/// One list, not two. A shortcut is a shortcut to the user reading the pane;
/// which of them registers a Carbon hotkey and which is read by the key
/// classifier is an implementation detail, and splitting the rows on it made
/// the reader ask what the split meant (USER 2026-08-21).
///
/// Two controls all the same, because the keys they hold differ: a global chord
/// always carries a modifier, so `KeyboardShortcuts.Recorder` records it, while
/// a composing key is mostly bare — Return, Space, `[` — so
/// `ComposingKeyRecorder` does. Both are recording fields of the same size and
/// shape, so the seam does not show.
///
/// `@AppStorage` for the settings the pane owns outright, and the store for the
/// per-action chords, whose write has to run conflict resolution first.
struct ShortcutSettingsView: View {
    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.candidateSlotModifier.name)
    private var candidateSlotModifier = SettingsStore.Keys.candidateSlotModifier.defaultValue

    /// Re-read after every write so the rows repaint together: recording a
    /// chord can empty the row that had it.
    @State private var bindings = SettingsStore().composingKeyBindings

    private let store = SettingsStore()

    var body: some View {
        Form {
            Section {
                // One row per action, off the same list the hotkey registration
                // uses, so a new action cannot appear in one and not the other.
                ForEach(ShortcutAction.allCases, id: \.self) { action in
                    KeyboardShortcuts.Recorder(action.label(language), name: action.name) { _ in
                        ShortcutConflicts.resolve(after: action)
                        // The other registry, by the same rule: this recording
                        // is the last writer, so a composing row holding the
                        // same key empties. Carbon dispatches before the
                        // classifier ever runs, so leaving that row would leave
                        // a key that reads as bound and does nothing.
                        ShortcutConflicts.resolveComposingRows(after: action, in: store)
                        reload()
                    }
                    // The nine candidate-slot chords are the one thing this
                    // recorder refuses rather than resolves: the slot tier is a
                    // picker, not a row, so it has nothing to empty.
                    .shortcutValidation { shortcut in
                        guard ShortcutConflicts.isSlotChord(shortcut, under: bindings.slotModifier)
                        else { return .allow }
                        // The same words the composing recorder refuses with.
                        return .disallow(reason: language.string(.macosShortcutRejectedSlotChord))
                    }
                }

                // Same order the input-source menu draws, so a user who learnt
                // the roster in one surface reads it in the other: the keys
                // that move through the candidates, then the keys that end the
                // composition (`ComposingAction.groups`).
                ForEach(ComposingAction.groups[0], id: \.self) { action in
                    recorderRow(action)
                }

                // Ends the moving-through group, because that is what it does.
                // Glyphs rather than translated words: a modifier is read off
                // the keyboard, and ⌃ and ⌥ are the same symbols in every
                // language the settings window speaks. A picker rather than a
                // recorder because this row is one modifier standing for nine
                // chords, not a key.
                Picker(language.string(.macosBindingSlotModifier), selection: $candidateSlotModifier) {
                    ForEach(CandidateSlotModifier.allCases, id: \.self) { modifier in
                        Text(verbatim: modifier.menuRange).tag(modifier)
                    }
                }

                ForEach(ComposingAction.groups[1], id: \.self) { action in
                    recorderRow(action)
                }
            }
        }
        .formStyle(.grouped)
        .frame(maxWidth: SettingsPaneLayout.maximumFormWidth)
        .onChange(of: candidateSlotModifier) { _, newModifier in
            // The picker is the last writer: the nine chords it just claimed
            // come off any global row that held one. The recorder refuses the
            // other order, so between them no global shortcut sits on a live
            // slot chord.
            ShortcutConflicts.resolveGlobalRows(afterSlotModifierChangedTo: newModifier)
            reload()
        }
    }

    private func recorderRow(_ action: ComposingAction) -> some View {
        LabeledContent(action.label(language)) {
            ComposingKeyRecorder(
                chord: bindings.chord(for: action),
                slotModifier: bindings.slotModifier,
                language: language,
            ) { chord in
                record(chord, for: action)
            }
        }
    }

    /// Writes `chord` to `action`, taking it off whichever row held it.
    ///
    /// Last writer wins, and the loser's row visibly empties — the same rule
    /// the global recorders use (`ShortcutConflicts`), and the one the System
    /// Settings keyboard pane behaves by.
    private func record(_ chord: ComposingKeyChord?, for action: ComposingAction) {
        if let chord {
            for loser in bindings.actionsHolding(chord, excluding: action) {
                store.setComposingChord(nil, for: loser)
            }
            // And across the seam, same rule: a global shortcut on this key
            // would fire instead of the row just recorded.
            ShortcutConflicts.resolveGlobalRows(after: chord)
        }
        store.setComposingChord(chord, for: action)
        reload()
    }

    /// Re-reads the resolved bindings, which is also what puts an always-bound
    /// action's default back after the user clears its row.
    private func reload() {
        bindings = store.composingKeyBindings
    }
}
