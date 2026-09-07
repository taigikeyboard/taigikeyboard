//! The four dictionary files the engine mmaps, located and validated before
//! the engine is asked to load them. Port of
//! `macos/Sources/TaigiInputMethodCore/Engine/DictionaryArtifacts.swift`.

use std::path::{Path, PathBuf};

/// Absolute paths to the four artefacts, every one of them required — a
/// missing `syllables.fst` is a corrupt install, not an optional degrade.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryArtifacts {
    pub trie_path: PathBuf,
    pub dictionary_bin_path: PathBuf,
    pub association_bin_path: PathBuf,
    pub syllable_inventory_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum DictionaryArtifactsError {
    #[error("dictionary artefact missing or empty: {0}")]
    MissingOrEmpty(PathBuf),
}

impl DictionaryArtifacts {
    /// File names as `dictionary/build/deploy.sh` writes them.
    /// The install directory's dictionary folder — where the installer
    /// copies the repo-root `dictionaries/`.
    pub const DIRECTORY_NAME: &'static str = "Dictionaries";

    pub const FILE_NAMES: [&'static str; 4] = [
        "dictionary.fst",
        "dictionary.bin",
        "association.bin",
        "syllables.fst",
    ];

    /// Resolves the four files under `base_dir` and checks each exists with
    /// a non-zero size. Fails on the first problem, naming the file.
    pub fn locate(base_dir: &Path) -> Result<Self, DictionaryArtifactsError> {
        let require = |name: &str| -> Result<PathBuf, DictionaryArtifactsError> {
            let path = base_dir.join(name);
            match std::fs::metadata(&path) {
                Ok(metadata) if metadata.is_file() && metadata.len() > 0 => Ok(path),
                _ => Err(DictionaryArtifactsError::MissingOrEmpty(path)),
            }
        };
        Ok(Self {
            trie_path: require(Self::FILE_NAMES[0])?,
            dictionary_bin_path: require(Self::FILE_NAMES[1])?,
            association_bin_path: require(Self::FILE_NAMES[2])?,
            syllable_inventory_path: require(Self::FILE_NAMES[3])?,
        })
    }

    /// A path as the engine's `InstallRequest` wants it: a UTF-8 string. The
    /// install dir is ASCII on every supported layout (`%ProgramFiles%`),
    /// and a user who relocates it into non-UTF-8 bytes gets a loud failure
    /// at install rather than a silently mangled path.
    pub fn wire(path: &Path) -> Option<String> {
        path.to_str().map(str::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locate_requires_every_file_non_empty() {
        let dir = std::env::temp_dir().join(format!("taigi-artifacts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in DictionaryArtifacts::FILE_NAMES {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        assert!(DictionaryArtifacts::locate(&dir).is_ok());

        std::fs::write(dir.join("syllables.fst"), b"").unwrap();
        let err = DictionaryArtifacts::locate(&dir).unwrap_err();
        assert!(err.to_string().contains("syllables.fst"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The dictionary stamp the engine caches under: a crate version as one
/// integer (`3.6.6` → `30606`), the same number the macOS `CFBundleVersion`
/// carries, because the data is rebuilt by the release that bumps it. Every
/// process that installs the lexicon (the DLL, the settings window) passes
/// its own `CARGO_PKG_VERSION` — one workspace version, one stamp.
pub fn dictionary_version(crate_version: &str) -> u32 {
    let mut parts = crate_version
        .split('.')
        .map(|part| part.parse::<u32>().unwrap_or(0));
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    let patch = parts.next().unwrap_or(0);
    assert!(
        minor < 100 && patch < 100,
        "version components must stay below 100"
    );
    major * 10_000 + minor * 100 + patch
}

#[cfg(test)]
mod version_tests {
    use super::*;

    #[test]
    fn the_stamp_has_the_macos_bundle_version_shape() {
        assert_eq!(dictionary_version("3.6.6"), 30_606);
        assert_eq!(dictionary_version("10.0.0"), 100_000);
        // The crate's own version, read for its patch rather than compared to
        // a literal: the literal was `6`, and the 3.6.7 bump turned a
        // property of the encoding into a failing test of the version number.
        let version = env!("CARGO_PKG_VERSION");
        let patch: u32 = version.rsplit('.').next().unwrap().parse().unwrap();
        assert_eq!(dictionary_version(version) % 100, patch);
    }
}
