//! The command line between the input method and its settings window: one
//! place both sides read, so the spawners
//! (`taigi-windows-tsf::settings_launcher`, `taigi-linux-platform::launcher`)
//! and the parser ([`LaunchOptions::parse`]) cannot drift.

use super::{SettingChoice, SettingsPane};

/// The settings window's executable, beside the DLL in the install
/// directory (roadmap W1).
pub const SETTINGS_EXE_NAME: &str = "TaigiKeyboardSettings.exe";
/// `--pane <raw>`: open on this pane (absent = where the user left it).
pub const PANE_FLAG: &str = "--pane";
/// `--check-now`: run an update check with the window up (the menu's
/// Check for Updates row, `TaigiInputController+Menu.swift` `checkForUpdates(_:)`).
pub const CHECK_NOW_FLAG: &str = "--check-now";
/// `--check-updates`: the headless check the scheduled task runs (W9);
/// no window.
pub const CHECK_UPDATES_FLAG: &str = "--check-updates";
/// `--prewarm`: map the Windows App Runtime this exe's window is built on
/// and exit. No window, no settings write, no stores — the process is
/// gone in about a second, and the images it mapped stay cached for the
/// launch that really opens the window (measured: a cold first open drops
/// from ~2.5 s to ~1.1 s to painted content).
pub const PREWARM_FLAG: &str = "--prewarm";
/// `--tps-keyboard`: show the on-screen TPS key panel, or close it when it
/// is up, without the settings window (desktop TPS roadmap D6). Linux only:
/// its panel is a window of the settings app; the Windows and macOS panels
/// belong to the input method and are never launched.
pub const TPS_KEYBOARD_FLAG: &str = "--tps-keyboard";

/// The parsed command line, the same for both settings windows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LaunchOptions {
    /// `--pane <raw>`; an unknown raw value is ignored (the stored pane wins).
    pub pane: Option<SettingsPane>,
    /// `--check-now`: the menu's Check for Updates — check with the window up.
    pub check_now: bool,
    /// `--check-updates`: the scheduled task's check, no window.
    pub headless_check: bool,
    /// `--prewarm`: map the WinUI runtime and exit, no window. The Windows
    /// `main` reads this before the other modes, so a launch carrying it
    /// writes no settings, opens no store and reaches no network.
    pub prewarm: bool,
    /// `--tps-keyboard`: toggle the TPS key panel, no settings window.
    pub toggles_tps_keyboard: bool,
    /// The first update / prewarm flag in argument order. Only the Windows
    /// window has a behaviour for those; the Linux one refuses the launch
    /// and names this flag.
    pub first_windows_only_flag: Option<&'static str>,
}

impl LaunchOptions {
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Self {
        let mut options = Self::default();
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            let windows_only_flag = match argument.as_str() {
                PANE_FLAG => {
                    options.pane = arguments
                        .next()
                        .and_then(|raw| SettingsPane::from_raw(&raw));
                    continue;
                }
                CHECK_NOW_FLAG => {
                    options.check_now = true;
                    CHECK_NOW_FLAG
                }
                CHECK_UPDATES_FLAG => {
                    options.headless_check = true;
                    CHECK_UPDATES_FLAG
                }
                PREWARM_FLAG => {
                    options.prewarm = true;
                    PREWARM_FLAG
                }
                TPS_KEYBOARD_FLAG => {
                    options.toggles_tps_keyboard = true;
                    continue;
                }
                other => {
                    log::warn!("cli.unknown_argument argument={other}");
                    continue;
                }
            };
            options
                .first_windows_only_flag
                .get_or_insert(windows_only_flag);
        }
        options
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> LaunchOptions {
        LaunchOptions::parse(arguments.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn the_launcher_contract_round_trips() {
        // trace: the Windows `settings_launcher::check_for_updates` spawns
        // `--pane general --check-now`; the Linux `launcher::open_settings(
        // Some("about"))` spawns `--pane about`.
        let options = parse(&["--pane", "general", "--check-now"]);
        assert_eq!(options.pane, Some(SettingsPane::General));
        assert!(options.check_now);
        assert!(!options.headless_check);
        assert_eq!(
            parse(&["--pane", "about"]),
            LaunchOptions {
                pane: Some(SettingsPane::About),
                ..LaunchOptions::default()
            }
        );
        assert_eq!(parse(&[]), LaunchOptions::default());
    }

    #[test]
    fn a_prewarm_flag_turns_on_only_the_prewarm_mode() {
        // trace: settings_launcher::prewarm_once spawns `--prewarm` alone.
        // Which mode wins when flags are mixed is `main`'s statement
        // order, not this parser's — nothing here can assert it.
        assert_eq!(
            parse(&["--prewarm"]),
            LaunchOptions {
                prewarm: true,
                first_windows_only_flag: Some(PREWARM_FLAG),
                ..LaunchOptions::default()
            }
        );
        assert!(!parse(&["--pane", "general"]).prewarm);
    }

    #[test]
    fn an_unknown_pane_or_flag_is_ignored_and_a_dangling_pane_flag_is_harmless() {
        assert_eq!(parse(&["--pane", "bogus"]).pane, None);
        assert_eq!(parse(&["--pane"]).pane, None);
        assert!(parse(&["--whatever", "--check-updates"]).headless_check);
        assert_eq!(
            parse(&["--pane", "shortcuts"]).pane,
            Some(SettingsPane::Shortcuts)
        );
    }

    #[test]
    fn the_last_pane_flag_wins_even_when_it_names_no_pane() {
        assert_eq!(
            parse(&["--pane", "general", "--pane", "about"]).pane,
            Some(SettingsPane::About)
        );
        assert_eq!(parse(&["--pane", "general", "--pane", "bogus"]).pane, None);
        assert_eq!(parse(&["--pane", "general", "--pane"]).pane, None);
    }

    #[test]
    fn the_first_windows_only_flag_is_the_first_one_in_argument_order() {
        assert_eq!(parse(&["--pane", "about"]).first_windows_only_flag, None);
        assert_eq!(
            parse(&["--check-now"]).first_windows_only_flag,
            Some(CHECK_NOW_FLAG)
        );
        assert_eq!(
            parse(&["--pane", "general", "--prewarm", "--check-updates"]).first_windows_only_flag,
            Some(PREWARM_FLAG)
        );
        assert_eq!(
            parse(&["--check-updates", "--check-now"]).first_windows_only_flag,
            Some(CHECK_UPDATES_FLAG)
        );
        // `--pane` takes the next argument as its value, whatever it is.
        let swallowed = parse(&["--pane", "--prewarm"]);
        assert_eq!(swallowed.first_windows_only_flag, None);
        assert!(!swallowed.prewarm);
    }

    #[test]
    fn the_tps_keyboard_flag_asks_for_the_panel_alone() {
        // trace: the Linux `launcher::toggle_tps_keyboard` spawns
        // `--tps-keyboard` alone; it is not a Windows-only flag (the Linux
        // window owns the behaviour).
        assert_eq!(
            parse(&["--tps-keyboard"]),
            LaunchOptions {
                toggles_tps_keyboard: true,
                ..LaunchOptions::default()
            }
        );
    }
}
