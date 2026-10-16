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
use crate::winui::window::{Message, ResetScope, SettingsWindow};
use taigi_desktop_core::keys::shortcut_labels::{
    caret_chords_label, fixed_keys_label, navigation_keys_label, slot_keys_label,
};
use taigi_desktop_core::keys::{
    rejection_message_key, ChordField, ComposingAction, ComposingKeyBindings, ComposingKeyChord,
    RecorderTarget, ShortcutAction,
};
use taigi_desktop_core::strings::{StringKey, StringResolver};
use taigi_windows_platform::DESKTOP_PLATFORM;
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
    let bindings = ComposingKeyBindings::from_document(document, DESKTOP_PLATFORM);
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
            slot_keys_label(bindings.slot_key_set(), DESKTOP_PLATFORM),
        ),
        // Shown, not recordable: the fixed navigation tier
        // (`ComposingKeyIntent::intent`), read before any binding so a user
        // who has mis-bound everything else still has a way through the
        // candidates.
        fixed_row(
            strings.resolve(StringKey::DesktopShortcutNavigateCandidates),
            navigation_keys_label(DESKTOP_PLATFORM).to_owned(),
        ),
        // Shown, not recordable (USER 2026-09-09): the caret inside the
        // composition rides the host's own word-jump chord, and the
        // classifier reads it before any binding (`ComposingKeyIntent`). With
        // the candidate movers, because moving the caret is what it is.
        fixed_row(
            strings.resolve(StringKey::DesktopShortcutMoveComposingCaret),
            caret_chords_label(DESKTOP_PLATFORM),
        ),
        // Block two: out of the composition and into the document. Shift on
        // a slot key is no row of its own (USER 2026-10-16): it is the
        // description of the row it shares a label with (`fixed_keys_label`).
        cards::section_title(strings.resolve(StringKey::DesktopShortcutSectionOutput)),
        composing_rows(
            window,
            strings,
            context,
            &bindings,
            ComposingAction::GROUPS[1],
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
                let target = RecorderTarget::Global(*action);
                let field = recorder_field(
                    window,
                    strings,
                    context,
                    target,
                    action.chord_in(document, DESKTOP_PLATFORM),
                );
                (
                    action.raw(),
                    recorder_row(strings, action.label_key(), None, [field]),
                )
            })
            .collect::<Vec<_>>(),
    )
}

/// One row per composing action, each with its two fields — both fire the
/// action.
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
                let fields = ChordField::ALL.map(|field| {
                    recorder_field(
                        window,
                        strings,
                        context,
                        RecorderTarget::Composing(*action, field),
                        bindings.chord(*action, field).cloned(),
                    )
                });
                let fixed_keys =
                    fixed_keys_label(*action, bindings.slot_key_set(), DESKTOP_PLATFORM);
                (
                    action.raw(),
                    recorder_row(strings, action.label_key(), fixed_keys.as_deref(), fields),
                )
            })
            .collect::<Vec<_>>(),
    )
}

/// One recorder row: its label, the fixed keys that also fire it under the
/// label, and its fields side by side.
fn recorder_row(
    strings: &StringResolver,
    label: StringKey,
    fixed_keys: Option<&str>,
    fields: impl IntoViews,
) -> View {
    let fields = StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(FIELD_GAP)
        .children(fields);
    match fixed_keys {
        Some(keys) => cards::described_row(strings.resolve(label), keys, fields),
        None => cards::row(strings.resolve(label), fields),
    }
}

/// One field and its × beside it. The field shows the chord, or the prompt
/// — or why the last press was refused — while this field is the one
/// recording. The × stays in place, disabled, while there is nothing to
/// clear, so a row's fields line up with the rows above and below it.
fn recorder_field(
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
            |chord| chord.display(DESKTOP_PLATFORM),
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
        // Two fields can share a row, so each is named apart for Narrator.
        .automation_name(target.field_accessible_name(strings, &label))
        .content(label);
    let clear = Button::new()
        .is_enabled(chord.is_some() && !is_recording)
        .automation_name(target.clear_accessible_name(strings))
        .on_click(context.callback(move |()| Message::ClearShortcut(target)))
        .content(FontIcon::new().glyph(CLEAR_GLYPH))
        .tooltip(strings.resolve(StringKey::CommonDelete));
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(FIELD_GAP)
        .children((field, clear))
}

/// Segoe Fluent Icons' `ChromeClose`, the × the egui field drew.
const CLEAR_GLYPH: &str = "\u{E8BB}";
