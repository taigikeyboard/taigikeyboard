//! The file-backed handle every native platform uses.

use std::fs::File;
use std::path::Path;

use memmap2::Mmap;

use crate::MmapError;

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
}
