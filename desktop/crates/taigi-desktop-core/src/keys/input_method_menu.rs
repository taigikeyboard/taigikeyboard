//! The input-method menu — the Windows tray button's popup and the Linux
//! panel menu — as one ordered list (USER 2026-09-24: "I want the macOS, Windows
//! and Linux menus to be identical, i18n included"). Each shell maps a command to its own
//! row id and draws the chord its own way; the rows, their order and their
//! words are decided here. The Mac builds the same list in Swift
//! (`TaigiInputController.menu()`), held to it by
//! `TaigiInputControllerMenuTests`, which asserts the same authored Hanji as
//! the test below.

use super::ShortcutAction;
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
            Self::About => StringKey::DesktopAboutTab,
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

/// The rows in order; `None` is a separator. The two switches first — not
/// the Hanji/Romanization Swap, whose bare-backtick default the Mac's menu can never
/// print; not the symbol picker, which needs the caret a click has no hold
/// of; not the Telex guide (USER 2026-09-20: "hardly anyone uses it") — then the
/// settings doorway, then the check and About.
pub const MENU: [Option<MenuCommand>; 7] = [
    Some(MenuCommand::Shortcut(ShortcutAction::ToggleRomanization)),
    Some(MenuCommand::Shortcut(
        ShortcutAction::CycleCandidateDisplayMode,
    )),
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
pub fn menu_rows(strings: &StringResolver, settings: &SettingsDocument) -> Vec<Option<MenuRow>> {
    MENU.iter()
        .map(|command| {
            command.map(|command| MenuRow {
                command,
                title: strings.resolve(command.title_key()).to_owned(),
                chord: command
                    .chord_action()
                    .and_then(|action| action.chord_in(settings))
                    .map(|chord| chord.display()),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strings::DisplayLanguage;

    #[test]
    fn the_menu_is_the_same_rows_on_every_desktop() {
        // trace: the Mac's TaigiInputControllerMenuTests literal oracle —
        // the authored Hanji, the default chords.
        let strings = StringResolver::new(DisplayLanguage::Hanji);
        let rows: Vec<Option<(String, Option<String>)>> =
            menu_rows(&strings, &SettingsDocument::default())
                .into_iter()
                .map(|row| row.map(|row| (row.title, row.chord)))
                .collect();
        let row =
            |title: &str, chord: Option<&str>| Some((title.to_owned(), chord.map(str::to_owned)));
        assert_eq!(
            rows,
            [
                row("切換台羅/白話字", Some("Ctrl+Alt+C")),
                row("切換候選詞顯示", Some("Ctrl+Alt+H")),
                None,
                row("台語齒盤設定", Some("Ctrl+Alt+S")),
                None,
                row("檢查更新", None),
                row("關於齒盤", None),
            ]
        );
    }

    #[test]
    fn a_cleared_chord_prints_the_title_alone() {
        let strings = StringResolver::new(DisplayLanguage::Hanji);
        let mut cleared = SettingsDocument::default();
        ShortcutAction::OpenLastSettingsPane.store_in(&mut cleared, None);
        let rows = menu_rows(&strings, &cleared);
        let settings = rows[3].as_ref().expect("the settings row");
        assert_eq!(settings.command, MenuCommand::OpenSettings);
        assert_eq!(settings.chord, None);
    }
}
