//! One key, end to end: snapshot → classifier → engine, answered as the
//! emits a shell replays (Linux roadmap L4 / L13). Port of
//! `taigi-windows-tsf::session` (`key_down` + `run_key` + `perform_work`),
//! itself the port of `TaigiInputController.handle(_:client:)`, minus what
//! the framework does for us: the edit session (there is none — a commit is
//! the one write and cannot be refused), the caret rectangle (the panel
//! anchors itself), the handover commit (the framework's focus-out reached
//! the previous engine first and committed its preedit).
//!
//! The engine's work runs under the coordinator lock (`run_key`) and emits
//! nothing; the global chords above it (`global`) take the lock only where
//! they need the engine. The caller replays [`KeyReply::emits`] after every
//! lock is dropped.

use super::global::{perform_global, pick_symbol};
use super::{is_vertical_layout, present_table, Emit, EngineState, KeyReply, KeySurface, Recorder};
use crate::composing::{
    pass_through_may_consume, perform_intent, ComposingEffectExecutor, ComposingManager,
    ContextToken,
};
use crate::engine::Effect;
use crate::keys::{
    CandidateNavigation, ComposingKeyBindings, ComposingKeyIntent, KeyEventSnapshot,
    ShortcutAction, SymbolPickerIntent,
};
use crate::runtime::DesktopRuntime;
use crate::settings::{keys, SettingsDocument};

/// The classifier's answer plus everything the key does — the Windows
/// `key_down` for one context: the global chords and the two overlays
/// first, then the composing contract.
pub fn process_key(
    runtime: &DesktopRuntime,
    token: ContextToken,
    state: &mut EngineState,
    snapshot: &KeyEventSnapshot,
) -> KeyReply {
    let settings = runtime.settings.current();
    let mut emits = Vec::new();
    let bindings = ComposingKeyBindings::from_document(&settings, runtime.platform());
    // A chord with no host modifier is a character in English mode: the bare
    // Hanji/romanization key types a backtick there.
    let global_action = ShortcutAction::matching(snapshot, &settings, runtime.platform())
        .filter(|_| !state.language_mode.is_english() || snapshot.modifiers.has_host_chord());
    // A held toggle chord repeats on the wire as presses with no release
    // between them; the repeats are consumed without firing again.
    if let Some(latched) = state.latched_chord {
        if global_action == Some(latched) {
            return KeyReply {
                handled: true,
                emits,
            };
        }
        state.latched_chord = None;
    }
    // The global chords, before the classifier — the Carbon hotkey's
    // position on the Mac, and like it independent of whether a
    // composition is running.
    if let Some(action) = global_action {
        if action.fires_once_per_press() {
            state.latched_chord = Some(action);
        }
        emits.append(&mut perform_global(runtime, token, state, action));
        return KeyReply {
            handled: true,
            emits,
        };
    }
    // A password field gets every key untouched — composing a secret would
    // show it in the preedit and teach it to the stores. The global chords
    // above still run, as on Windows (`run_key` answers ToHost there). A
    // picker left up from before the field changed goes down unpicked, and
    // the auto-space promise is spent: its swap would delete in this field.
    if state.is_password_field {
        state.armed_auto_space = false;
        if state.symbol_picker.take().is_some() {
            present_table(runtime, state, &settings, &bindings, &mut emits);
        }
        return KeyReply {
            handled: false,
            emits,
        };
    }
    // With the picker up, every key is the picker's first — read before the
    // composing contract so the slot keys and the arrows reach it rather
    // than a list that is not showing. A key the picker has no use for
    // takes it down and goes on below.
    if state.symbol_picker.is_some() {
        match SymbolPickerIntent::intent(snapshot, &bindings, runtime.platform()) {
            SymbolPickerIntent::Close => {
                state.symbol_picker = None;
                present_table(runtime, state, &settings, &bindings, &mut emits);
                return KeyReply {
                    handled: true,
                    emits,
                };
            }
            SymbolPickerIntent::Navigate(direction) => {
                if let Some(picker) = &mut state.symbol_picker {
                    picker
                        .selection
                        .navigate(direction, is_vertical_layout(&settings));
                }
                present_table(runtime, state, &settings, &bindings, &mut emits);
                return KeyReply {
                    handled: true,
                    emits,
                };
            }
            SymbolPickerIntent::PickSlot(slot) => {
                // An empty slot on a short last page is consumed all the
                // same: the key is the picker's while it is up.
                let index = state
                    .symbol_picker
                    .as_ref()
                    .and_then(|picker| picker.selection.candidate_index_for_key_slot(slot));
                emits.append(&mut pick_symbol(
                    runtime, token, state, &settings, &bindings, index,
                ));
                return KeyReply {
                    handled: true,
                    emits,
                };
            }
            SymbolPickerIntent::Confirm => {
                let index = state
                    .symbol_picker
                    .as_ref()
                    .and_then(|picker| picker.selection.selected_index());
                emits.append(&mut pick_symbol(
                    runtime, token, state, &settings, &bindings, index,
                ));
                return KeyReply {
                    handled: true,
                    emits,
                };
            }
            SymbolPickerIntent::CloseAndPassThrough => {
                state.symbol_picker = None;
                present_table(runtime, state, &settings, &bindings, &mut emits);
            }
        }
    }
    // English mode: below the global chords and the picker, above everything
    // that composes (the Windows `key_down` order). Every key is the
    // document's; entering the mode already ended the composition.
    if state.language_mode.is_english() {
        return KeyReply {
            handled: false,
            emits,
        };
    }
    // A list the user switched off since the last key comes down HERE,
    // before the key is read (`TaigiInputController.handle`): a Return
    // classified against a list still up would pick a candidate the user
    // asked never to see.
    if !settings.bool(&keys::IS_CANDIDATE_WINDOW_ENABLED) && !state.candidates.is_empty() {
        state.clear_list();
        emits.push(Emit::HideLookupTable);
        state.is_table_shown = false;
    }
    commit_left_by_mode_change(runtime, token, state, &settings, &bindings, &mut emits);
    let is_composing = self::is_composing(runtime, token);
    let is_showing_candidates = !state.candidates.is_empty();
    let intent = ComposingKeyIntent::intent(
        snapshot,
        is_composing,
        is_showing_candidates,
        &bindings,
        runtime.platform(),
    );
    log::debug!(
        "key.intent {intent:?} composing={is_composing} candidates={is_showing_candidates}"
    );
    let would_consume = match &intent {
        ComposingKeyIntent::PassThrough => pass_through_may_consume(
            snapshot,
            &settings,
            state.armed_auto_space && state.can_delete_surrounding,
        ),
        ComposingKeyIntent::CommitThenPassThrough => is_composing,
        _ => true,
    };
    if !would_consume && !matches!(intent, ComposingKeyIntent::CommitThenPassThrough) {
        // Not ours, and nothing to finish — but a character reaching the
        // document outside a composition still ends the next-word context.
        if let Some(typed) = ComposingKeyIntent::document_text(snapshot, bindings.input_mode) {
            if let Some(coordinator) = runtime.coordinator_if_built() {
                if let Ok(mut coordinator) = coordinator.try_lock() {
                    if let Some(manager) = coordinator.manager(token) {
                        manager.note_character_typed_outside_composition(&typed);
                    }
                }
            }
        }
        return KeyReply {
            handled: false,
            emits,
        };
    }

    // From here the key is ours: the runtime comes up now, never for a key
    // merely observed.
    let setup = runtime.prepare_for_first_key();
    if setup.lexicon.is_none() {
        log::warn!("key.no_lexicon");
    }
    let mut reply = run_key(
        runtime, token, state, snapshot, intent, &settings, &bindings,
    );
    emits.append(&mut reply.emits);
    KeyReply {
        handled: reply.handled,
        emits,
    }
}

/// The lifecycle end of a composition: the framework has just committed (or
/// dropped) the preedit itself — on IBus `FocusOut` under `PREEDIT_COMMIT`,
/// `Reset`, `Disable`, `Destroy` — so only the engine is reset and the list
/// taken down. The token's ownership is released so the next context claims a
/// fresh session.
pub fn end_session(
    runtime: &DesktopRuntime,
    token: ContextToken,
    state: &mut EngineState,
) -> Vec<Emit> {
    let mut emits = Vec::new();
    if let Some(coordinator) = runtime.coordinator_if_built() {
        let mut coordinator = coordinator
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(manager) = coordinator.manager(token) {
            if manager.is_composing() {
                manager.cancel_composition(&mut NullRecorder);
            }
        }
        coordinator.release(token);
    }
    if state.is_table_shown {
        emits.push(Emit::HideLookupTable);
        state.is_table_shown = false;
    }
    state.clear_list();
    state.armed_auto_space = false;
    state.symbol_picker = None;
    state.latched_chord = None;
    emits
}

/// A session gaining focus, or going on after a client's reset: the table as
/// things stand, which is what puts a wanted Telex guide back up.
pub fn begin_session(runtime: &DesktopRuntime, state: &mut EngineState) -> Vec<Emit> {
    let settings = runtime.settings.current();
    let bindings = ComposingKeyBindings::from_document(&settings, runtime.platform());
    let mut emits = Vec::new();
    present_table(runtime, state, &settings, &bindings, &mut emits);
    emits
}

/// The composition written to the document before the caret leaves it: as
/// typed, no auto space, because the user did not finish a word there (macOS
/// `Session::commit_composition`). What was on screen for it goes too. Linux
/// has no twin: the framework commits its own preedit ([`end_session`]).
pub fn commit_composition(
    runtime: &DesktopRuntime,
    token: ContextToken,
    state: &mut EngineState,
) -> Vec<Emit> {
    let mut recorder = Recorder::new(state.can_delete_surrounding);
    if is_composing(runtime, token) {
        let mut coordinator = runtime.lock_coordinator();
        if let Some(manager) = coordinator.manager(token) {
            manager.commit_composition(&mut recorder);
        }
    }
    let mut emits = recorder.emits;
    let settings = runtime.settings.current();
    let bindings = ComposingKeyBindings::from_document(&settings, runtime.platform());
    clear_surface(runtime, state, &settings, &bindings, &mut emits);
    emits
}

/// What was on screen for the text just left, and the arm that promised it a
/// swap, go down together. A wanted Telex guide is a setting, not part of
/// that text, so the table it leaves is the guide (#684).
fn clear_surface(
    runtime: &DesktopRuntime,
    state: &mut EngineState,
    settings: &SettingsDocument,
    bindings: &ComposingKeyBindings,
    emits: &mut Vec<Emit>,
) {
    state.clear_list();
    state.armed_auto_space = false;
    state.symbol_picker = None;
    present_table(runtime, state, settings, bindings, emits);
}

/// The Shift tap's switch between Taigi and English — the Windows
/// `toggle_language_mode`: the composition is committed as rendered, a keyless
/// `Commit`, where [`commit_composition`] writes it as typed: this switch is
/// the user's own, like Enter. A commit that left it standing skips the switch
/// whole, because the
/// reset below would drop text still on screen. The arm, the list and the
/// picker belong to the mode being left; a wanted guide stays, as the Windows
/// card does across the switch.
pub fn toggle_language_mode(
    runtime: &DesktopRuntime,
    token: ContextToken,
    state: &mut EngineState,
) -> Vec<Emit> {
    let settings = runtime.settings.current();
    let bindings = ComposingKeyBindings::from_document(&settings, runtime.platform());
    let mut emits = Vec::new();
    if is_composing(runtime, token) {
        let mut reply = run_key(
            runtime,
            token,
            state,
            &KeyEventSnapshot::default(),
            ComposingKeyIntent::Commit,
            &settings,
            &bindings,
        );
        emits.append(&mut reply.emits);
        if is_composing(runtime, token) {
            log::warn!("language_mode.commit_refused — switch skipped");
            return emits;
        }
    }
    clear_surface(runtime, state, &settings, &bindings, &mut emits);
    if let Some(coordinator) = runtime.coordinator_if_built() {
        let mut coordinator = coordinator
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(manager) = coordinator.manager(token) {
            manager.start_new_session();
        }
    }
    state.language_mode = state.language_mode.toggled();
    emits.push(Emit::ModeChanged);
    emits.push(Emit::AnnounceMode);
    emits
}

/// Whether `token`'s engine is mid-composition, read without bringing the
/// runtime up and without waiting on a held lock.
pub fn is_composing(runtime: &DesktopRuntime, token: ContextToken) -> bool {
    runtime
        .coordinator_if_built()
        .and_then(|coordinator| {
            coordinator
                .try_lock()
                .ok()
                .and_then(|guard| guard.manager_ref(token).map(ComposingManager::is_composing))
        })
        .unwrap_or(false)
}

/// The commit the picker chord runs before it opens: what is highlighted
/// when a list shows, the composition as rendered otherwise
/// (`ComposingKeyIntent::commit_first`).
pub(super) fn commit_for_picker(
    runtime: &DesktopRuntime,
    token: ContextToken,
    state: &mut EngineState,
    settings: &SettingsDocument,
    bindings: &ComposingKeyBindings,
) -> KeyReply {
    let intent = ComposingKeyIntent::commit_first(
        !state.candidates.is_empty(),
        settings.choice(&keys::INPUT_MODE),
    );
    runtime.prepare_for_first_key();
    run_key(
        runtime,
        token,
        state,
        &KeyEventSnapshot::default(),
        intent,
        settings,
        bindings,
    )
}

/// The first key after a switch across TPS that left a composition behind
/// commits it as shown before the key is read: a `Commit` with no key, which
/// `perform_intent` opens with `commit_composition_left_by_mode_change` and
/// then finds nothing left to write. Only for an engine a key already built
/// — with none there is no composition.
fn commit_left_by_mode_change(
    runtime: &DesktopRuntime,
    token: ContextToken,
    state: &mut EngineState,
    settings: &SettingsDocument,
    bindings: &ComposingKeyBindings,
    emits: &mut Vec<Emit>,
) {
    let Some(coordinator) = runtime.coordinator_if_built() else {
        return;
    };
    let coordinator = coordinator
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let is_left_by_mode_change = coordinator
        .manager_ref(token)
        .is_some_and(|manager| manager.is_left_by_mode_change(settings.choice(&keys::INPUT_MODE)));
    if !is_left_by_mode_change {
        return;
    }
    drop(coordinator);
    let mut reply = run_key(
        runtime,
        token,
        state,
        &KeyEventSnapshot::default(),
        ComposingKeyIntent::Commit,
        settings,
        bindings,
    );
    emits.append(&mut reply.emits);
}

/// A panel navigation (`PageUp` … `CursorDown`): the same intents the keys
/// carry, run without a key.
pub fn navigate_from_panel(
    runtime: &DesktopRuntime,
    token: ContextToken,
    state: &mut EngineState,
    direction: CandidateNavigation,
) -> Vec<Emit> {
    let settings = runtime.settings.current();
    let bindings = ComposingKeyBindings::from_document(&settings, runtime.platform());
    if let Some(picker) = &mut state.symbol_picker {
        picker
            .selection
            .navigate(direction, is_vertical_layout(&settings));
        let mut emits = Vec::new();
        present_table(runtime, state, &settings, &bindings, &mut emits);
        return emits;
    }
    if state.candidates.is_empty() {
        return Vec::new();
    }
    run_key(
        runtime,
        token,
        state,
        &KeyEventSnapshot::default(),
        ComposingKeyIntent::Navigate(direction),
        &settings,
        &bindings,
    )
    .emits
}

/// A click on the cell at `position` (`ListSelection::candidate_index_at`):
/// selects, never commits (macOS `CandidateItemView.onClick`, identical semantics).
pub fn click_from_panel(
    runtime: &DesktopRuntime,
    state: &mut EngineState,
    position: usize,
) -> Vec<Emit> {
    let settings = runtime.settings.current();
    let bindings = ComposingKeyBindings::from_document(&settings, runtime.platform());
    if let Some(picker) = &mut state.symbol_picker {
        let Some(index) = picker.selection.candidate_index_at(position) else {
            return Vec::new();
        };
        picker.selection.select(index);
    } else {
        let Some(index) = state.selection.candidate_index_at(position) else {
            return Vec::new();
        };
        state.selection.select(index);
    }
    let mut emits = Vec::new();
    present_table(runtime, state, &settings, &bindings, &mut emits);
    emits
}

/// A click on a cap of the on-screen TPS key panel: its glyph typed through
/// the engine's `TpsKey`, whatever the list or the slot keys would make of
/// that key (`ComposingKeyIntent::tps_keyboard_press`). Nothing outside TPS,
/// off the layout or in English mode, where a panel can still be clicked. A
/// picker goes down first: the glyph is the document's, and it answers keys
/// the click did not press (`typeTpsKeyboardGlyph` in macOS). A wanted Telex
/// guide stays, as it does for a key.
pub fn tps_keyboard_press(
    runtime: &DesktopRuntime,
    token: ContextToken,
    state: &mut EngineState,
    glyph: &str,
) -> Vec<Emit> {
    let settings = runtime.settings.current();
    let Some(intent) =
        ComposingKeyIntent::tps_keyboard_press(glyph, settings.choice(&keys::INPUT_MODE))
    else {
        return Vec::new();
    };
    if state.language_mode.is_english() {
        return Vec::new();
    }
    let bindings = ComposingKeyBindings::from_document(&settings, runtime.platform());
    let mut emits = Vec::new();
    if state.symbol_picker.take().is_some() {
        present_table(runtime, state, &settings, &bindings, &mut emits);
    }
    runtime.prepare_for_first_key();
    // No key: `perform_intent` commits a composition a switch across TPS left
    // behind before it types.
    let mut reply = run_key(
        runtime,
        token,
        state,
        &KeyEventSnapshot::default(),
        intent,
        &settings,
        &bindings,
    );
    emits.append(&mut reply.emits);
    emits
}

/// The Windows `run_key` + `perform_work`, against the recorder.
fn run_key(
    runtime: &DesktopRuntime,
    token: ContextToken,
    state: &mut EngineState,
    snapshot: &KeyEventSnapshot,
    intent: ComposingKeyIntent,
    settings: &SettingsDocument,
    bindings: &ComposingKeyBindings,
) -> KeyReply {
    // Every key gets exactly one chance at the swap: the arm is consumed
    // here, and only the auto-space paths re-arm it.
    let is_swap_armed = std::mem::take(&mut state.armed_auto_space);
    let mut coordinator = runtime.lock_coordinator();
    // Ownership: a key from this context while another owns the engine
    // takes it over. The other context's composition was already finished
    // by the framework's focus-out (`end_session`), so `claim` finds nothing
    // to drop in the ordinary case; if it does (a client that skipped it),
    // the outgoing composition is dropped — the text the framework shows
    // for it stays as it is.
    if coordinator.current_owner() != Some(token) {
        log::info!(
            "key.claim token={token:?} previous={:?}",
            coordinator.current_owner()
        );
    }
    let manager = coordinator.claim(token);
    let mut surface = KeySurface {
        recorder: Recorder::new(state.can_delete_surrounding),
        is_swap_armed,
        selection: &mut state.selection,
        is_vertical: is_vertical_layout(settings),
    };
    let handled = perform_intent(
        &intent,
        snapshot,
        settings,
        manager,
        &mut state.candidates,
        &mut surface,
    );
    let recorder = surface.recorder;
    state.armed_auto_space = recorder.armed_swap;
    drop(coordinator);
    let mut emits = recorder.emits;
    present_table(runtime, state, settings, bindings, &mut emits);
    KeyReply { handled, emits }
}

/// An executor that records nothing: for a composition the FRAMEWORK already
/// ended — only the engine is reset.
struct NullRecorder;

impl ComposingEffectExecutor for NullRecorder {
    fn execute(&mut self, _effect: &Effect) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::KeyModifiers;
    use crate::session::test_support;
    use crate::session::PAGE_SIZE;

    fn runtime() -> DesktopRuntime {
        test_support::runtime(SettingsDocument::default())
    }

    fn letter(character: &str) -> KeyEventSnapshot {
        KeyEventSnapshot::text(character, KeyModifiers::NONE)
    }

    #[test]
    fn a_password_field_gets_the_key_and_no_engine_comes_up() {
        let runtime = runtime();
        let mut state = EngineState {
            is_password_field: true,
            armed_auto_space: true,
            ..EngineState::default()
        };
        let reply = process_key(&runtime, ContextToken(1), &mut state, &letter("a"));
        assert_eq!(
            reply,
            KeyReply {
                handled: false,
                emits: Vec::new()
            }
        );
        assert!(!state.armed_auto_space);
        assert!(runtime.coordinator_if_built().is_none());
    }

    #[test]
    fn a_letter_composes_and_end_session_leaves_nothing_behind() {
        let runtime = runtime();
        let token = ContextToken(1);
        let mut state = EngineState::default();
        let reply = process_key(&runtime, token, &mut state, &letter("a"));
        assert!(reply.handled);
        assert!(is_composing(&runtime, token));
        end_session(&runtime, token, &mut state);
        assert!(!is_composing(&runtime, token));
    }

    #[test]
    fn a_click_past_the_page_does_nothing() {
        let mut state = EngineState::default();
        assert!(click_from_panel(&runtime(), &mut state, PAGE_SIZE).is_empty());
    }
}
