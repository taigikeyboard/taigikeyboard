//! The four dictionary files the engine mmaps, located and validated before
//! the engine is asked to load them. Port of
//! `macos/Sources/TaigiInputMethodCore/Engine/DictionaryArtifacts.swift`.

// 中文: 引擎要 mmap 的四個辭典檔;找到並驗證存在且非空,再交給引擎。

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
