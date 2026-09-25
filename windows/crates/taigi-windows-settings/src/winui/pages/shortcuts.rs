//! The Shortcuts pane: every key the user can put an action on, in three titled
//! blocks (`ShortcutSettingsView.swift`) — Candidate Selection, the keys that move through
//! the candidates; Output, the keys that end the composition into the document;
//! Other, the switches and the windows a key raises — plus the reset card.
//! Which keys pick a candidate is not chosen here: it follows from Tone Keys
//! on the General pane (`ToneInputScheme`). Every row is the same recorder;
//! which registry it writes to and what it refuses on top of the shared gate
//! is the tier's. Last writer wins across both registries
//! (`ShortcutConflicts`), and the loser's row visibly empties.

use super::reset_row;
use crate::winui::cards;
use crate::winui::window::{Message, RecorderTarget, ResetScope, SettingsWindow};
use taigi_desktop_core::candidates::HorizontalPageLayout;
use taigi_desktop_core::keys::{
    rejection_message_key, CandidateSlotKeySet, ComposingAction, ComposingKeyBindings,
    ComposingKeyChord, KeyModifiers, ShortcutAction, CARET_CHORD_MODIFIERS, WIDTH_FLIP_MODIFIERS,
};
use taigi_desktop_core::strings::{StringKey, StringResolver};
use windows_reactor::*;

/// Upstream's width for the same control (`RecorderCocoa`), so the rows do
/// not step.
const FIELD_WIDTH: f64 = 130.0;
/// Between the field and its clear button.
const FIELD_GAP: f64 = 4.0;

pub fn view(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let document = window.document();
    let bindings = ComposingKeyBindings::from_document(document);
    View::fragment((
        // Block one: through the candidates.
        //
        // Typing order twice over, the order upstream draws
        // (`ShortcutSettingsView.swift`): the blocks in the order a user meets
        // them, and the rows inside each in their roster's own order — the
        // global block's switches first, windows last (`ShortcutAction`).
        cards::section_title(strings.resolve(StringKey::DesktopShortcutSectionCandidateSelection)),
        composing_rows(
            window,
            strings,
            context,
            &bindings,
            ComposingAction::GROUPS[0],
        ),
        // Shown, not recordable (USER 2026-09-20): the bare slot keys pick a
        // candidate off the visible page, and which keys they are follows
        // the Tone Keys picker (`ToneInputScheme`) — so the row follows it too.
        // First of the fixed rows because it is the main way through the
        // bar; its Shift twin sits with the commit rows below.
        fixed_row(
            strings.resolve(StringKey::DesktopShortcutSelectCandidateSlot),
            slot_keys_label(bindings.slot_key_set()),
        ),
        // Shown, not recordable: the fixed navigation tier
        // (`ComposingKeyIntent::intent`), read before any binding so a user
        // who has mis-bound everything else still has a way through the
        // candidates.
        fixed_row(
            strings.resolve(StringKey::DesktopShortcutNavigateCandidates),
            NAVIGATION_KEYS_LABEL.to_owned(),
        ),
        // Shown, not recordable (USER 2026-09-09): the caret inside the
        // composition rides the host's own word-jump chord, and the
        // classifier reads it before any binding (`ComposingKeyIntent`). With
        // the candidate movers, because moving the caret is what it is.
        fixed_row(
            strings.resolve(StringKey::DesktopShortcutMoveComposingCaret),
            caret_chords_label(),
        ),
        // Block two: out of the composition and into the document.
        cards::section_title(strings.resolve(StringKey::DesktopShortcutSectionOutput)),
        composing_rows(
            window,
            strings,
            context,
            &bindings,
            ComposingAction::GROUPS[1],
        ),
        // Shown, not recordable (USER 2026-09-10): Shift on a slot key is
        // the Hanji/romanization commit aimed at that slot, and the slot keys follow the
        // tone scheme — so the row follows it too, and there is nothing to
        // record. After the commit rows, because it is one.
        fixed_row(
            strings.resolve(StringKey::DesktopActionCommitAlternateScript),
            shifted_slot_keys_label(bindings.slot_key_set()),
        ),
        // Shown, not recordable (USER 2026-09-20): Ctrl on a punctuation key
        // types it in the other width once, whatever the Hanji/romanization mode would have
        // typed (`ComposingKeyIntent::width_flip_character`). Here because it
        // writes into the document; three sample chords, since the row stands
        // for every key of the map.
        fixed_row(
            strings.resolve(StringKey::DesktopShortcutFlipPunctuationWidth),
            width_flip_chords_label(),
        ),
        // Shown, not recordable: Escape drops the composition without
        // writing to the document (`ComposingKeyIntent::intent`). Last in
        // the block (USER 2026-09-21): every row above it writes something;
        // this is the one way out that writes nothing.
        fixed_row(
            strings.resolve(StringKey::DesktopShortcutCancelComposing),
            CANCEL_KEY_LABEL.to_owned(),
        ),
        // Block three: the switches, and the windows a key raises. What these
        // have in common is that none of them needs a composition running —
        // which is also why they are the roster that holds a chord in the
        // global registry, though the block is drawn on what they DO. Not on
        // their modifiers: Hanji/Romanization Swap ships on a bare backtick, so Ctrl+Alt
        // names no boundary here.
        //
        // One block (2026-08-25, +1 on 2026-09-02, +1 on 2026-09-09): three
        // switches, the symbol picker, the Telex guide and one doorway.
        cards::section_title(strings.resolve(StringKey::CommonOther)),
        global_rows(window, strings, context, &ShortcutAction::ALL),
        // Both registries at once, and no conflict pass afterwards: the
        // shipped defaults hold no chord in common
        // (`ShortcutSettingsView.swift` `restoreDefaults`).
        reset_row(strings, ResetScope::Shortcuts, context),
    ))
}

/// One read-only row: the label, and the fixed keys greyed where a recorder
/// row shows its field — the greyed text is what tells it from the rows
/// that record.
fn fixed_row(label: &str, keys: String) -> View {
    cards::row(
        label,
        TextBlock::new()
            .text(keys)
            .opacity(0.65)
            .vertical_alignment(VerticalAlignment::Center),
    )
}

/// The six keys the fixed navigation tier reads, in the keycap legends
/// Windows prints (`ShortcutSettingsView.swift` `navigationKeysLabel`).
const NAVIGATION_KEYS_LABEL: &str = "←  →  ↑  ↓  PgUp  PgDn";

/// The cancel key's keycap legend (`ShortcutSettingsView.swift`
/// `cancelKeyLabel`).
const CANCEL_KEY_LABEL: &str = "Esc";

/// `Ctrl+←  Ctrl+→`, named by the same modifier labels the recorder rows
/// use, from the modifier the classifier reads (`ShortcutSettingsView.swift`
/// `caretChordsLabel`).
fn caret_chords_label() -> String {
    ["←", "→"]
        .map(|arrow| {
            ComposingKeyChord::modifier_labels(CARET_CHORD_MODIFIERS)
                .chain([arrow.to_owned()])
                .collect::<Vec<_>>()
                .join("+")
        })
        .join("  ")
}

/// `Ctrl+,  Ctrl+.  Ctrl+;` — three of the keys the width flip reaches, in
/// the recorder rows' own spelling (`ShortcutSettingsView.swift`
/// `widthFlipChordsLabel`).
fn width_flip_chords_label() -> String {
    [",", ".", ";"]
        .map(|key| {
            ComposingKeyChord {
                key: key.to_owned(),
                modifiers: WIDTH_FLIP_MODIFIERS,
            }
            .display()
        })
        .join("  ")
}

/// `qwdfzxvy;` under Standard, `123456789` under Telex: every key of the
/// live slot set, bare, in the recorder rows' own spelling — lowercase
/// because a bare key shows the character it types
/// (`ShortcutSettingsView.swift` `slotKeysLabel`).
fn slot_keys_label(slot_keys: CandidateSlotKeySet) -> String {
    ComposingKeyChord {
        key: slot_keys_run(slot_keys),
        modifiers: KeyModifiers::NONE,
    }
    .display()
}

/// `Shift+QWDFZXVY;` under Standard, `Shift+123456789` under Telex: every key
/// of the live slot set behind ONE Shift, in the recorder rows' own spelling
/// (`ShortcutSettingsView.swift` `shiftedSlotKeysLabel`).
fn shifted_slot_keys_label(slot_keys: CandidateSlotKeySet) -> String {
    ComposingKeyChord {
        key: slot_keys_run(slot_keys),
        modifiers: KeyModifiers::SHIFT,
    }
    .display()
}

/// The nine slot keys of `slot_keys` as one run, in page order.
fn slot_keys_run(slot_keys: CandidateSlotKeySet) -> String {
    (0..HorizontalPageLayout::PAGE_SIZE)
        .map(|slot| slot_keys.label_for_slot(slot))
        .collect()
}

fn global_rows(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
    actions: &'static [ShortcutAction],
) -> View {
    let document = window.document();
    View::keyed_fragment(
        actions
            .iter()
            .map(|action| {
                (
                    action.raw(),
                    recorder_row(
                        window,
                        strings,
                        context,
                        RecorderTarget::Global(*action),
                        action.chord_in(document),
                    ),
                )
            })
            .collect::<Vec<_>>(),
    )
}

fn composing_rows(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
    bindings: &ComposingKeyBindings,
    actions: &'static [ComposingAction],
) -> View {
    View::keyed_fragment(
        actions
            .iter()
            .map(|action| {
                let target = RecorderTarget::Composing(*action);
                (
                    action.raw(),
                    recorder_row(
                        window,
                        strings,
                        context,
                        target,
                        bindings.chord(*action).cloned(),
                    ),
                )
            })
            .collect::<Vec<_>>(),
    )
}

/// One recorder row: the field, and a × beside a bound one. The field
/// shows the chord, or the prompt — or why the last press was refused —
/// while this row is the one recording.
fn recorder_row(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
    target: RecorderTarget,
    chord: Option<ComposingKeyChord>,
) -> View {
    let is_recording = window.is_recording(target);
    let label = if is_recording {
        strings
            .resolve(
                window
                    .recorder_rejection()
                    .map_or(StringKey::DesktopShortcutRecording, rejection_message_key),
            )
            .to_owned()
    } else {
        chord.as_ref().map_or_else(
            || {
                strings
                    .resolve(StringKey::DesktopShortcutUnbound)
                    .to_owned()
            },
            ComposingKeyChord::display,
        )
    };
    let field = Button::new()
        .width(FIELD_WIDTH)
        // The accent is what a WinUI control wears while it is the one
        // taking input; the egui field drew itself selected for the same
        // reason.
        .style(if is_recording {
            ButtonStyle::Accent
        } else {
            ButtonStyle::Default
        })
        .on_click(context.callback(move |()| Message::StartRecording(target)))
        .content(label);
    let clear = if chord.is_some() && !is_recording {
        Button::new()
            .on_click(context.callback(move |()| Message::ClearShortcut(target)))
            .content(FontIcon::new().glyph(CLEAR_GLYPH))
            .tooltip(strings.resolve(StringKey::CommonDelete))
    } else {
        View::empty()
    };
    cards::row(
        strings.resolve(target.label_key()),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(FIELD_GAP)
            .children((field, clear)),
    )
}

/// Segoe Fluent Icons' `ChromeClose`, the × the egui field drew.
const CLEAR_GLYPH: &str = "\u{E8BB}";
