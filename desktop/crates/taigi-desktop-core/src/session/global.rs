//! The global shortcut actions — romanization / Hanji-romanization /
//! display-mode switches, the Telex guide, the symbol picker, the TPS key
//! panel's launch, the settings doorway. Port of the Windows
//! `session.rs::perform_global`, with the HUD and the two floating windows
//! replaced by what a panel can draw: the guide and the picker are lookup
//! tables, the flash is the mode label announced ([`Emit::AnnounceMode`]).
//!
//! Everything here runs under the coordinator lock where it needs the
//! engine and emits nothing; the shell replays the returned [`Emit`]s.

use super::key::{commit_for_picker, is_composing};
use super::{
    is_vertical_layout, present_table, Emit, EngineState, KeySurface, ListSelection, Recorder,
    SymbolPicker,
};
use crate::composing::{insert_symbol, represent_list, ContextToken};
use crate::global_action::{apply_global_action, ListEffect, ShellRequest};
use crate::keys::{ComposingKeyBindings, ShortcutAction};
use crate::runtime::DesktopRuntime;
use crate::settings::presentation::output_script_label;
use crate::settings::{keys, CandidateDisplayMode, InputMode, SettingsDocument};
use crate::symbols::symbol_picker_symbols;

/// A global shortcut fired — from its chord in the key path or from the
/// menu. The Windows `perform_global`, minus the windows; what only the
/// shell can start comes back as [`Emit::OpenSettings`] and
/// [`Emit::ToggleTpsKeyboard`].
pub fn perform_global(
    runtime: &DesktopRuntime,
    token: ContextToken,
    state: &mut EngineState,
    action: ShortcutAction,
) -> Vec<Emit> {
    let settings = runtime.settings.current();
    let mut emits = Vec::new();
    if action.is_inert_under(settings.choice(&keys::INPUT_MODE)) {
        return emits;
    }
    // Resolved once, before the action's write: the tables below are laid
    // out with the key set in force when the chord was pressed.
    let bindings = ComposingKeyBindings::from_document(&settings, runtime.platform());
    // The picker comes down BEFORE any other action runs, unless the action
    // IS its toggle.
    if action != ShortcutAction::ShowSymbolPicker && state.symbol_picker.is_some() {
        state.symbol_picker = None;
        present_table(runtime, state, &settings, &bindings, &mut emits);
    }
    let Some(outcome) = apply_global_action(runtime, action) else {
        return emits;
    };
    match outcome.shell {
        Some(ShellRequest::OpenSettings) => emits.push(Emit::OpenSettings),
        Some(ShellRequest::ToggleSymbolPicker) => {
            toggle_symbol_picker(runtime, token, state, &settings, &bindings, &mut emits);
        }
        // A window of the settings app, not of this engine (desktop TPS
        // roadmap D6, U6): the app's one instance opens it or closes it. The
        // composition is not touched — the panel is looked at, and its window
        // taking focus is what would end the session.
        Some(ShellRequest::ToggleTpsKeyboard) => emits.push(Emit::ToggleTpsKeyboard),
        Some(ShellRequest::TelexGuideChanged) => {
            let settings = runtime.settings.current();
            present_table(runtime, state, &settings, &bindings, &mut emits);
        }
        None => {}
    }
    // Only a mode switch touches the list (`GlobalOutcome::mode_switch`).
    let settings = runtime.settings.current();
    match outcome.list {
        ListEffect::Untouched => return emits,
        ListEffect::Clear => state.clear_list(),
        ListEffect::Represent { refetch } => {
            represent_open_list(runtime, token, state, &settings, refetch);
        }
    }
    present_table(runtime, state, &settings, &bindings, &mut emits);
    emits.push(Emit::ModeChanged);
    if outcome.announces {
        emits.push(Emit::AnnounceMode);
    }
    emits
}

/// The full mode text a switch announces: the romanization and the candidate
/// display mode, the two states the global chords switch and the mode flash
/// names on the other desktops (`台羅 · 漢羅對應`), plus what a commit writes
/// under side by side, the one display mode where the swap picks it
/// (`台羅 · 漢羅對應 · 漢字`). TPS alone under TPS, where the display mode
/// switches nothing.
pub fn mode_label(runtime: &DesktopRuntime, settings: &SettingsDocument) -> String {
    let strings = runtime.strings();
    let input_mode: InputMode = settings.choice(&keys::INPUT_MODE);
    if input_mode == InputMode::Tps {
        return strings.resolve(input_mode.label_key()).to_owned();
    }
    let romanization = input_mode.label_key();
    let engine_settings = settings.engine_settings();
    let display_mode = engine_settings.candidate_display_mode;
    let label = format!(
        "{} · {}",
        strings.resolve(romanization),
        strings.resolve(display_mode.label_key())
    );
    if display_mode != CandidateDisplayMode::SideBySide {
        return label;
    }
    let output_script = output_script_label(engine_settings.is_hanji_first);
    format!("{label} · {}", strings.resolve(output_script))
}

/// The open list for `token` under the settings in force right now:
/// re-fetched first when the change alters which candidates exist
/// (`refetch`), otherwise the same list re-presented in place. Only for
/// an engine a key already built — a switch must not bring the runtime up.
fn represent_open_list(
    runtime: &DesktopRuntime,
    token: ContextToken,
    state: &mut EngineState,
    settings: &SettingsDocument,
    refetch: bool,
) {
    if state.candidates.is_empty() {
        return;
    }
    let Some(coordinator) = runtime.coordinator_if_built() else {
        return;
    };
    let mut coordinator = coordinator
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(manager) = coordinator.manager(token) else {
        return;
    };
    represent_list(settings, manager, &mut state.candidates, refetch);
    if refetch {
        // A refetched list starts over at its first cell.
        state.selection = ListSelection::new(state.candidates.len());
    } else {
        // The same cells in other scripts: other widths.
        state.selection.mark_stale();
    }
    // Presented again under new settings, the list may begin with the §34
    // literal where it did not, or the other way round.
    state
        .selection
        .set_unkeyed_lead(state.candidates.leads_with_literal_roman());
}

/// The picker chord: down if up; otherwise the composition is ended first
/// — commit first, as vChewing does (`InputHandler_HandleStates.swift:1110`):
/// the picker writes into the document, and a composition still marked
/// there would have the symbol land inside it. A visible highlight commits
/// what is highlighted, the way Enter does; a commit that only NAILED a
/// segment leaves the composition running, and the picker waits.
fn toggle_symbol_picker(
    runtime: &DesktopRuntime,
    token: ContextToken,
    state: &mut EngineState,
    settings: &SettingsDocument,
    bindings: &ComposingKeyBindings,
    emits: &mut Vec<Emit>,
) {
    if state.symbol_picker.is_some() {
        state.symbol_picker = None;
        present_table(runtime, state, settings, bindings, emits);
        return;
    }
    // Read once, before the commit: this is the list the pick will index.
    let Some(symbols) = symbol_picker_symbols(settings, state.is_password_field) else {
        return;
    };
    if is_composing(runtime, token) {
        let mut reply = commit_for_picker(runtime, token, state, settings, bindings);
        emits.append(&mut reply.emits);
        if is_composing(runtime, token) {
            return;
        }
    }
    let count = symbols.len();
    state.symbol_picker = Some(SymbolPicker {
        symbols,
        selection: ListSelection::new(count),
    });
    present_table(runtime, state, settings, bindings, emits);
}

/// Writes the symbol at `index`, closes the picker and moves the symbol to
/// the front of the recents. `None` — a slot with no cell — does nothing
/// and keeps the picker up. An attaching mark swaps with the auto space a
/// commit left, as a typed one would (Windows `KeyWork::InsertSymbol`).
pub(super) fn pick_symbol(
    runtime: &DesktopRuntime,
    token: ContextToken,
    state: &mut EngineState,
    settings: &SettingsDocument,
    bindings: &ComposingKeyBindings,
    index: Option<usize>,
) -> Vec<Emit> {
    let symbol = index.and_then(|index| {
        state
            .symbol_picker
            .as_ref()
            .and_then(|picker| picker.symbols.get(index).cloned())
    });
    let Some(symbol) = symbol else {
        return Vec::new();
    };
    state.symbol_picker = None;
    // A pick is a key this input method consumes, and the runtime comes up
    // on the first such key: the engine claimed for the context has to
    // exist, and it hears about the character as the end of a next-word
    // context.
    runtime.prepare_for_first_key();
    let is_swap_armed = std::mem::take(&mut state.armed_auto_space);
    let mut surface = KeySurface {
        recorder: Recorder::new(state.can_delete_surrounding, state.text_before_caret),
        is_swap_armed,
        selection: &mut state.selection,
        is_vertical: is_vertical_layout(settings),
    };
    {
        let mut coordinator = runtime.lock_coordinator();
        let manager = coordinator.claim(token);
        insert_symbol(&symbol, settings, manager, &mut surface);
    }
    let recorder = surface.recorder;
    state.armed_auto_space = recorder.armed_swap;
    if recorder.has_written() {
        state.text_before_caret.outdate();
    }
    let mut emits = recorder.emits;
    present_table(runtime, state, settings, bindings, &mut emits);
    // The recents write is on the key path (the shell replays the commit
    // after this returns); a store that cannot be written still got its
    // symbol, the failure is only logged.
    runtime.update_settings("record_recent_symbol", |document| {
        document.note_recent_symbol(&symbol);
    });
    emits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::test_support::runtime;
    use crate::settings::InputMode;

    fn fire(document: SettingsDocument, action: ShortcutAction) -> Vec<Emit> {
        perform_global(
            &runtime(document),
            ContextToken(1),
            &mut EngineState::default(),
            action,
        )
    }

    #[test]
    fn opening_settings_is_asked_of_the_shell_and_leaves_the_guide_up() {
        // trace: only the chord takes the guide down (#684), so a wanted
        // guide on screen gets no HideLookupTable here.
        assert_eq!(
            fire(
                SettingsDocument::default(),
                ShortcutAction::OpenLastSettingsPane
            ),
            vec![Emit::OpenSettings]
        );
        let mut guide_wanted = SettingsDocument::default();
        guide_wanted.set_bool(&keys::TELEX_GUIDE_SHOWN, true);
        let mut state = EngineState {
            is_table_shown: true,
            ..EngineState::default()
        };
        assert_eq!(
            perform_global(
                &runtime(guide_wanted),
                ContextToken(1),
                &mut state,
                ShortcutAction::OpenLastSettingsPane,
            ),
            vec![Emit::OpenSettings]
        );
    }

    #[test]
    fn the_tps_key_panel_is_asked_of_the_shell_under_tps_only() {
        let mut tps = SettingsDocument::default();
        tps.set_choice(&keys::INPUT_MODE, InputMode::Tps);
        assert_eq!(
            fire(tps, ShortcutAction::ShowTpsKeyboard),
            vec![Emit::ToggleTpsKeyboard]
        );
        assert!(fire(SettingsDocument::default(), ShortcutAction::ShowTpsKeyboard).is_empty());
    }

    #[test]
    fn an_inert_action_leaves_an_open_picker_up() {
        // trace: the TPS key panel is inert outside TPS (`is_inert_under`),
        // refused before the picker would come down.
        let mut state = EngineState {
            symbol_picker: Some(SymbolPicker {
                symbols: vec!["。".to_owned()],
                selection: ListSelection::new(1),
            }),
            ..EngineState::default()
        };
        let emits = perform_global(
            &runtime(SettingsDocument::default()),
            ContextToken(1),
            &mut state,
            ShortcutAction::ShowTpsKeyboard,
        );
        assert!(emits.is_empty());
        assert!(state.symbol_picker.is_some());
    }
}
