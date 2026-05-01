//! `EngineHandle` — process-singleton lifecycle for the lexicon engine.
//!
//! Holds `Mutex<Option<EngineState>>`. `install` builds a fresh
//! `EngineState` from validated paths and atomically swaps it in
//! ON SUCCESS — if any step fails, the previous state stays intact.
//!
//! `with_state` borrows the active state under the mutex, runs the
//! caller's closure, and returns. Concurrent `search` calls serialize
//! with each other and with `install`. No read/write split until
//! profiling proves contention (audit § scope D8 deferred).

use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::association_reader::AssociationReader;
use crate::dictionary_reader::DictionaryReader;
use crate::error::LexiconError;
use crate::paths::LexiconPaths;
use crate::prefix_index::PrefixIndex;

/// Active lexicon state. All fields are populated on a successful install;
/// `with_state` callers verify they are present (defensive — not expected
/// to fail post-install).
pub struct EngineState {
    pub dictionary_version: u32,
    pub prefix_index: Option<PrefixIndex>,
    pub dictionary: Option<DictionaryReader>,
    pub association: Option<AssociationReader>,
}

pub struct EngineHandle;

static STATE: Lazy<Mutex<Option<EngineState>>> = Lazy::new(|| Mutex::new(None));

#[derive(Debug, Clone, Copy)]
pub struct InstallStats {
    pub dictionary_record_count: u64,
    pub prefix_index_entry_count: u64,
}

impl EngineHandle {
    /// Install (or reinstall) the lexicon state. Idempotent: on success the
    /// new state atomically replaces any previous; on failure (open / mmap
    /// / format error) the previous state stays intact.
    pub fn install(paths: LexiconPaths) -> Result<InstallStats, LexiconError> {
        let prefix_index = PrefixIndex::open(&paths.fst)?;
        let dictionary = DictionaryReader::open(&paths.dictionary_bin)?;
        let association = AssociationReader::open(&paths.association_bin)?;

        let stats = InstallStats {
            dictionary_record_count: dictionary.record_count() as u64,
            prefix_index_entry_count: prefix_index.entry_count(),
        };

        let new_state = EngineState {
            dictionary_version: paths.dictionary_version,
            prefix_index: Some(prefix_index),
            dictionary: Some(dictionary),
            association: Some(association),
        };

        let mut guard = STATE
            .lock()
            .map_err(|_| LexiconError::Internal("install: state mutex poisoned".into()))?;
        *guard = Some(new_state);

        Ok(stats)
    }

    pub fn with_state<F, R>(f: F) -> Result<R, LexiconError>
    where
        F: FnOnce(&EngineState) -> Result<R, LexiconError>,
    {
        let guard = STATE
            .lock()
            .map_err(|_| LexiconError::Internal("with_state: state mutex poisoned".into()))?;
        match guard.as_ref() {
            Some(state) => f(state),
            None => Err(LexiconError::NotInitialized),
        }
    }
}
