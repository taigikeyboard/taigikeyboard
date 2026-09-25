//! Where things live on a Linux desktop (roadmap L7).
//!
//! User data follows the XDG Base Directory specification, split the way
//! the spec splits it: `settings.json` is configuration
//! (`$XDG_CONFIG_HOME/taigikeyboard`), the learning databases and the custom
//! dictionary are data (`$XDG_DATA_HOME/taigikeyboard`). Windows keeps both
//! in one `%APPDATA%\TaigiKeyboard` — a named divergence; the desktop
//! storage crate does not care which directory it is handed.
//!
//! Read-only assets come from the install prefix baked in at build time
//! (`TAIGIKEYBOARD_PREFIX`, default `/usr`), the way a distribution package
//! expects; `TAIGIKEYBOARD_DATA_DIR` at runtime points a development tree at
//! the repository's own `dictionaries/` without installing anything.
//!
//! Every function takes the environment as a closure so a test can hand in
//! any combination without mutating the process environment.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The application's directory name under each XDG base directory. Also
/// what the installer, the `.deb` and the settings window agree on.
pub const APPLICATION_DIRECTORY_NAME: &str = "taigikeyboard";

/// The runtime override for a development tree: a directory holding
/// `dictionaries/` (the repository root, typically).
pub const DATA_DIR_OVERRIDE: &str = "TAIGIKEYBOARD_DATA_DIR";

/// The prefix the build was configured for, or `/usr`.
pub fn install_prefix() -> &'static str {
    option_env!("TAIGIKEYBOARD_PREFIX").unwrap_or("/usr")
}

/// The two per-user directories, resolved but not yet created.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserDirectories {
    /// `settings.json`.
    pub config: PathBuf,
    /// The learning databases and the custom dictionary.
    pub data: PathBuf,
}

impl UserDirectories {
    /// From the process environment.
    pub fn resolve() -> Option<Self> {
        Self::resolve_with(|name| std::env::var_os(name))
    }

    /// From `env` — `$XDG_CONFIG_HOME` / `$XDG_DATA_HOME`, falling back to
    /// the spec's defaults under `$HOME`. `None` with no `HOME` and no XDG
    /// variable: a process with no user (a login manager's session, a
    /// service) has nowhere to keep learning data and runs on defaults.
    pub fn resolve_with(env: impl Fn(&str) -> Option<OsString>) -> Option<Self> {
        let config = xdg_base(&env, "XDG_CONFIG_HOME", ".config")?;
        let data = xdg_base(&env, "XDG_DATA_HOME", ".local/share")?;
        Some(Self {
            config: config.join(APPLICATION_DIRECTORY_NAME),
            data: data.join(APPLICATION_DIRECTORY_NAME),
        })
    }
}

/// `$<variable>` when set to an absolute path (the spec ignores a relative
/// one), else `$HOME/<default>`.
fn xdg_base(
    env: &impl Fn(&str) -> Option<OsString>,
    variable: &str,
    default_under_home: &str,
) -> Option<PathBuf> {
    if let Some(value) = env(variable) {
        let path = PathBuf::from(value);
        if path.is_absolute() {
            return Some(path);
        }
    }
    let home = env("HOME")?;
    if home.is_empty() {
        return None;
    }
    Some(PathBuf::from(home).join(default_under_home))
}

/// The configuration directory (`settings.json`).
pub fn config_directory() -> Option<PathBuf> {
    UserDirectories::resolve().map(|directories| directories.config)
}

/// The data directory (databases).
pub fn data_directory() -> Option<PathBuf> {
    UserDirectories::resolve().map(|directories| directories.data)
}

/// Where the read-only assets and the settings window are, for one prefix.
/// The IBus engine's directory is per distribution (linux/Makefile LAYOUT)
/// and only the component XML names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallLayout {
    prefix: PathBuf,
}

impl InstallLayout {
    pub fn new(prefix: impl AsRef<Path>) -> Self {
        Self {
            prefix: prefix.as_ref().to_path_buf(),
        }
    }

    /// The prefix this build was configured for.
    pub fn shipped() -> Self {
        Self::new(install_prefix())
    }

    /// `<prefix>/share/taigikeyboard` — the read-only assets.
    pub fn share_directory(&self) -> PathBuf {
        self.prefix.join("share").join(APPLICATION_DIRECTORY_NAME)
    }

    /// `<prefix>/share/taigikeyboard/dictionaries` — the four engine
    /// artifacts (`DictionaryArtifacts::FILE_NAMES`).
    pub fn dictionaries_directory(&self) -> PathBuf {
        self.share_directory().join("dictionaries")
    }

    /// `<prefix>/bin/taigikeyboard-settings`.
    pub fn settings_binary(&self) -> PathBuf {
        self.prefix.join("bin").join(SETTINGS_BINARY_NAME)
    }
}

/// The settings window's binary name (the Linux spelling of
/// `taigi_desktop_core::settings::launch::SETTINGS_EXE_NAME`).
pub const SETTINGS_BINARY_NAME: &str = "taigikeyboard-settings";

/// The dictionaries: `$TAIGIKEYBOARD_DATA_DIR/dictionaries` when the
/// override is set (a development tree), else the shipped prefix.
pub fn dictionaries_directory() -> PathBuf {
    dictionaries_directory_with(|name| std::env::var_os(name), &InstallLayout::shipped())
}

pub fn dictionaries_directory_with(
    env: impl Fn(&str) -> Option<OsString>,
    layout: &InstallLayout,
) -> PathBuf {
    match env(DATA_DIR_OVERRIDE) {
        Some(root) if !root.is_empty() => PathBuf::from(root).join("dictionaries"),
        _ => layout.dictionaries_directory(),
    }
}

/// The settings window's binary for the shipped prefix.
pub fn settings_binary() -> PathBuf {
    InstallLayout::shipped().settings_binary()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let map: HashMap<String, OsString> = pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), OsString::from(value)))
            .collect();
        move |name| map.get(name).cloned()
    }

    #[test]
    fn xdg_variables_win_over_home() {
        // trace: spec — an absolute $XDG_CONFIG_HOME replaces ~/.config.
        let directories = UserDirectories::resolve_with(env(&[
            ("HOME", "/home/u"),
            ("XDG_CONFIG_HOME", "/cfg"),
            ("XDG_DATA_HOME", "/dat"),
        ]))
        .unwrap();
        assert_eq!(directories.config, PathBuf::from("/cfg/taigikeyboard"));
        assert_eq!(directories.data, PathBuf::from("/dat/taigikeyboard"));
    }

    #[test]
    fn home_defaults_apply_when_the_variables_are_unset_or_relative() {
        // trace: spec — a relative XDG path is ignored, the default is used.
        let directories = UserDirectories::resolve_with(env(&[
            ("HOME", "/home/u"),
            ("XDG_DATA_HOME", "relative/data"),
        ]))
        .unwrap();
        assert_eq!(
            directories.config,
            PathBuf::from("/home/u/.config/taigikeyboard")
        );
        assert_eq!(
            directories.data,
            PathBuf::from("/home/u/.local/share/taigikeyboard")
        );
    }

    #[test]
    fn no_home_and_no_variable_means_no_directory() {
        assert_eq!(UserDirectories::resolve_with(env(&[])), None);
        assert_eq!(UserDirectories::resolve_with(env(&[("HOME", "")])), None);
    }

    #[test]
    fn the_shipped_layout_puts_assets_under_share_and_the_settings_window_under_bin() {
        let layout = InstallLayout::new("/usr/local");
        assert_eq!(
            layout.dictionaries_directory(),
            PathBuf::from("/usr/local/share/taigikeyboard/dictionaries")
        );
        assert_eq!(
            layout.settings_binary(),
            PathBuf::from("/usr/local/bin/taigikeyboard-settings")
        );
    }

    #[test]
    fn the_data_dir_override_points_a_dev_tree_at_its_own_dictionaries() {
        let layout = InstallLayout::new("/usr");
        assert_eq!(
            dictionaries_directory_with(env(&[(DATA_DIR_OVERRIDE, "/src/taigikeyboard")]), &layout),
            PathBuf::from("/src/taigikeyboard/dictionaries")
        );
        assert_eq!(
            dictionaries_directory_with(env(&[(DATA_DIR_OVERRIDE, "")]), &layout),
            PathBuf::from("/usr/share/taigikeyboard/dictionaries")
        );
        assert_eq!(
            dictionaries_directory_with(env(&[]), &layout),
            PathBuf::from("/usr/share/taigikeyboard/dictionaries")
        );
    }
}
