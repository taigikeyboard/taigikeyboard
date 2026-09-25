//! The command line the engine spawns this window with (`settings::launch`,
//! one contract for both sides): which pane to open on. The Windows-only
//! flags are refused with a readable error rather than ignored (roadmap L8).

use std::fmt;
use taigi_desktop_core::settings::launch::{
    CHECK_NOW_FLAG, CHECK_UPDATES_FLAG, PANE_FLAG, PREWARM_FLAG,
};
use taigi_desktop_core::settings::{SettingChoice, SettingsPane};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LaunchOptions {
    /// `--pane <raw>`; an unknown raw value is ignored (the stored pane wins).
    pub pane: Option<SettingsPane>,
}

/// A flag this platform has no behaviour for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsupportedFlag(pub String);

impl fmt::Display for UnsupportedFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` is not supported on Linux (updates are the package manager's); only `{PANE_FLAG} <pane>` is accepted",
            self.0
        )
    }
}

impl std::error::Error for UnsupportedFlag {}

impl LaunchOptions {
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, UnsupportedFlag> {
        let mut options = Self::default();
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                PANE_FLAG => {
                    options.pane = arguments
                        .next()
                        .and_then(|raw| SettingsPane::from_raw(&raw));
                }
                CHECK_NOW_FLAG | CHECK_UPDATES_FLAG | PREWARM_FLAG => {
                    return Err(UnsupportedFlag(argument));
                }
                other => log::warn!("cli.unknown_argument argument={other}"),
            }
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<LaunchOptions, UnsupportedFlag> {
        LaunchOptions::parse(arguments.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn the_launcher_contract_round_trips() {
        // trace: launcher::open_settings(Some("about")) spawns `--pane about`.
        assert_eq!(
            parse(&["--pane", "about"]).unwrap().pane,
            Some(SettingsPane::About)
        );
        assert_eq!(parse(&[]).unwrap(), LaunchOptions::default());
        assert_eq!(parse(&["--pane", "bogus"]).unwrap().pane, None);
        assert_eq!(parse(&["--pane"]).unwrap().pane, None);
    }

    #[test]
    fn the_windows_only_flags_are_refused_by_name() {
        let error = parse(&["--check-now"]).unwrap_err();
        assert_eq!(error, UnsupportedFlag("--check-now".into()));
        assert!(error.to_string().contains("--check-now"));
        assert!(parse(&["--prewarm"]).is_err());
        assert!(parse(&["--check-updates"]).is_err());
    }
}
