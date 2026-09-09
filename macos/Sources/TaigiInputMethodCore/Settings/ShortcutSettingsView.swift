// The 快捷鍵 pane: every key the user can put an action on, in three blocks.

import AppKit
import KeyboardShortcuts
import SwiftUI

/// The 快捷鍵 pane of the settings window.
///
/// Three blocks, by what a key DOES: 選字, the keys that move through the
/// candidates; 拍字, the keys that end the composition plus the switch that
/// says which script they write; and 其他, the rest of the switches and the
/// windows a key raises. Titled since 2026-09-10 (USER) — the blocks shipped
/// headerless the same day, and a header is what tells a reader which of the
/// three a key they are hunting for lives in.
///
/// Not the split the pane had until 2026-08-21, which was two blocks divided
/// on which rows register a Carbon hotkey and which the key classifier reads.
/// That is an implementation detail, and a reader asked what it meant (USER).
/// The last boundary here falls in the same place — the switches are exactly
/// the actions with a global chord — but the first two blocks are both the
/// classifier's, so the boundaries as a set follow the actions rather than
/// the registries.
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
/// No row writes its own chord: each recording goes through a `record`
/// handler, which resolves the conflicts it creates across BOTH registries
/// before the pane re-reads what that left. The two registries are the
/// composing bindings in `SettingsStore` and the global chords in
/// `KeyboardShortcuts`.
struct ShortcutSettingsView: View {
    @Environment(DisplayLanguageStore.self) private var language

    /// Re-read after every write so the rows repaint together: recording a
    /// chord can empty the row that had it.
    @State private var bindings = SettingsStore().composingKeyBindings

    private let store = SettingsStore()

    var body: some View {
        Form {
            // Block one: through the candidates.
            //
            // Typing order twice over: the blocks in the order a user meets
            // them, and the rows inside each in their roster's own order
            // (USER 2026-09-09) — so a row cannot move here without moving in
            // the roster itself.
            //
            // The candidate-slot keys have no row anywhere on this pane: they
            // follow from the 聲調拍法 picker on the 一般 pane
            // (`ToneInputScheme`), so the two halves of the key contract
            // cannot be set apart.
            Section {
                ForEach(ComposingAction.groups[0], id: \.self) { action in
                    recorderRow(action)
                }

                // Shown, not recordable (USER 2026-09-09): the caret inside the
                // composition rides the host's own word-jump chord, and the
                // classifier reads it before any binding (`ComposingKeyIntent`).
                // With the candidate movers, because moving the caret is what
                // it is — the greyed field is what tells it from the rows that
                // record.
                LabeledContent(language.string(.desktopShortcutMoveComposingCaret)) {
                    Text(Self.caretChordsLabel)
                        .foregroundStyle(.secondary)
                }
            } header: {
                Text(language.string(.desktopShortcutSectionCandidateSelection))
            }

            // Block two: out of the composition and into the document.
            Section {
                ForEach(ComposingAction.groups[1], id: \.self) { action in
                    recorderRow(action)
                }

                // Shown, not recordable (USER 2026-09-10): ⇧ on a slot key is
                // the 漢羅 commit aimed at that slot, and the slot keys follow
                // the tone scheme — so the row follows it too, and there is
                // nothing to record. After the commit rows, because it is one.
                LabeledContent(language.string(.desktopShortcutCommitAlternateScriptInSlot)) {
                    Text(Self.shiftedSlotKeysLabel(bindings.slotKeySet))
                        .foregroundStyle(.secondary)
                }

                // A global row inside the composing block (USER 2026-09-10):
                // 輸出漢字/羅馬字 says which script every commit above it
                // writes, so it belongs with them rather than among the
                // switches. Last of the block, after the two rows that flip
                // the script for one commit — the standing switch follows the
                // temporary ones.
                ForEach(ShortcutAction.groups[0], id: \.self) { action in
                    globalRecorderRow(action)
                }
            } header: {
                Text(language.string(.desktopShortcutSectionTyping))
            }

            // Block three: the remaining switches, and the windows a key
            // raises. What these have in common is that none of them needs a
            // composition running. Not their modifiers: 漢羅對調 ships on a
            // bare backtick, so ⌃⌘ names no boundary here. Nor dispatch: the
            // symbol picker is on this list and registers no Carbon hotkey
            // (`ShortcutAction.firesFromTheKeyPath`). Nor the registry — the
            // block was the whole global roster until 2026-09-10, when
            // 輸出漢字/羅馬字 moved up to the commits it describes, so the
            // roster now spans two blocks and only `ShortcutAction.groups`
            // says which.
            //
            // One row per global action across the two groups, off a roster
            // a test pins against the list the hotkey registration uses, so a
            // new action cannot appear in one and not the other.
            //
            // One block, since 2026-08-25. There were two — the keys that
            // opened a settings pane, then the keys that change what the
            // user is typing — until the five pane chords were retired
            // (USER: five chords for panes visited about once a day, which
            // the menu bar already lists by name).
            Section {
                ForEach(ShortcutAction.groups[1], id: \.self) { action in
                    globalRecorderRow(action)
                }
            } header: {
                Text(language.string(.desktopShortcutSectionOther))
            }

            // Its own section, at the end: it acts on every block above it
            // rather than on any one row.
            Section {
                WideActionRow(titleKey: .themeEditorResetAll, action: restoreDefaults)
            }
        }
        .formStyle(.grouped)
    }

    /// `⌥←  ⌥→`, drawn by the same renderer as the recorder rows so the two
    /// speak one glyph vocabulary, from the modifier the classifier reads.
    static let caretChordsLabel = [NSLeftArrowFunctionKey, NSRightArrowFunctionKey]
        .map { arrow in
            ShortcutKeyDisplay.text(for: ComposingKeyChord(
                key: String(UnicodeScalar(arrow)!),
                modifiers: ComposingKeyIntent.caretChordModifiers,
            ))
        }
        .joined(separator: "  ")

    /// `⇧Q … ⇧;` under Standard, `⇧1 … ⇧9` under Telex: the first and last
    /// key of the live slot set, drawn by the recorder rows' renderer.
    static func shiftedSlotKeysLabel(_ keySet: CandidateSlotKeySet) -> String {
        [0, HorizontalPageLayout.pageSize - 1]
            .map { slot in
                ShortcutKeyDisplay.text(for: ComposingKeyChord(key: keySet.label(forSlot: slot), modifiers: .shift))
            }
            .joined(separator: " … ")
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
