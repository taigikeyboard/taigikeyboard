// The 快捷鍵 pane: the global hotkeys, and which key runs which composing action.

import KeyboardShortcuts
import SwiftUI

/// The 快捷鍵 pane of the settings window.
///
/// Two rosters, recorded by two controls, because the keys they hold are
/// different in kind. A global chord is a Carbon hotkey and always carries a
/// modifier, so `KeyboardShortcuts.Recorder` takes it. A composing key is
/// mostly bare — Return, Space, `[` — so `ComposingKeyRecorder` takes it and
/// refuses the keys a syllable is spelled with.
///
/// Defaults follow the system Zhuyin input method's candidate window wherever
/// the romanization allows, so a user arriving from that keyboard is not
/// relearning anything (`ComposingAction`). The one place they cannot: Zhuyin
/// picks candidates with bare digits, which are TL and POJ tone markers here,
/// so selection stays a modifier chord.
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

    /// Which row is listening. Owned here rather than per row so that exactly
    /// one is armed: two rows listening would install two event monitors, and
    /// whichever ran first would swallow the key.
    @State private var recordingAction: ComposingAction?

    private let store = SettingsStore()

    var body: some View {
        Form {
            Section {
                // One row per action, off the same list the hotkey registration
                // uses, so a new action cannot appear in one and not the other.
                ForEach(ShortcutAction.allCases, id: \.self) { action in
                    KeyboardShortcuts.Recorder(action.label(language), name: action.name) { _ in
                        ShortcutConflicts.resolve(after: action)
                    }
                }
            } header: {
                Text(language.string(.macosShortcutsGlobalSection))
            }

            Section {
                ForEach(ComposingAction.allCases, id: \.self) { action in
                    ComposingKeyRecorder(
                        action: action,
                        chord: bindings.chord(for: action),
                        slotModifier: bindings.slotModifier,
                        recordingAction: $recordingAction,
                    ) { chord in
                        record(chord, for: action)
                    }
                }

                // Glyphs rather than translated words: a modifier is read off
                // the keyboard, and ⌃ and ⌥ are the same symbols in every
                // language the settings window speaks. A picker rather than a
                // recorder because this row is one modifier standing for nine
                // chords, not a key.
                Picker(language.string(.macosBindingSlotModifier), selection: $candidateSlotModifier) {
                    Text(verbatim: "⌃1 – ⌃9").tag(CandidateSlotModifier.control)
                    Text(verbatim: "⌥1 – ⌥9").tag(CandidateSlotModifier.option)
                }
            } header: {
                Text(language.string(.macosShortcutsComposingSection))
            } footer: {
                Text(language.string(.macosShortcutsFixedKeysNote))
            }
        }
        .formStyle(.grouped)
        .frame(maxWidth: SettingsPaneLayout.maximumFormWidth)
        .onChange(of: candidateSlotModifier) { _, _ in reload() }
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
