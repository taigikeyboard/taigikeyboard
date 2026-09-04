//! The command line the DLL spawns this window with (`settings::launch`,
//! one contract for both sides): which pane to open on, whether to run an
//! update check with the window up, or headless.

// 中文: 解析 DLL 傳來的命令列 — 開哪個 pane、要不要檢查更新。

use taigi_windows_core::settings::launch::{
    CHECK_NOW_FLAG, CHECK_UPDATES_FLAG, PANE_FLAG, PREWARM_FLAG,
};
use taigi_windows_core::settings::{SettingChoice, SettingsPane};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LaunchOptions {
    /// `--pane <raw>`; an unknown raw value is ignored (the stored pane wins).
    pub pane: Option<SettingsPane>,
    /// `--check-now`: the menu's 檢查更新 — check with the window up (PR9).
    pub check_now: bool,
    /// `--check-updates`: the scheduled task's check, no window (PR9).
    pub headless_check: bool,
    /// `--prewarm`: map the WinUI runtime and exit, no window (`prewarm`).
    /// `main` reads this before the other modes, so a launch carrying it
    /// writes no settings, opens no store and reaches no network.
    pub prewarm: bool,
}

impl LaunchOptions {
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Self {
        let mut options = Self::default();
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                PANE_FLAG => {
                    options.pane = arguments
                        .next()
                        .and_then(|raw| SettingsPane::from_raw(&raw));
                }
                CHECK_NOW_FLAG => options.check_now = true,
                CHECK_UPDATES_FLAG => options.headless_check = true,
                PREWARM_FLAG => options.prewarm = true,
                other => log::warn!("cli.unknown_argument argument={other}"),
            }
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
        // trace: settings_launcher::check_for_updates spawns
        // `--pane general --check-now`.
        let options = parse(&["--pane", "general", "--check-now"]);
        assert_eq!(options.pane, Some(SettingsPane::General));
        assert!(options.check_now);
        assert!(!options.headless_check);
        assert_eq!(parse(&[]), LaunchOptions::default());
    }

    #[test]
    fn a_prewarm_flag_sets_only_the_prewarm_field() {
        // trace: settings_launcher::prewarm_once spawns `--prewarm` alone.
        // Which mode wins when flags are mixed is `main`'s statement
        // order, not this parser's — nothing here can assert it.
        assert_eq!(
            parse(&["--prewarm"]),
            LaunchOptions {
                prewarm: true,
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
}
