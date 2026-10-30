//! Helpers shared by the engine crates' integration tests (dev-dependency
//! only): temp files, the `dictionary.fst` / `dictionary.bin` /
//! `association.bin` fixture serializers, the per-binary install lock, and
//! the production artifacts (`assets/dictionaries/`, `dictionary/output/
//! dictionary.csv`).
//!
//! Only crate-neutral helpers live here. Anything that needs an engine type
//! (`lexicon::LexiconPaths`, composing requests) stays in that crate's
//! `tests/common/mod.rs`.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

mod files;
mod production;
mod registration;
mod tkdb;

pub use files::{fst_entry, write_fst_set, write_temp};
pub use production::{
    dictionary_cjk_py_path, dictionary_csv, dictionary_csv_or_skip, dictionary_csv_path,
    production_artifact, DictionaryCsv, ProductionArtifacts,
};
pub use registration::assert_every_test_file_is_declared;
pub use tkdb::{
    build_tkdb, build_tkdb_with_bigrams, build_tkdb_with_model, build_tkwa,
    walker_cost_from_fixture_frequency, TkdbBigrams, TkdbModel, TkdbRow, TkwaEntry, TKDB_VERSION,
};

/// Serializes `lexicon::EngineHandle::install` vs. assertion within ONE test
/// binary. Each binary is its own process with its own global lexicon
/// singleton, so every test that installs a fixture holds this lock from
/// install through its last assertion — cargo runs a binary's `#[test]`s in
/// parallel. A panicking holder does not wedge the rest of the binary.
pub fn engine_install_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

/// The repository root (`engine/test-support` → two levels up).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
