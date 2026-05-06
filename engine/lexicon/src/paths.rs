//! `LexiconPaths` — install-time absolute-path snapshot.
//!
//! Built from `engine/protos::InstallRequest` at install/reinstall time.
//! Validates each path before mmap: rejects NUL bytes, rejects non-absolute
//! paths. Engine NEVER calls `canonicalize` (would surprise iOS Bundle
//! paths + introduce extra syscalls) — it accepts the platform-supplied
//! path verbatim.

// 中文: 安裝期間的絕對路徑快照 — 拒絕 NUL byte 與相對路徑;不做 canonicalize 以避免 iOS Bundle 路徑被改寫。

use std::path::{Path, PathBuf};

use crate::error::LexiconError;

// 中文: 安裝時的絕對路徑與字典版本快照,作為 EngineHandle::install 的輸入。
#[derive(Debug, Clone)]
pub struct LexiconPaths {
    // 中文: dictionary.fst 的絕對路徑 (前綴索引)。
    pub fst: PathBuf,
    // 中文: dictionary.bin 的絕對路徑 (TKDB 詞庫)。
    pub dictionary_bin: PathBuf,
    // 中文: association.bin 的絕對路徑 (TKWA bigram)。
    pub association_bin: PathBuf,
    // 中文: 字典版本號,供平台對齊驗證。
    pub dictionary_version: u32,
}

impl LexiconPaths {
    /// Build from raw `InstallRequest` fields. Validates each path.
    // 中文: 從原始 InstallRequest 字串建立 LexiconPaths,並驗證三個路徑。
    pub fn validated(
        fst: &str,
        dictionary_bin: &str,
        association_bin: &str,
        dictionary_version: u32,
    ) -> Result<Self, LexiconError> {
        let fst = validate(fst)?;
        let dictionary_bin = validate(dictionary_bin)?;
        let association_bin = validate(association_bin)?;
        Ok(Self {
            fst,
            dictionary_bin,
            association_bin,
            dictionary_version,
        })
    }
}

fn validate(raw: &str) -> Result<PathBuf, LexiconError> {
    if raw.bytes().any(|b| b == 0) {
        return Err(LexiconError::InvalidPath(raw.to_string()));
    }
    let path = Path::new(raw);
    if !path.is_absolute() {
        return Err(LexiconError::PathNotAbsolute(raw.to_string()));
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_nul_byte() {
        let err = LexiconPaths::validated("/foo\0bar.fst", "/dict.bin", "/assoc.bin", 1)
            .err()
            .expect("expected NUL byte rejection");
        assert!(matches!(err, LexiconError::InvalidPath(_)), "{err:?}");
    }

    #[test]
    fn rejects_non_absolute() {
        let err = LexiconPaths::validated("relative.fst", "/dict.bin", "/assoc.bin", 1)
            .err()
            .expect("expected non-absolute rejection");
        assert!(matches!(err, LexiconError::PathNotAbsolute(_)), "{err:?}");
    }

    #[test]
    fn accepts_absolute_clean_paths() {
        let paths = LexiconPaths::validated(
            "/var/lib/taigi/dictionary.fst",
            "/var/lib/taigi/dictionary.bin",
            "/var/lib/taigi/association.bin",
            42,
        )
        .expect("validated paths");
        assert_eq!(paths.dictionary_version, 42);
        assert_eq!(paths.fst.to_string_lossy(), "/var/lib/taigi/dictionary.fst");
    }
}
