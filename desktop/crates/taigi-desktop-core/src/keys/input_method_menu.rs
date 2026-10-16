//! The input-method menu — the Windows tray button's popup and the Linux
//! panel menu — as one ordered list (USER 2026-09-24: "I want the macOS, Windows
//! and Linux menus to be identical, i18n included"). Each shell maps a command to its own
//! row id and draws the chord its own way; the rows, their order and their
//! words are decided here. The Mac builds the same list in Swift
//! (`TaigiInputController.menu()`), held to it by
//! `TaigiInputControllerMenuTests`, which asserts the same authored Hanji as
//! the test below.

use super::ShortcutAction;
use crate::platform::DesktopPlatform;
use crate::settings::SettingsDocument;
use crate::strings::{StringKey, StringResolver};

/// What a menu row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuCommand {
    /// A global shortcut a click stands in for, under the Shortcuts pane's own
    /// name for it.
    Shortcut(ShortcutAction),
    /// The settings window, wherever the user left it.
    OpenSettings,
    /// The manual update check, in the settings window on General. macOS and
    /// Windows only: Linux draws no row for it — the distribution's package
    /// manager updates an input method (USER 2026-09-25).
    CheckForUpdates,
    /// The About page — it has no sidebar row (USER 2026-09-20), so the menu
    /// is its one doorway.
    About,
}

impl MenuCommand {
    pub fn title_key(self) -> StringKey {
        match self {
            Self::Shortcut(action) => action.label_key(),
            Self::OpenSettings => StringKey::DesktopMenuSettings,
            Self::CheckForUpdates => StringKey::DesktopUpdateCheckNow,
            Self::About => StringKey::HomeAboutKeyboard,
        }
    }

    /// The shortcut whose recorded chord the row prints; `None` for the two
    /// commands with no chord by design (a key claimed for a once-in-a-while
    /// command is taken from every application).
    pub fn chord_action(self) -> Option<ShortcutAction> {
        match self {
            Self::Shortcut(action) => Some(action),
            Self::OpenSettings => Some(ShortcutAction::OpenLastSettingsPane),
            Self::CheckForUpdates | Self::About => None,
        }
    }
}

/// The rows in order; `None` is a separator. The switches first — the two
/// input-script switches, then Candidate Display; not
/// the Hanji/Romanization Swap, whose bare-backtick default the Mac's menu can never
/// print; not the symbol picker, which needs the caret a click has no hold
/// of; not the Telex guide (USER 2026-09-20: "hardly anyone uses it") — then
/// the TPS key panel, which on Linux is a window the user may want to open
/// with the mouse (desktop TPS roadmap D6) — then the settings doorway, then
/// the check and About. The Mac's menu gains Switch TPS in desktop TPS P4
/// and the TPS key panel in P5.
pub const MENU: [Option<MenuCommand>; 9] = [
    Some(MenuCommand::Shortcut(ShortcutAction::ToggleRomanization)),
    Some(MenuCommand::Shortcut(ShortcutAction::ToggleTps)),
    Some(MenuCommand::Shortcut(
        ShortcutAction::CycleCandidateDisplayMode,
    )),
    Some(MenuCommand::Shortcut(ShortcutAction::ShowTpsKeyboard)),
    None,
    Some(MenuCommand::OpenSettings),
    None,
    Some(MenuCommand::CheckForUpdates),
    Some(MenuCommand::About),
];

/// One drawn row: its command, its title in the display language, and the
/// chord the user last recorded for it, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuRow {
    pub command: MenuCommand,
    pub title: String,
    pub chord: Option<String>,
}

/// [`MENU`], resolved against the strings and the settings as they are now.
/// The same rows under every input mode: Fcitx5 registers its actions once
/// and re-titles them by position (`linux/fcitx5/src/engine.cpp`
/// `refreshMenu`), and IBus updates only the root property on a mode change,
/// so a row that came and went would shift every row after it. A row whose
/// shortcut is inert under the mode (`ShortcutAction::is_inert_under`) does
/// nothing when clicked, as its chord does.
pub fn menu_rows(
    strings: &StringResolver,
    settings: &SettingsDocument,
    platform: DesktopPlatform,
) -> Vec<Option<MenuRow>> {
    MENU.iter()
        .map(|command| {
            command.map(|command| MenuRow {
                command,
                title: strings.resolve(command.title_key()).to_owned(),
                chord: command
                    .chord_action()
                    .and_then(|action| action.chord_in(settings, platform))
                    .map(|chord| chord.display(platform)),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::TEST_PLATFORM as PLATFORM;
    use crate::settings::SettingChoice;
    use crate::strings::DisplayLanguage;

    #[test]
    fn the_menu_is_the_same_rows_on_every_desktop() {
        // trace: `MENU` resolved through the Hanji strings over an empty
        // document — the authored Hanji, the default chords. The Mac's
        // `TaigiInputControllerMenuTests` asserts the same literals (the
        // Switch TPS row from desktop TPS P4, the TPS key panel from P5).
        let strings = StringResolver::new(DisplayLanguage::Hanji);
        let rows: Vec<Option<(String, Option<String>)>> =
            menu_rows(&strings, &SettingsDocument::default(), PLATFORM)
                .into_iter()
                .map(|row| row.map(|row| (row.title, row.chord)))
                .collect();
        let row =
            |title: &str, chord: Option<&str>| Some((title.to_owned(), chord.map(str::to_owned)));
        assert_eq!(
            rows,
            [
                row("切換台羅/白話字", Some("Ctrl+Alt+C")),
                row("切換方音符號", Some("Ctrl+Alt+P")),
                row("切換羅馬字顯示", Some("Ctrl+Alt+H")),
                row("顯示方音符號小齒盤", Some("Ctrl+Alt+J")),
                None,
                row("台語齒盤設定", Some("Ctrl+Alt+S")),
                None,
                row("檢查更新", None),
                row("關於齒盤", None),
            ]
        );
    }

    #[test]
    fn the_menu_keeps_its_shape_under_every_input_mode() {
        // Fcitx5 re-titles its registered rows by position: a row that came
        // and went with the mode would shift the rest (P3 cloud review).
        let strings = StringResolver::new(DisplayLanguage::Hanji);
        let commands = |mode| -> Vec<Option<MenuCommand>> {
            let mut document = SettingsDocument::default();
            document.set_choice(&crate::settings::keys::INPUT_MODE, mode);
            menu_rows(&strings, &document, PLATFORM)
                .into_iter()
                .map(|row| row.map(|row| row.command))
                .collect()
        };
        for mode in crate::settings::InputMode::ALL {
            assert_eq!(commands(*mode), MENU, "{mode:?}");
        }
    }

    #[test]
    fn a_cleared_chord_prints_the_title_alone() {
        let strings = StringResolver::new(DisplayLanguage::Hanji);
        let mut cleared = SettingsDocument::default();
        ShortcutAction::OpenLastSettingsPane.store_in(&mut cleared, None, PLATFORM);
        let rows = menu_rows(&strings, &cleared, PLATFORM);
        let settings = rows[5].as_ref().expect("the settings row");
        assert_eq!(settings.command, MenuCommand::OpenSettings);
        assert_eq!(settings.chord, None);
    }
}
