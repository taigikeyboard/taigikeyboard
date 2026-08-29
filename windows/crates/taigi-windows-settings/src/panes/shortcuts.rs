//! The 快捷鍵 pane: every key the user can put an action on, in one list
//! (`ShortcutSettingsView.swift`) — the three global chords, the keys that
//! move through the candidates, the slot-key set, the keys that end the
//! composition, and the reset row. Every row is the same recorder; which
//! registry it writes to and what it refuses on top of the shared gate is
//! the tier's. Last writer wins across both registries
//! (`ShortcutConflicts`), and the loser's row visibly empties.

// 中文: 快捷鍵 pane — 全域三顆 + 組字動作 + 選字鍵組 + 恢復預設;後寫者贏。

use super::{choice_combo, labelled_row, section_break};
use crate::app::SettingsApp;
use crate::widgets::recorder::{self, RecorderEvent};
use crate::widgets::wide_action_row;
use taigi_windows_core::keys::{
    CandidateSlotKeySet, ComposingAction, ComposingKeyBindings, RecorderTier, ShortcutAction,
    ShortcutConflicts,
};
use taigi_windows_core::settings::{keys, SettingChoice};
use taigi_windows_core::strings::StringKey;

pub fn show(ui: &mut egui::Ui, app: &mut SettingsApp) {
    let strings = app.strings();
    let document = app.document();
    let bindings = ComposingKeyBindings::from_document(document);
    let slot_key_set = bindings.slot_key_set;
    let global_chords: Vec<_> = ShortcutAction::ALL
        .into_iter()
        .map(|action| (action, action.chord_in(document)))
        .collect();

    // One group (2026-08-25): one doorway and two switches.
    for (action, chord) in &global_chords {
        labelled_row(ui, strings.resolve(action.label_key()), |ui| {
            let event = recorder::show(
                ui,
                action.raw(),
                chord.as_ref(),
                RecorderTier::Global,
                slot_key_set,
                &strings,
                &mut app.recorder,
            );
            if let Some(event) = event {
                let chord = match event {
                    RecorderEvent::Record(chord) => Some(chord),
                    RecorderEvent::Clear => None,
                };
                // This recording is the last writer: the other globals and
                // the composing rows holding the same key empty.
                app.update_document(|document| {
                    action.store_in(document, chord.as_ref());
                    ShortcutConflicts::resolve_after_global_recording(document, *action);
                });
            }
        });
    }

    // The keys that move through the candidates, then the keys that end
    // the composition (`ComposingAction::GROUPS`).
    for action in ComposingAction::GROUPS[0] {
        composing_row(ui, app, &strings, &bindings, *action);
    }

    // Ends the moving-through group. Glyphs rather than translated words:
    // the keys are read off the keyboard. A picker, not a recorder: one
    // set standing for nine slots.
    labelled_row(
        ui,
        strings.resolve(StringKey::DesktopBindingSlotModifier),
        |ui| {
            let mut chosen = slot_key_set;
            let choices = CandidateSlotKeySet::ALL
                .iter()
                .map(|set| (*set, set.menu_label()));
            if choice_combo(
                ui,
                "slot_key_set",
                &slot_key_set.menu_label(),
                &mut chosen,
                choices,
            ) {
                // The picker is the last writer: the keys it just claimed come
                // off any global row that held one. Composing rows need no
                // write — re-resolved from storage on every read.
                app.update_document(|document| {
                    document.set_choice(&keys::CANDIDATE_SLOT_MODIFIER, chosen);
                    ShortcutConflicts::resolve_after_slot_key_set_change(document, chosen);
                });
            }
        },
    );

    for action in ComposingAction::GROUPS[1] {
        composing_row(ui, app, &strings, &bindings, *action);
    }

    // Both registries at once, and no conflict pass afterwards: the shipped
    // defaults hold no chord in common (`ShortcutSettingsView.swift:578-603`).
    section_break(ui);
    if wide_action_row::show(ui, strings.resolve(StringKey::ThemeEditorResetAll), false) {
        app.update_document(|document| {
            document.reset_composing_shortcuts();
            document.reset_global_shortcuts();
        });
        app.recorder = Default::default();
    }
}

fn composing_row(
    ui: &mut egui::Ui,
    app: &mut SettingsApp,
    strings: &taigi_windows_core::strings::StringResolver,
    bindings: &ComposingKeyBindings,
    action: ComposingAction,
) {
    labelled_row(ui, strings.resolve(action.label_key()), |ui| {
        let event = recorder::show(
            ui,
            action.raw(),
            bindings.chord(action),
            RecorderTier::Composing,
            bindings.slot_key_set,
            strings,
            &mut app.recorder,
        );
        if let Some(event) = event {
            let chord = match event {
                RecorderEvent::Record(chord) => Some(chord),
                RecorderEvent::Clear => None,
            };
            app.update_document(|document| {
                // Last writer wins: the composing row holding this chord
                // and the global row across the seam both empty.
                if let Some(chord) = &chord {
                    ShortcutConflicts::resolve_after_composing_recording(document, action, chord);
                }
                document.set_composing_chord(action, chord.as_ref());
            });
        }
    });
}
