//! The in-memory handle `wasm32` uses: this target has no file to map, so the
//! host hands the dictionary bytes in before it installs the lexicon, under
//! the paths the install request will name.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::MmapError;

/// Written at install time only, the exception named in
/// `docs/contributing/rust-best-practices.md` §8. An entry stays after a
/// handle opens it, so a reinstall finds it; a handle keeps its own bytes
/// alive when the entry is replaced, as a mapping outlives its file.
static REGISTRY: Mutex<BTreeMap<PathBuf, Arc<Vec<u8>>>> = Mutex::new(BTreeMap::new());

/// Makes `bytes` what [`MmapHandle::open_readonly`] answers for `path`.
pub fn register(path: &Path, bytes: Vec<u8>) {
    entries().insert(path.to_path_buf(), Arc::new(bytes));
}

/// A lock poisoned by a panic elsewhere still holds a whole map.
fn entries() -> std::sync::MutexGuard<'static, BTreeMap<PathBuf, Arc<Vec<u8>>>> {
    REGISTRY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Read-only handle on registered bytes.
pub struct MmapHandle {
    bytes: Arc<Vec<u8>>,
}

impl MmapHandle {
    /// The bytes registered for `path`.
    ///
    /// # Errors
    /// Returns [`MmapError::Open`] when nothing is registered there.
    pub fn open_readonly(path: &Path) -> Result<Self, MmapError> {
        match entries().get(path) {
            Some(bytes) => Ok(Self {
                bytes: Arc::clone(bytes),
            }),
            None => Err(MmapError::Open {
                path: path.display().to_string(),
                source: io::ErrorKind::NotFound.into(),
            }),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }
}
