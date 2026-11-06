//! The chrome both shells share (roadmap L6, PR5): the panel menu rows and
//! the mode label beside the icon. Port of the Windows
//! `lang_bar.rs::menu_rows`. A row that runs a global shortcut goes through
//! `taigi_desktop_core::session::perform_global`; the shell replays the
//! returned [`Emit`]s.

use crate::runtime::Runtime;
use crate::session::{self, EngineState};
use taigi_desktop_core::composing::ContextToken;
use taigi_desktop_core::keys::{menu_rows, LanguageMode, MenuCommand, ShortcutAction, MENU};
use taigi_desktop_core::mode_indicator::ModeIndicator;
pub use taigi_desktop_core::session::mode_label;
use taigi_desktop_core::session::{perform_global, Emit};
use taigi_desktop_core::settings::SettingsDocument;
use taigi_linux_platform::{open_settings, DESKTOP_PLATFORM};

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
    menu_rows(&runtime.strings(), &settings, DESKTOP_PLATFORM)
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

/// The tray / panel indicator: its symbol is the IBus `InputMode` text and
/// the Fcitx5 label, its icon name the Fcitx5 sub-mode icon and the IBus
/// property icon. Linux has no English mode — switching engines is the
/// desktop's (roadmap L6).
pub fn mode_indicator(settings: &SettingsDocument) -> ModeIndicator {
    ModeIndicator::of(&settings.engine_settings(), LanguageMode::Taigi)
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
    let action = match command {
        Some(MenuCommand::Shortcut(action)) => action,
        Some(MenuCommand::OpenSettings) => ShortcutAction::OpenLastSettingsPane,
        Some(MenuCommand::About) => {
            open_settings(Some("about"));
            return Vec::new();
        }
        Some(MenuCommand::CheckForUpdates) | None => {
            log::warn!("menu.unknown_row id={id}");
            return Vec::new();
        }
    };
    let mut emits = perform_global(runtime, token, state, action);
    session::run_launches(&mut emits);
    emits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::process_raw_key;
    use taigi_desktop_core::session::{is_composing, PAGE_SIZE};
    use taigi_desktop_core::settings::{keys, CandidateDisplayMode, InputMode, SettingChoice};
    use taigi_desktop_core::strings::StringKey;
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

    fn is_guide_wanted(runtime: &Runtime) -> bool {
        runtime.settings.current().is_telex_guide_wanted()
    }

    fn is_guide_wanted_stored(runtime: &Runtime) -> bool {
        runtime.settings.current().bool(&keys::TELEX_GUIDE_SHOWN)
    }

    /// The rows of the guide table in `emits`, which has no labels and no
    /// highlight; empty when no such table is there.
    fn guide_rows(emits: &[Emit]) -> Vec<String> {
        emits
            .iter()
            .find_map(|emit| match emit {
                Emit::LookupTable(table) if !table.cursor_visible => {
                    Some(table.joined_candidates())
                }
                _ => None,
            })
            .unwrap_or_default()
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
        assert!(is_guide_wanted(&runtime));
        assert!(press(&runtime, &mut engine, SLASH, CTRL_ALT).is_empty());
        assert!(is_guide_wanted(&runtime));
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
        assert!(!is_guide_wanted(&runtime));
    }

    #[test]
    fn an_idle_escape_reaches_the_host_and_leaves_the_guide_up() {
        // trace: nothing composing, so Escape is PassThrough: unhandled and
        // nothing emitted (#684).
        let (_directory, runtime) = runtime();
        let mut engine = EngineState::default();
        press(&runtime, &mut engine, SLASH, CTRL_ALT);
        let reply = process_raw_key(
            &runtime,
            ContextToken(1),
            &mut engine,
            RawKeyEvent {
                keyval: ESCAPE,
                keycode: 0,
                state: 0,
            },
        );
        assert!(!reply.handled);
        assert!(reply.emits.is_empty());
        assert!(is_guide_wanted(&runtime));
        assert!(engine.is_table_shown);
    }

    #[test]
    fn the_guide_leaves_with_tps_and_returns_respelled_after_a_switch() {
        // trace: TELEX_GUIDE_SHOWN survives both switches; `z` reads `ts`
        // under TL and `ch` under POJ (`telex_guide_rows`).
        let (_directory, runtime) = runtime();
        let mut engine = EngineState::default();
        press(&runtime, &mut engine, SLASH, CTRL_ALT);
        let token = ContextToken(1);

        let emits = perform_global(&runtime, token, &mut engine, ShortcutAction::ToggleTps);
        assert!(emits.contains(&Emit::HideLookupTable));
        assert!(is_guide_wanted_stored(&runtime));
        assert!(!is_guide_wanted(&runtime));

        let emits = perform_global(&runtime, token, &mut engine, ShortcutAction::ToggleTps);
        assert!(guide_rows(&emits).iter().any(|row| row.contains("ts")));

        let emits = perform_global(
            &runtime,
            token,
            &mut engine,
            ShortcutAction::ToggleRomanization,
        );
        let rows = guide_rows(&emits);
        assert!(rows.iter().any(|row| row.contains("ch")), "{rows:?}");
        assert!(!rows.iter().any(|row| row.contains("ts")), "{rows:?}");
    }

    #[test]
    fn the_picker_covers_the_guide_and_it_returns_when_the_picker_closes() {
        let (_directory, runtime) = runtime();
        let mut engine = EngineState::default();
        press(&runtime, &mut engine, SLASH, CTRL_ALT);
        let token = ContextToken(1);

        let emits = perform_global(
            &runtime,
            token,
            &mut engine,
            ShortcutAction::ShowSymbolPicker,
        );
        assert!(matches!(emits.as_slice(), [Emit::LookupTable(table)] if table.cursor_visible));

        let emits = perform_global(
            &runtime,
            token,
            &mut engine,
            ShortcutAction::ShowSymbolPicker,
        );
        assert!(!guide_rows(&emits).is_empty());
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
        let first = table.joined_candidates()[0].clone();
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
                "toggleTps",
                ShortcutAction::CycleCandidateDisplayMode.raw(),
                ShortcutAction::ShowTpsKeyboard.raw(),
                "-",
                MENU_SETTINGS,
                "-",
                MENU_ABOUT
            ]
        );
        assert!(mode_label(&runtime, &runtime.settings.current()).contains(" · "));
        let mut engine = EngineState::default();
        let before: InputMode = runtime.settings.current().choice(&keys::INPUT_MODE);
        let emits = activate_menu(&runtime, ContextToken(1), &mut engine, "toggleRomanization");
        let after: InputMode = runtime.settings.current().choice(&keys::INPUT_MODE);
        assert_ne!(before.raw(), after.raw());
        assert_eq!(emits, vec![Emit::ModeChanged, Emit::AnnounceMode]);
        // trace: default TL side by side, hanji first → toggle → POJ → 白
        assert_eq!(
            mode_indicator(&runtime.settings.current()),
            ModeIndicator::PojHanji
        );

        // The third row cycles the candidate display mode (`keys::MENU`),
        // which the label's second half names.
        let label_before = mode_label(&runtime, &runtime.settings.current());
        let emits = activate_menu(
            &runtime,
            ContextToken(1),
            &mut engine,
            ShortcutAction::CycleCandidateDisplayMode.raw(),
        );
        assert_eq!(emits, vec![Emit::ModeChanged, Emit::AnnounceMode]);
        assert_ne!(
            mode_label(&runtime, &runtime.settings.current()),
            label_before
        );
    }

    #[test]
    fn the_tps_key_panel_row_does_nothing_outside_tps() {
        // `ShowTpsKeyboard` is inert under TL / POJ (`is_inert_under`): no
        // emit, no settings write — and no settings app spawned.
        let (_directory, runtime) = runtime();
        let mut engine = EngineState::default();
        let revision = runtime.settings.current().revision;
        let emits = activate_menu(
            &runtime,
            ContextToken(1),
            &mut engine,
            ShortcutAction::ShowTpsKeyboard.raw(),
        );
        assert!(emits.is_empty());
        assert_eq!(runtime.settings.current().revision, revision);
    }

    #[test]
    fn the_swap_redraws_the_indicator_and_announces_only_under_side_by_side() {
        // trace: default TL, side by side, hanji first = 台; the swap writes
        // the stored flag → Ts, announced with the output script as the
        // label's third part. Combined: the swap writes the stored flag
        // (punctuation width only), a commit still writes the Hanji → no
        // announce, two-part label. Romanization Only: the swap writes
        // nothing (`allows_swap_toggle`) and the indicator stays Ts.
        let (_directory, runtime) = runtime();
        let mut engine = EngineState::default();
        let indicator = || mode_indicator(&runtime.settings.current());
        assert_eq!(indicator(), ModeIndicator::TlHanji);
        let emits = perform_global(
            &runtime,
            ContextToken(1),
            &mut engine,
            ShortcutAction::ToggleTranslateSwapped,
        );
        assert_eq!(
            emits[emits.len() - 2..],
            [Emit::ModeChanged, Emit::AnnounceMode]
        );
        assert_eq!(indicator(), ModeIndicator::TlRomanization);
        let label = || mode_label(&runtime, &runtime.settings.current());
        let roman = runtime
            .strings()
            .resolve(StringKey::SettingsOutputScriptRoman)
            .to_owned();
        assert!(label().ends_with(&format!(" · {roman}")));

        assert!(runtime.update_settings("test", |document| {
            document.set_choice(
                &keys::CANDIDATE_DISPLAY_MODE,
                CandidateDisplayMode::Combined,
            );
        }));
        let emits = perform_global(
            &runtime,
            ContextToken(1),
            &mut engine,
            ShortcutAction::ToggleTranslateSwapped,
        );
        assert_eq!(emits.last(), Some(&Emit::ModeChanged));
        assert!(!emits.contains(&Emit::AnnounceMode));
        assert_eq!(label().matches(" · ").count(), 1);

        assert!(runtime.update_settings("test", |document| {
            document.set_choice(
                &keys::CANDIDATE_DISPLAY_MODE,
                CandidateDisplayMode::RomanOnly,
            );
        }));
        let emits = perform_global(
            &runtime,
            ContextToken(1),
            &mut engine,
            ShortcutAction::ToggleTranslateSwapped,
        );
        assert!(emits.is_empty());
        assert_eq!(indicator(), ModeIndicator::TlRomanization);
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
        assert!(!is_composing(&runtime, ContextToken(1)));
        press(&runtime, &mut engine, SLASH, CTRL_ALT);
        assert!(is_guide_wanted(&runtime));

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
