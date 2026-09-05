//! `mmap-host` — read-only mmap helper crate.
//!
//! Owns the only NEW `unsafe_code = "allow"` carve-out introduced by the
//! v3.5.6 lexicon slice (`docs/engine/lexicon-slice-plan.md` G4). The
//! `unsafe` block lives in `MmapHandle::open_readonly` and is gated by
//! a single SAFETY comment per `.claude/rules/rust-best-practices.md` §4.
//!
//! All consumers (currently `engine/lexicon`) take a `&[u8]` whose lifetime
//! is tied to the `&MmapHandle` borrow, so the unsafe-ness does not leak.

// 唯讀記憶體映射輔助 crate,集中管理整個 workspace 唯一的 unsafe 開放區。
// 對外只暴露受借用生命週期約束的 &[u8],使 unsafe 不外漏給呼叫端。

use std::fs::File;
use std::path::Path;

use memmap2::Mmap;

// mmap-host 的錯誤型別,區分檔案開啟失敗與記憶體映射失敗兩種情境。
#[derive(thiserror::Error, Debug)]
pub enum MmapError {
    // 檔案無法開啟,通常是路徑不存在或權限不足。
    #[error("open `{path}`: {source}")]
    Open {
        path: String,
        #[source]
        source: std::io::Error,
    },
    // 檔案開啟成功但 mmap 系統呼叫失敗。
    #[error("mmap `{path}`: {source}")]
    Map {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Read-only mmap handle. Drop closes the mmap.
// 唯讀的 mmap 把手,Drop 時自動釋放映射區。
pub struct MmapHandle {
    mmap: Mmap,
}

impl MmapHandle {
    /// Open `path` read-only and mmap the entire file.
    ///
    /// # Errors
    /// Returns [`MmapError::Open`] when the file cannot be opened or
    /// [`MmapError::Map`] when mmap fails.
    // 以唯讀模式開啟檔案並對整份內容做記憶體映射,失敗時回傳對應錯誤分支。
    pub fn open_readonly(path: &Path) -> Result<Self, MmapError> {
        let path_string = path.display().to_string();
        let file = File::open(path).map_err(|source| MmapError::Open {
            path: path_string.clone(),
            source,
        })?;
        // SAFETY: `file` is a freshly-opened read-only File. The mmap region
        // is exposed only as `&[u8]` through `as_slice` whose lifetime is
        // tied to `&self`, preventing dangling references after Drop. The
        // mmap region MAY be mutated by another process holding a
        // read-write fd, but the only such writer for our bundled assets
        // is the build pipeline's `fst-builder`, which never runs
        // concurrently with the engine on a user device.
        let mmap = unsafe { Mmap::map(&file) }.map_err(|source| MmapError::Map {
            path: path_string,
            source,
        })?;
        Ok(Self { mmap })
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.mmap
    }

    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mmap.is_empty()
    }
}
