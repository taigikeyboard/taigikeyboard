//! The 快捷鍵 pane: every key the user can put an action on, in one list
//! (`ShortcutSettingsView.swift`) — the four global chords, the keys that
//! move through the candidates, the keys that end the composition, and the
//! reset card. Which keys pick a candidate is not chosen here: it follows
//! from 聲調拍法 on the 一般 pane (`ToneInputScheme`). Every row is the same
//! recorder; which registry it writes to and what it refuses on top of the
//! shared gate is the tier's. Last writer wins across both registries
//! (`ShortcutConflicts`), and the loser's row visibly empties.

use crate::winui::cards;
use crate::winui::window::{Message, RecorderTarget, ResetScope, SettingsWindow};
use taigi_windows_core::keys::{
    rejection_message_key, ComposingAction, ComposingKeyBindings, ComposingKeyChord, ShortcutAction,
};
use taigi_windows_core::strings::{StringKey, StringResolver};
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
        // One group (2026-08-25, +1 on 2026-09-02): one doorway and three switches.
        View::keyed_fragment(ShortcutAction::ALL.map(|action| {
            (
                action.raw(),
                recorder_row(
                    window,
                    strings,
                    context,
                    RecorderTarget::Global(action),
                    action.chord_in(document),
                ),
            )
        })),
        // The keys that move through the candidates.
        composing_rows(
            window,
            strings,
            context,
            &bindings,
            ComposingAction::GROUPS[0],
        ),
        // Then the keys that end the composition.
        composing_rows(
            window,
            strings,
            context,
            &bindings,
            ComposingAction::GROUPS[1],
        ),
        // Both registries at once, and no conflict pass afterwards: the
        // shipped defaults hold no chord in common
        // (`ShortcutSettingsView.swift:578-603`).
        cards::section_gap(),
        cards::action_row(
            strings.resolve(StringKey::ThemeEditorResetAll),
            strings.resolve(StringKey::SettingsReset),
            false,
            true,
            context.callback(|()| Message::Reset(ResetScope::Shortcuts)),
        ),
    ))
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
