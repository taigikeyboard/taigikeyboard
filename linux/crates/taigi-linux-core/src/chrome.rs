//! The chrome both shells share (roadmap L6, PR5): the panel menu rows,
//! the mode label beside the icon, and the global shortcut actions —
//! romanization / Hanji-romanization / display-mode switches, the Telex guide, the
//! symbol picker, the settings doorway. Port of the Windows
//! `session.rs::perform_global` + `lang_bar.rs::menu_rows`, with the
//! HUD and the two floating windows replaced by what a panel can draw:
//! the guide and the picker are lookup tables, the flash is the mode
//! label announced (`Emit::AnnounceMode`).
//!
//! Everything here runs under the coordinator lock where it needs the
//! engine and emits nothing; the shell replays the returned [`Emit`]s.

use crate::executor::{Emit, LookupTableContent, Recorder};
use crate::runtime::Runtime;
use crate::selection::LookupSelection;
use crate::session::{self, EngineState, SymbolPicker, PAGE_SIZE};
use taigi_desktop_core::composing::ContextToken;
use taigi_desktop_core::keys::{
    menu_rows, telex_guide_rows, ComposingKeyBindings, MenuCommand, ShortcutAction, MENU,
};
use taigi_desktop_core::settings::{keys, InputMode, SettingsDocument};
use taigi_desktop_core::strings::StringKey;
use taigi_desktop_core::symbols::SymbolTable;
use taigi_linux_platform::open_settings;

/// The menu row that opens the settings window on the last pane.
pub const MENU_SETTINGS: &str = "settings";
/// The menu row that opens the settings window on About.
pub const MENU_ABOUT: &str = "about";

/// One row of the panel menu — a status-area action on Fcitx5, a
/// sub-property on IBus. `id` is what `activate_menu` takes back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuItem {
    Action {
        id: &'static str,
        title: String,
        /// The chord the row answers to, for the accelerator column;
        /// `None` for a row without one.
        detail: Option<String>,
    },
    Separator,
}

/// The rows every desktop shares (`taigi_desktop_core::keys::MENU`: the
/// two switches, TaigiKeyboard Settings — Fcitx5 lists its own Input Method Settings in the same
/// menu — then About), each with its Linux id. No Check for Updates: the distribution's
/// package manager updates an input method (USER 2026-09-25).
pub fn menu_items(runtime: &Runtime) -> Vec<MenuItem> {
    let settings = runtime.settings.current();
    menu_rows(&runtime.strings(), &settings)
        .into_iter()
        .filter_map(|row| match row {
            Some(row) => Some(MenuItem::Action {
                id: menu_id(row.command)?,
                title: row.title,
                detail: row.chord,
            }),
            None => Some(MenuItem::Separator),
        })
        .collect()
}

/// A command's row id — what `activate_menu` takes back. A shortcut row is
/// its action's persisted raw spelling; `None` for the one command Linux has
/// no row for.
fn menu_id(command: MenuCommand) -> Option<&'static str> {
    match command {
        MenuCommand::Shortcut(action) => Some(action.raw()),
        MenuCommand::OpenSettings => Some(MENU_SETTINGS),
        MenuCommand::CheckForUpdates => None,
        MenuCommand::About => Some(MENU_ABOUT),
    }
}

/// The label the panel shows beside the icon: the romanization and the
/// candidate display mode, the two states the global chords switch and the
/// mode flash names on the other desktops (`台羅 · 漢字優先`).
pub fn mode_label(runtime: &Runtime) -> String {
    let strings = runtime.strings();
    let settings = runtime.settings.current();
    let romanization = match settings.choice(&keys::INPUT_MODE) {
        InputMode::Poj => StringKey::SettingsPojMode,
        _ => StringKey::SettingsTlMode,
    };
    let display_mode = settings.engine_settings().candidate_display_mode;
    format!(
        "{} · {}",
        strings.resolve(romanization),
        strings.resolve(display_mode.label_key())
    )
}

/// The indicator text for a panel that draws at most two characters: GNOME
/// Shell shows an IBus engine's `InputMode` property symbol in the top bar
/// only when it is one or two characters long (`js/ui/status/keyboard.js`,
/// GNOME 46). The romanization alone, as two hanji.
pub fn mode_symbol(runtime: &Runtime) -> &'static str {
    match runtime.settings.current().choice(&keys::INPUT_MODE) {
        InputMode::Poj => "白話",
        _ => "台羅",
    }
}

/// A menu row was activated (`PropertyActivate` / `SimpleAction::Activated`).
pub fn activate_menu(
    runtime: &Runtime,
    token: ContextToken,
    state: &mut EngineState,
    id: &str,
) -> Vec<Emit> {
    let command = MENU
        .into_iter()
        .flatten()
        .find(|command| menu_id(*command) == Some(id));
    match command {
        Some(MenuCommand::Shortcut(action)) => perform_global(runtime, token, state, action),
        Some(MenuCommand::OpenSettings) => {
            perform_global(runtime, token, state, ShortcutAction::OpenLastSettingsPane)
        }
        Some(MenuCommand::About) => {
            open_settings(Some("about"));
            Vec::new()
        }
        Some(MenuCommand::CheckForUpdates) | None => {
            log::warn!("menu.unknown_row id={id}");
            Vec::new()
        }
    }
}

/// A global shortcut fired — from its chord in the key path or from the
/// menu. The Windows `perform_global`, minus the windows.
pub fn perform_global(
    runtime: &Runtime,
    token: ContextToken,
    state: &mut EngineState,
    action: ShortcutAction,
) -> Vec<Emit> {
    let settings = runtime.settings.current();
    let bindings = ComposingKeyBindings::from_document(&settings);
    let mut emits = Vec::new();
    // The guide comes down BEFORE any other action runs: a switch under an
    // open card would leave a table spelled for the romanization the user
    // just left (`TaigiInputController.performShortcutAction`). The picker
    // likewise, unless the action IS the picker's toggle.
    if action != ShortcutAction::ShowTelexGuide && state.telex_guide_shown {
        state.telex_guide_shown = false;
        session::present_table(state, &settings, &bindings, &mut emits);
    }
    if action != ShortcutAction::ShowSymbolPicker && state.symbol_picker.is_some() {
        state.symbol_picker = None;
        session::present_table(state, &settings, &bindings, &mut emits);
    }
    match action {
        ShortcutAction::OpenLastSettingsPane => {
            open_settings(None);
        }
        ShortcutAction::ToggleRomanization => {
            if !runtime.update_settings("toggle_romanization", |document| {
                let next = match document.choice(&keys::INPUT_MODE) {
                    InputMode::Tl => InputMode::Poj,
                    _ => InputMode::Tl,
                };
                document.set_choice(&keys::INPUT_MODE, next);
            }) {
                return emits;
            }
            // The candidates on screen were fetched under the old
            // romanization; they go with the mode that produced them — then
            // the label, announced, because the chord fires from anywhere
            // and a romanization that changed with no notice reads as the
            // keyboard breaking (USER 2026-08-26).
            state.clear_list();
            let settings = runtime.settings.current();
            session::present_table(state, &settings, &bindings, &mut emits);
            emits.push(Emit::ModeChanged);
            emits.push(Emit::AnnounceMode);
        }
        ShortcutAction::ToggleTranslateSwapped => {
            // Inert under roman-only (`allows_swap_toggle`): no write.
            if !settings
                .engine_settings()
                .candidate_display_mode
                .allows_swap_toggle()
            {
                return emits;
            }
            if !runtime.update_settings("toggle_translate_swapped", |document| {
                let swapped = document.bool(&keys::IS_TRANSLATE_SWAPPED);
                document.set_bool(&keys::IS_TRANSLATE_SWAPPED, !swapped);
            }) {
                return emits;
            }
            // The list STAYS: the swap changes how a candidate displays,
            // never which exist — re-presented in place, selection kept.
            let settings = runtime.settings.current();
            represent_open_list(runtime, token, state, &settings, false);
            session::present_table(state, &settings, &bindings, &mut emits);
        }
        ShortcutAction::CycleCandidateDisplayMode => {
            if !runtime.update_settings("cycle_candidate_display_mode", |document| {
                let next = document.choice(&keys::CANDIDATE_DISPLAY_MODE).next();
                document.set_choice(&keys::CANDIDATE_DISPLAY_MODE, next);
            }) {
                return emits;
            }
            // The mode changes which candidates exist (invariants §44), not
            // only how they draw — re-fetched under the new mode.
            let settings = runtime.settings.current();
            represent_open_list(runtime, token, state, &settings, true);
            session::present_table(state, &settings, &bindings, &mut emits);
            emits.push(Emit::ModeChanged);
            emits.push(Emit::AnnounceMode);
        }
        ShortcutAction::ShowTelexGuide => {
            state.telex_guide_shown = !state.telex_guide_shown;
            if state.telex_guide_shown {
                emits.push(Emit::LookupTable(telex_guide_table(runtime, &settings)));
                state.is_table_shown = true;
            } else {
                session::present_table(state, &settings, &bindings, &mut emits);
            }
        }
        ShortcutAction::ShowSymbolPicker => {
            toggle_symbol_picker(runtime, token, state, &settings, &bindings, &mut emits);
        }
    }
    emits
}

/// The open list for `token` under the settings in force right now:
/// re-fetched first when the change alters which candidates exist
/// (`refetch`), otherwise the same list re-presented in place. Only for
/// an engine a key already built — a switch must not bring the runtime up.
fn represent_open_list(
    runtime: &Runtime,
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
    if refetch {
        session::refresh_candidates(settings, manager, state);
    } else {
        state.candidates.refresh_presentation(manager);
    }
}

/// The Telex key table as a lookup table: the title row, then `key  meaning`
/// per row, no labels, no highlight, one vertical page (roadmap L4 —
/// platform-adapted presentation of `keys::telex_guide_rows`).
fn telex_guide_table(runtime: &Runtime, settings: &SettingsDocument) -> LookupTableContent {
    let strings = runtime.strings();
    let mut rows = vec![strings
        .resolve(StringKey::SettingsToneSchemeTelex)
        .to_owned()];
    rows.extend(
        telex_guide_rows(settings.choice(&keys::INPUT_MODE), &strings)
            .into_iter()
            .map(|row| format!("{}  {}", row.key, row.meaning)),
    );
    let page_size = rows.len() as u32;
    LookupTableContent {
        candidates: rows,
        labels: Vec::new(),
        cursor: 0,
        cursor_visible: false,
        page_size,
        vertical: true,
    }
}

/// The picker chord: down if up; otherwise the composition is ended first
/// — commit first, as vChewing does (`InputHandler_HandleStates.swift:1110`):
/// the picker writes into the document, and a composition still marked
/// there would have the symbol land inside it. A visible highlight commits
/// what is highlighted, the way Enter does; a commit that only NAILED a
/// segment leaves the composition running, and the picker waits.
fn toggle_symbol_picker(
    runtime: &Runtime,
    token: ContextToken,
    state: &mut EngineState,
    settings: &SettingsDocument,
    bindings: &ComposingKeyBindings,
    emits: &mut Vec<Emit>,
) {
    if state.symbol_picker.is_some() {
        state.symbol_picker = None;
        session::present_table(state, settings, bindings, emits);
        return;
    }
    // The picker writes into the document and learns the pick — neither
    // belongs in a password field (the key path's gate, `process_key`).
    if state.is_password_field {
        return;
    }
    let Some(table) = SymbolTable::bundled() else {
        return;
    };
    if session::is_composing(runtime, token) {
        let mut reply = session::commit_for_picker(runtime, token, state, settings, bindings);
        emits.append(&mut reply.emits);
        if session::is_composing(runtime, token) {
            return;
        }
    }
    // The recents lead (`RecentSymbols`), read once: this is the list the
    // pick will index.
    let symbols = settings.recent_symbols().ordered(table.symbols());
    let count = symbols.len();
    state.symbol_picker = Some(SymbolPicker {
        symbols,
        selection: LookupSelection::new(count, PAGE_SIZE),
    });
    session::present_table(state, settings, bindings, emits);
}

/// The picker as the panel draws it: the symbols with the slot-key labels,
/// the highlight, the layout's orientation.
pub(crate) fn symbol_picker_table(
    picker: &SymbolPicker,
    settings: &SettingsDocument,
    bindings: &ComposingKeyBindings,
) -> LookupTableContent {
    let slot_key_set = bindings.slot_key_set();
    LookupTableContent {
        candidates: picker.symbols.clone(),
        labels: (0..PAGE_SIZE)
            .map(|slot| slot_key_set.label_for_slot(slot))
            .collect(),
        cursor: picker.selection.selected_index().unwrap_or(0) as u32,
        cursor_visible: true,
        page_size: PAGE_SIZE as u32,
        vertical: session::is_vertical_layout(settings),
    }
}

/// Writes the symbol at `index`, closes the picker and moves the symbol to
/// the front of the recents. `None` — a slot with no cell — does nothing
/// and keeps the picker up. An attaching mark swaps with the auto space a
/// commit left, as a typed one would (Windows `KeyWork::InsertSymbol`).
pub(crate) fn pick_symbol(
    runtime: &Runtime,
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
    let armed_swap = std::mem::take(&mut state.armed_auto_space);
    let mut recorder = Recorder::new(state.can_delete_surrounding());
    {
        let mut coordinator = runtime.lock_coordinator();
        let manager = coordinator.claim(token);
        if !session::swap_auto_space(&symbol, armed_swap, settings, manager, &mut recorder) {
            recorder.insert_external(&symbol);
            manager.note_character_typed_outside_composition(&symbol);
        }
    }
    state.armed_auto_space = recorder.armed_swap;
    let mut emits = recorder.emits;
    session::present_table(state, settings, bindings, &mut emits);
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
    use crate::session::process_raw_key;
    use taigi_desktop_core::settings::SettingChoice;
    use taigi_linux_platform::key_translation::state;
    use taigi_linux_platform::RawKeyEvent;

    fn runtime() -> (tempfile::TempDir, Runtime) {
        crate::runtime::temporary_runtime()
    }

    fn press(runtime: &Runtime, state: &mut EngineState, keysym: u32, mask: u32) -> Vec<Emit> {
        let reply = process_raw_key(
            runtime,
            ContextToken(1),
            state,
            RawKeyEvent {
                keyval: keysym,
                keycode: 0,
                state: mask,
            },
        );
        assert!(reply.handled, "a chord is consumed");
        reply.emits
    }

    const CTRL_ALT: u32 = state::CONTROL | state::MOD1;
    const SLASH: u32 = 0x2f;
    const COMMA: u32 = 0x2c;
    const RETURN: u32 = 0xff0d;
    const ESCAPE: u32 = 0xff1b;

    #[test]
    fn a_held_toggle_chord_fires_once_until_released() {
        // trace: Ctrl+Alt+/ = ShowTelexGuide (default chord). Press → the
        // guide table; auto-repeat press → consumed, nothing emitted; release
        // + press → the guide comes down.
        let (_directory, runtime) = runtime();
        let mut engine = EngineState::default();
        let emits = press(&runtime, &mut engine, SLASH, CTRL_ALT);
        assert!(
            matches!(emits.as_slice(), [Emit::LookupTable(table)] if !table.cursor_visible && table.labels.is_empty())
        );
        assert!(engine.telex_guide_shown);
        assert!(press(&runtime, &mut engine, SLASH, CTRL_ALT).is_empty());
        assert!(engine.telex_guide_shown);
        let release = process_raw_key(
            &runtime,
            ContextToken(1),
            &mut engine,
            RawKeyEvent {
                keyval: SLASH,
                keycode: 0,
                state: CTRL_ALT | state::RELEASE,
            },
        );
        assert!(!release.handled);
        let emits = press(&runtime, &mut engine, SLASH, CTRL_ALT);
        assert_eq!(emits, vec![Emit::HideLookupTable]);
        assert!(!engine.telex_guide_shown);
    }

    #[test]
    fn the_open_guide_swallows_a_bare_escape_and_comes_down() {
        let (_directory, runtime) = runtime();
        let mut engine = EngineState::default();
        press(&runtime, &mut engine, SLASH, CTRL_ALT);
        let emits = press(&runtime, &mut engine, ESCAPE, 0);
        assert_eq!(emits, vec![Emit::HideLookupTable]);
        assert!(!engine.telex_guide_shown);
    }

    #[test]
    fn the_symbol_picker_lists_the_table_and_enter_commits_the_highlight() {
        // trace: Ctrl+Alt+, = ShowSymbolPicker; outside a composition the
        // picker comes up with the bundled table in file order (no recents
        // yet); Return = Confirm → the first symbol is committed, the picker
        // goes down, the symbol leads the recents.
        let (_directory, runtime) = runtime();
        let mut engine = EngineState::default();
        let emits = press(&runtime, &mut engine, COMMA, CTRL_ALT);
        let [Emit::LookupTable(table)] = emits.as_slice() else {
            panic!("the picker is one lookup table, got {emits:?}");
        };
        assert!(table.cursor_visible);
        assert_eq!(table.labels.len(), PAGE_SIZE);
        let first = table.candidates[0].clone();
        assert!(engine.symbol_picker.is_some());
        let emits = press(&runtime, &mut engine, RETURN, 0);
        assert_eq!(
            emits,
            vec![Emit::Commit(first.clone()), Emit::HideLookupTable]
        );
        assert!(engine.symbol_picker.is_none());
        assert_eq!(
            runtime.settings.current().recent_symbols().symbols(),
            &[first]
        );
    }

    #[test]
    fn the_menu_is_the_shared_rows_and_a_row_switches_the_romanization() {
        let (_directory, runtime) = runtime();
        let ids: Vec<&str> = menu_items(&runtime)
            .iter()
            .map(|item| match item {
                MenuItem::Action { id, .. } => *id,
                MenuItem::Separator => "-",
            })
            .collect();
        assert_eq!(
            ids,
            [
                "toggleRomanization",
                ShortcutAction::CycleCandidateDisplayMode.raw(),
                "-",
                MENU_SETTINGS,
                "-",
                MENU_ABOUT
            ]
        );
        assert!(mode_label(&runtime).contains(" · "));
        let mut engine = EngineState::default();
        let before: InputMode = runtime.settings.current().choice(&keys::INPUT_MODE);
        let emits = activate_menu(&runtime, ContextToken(1), &mut engine, "toggleRomanization");
        let after: InputMode = runtime.settings.current().choice(&keys::INPUT_MODE);
        assert_ne!(before.raw(), after.raw());
        assert_eq!(emits, vec![Emit::ModeChanged, Emit::AnnounceMode]);
        assert_eq!(mode_symbol(&runtime).chars().count(), 2);

        // The second row cycles the candidate display mode (`keys::MENU`),
        // which the label's second half names.
        let label_before = mode_label(&runtime);
        let emits = activate_menu(
            &runtime,
            ContextToken(1),
            &mut engine,
            ShortcutAction::CycleCandidateDisplayMode.raw(),
        );
        assert_eq!(emits, vec![Emit::ModeChanged, Emit::AnnounceMode]);
        assert_ne!(mode_label(&runtime), label_before);
    }

    #[test]
    fn a_password_field_passes_letters_through_but_keeps_the_chords() {
        // trace: `a` (0x61) in a text field = a composing key, consumed; in a
        // password field = unhandled, no emits (Windows `run_key` ToHost).
        // Ctrl+Alt+/ still brings the Telex guide up there.
        const LETTER_A: u32 = 0x61;
        let (_directory, runtime) = runtime();
        let mut engine = EngineState::default();
        let key = |engine: &mut EngineState, keysym: u32| {
            process_raw_key(
                &runtime,
                ContextToken(1),
                engine,
                RawKeyEvent {
                    keyval: keysym,
                    keycode: 0,
                    state: 0,
                },
            )
        };
        engine.is_password_field = true;
        engine.armed_auto_space = true;
        let reply = key(&mut engine, LETTER_A);
        assert!(!reply.handled);
        assert!(reply.emits.is_empty());
        assert!(!engine.armed_auto_space);
        // The picker writes into the document: it stays down here.
        perform_global(
            &runtime,
            ContextToken(1),
            &mut engine,
            ShortcutAction::ShowSymbolPicker,
        );
        assert!(engine.symbol_picker.is_none());
        assert!(!session::is_composing(&runtime, ContextToken(1)));
        press(&runtime, &mut engine, SLASH, CTRL_ALT);
        assert!(engine.telex_guide_shown);

        let mut control = EngineState::default();
        assert!(key(&mut control, LETTER_A).handled);
        let mut picker_control = EngineState::default();
        perform_global(
            &runtime,
            ContextToken(2),
            &mut picker_control,
            ShortcutAction::ShowSymbolPicker,
        );
        assert!(picker_control.symbol_picker.is_some());
        // The field turns into a password field under the open picker: Enter
        // reaches the app, the picker goes down unpicked, nothing commits.
        picker_control.is_password_field = true;
        let reply = key(&mut picker_control, RETURN);
        assert!(!reply.handled);
        assert!(picker_control.symbol_picker.is_none());
        assert!(!reply
            .emits
            .iter()
            .any(|emit| matches!(emit, Emit::Commit(_) | Emit::DeleteSurrounding { .. })));
        // And back to a text field: composing resumes.
        picker_control.is_password_field = false;
        assert!(key(&mut picker_control, LETTER_A).handled);
    }
}
