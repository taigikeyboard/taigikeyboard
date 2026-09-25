// The Shortcuts pane: every key the user can put an action on, in three blocks.

import AppKit
import KeyboardShortcuts
import SwiftUI

/// The Shortcuts pane of the settings window.
///
/// Three blocks, by what a key DOES: Candidate Selection, the keys that move through the
/// candidates; Output, the keys that end the composition into the document; and
/// Other, the switches and the windows a key raises. Titled since 2026-09-10 (USER) — the blocks shipped
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
/// (`RecorderCocoa.swift:404-410`) — so the Hanji/romanization swap could not be moved to a bare
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
            // The candidate-slot keys have no RECORDER row on this pane: they
            // follow from the Tone Keys picker on the General pane
            // (`ToneInputScheme`), so the two halves of the key contract
            // cannot be set apart. They are shown read-only below.
            Section {
                ForEach(ComposingAction.groups[0], id: \.self) { action in
                    recorderRow(action)
                }

                // Shown, not recordable (USER 2026-09-20): the bare slot keys
                // pick a candidate off the visible page, and which keys they
                // are follows the Tone Keys picker (`ToneInputScheme.slotKeySet`)
                // — so the row follows it too. First of the fixed rows because
                // it is the main way through the bar; its ⇧ twin sits with the
                // commit rows below.
                fixedRow(.desktopShortcutSelectCandidateSlot, Self.slotKeysLabel(bindings.slotKeySet))

                // Shown, not recordable: the fixed navigation tier
                // (`ComposingKeyIntent.intent`), read before any binding so a
                // user who has mis-bound everything else still has a way
                // through the candidates.
                fixedRow(.desktopShortcutNavigateCandidates, Self.navigationKeysLabel)

                // Shown, not recordable (USER 2026-09-09): the caret inside the
                // composition rides the host's own word-jump chord, and the
                // classifier reads it before any binding (`ComposingKeyIntent`).
                // With the candidate movers, because moving the caret is what
                // it is — the greyed field is what tells it from the rows that
                // record.
                fixedRow(.desktopShortcutMoveComposingCaret, Self.caretChordsLabel)
            } header: {
                Text(language.string(.desktopShortcutSectionCandidateSelection))
            }

            // Block two: out of the composition and into the document.
            Section {
                ForEach(ComposingAction.groups[1], id: \.self) { action in
                    recorderRow(action)
                }

                // Shown, not recordable (USER 2026-09-10): ⇧ on a slot key is
                // the Hanji/romanization commit aimed at that slot, and the slot keys follow
                // the tone scheme — so the row follows it too, and there is
                // nothing to record. After the commit rows, because it is one.
                fixedRow(.desktopActionCommitAlternateScript, Self.shiftedSlotKeysLabel(bindings.slotKeySet))

                // Shown, not recordable (USER 2026-09-20): ⌃ on a punctuation
                // key types it in the other width once, whatever the Hanji/romanization
                // mode would have typed (`ComposingKeyIntent.widthFlipCharacter`).
                // Here because it writes into the document; three sample
                // chords, since the row stands for every key of the map.
                fixedRow(.desktopShortcutFlipPunctuationWidth, Self.widthFlipChordsLabel)

                // Shown, not recordable: Escape drops the composition without
                // writing to the document (`ComposingKeyIntent.intent`). Last
                // in the block (USER 2026-09-21): every row above it writes
                // something; this is the one way out that writes nothing.
                fixedRow(.desktopShortcutCancelComposing, Self.cancelKeyLabel)
            } header: {
                Text(language.string(.desktopShortcutSectionOutput))
            }

            // Block three: the switches, and the windows a key raises. What
            // these have in common is that none of them needs a composition
            // running — which is also why they are the roster that holds a
            // chord in the global registry, though the block is drawn on what
            // they DO. Not on their modifiers: the Hanji/romanization swap ships on a bare
            // backtick, so ⌃⌘ names no boundary here. Nor on dispatch: the
            // symbol picker is on this list and registers no Carbon hotkey
            // (`ShortcutAction.firesFromTheKeyPath`).
            //
            // One row per global action, off the same list the hotkey
            // registration uses, so a new action cannot appear in one and
            // not the other.
            //
            // One block, since 2026-08-25. There were two — the keys that
            // opened a settings pane, then the keys that change what the
            // user is typing — until the five pane chords were retired
            // (USER: five chords for panes visited about once a day, which
            // the menu bar already lists by name).
            Section {
                ForEach(ShortcutAction.allCases, id: \.self) { action in
                    globalRecorderRow(action)
                }
            } header: {
                Text(language.string(.commonOther))
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

    /// `←  →  ↑  ↓  ⇞  ⇟` — the fixed navigation tier's own key list, drawn
    /// by the recorder rows' renderer.
    static let navigationKeysLabel = ComposingKeyChord.fixedNavigationKeys
        .map { ShortcutKeyDisplay.text(for: ComposingKeyChord(key: $0, modifiers: [])) }
        .joined(separator: "  ")

    /// `⎋`, drawn by the recorder rows' renderer.
    static let cancelKeyLabel = ShortcutKeyDisplay.text(for: ComposingKeyChord(key: ComposingKeyChord.cancelKey, modifiers: []))

    /// `⌃,  ⌃.  ⌃;` — three of the keys the width flip reaches, drawn by the
    /// recorder rows' renderer from the modifier the classifier reads.
    static let widthFlipChordsLabel = [",", ".", ";"]
        .map { key in
            ShortcutKeyDisplay.text(for: ComposingKeyChord(
                key: key,
                modifiers: ComposingKeyIntent.widthFlipModifiers,
            ))
        }
        .joined(separator: "  ")

    /// `qwdfzxvy;` under Standard, `123456789` under Telex: every key of the
    /// live slot set, bare, drawn by the recorder rows' renderer — lowercase
    /// because a bare key shows the character it types (USER 2026-08-22).
    static func slotKeysLabel(_ keySet: CandidateSlotKeySet) -> String {
        ShortcutKeyDisplay.text(for: ComposingKeyChord(key: slotKeys(keySet), modifiers: []))
    }

    /// `⇧QWDFZXVY;` under Standard, `⇧123456789` under Telex: every key of the
    /// live slot set behind ONE ⇧, drawn by the recorder rows' renderer.
    ///
    /// The whole run rather than the first and last with an ellipsis between
    /// (USER 2026-09-10): the set is not alphabetical, so `⇧Q … ⇧;` named no
    /// series a reader could fill in. One ⇧ rather than one per key, because
    /// repeating it nine times says the modifier nine times and the keys once.
    static func shiftedSlotKeysLabel(_ keySet: CandidateSlotKeySet) -> String {
        ShortcutKeyDisplay.text(for: ComposingKeyChord(key: slotKeys(keySet), modifiers: .shift))
    }

    /// The nine slot keys of `keySet` as one run, in page order.
    private static func slotKeys(_ keySet: CandidateSlotKeySet) -> String {
        (0 ..< HorizontalPageLayout.pageSize)
            .map { keySet.label(forSlot: $0) }
            .joined()
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

    /// One read-only row: the label, and the fixed keys greyed where a
    /// recorder row shows its field — the greyed text is what tells it from
    /// the rows that record.
    private func fixedRow(_ label: StringKey, _ keys: String) -> some View {
        LabeledContent(language.string(label)) {
            Text(keys)
                .foregroundStyle(.secondary)
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
    /// restored half of them would leave rows it visibly did not touch.
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

    /// Re-reads the resolved bindings, which is also what puts a commit row's
    /// default back after the user clears it — when no other row holds that
    /// key (`ComposingAction.refilledFromDefault`).
    private func reload() {
        bindings = store.composingKeyBindings
    }
}
