//! The field that records which key runs a shortcut, either tier. Port of
//! `ShortcutKeyRecorderField`: click (focus) starts recording, the field
//! shows the prompt — or why the last press was refused — until a key is
//! recorded, Escape / Tab / a click elsewhere / losing the window ends it,
//! and a × beside a bound row clears it. The decision itself is the core's
//! (`keys::evaluate_press`); this is the egui around it.
//!
//! While a row records, the frame's key events are taken at the START of
//! the frame (`RecorderState::intercept`, before any widget runs) — the one
//! place a key can be kept from every other control, since egui hands
//! Space / Enter to a focused button as it builds that button.

// 中文: 快捷鍵錄製欄 — 點擊開始錄製,決策交給 core;錄製中的按鍵在該幀開頭就先攔下,其他控制項看不到。

use crate::keys::{modifiers_of, press_from_event};
use taigi_windows_core::keys::{
    evaluate_press, rejection_message_key, CandidateSlotKeySet, ChordRejection, ComposingKeyChord,
    RecordedPress, RecorderOutcome, RecorderTier,
};
use taigi_windows_core::strings::{StringKey, StringResolver};

/// Upstream's width for the same control (`RecorderCocoa`), so the rows
/// do not step.
const FIELD_WIDTH: f32 = 130.0;

/// Which row is recording (at most one), why its last press was turned
/// down — shown in place of the prompt until the next press — and the
/// presses intercepted this frame for it.
#[derive(Default)]
pub struct RecorderState {
    active: Option<egui::Id>,
    rejection: Option<ChordRejection>,
    presses: Vec<RecordedPress>,
    lost_window: bool,
}

pub enum RecorderEvent {
    Record(ComposingKeyChord),
    Clear,
}

impl RecorderState {
    /// Takes this frame's key input away from every widget while a row is
    /// recording: each key press becomes a `RecordedPress`, then the key
    /// and text events are removed from the frame. A bare press's typed
    /// character (the `Text` event that follows it) names the key — that is
    /// the layout's answer, where the `Key` table is the US layout's; a
    /// chord keeps the table, since the text of Shift+; is `:` while the
    /// chord stores `;` (`charactersIgnoringModifiers`).
    pub fn intercept(&mut self, ctx: &egui::Context) {
        self.presses.clear();
        self.lost_window = false;
        if self.active.is_none() {
            return;
        }
        ctx.input_mut(|input| {
            for event in &input.events {
                match event {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        repeat,
                        modifiers,
                        ..
                    } => self
                        .presses
                        .push(press_from_event(*key, *modifiers, *repeat)),
                    egui::Event::Text(text) => {
                        if let Some(last) = self.presses.last_mut() {
                            if modifiers_of(input.modifiers).is_empty() && !text.is_empty() {
                                last.key = Some(text.to_lowercase());
                            }
                        }
                    }
                    egui::Event::WindowFocused(false) => self.lost_window = true,
                    _ => {}
                }
            }
            input
                .events
                .retain(|event| !matches!(event, egui::Event::Key { .. } | egui::Event::Text(_)));
        });
    }
}

/// One recorder row; answers what the user did to it this frame.
pub fn show(
    ui: &mut egui::Ui,
    id_salt: &str,
    chord: Option<&ComposingKeyChord>,
    tier: RecorderTier,
    slot_key_set: CandidateSlotKeySet,
    strings: &StringResolver,
    state: &mut RecorderState,
) -> Option<RecorderEvent> {
    let id = ui.make_persistent_id(("shortcut_recorder", id_salt));
    let is_active = state.active == Some(id);
    let (text, is_prompt) = if is_active {
        let key = state
            .rejection
            .map_or(StringKey::DesktopShortcutRecording, rejection_message_key);
        (strings.resolve(key).to_owned(), true)
    } else if let Some(chord) = chord {
        (chord.display(), false)
    } else {
        (
            strings
                .resolve(StringKey::DesktopShortcutUnbound)
                .to_owned(),
            true,
        )
    };
    let mut event = None;
    ui.horizontal(|ui| {
        let label = if is_prompt {
            egui::RichText::new(text).weak()
        } else {
            egui::RichText::new(text)
        };
        let field = egui::Button::new(label)
            .min_size(egui::vec2(FIELD_WIDTH, ui.spacing().interact_size.y))
            .selected(is_active);
        let response = ui.add(field);
        if response.clicked() && !is_active {
            state.active = Some(id);
            state.rejection = None;
        }
        if is_active {
            event = record(&response, tier, slot_key_set, state);
        }
        if chord.is_some() && !is_active {
            let clear = ui
                .small_button("×")
                .on_hover_text(strings.resolve(StringKey::CommonDelete));
            if clear.clicked() {
                event = Some(RecorderEvent::Clear);
            }
        }
    });
    event
}

/// The recording frame: every press intercepted this frame is judged.
fn record(
    response: &egui::Response,
    tier: RecorderTier,
    slot_key_set: CandidateSlotKeySet,
    state: &mut RecorderState,
) -> Option<RecorderEvent> {
    let end = |state: &mut RecorderState| {
        state.active = None;
        state.rejection = None;
    };
    // A click outside the field is how most people leave one; a window
    // that lost focus must not keep recording either.
    if state.lost_window || response.clicked_elsewhere() {
        end(state);
        return None;
    }
    let presses = std::mem::take(&mut state.presses);
    for press in presses {
        match evaluate_press(tier, slot_key_set, &press) {
            RecorderOutcome::Recorded(chord) => {
                end(state);
                return Some(RecorderEvent::Record(chord));
            }
            RecorderOutcome::Refused(reason) => {
                state.rejection = Some(reason);
                taigi_windows_platform::beep();
            }
            RecorderOutcome::Blurred => {
                end(state);
                return None;
            }
            // Tab ends the recording; the intercepted Tab is gone with the
            // frame, so the form walks on from the next one — the trade
            // every shortcut field makes.
            RecorderOutcome::PassThrough => {
                end(state);
                return None;
            }
            RecorderOutcome::Ignored => {}
        }
    }
    None
}
