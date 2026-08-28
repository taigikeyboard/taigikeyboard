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
/// One CONTROL too, since 2026-08-26: every row is a `ShortcutKeyRecorder`.
/// The global rows were `KeyboardShortcuts.Recorder` until then, and it beeps
/// at any modifier-less key before validation of ours can run
/// (`RecorderCocoa.swift:404-410`) — so 漢羅代先 could not be moved to a bare
/// `z` even though it SHIPS on a bare backtick (USER, real device). What the
/// two tiers still differ in is where the recorded key is written and what
/// each refuses on top of the shared gate, which is what `onRecord` and
/// `additionalRejection` carry.
///
/// `@AppStorage` for the settings the pane owns outright, and the store for the
/// per-action chords, whose write has to run conflict resolution first.
struct ShortcutSettingsView: View {
    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.candidateSlotModifier.name)
    private var candidateSlotKeySet = SettingsStore.Keys.candidateSlotModifier.defaultValue

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

                // The keys that move through the candidates, then the keys
                // that end the composition (`ComposingAction.groups`). No
                // other surface shows this roster: the input-source menu never
                // could, since the agent dispatches whatever it draws.
                ForEach(ComposingAction.groups[0], id: \.self) { action in
                    recorderRow(action)
                }

                // Ends the moving-through group, because that is what it does.
                // Glyphs rather than translated words: the keys are read off
                // the keyboard, and `q w d f z x v y ;`, ⇧, ⌃ and ⌥ are the same in
                // every language the settings window speaks. A picker rather
                // than a recorder because this row is one set standing for
                // nine slots, not a key.
                Picker(language.string(.macosBindingSlotModifier), selection: $candidateSlotKeySet) {
                    ForEach(CandidateSlotKeySet.allCases, id: \.self) { keySet in
                        Text(verbatim: keySet.menuLabel).tag(keySet)
                    }
                }

                ForEach(ComposingAction.groups[1], id: \.self) { action in
                    recorderRow(action)
                }
            }

            // Its own section, at the end: it acts on every row above it rather
            // than on any one of them.
            Section {
                WideActionRow(titleKey: .themeEditorResetAll, action: restoreDefaults)
            }
        }
        .formStyle(.grouped)
        .onChange(of: candidateSlotKeySet) { _, newKeySet in
            // The picker is the last writer: the keys it just claimed come off
            // any global row that held one. The recorder refuses the other
            // order, so between them no global shortcut sits on a live slot
            // key. Composing rows need no write — they are re-resolved from
            // storage on every read, and a row the new set shadows comes back
            // if the picker moves off it again.
            ShortcutConflicts.resolveGlobalRows(afterSlotKeySetChangedTo: newKeySet)
            reload()
        }
    }

    /// One global-hotkey row.
    ///
    /// The chord is read back THROUGH the cross-registry bridge rather than
    /// rendered by the library: the field speaks `ComposingKeyChord`, and the
    /// bridge is already the one translation between a Carbon key code and the
    /// character it types (`ShortcutConflicts.composingChord(occupiedBy:)`).
    private func globalRecorderRow(_ action: ShortcutAction) -> some View {
        LabeledContent(action.label(language)) {
            ShortcutKeyRecorder(
                chord: KeyboardShortcuts.getShortcut(for: action.name)
                    .flatMap(ShortcutConflicts.composingChord(occupiedBy:)),
                slotKeySet: bindings.slotKeySet,
                language: language,
                additionalRejection: GlobalShortcutPolicy.rejection(for:),
            ) { key in
                record(key?.globalShortcut, for: action)
            }
        }
    }

    private func recorderRow(_ action: ComposingAction) -> some View {
        LabeledContent(action.label(language)) {
            ShortcutKeyRecorder(
                chord: bindings.chord(for: action),
                slotKeySet: bindings.slotKeySet,
                language: language,
            ) { key in
                record(key?.chord, for: action)
            }
        }
    }

    /// Writes `shortcut` to `action`, taking it off whichever row held it.
    ///
    /// The global-tier twin of `record(_:for:)` below, and the same rule: last
    /// writer wins across both registries.
    private func record(_ shortcut: KeyboardShortcuts.Shortcut?, for action: ShortcutAction) {
        KeyboardShortcuts.setShortcut(shortcut, for: action.name)
        ShortcutConflicts.resolve(after: action)
        // The other registry, by the same rule: this recording is the last
        // writer, so a composing row holding the same key empties. Carbon
        // dispatches before the classifier ever runs, so leaving that row would
        // leave a key that reads as bound and does nothing.
        ShortcutConflicts.resolveComposingRows(after: action, in: store)
        reload()
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
