//! The 快捷鍵 pane: every key the user can put an action on, in three titled
//! blocks (`ShortcutSettingsView.swift`) — 選字, the keys that move through
//! the candidates; 輸出, the keys that end the composition into the document;
//! 其他, the switches and the windows a key raises — plus the reset card.
//! Which keys pick a candidate is not chosen here: it follows from 聲調拍法
//! on the 一般 pane (`ToneInputScheme`). Every row is the same recorder;
//! which registry it writes to and what it refuses on top of the shared gate
//! is the tier's. Last writer wins across both registries
//! (`ShortcutConflicts`), and the loser's row visibly empties.

use crate::winui::cards;
use crate::winui::window::{Message, RecorderTarget, ResetScope, SettingsWindow};
use taigi_windows_core::candidates::HorizontalPageLayout;
use taigi_windows_core::keys::{
    rejection_message_key, CandidateSlotKeySet, ComposingAction, ComposingKeyBindings,
    ComposingKeyChord, KeyModifiers, ShortcutAction, CARET_CHORD_MODIFIERS,
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
        // Shown, not recordable (USER 2026-09-09): the caret inside the
        // composition rides the host's own word-jump chord, and the
        // classifier reads it before any binding (`ComposingKeyIntent`). With
        // the candidate movers, because moving the caret is what it is.
        cards::row(
            strings.resolve(StringKey::DesktopShortcutMoveComposingCaret),
            TextBlock::new()
                .text(caret_chords_label())
                .opacity(0.65)
                .vertical_alignment(VerticalAlignment::Center),
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
        // the 漢羅 commit aimed at that slot, and the slot keys follow the
        // tone scheme — so the row follows it too, and there is nothing to
        // record. After the commit rows, because it is one.
        cards::row(
            strings.resolve(StringKey::DesktopShortcutCommitAlternateScriptInSlot),
            TextBlock::new()
                .text(shifted_slot_keys_label(bindings.slot_key_set()))
                .opacity(0.65)
                .vertical_alignment(VerticalAlignment::Center),
        ),
        // Block three: the switches, and the windows a key raises. What these
        // have in common is that none of them needs a composition running —
        // which is also why they are the roster that holds a chord in the
        // global registry, though the block is drawn on what they DO. Not on
        // their modifiers: 漢羅對調 ships on a bare backtick, so Ctrl+Alt
        // names no boundary here.
        //
        // One block (2026-08-25, +1 on 2026-09-02, +1 on 2026-09-09): three
        // switches, the symbol picker, the Telex guide and one doorway.
        cards::section_title(strings.resolve(StringKey::DesktopShortcutSectionOther)),
        global_rows(window, strings, context, &ShortcutAction::ALL),
        // Both registries at once, and no conflict pass afterwards: the
        // shipped defaults hold no chord in common
        // (`ShortcutSettingsView.swift` `restoreDefaults`).
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

/// `Shift+QWDFZXVY;` under Standard, `Shift+123456789` under Telex: every key
/// of the live slot set behind ONE Shift, in the recorder rows' own spelling
/// (`ShortcutSettingsView.swift` `shiftedSlotKeysLabel`).
fn shifted_slot_keys_label(slot_keys: CandidateSlotKeySet) -> String {
    let keys: String = (0..HorizontalPageLayout::PAGE_SIZE)
        .map(|slot| slot_keys.label_for_slot(slot))
        .collect();
    ComposingKeyChord {
        key: keys,
        modifiers: KeyModifiers::SHIFT,
    }
    .display()
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
