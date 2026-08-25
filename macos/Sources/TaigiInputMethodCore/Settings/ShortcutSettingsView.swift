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
                // One row per global action, off the same list the hotkey
                // registration uses, so a new action cannot appear in one and
                // not the other.
                //
                // One group, since 2026-08-25. There were two — the keys that
                // opened a settings pane, then the keys that change what the
                // user is typing — until the five pane chords were retired
                // (USER: five chords for panes visited about once a day, which
                // the menu bar already lists by name). What is left is one
                // doorway and two switches, which is not two groups' worth.
                ForEach(ShortcutAction.allCases, id: \.self) { action in
                    globalRecorderRow(action)
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
            } header: {
                Text(language.string(.macosKeyboardActionsSection))
            }

            // Its own section, at the end: it acts on every row above it rather
            // than on any one of them.
            Section {
                WideActionRow(titleKey: .themeEditorResetAll, action: restoreDefaults)
            }
        }
        .formStyle(.grouped)
        .onChange(of: candidateSlotModifier) { _, newModifier in
            // The picker is the last writer: the nine chords it just claimed
            // come off any global row that held one. The recorder refuses the
            // other order, so between them no global shortcut sits on a live
            // slot chord.
            ShortcutConflicts.resolveGlobalRows(afterSlotModifierChangedTo: newModifier)
            reload()
        }
    }

    /// One global-hotkey row. Shared by both groups, so the conflict rules
    /// below cannot come to differ between them.
    private func globalRecorderRow(_ action: ShortcutAction) -> some View {
        KeyboardShortcuts.Recorder(action.label(language), name: action.name) { _ in
            ShortcutConflicts.resolve(after: action)
            // The other registry, by the same rule: this recording is the last
            // writer, so a composing row holding the same key empties. Carbon
            // dispatches before the classifier ever runs, so leaving that row
            // would leave a key that reads as bound and does nothing.
            ShortcutConflicts.resolveComposingRows(after: action, in: store)
            reload()
        }
        // The nine candidate-slot chords are the one thing this recorder
        // refuses rather than resolves: the slot tier is a picker, not a row,
        // so it has nothing to empty.
        .shortcutValidation { shortcut in
            guard ShortcutConflicts.isSlotChord(shortcut, under: bindings.slotModifier)
            else { return .allow }
            // The same words the composing recorder refuses with.
            return .disallow(reason: language.string(.macosShortcutRejectedSlotChord))
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

    /// Puts every row on this pane back to the key it ships with.
    ///
    /// Both registries at once, for the reason the pane is one list to begin
    /// with: which rows register a Carbon hotkey and which are read by the key
    /// classifier is not a distinction the reader makes, so a button that
    /// restored half of them would leave rows it visibly did not touch. It is
    /// also the only way home for a modifierless default — the library's
    /// recorder refuses to record one, so nothing else can put the bare
    /// backtick back on 漢羅對調.
    ///
    /// Told apart by how each half stores a default: this side removes the
    /// stored value so the row reads as never touched, while
    /// `KeyboardShortcuts.reset` writes each name's initial shortcut back into
    /// its own registry. The user-visible outcome is the same; only the first
    /// keeps a later version free to change what the default is.
    ///
    /// No conflict resolution afterwards. The three tiers' shipped defaults
    /// hold no chord in common — `ShortcutDefaultsTests` pins that — and
    /// running the resolver anyway would be worse than redundant: on a future
    /// collision it would quietly empty one of the rows this button had just
    /// restored, so the button would stop meaning "the defaults".
    private func restoreDefaults() {
        store.resetComposingShortcuts()
        KeyboardShortcuts.reset(ShortcutAction.allCases.map(\.name))
        reload()
    }

    /// Re-reads the resolved bindings, which is also what puts an always-bound
    /// action's default back after the user clears its row.
    private func reload() {
        bindings = store.composingKeyBindings
    }
}
