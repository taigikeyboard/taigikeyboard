//! `mmap-host` — read-only mmap helper crate.
//!
//! Owns the only NEW `unsafe_code = "allow"` carve-out introduced by the
//! v3.5.6 lexicon slice. The
//! `unsafe` block lives in `MmapHandle::open_readonly` and is gated by
//! a single SAFETY comment per `.claude/rules/rust-best-practices.md` §4.
//!
//! All consumers (currently `engine/lexicon`) take a `&[u8]` whose lifetime
//! is tied to the `&MmapHandle` borrow, so the unsafe-ness does not leak.

use std::fs::File;
use std::path::Path;

use memmap2::Mmap;

#[derive(thiserror::Error, Debug)]
pub enum MmapError {
    #[error("open `{path}`: {source}")]
    Open {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("mmap `{path}`: {source}")]
    Map {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Read-only mmap handle. Drop closes the mmap.
pub struct MmapHandle {
    mmap: Mmap,
}

impl MmapHandle {
    /// Open `path` read-only and mmap the entire file.
    ///
    /// # Errors
    /// Returns [`MmapError::Open`] when the file cannot be opened or
    /// [`MmapError::Map`] when mmap fails.
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
