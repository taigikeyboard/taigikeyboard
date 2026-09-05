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
use crate::syllable_inventory::SyllableInventory;

/// Active lexicon state. The mandatory readers (`prefix_index`,
/// `dictionary`, `association`) are populated on every successful install;
/// `syllable_inventory` is opt-in (Phase 6 adds the wiring; the platform
/// only supplies the path once Phase 7 / 8 bundles `syllables.fst`).
/// `with_state` callers verify presence defensively.
pub struct EngineState {
    // Version the platform reported; checked for alignment with association.bin / dictionary.bin.
    pub dictionary_version: u32,
    pub prefix_index: Option<PrefixIndex>,
    // TKDB reader: rowid → entry.
    pub dictionary: Option<DictionaryReader>,
    // TKWA bigram reader: previous word → follow-on candidates.
    pub association: Option<AssociationReader>,
    /// v3.5.8 Phase 6 — TL syllable inventory backed by `syllables.fst`.
    /// `None` when the platform did not pass a path; the composing
    /// continuous-input dispatcher returns an empty candidate list in
    /// that case.
    pub syllable_inventory: Option<SyllableInventory>,
}

// Zero-sized control handle; every API is an associated function.
pub struct EngineHandle;

static STATE: Lazy<Mutex<Option<EngineState>>> = Lazy::new(|| Mutex::new(None));

// Post-install stats, surfaced to the platform UI and health checks.
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
        let syllable_inventory = paths
            .syllables_fst
            .as_deref()
            .map(SyllableInventory::open)
            .transpose()?;

        let stats = InstallStats {
            dictionary_record_count: dictionary.record_count() as u64,
            prefix_index_entry_count: prefix_index.entry_count(),
        };

        let new_state = EngineState {
            dictionary_version: paths.dictionary_version,
            prefix_index: Some(prefix_index),
            dictionary: Some(dictionary),
            association: Some(association),
            syllable_inventory,
        };

        let mut guard = STATE
            .lock()
            .map_err(|_| LexiconError::Internal("install: state mutex poisoned".into()))?;
        *guard = Some(new_state);

        Ok(stats)
    }

    // Runs the closure over the active EngineState under the mutex; NotInitialized before install.
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
