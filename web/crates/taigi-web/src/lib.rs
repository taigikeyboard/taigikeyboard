//! The engine and the dictionaries in a browser tab
//! (`docs/architecture/web-roadmap.md`). The key path arrives with the shared
//! session module; until then `top_candidate` is what `make -C web check`
//! drives to prove the engine runs here.

use std::path::Path;
use std::sync::Arc;

use taigi_desktop_core::composing::{
    CandidateFetchOutcome, Clock, ComposingEffectExecutor, ContextToken,
};
use taigi_desktop_core::dictionary_artifacts::DictionaryArtifacts;
use taigi_desktop_core::engine::{lexicon_install, Effect};
use taigi_desktop_core::platform::DesktopPlatform;
use taigi_desktop_core::runtime::{DesktopRuntime, RuntimeParts};
use taigi_desktop_core::settings::{SettingsDocument, StaticSettingsProvider};
use wasm_bindgen::prelude::*;

/// Where the dictionary bytes are registered. No file exists there; the
/// engine only needs a rooted name to ask `mmap_host` for.
const DICTIONARIES_ROOT: &str = "/dictionaries";
/// A stamp for the log only, until a page ships dictionaries of a version.
const DICTIONARY_VERSION: u32 = 1;
const CONTEXT: ContextToken = ContextToken(1);

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Date)]
    fn now() -> f64;
}

/// `SystemTime::now` panics on this target.
struct BrowserClock;

impl Clock for BrowserClock {
    fn now_ms(&self) -> i64 {
        now() as i64
    }
}

/// `top_candidate` reads a list; it writes to no document.
struct NoDocument;

impl ComposingEffectExecutor for NoDocument {
    fn execute(&mut self, _effect: &Effect) {}
}

#[wasm_bindgen]
pub struct Session {
    runtime: DesktopRuntime,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl Session {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let settings = StaticSettingsProvider::new(SettingsDocument::default());
        Self {
            runtime: DesktopRuntime::new(RuntimeParts {
                settings: Arc::new(settings),
                settings_store: None,
                data_directory: None,
                // `DictionaryArtifacts::locate` needs a file system; `install`
                // names the registered paths itself.
                dictionaries: Box::new(|| None),
                dictionary_version: DICTIONARY_VERSION,
                system_locale: Box::new(|| "en".to_owned()),
                platform: DesktopPlatform::Linux,
                clock: Arc::new(BrowserClock),
            }),
        }
    }

    /// Hands in one of `DictionaryArtifacts::FILE_NAMES`.
    #[wasm_bindgen(js_name = addDictionary)]
    pub fn add_dictionary(&self, name: &str, bytes: Vec<u8>) {
        mmap_host::register(&Path::new(DICTIONARIES_ROOT).join(name), bytes);
    }

    /// Installs the dictionaries handed in and answers the record count.
    pub fn install(&self) -> Result<u64, JsError> {
        let artifacts = DictionaryArtifacts::under(Path::new(DICTIONARIES_ROOT));
        lexicon_install(&artifacts, DICTIONARY_VERSION)
            .map(|stats| stats.dictionary_record_count)
            .ok_or_else(|| JsError::new("the engine refused the dictionaries"))
    }

    /// The first candidate for `raw`, typed one key at a time, as a shell
    /// would: its Hanji, or its romanization when it has none.
    #[wasm_bindgen(js_name = topCandidate)]
    pub fn top_candidate(&self, raw: &str) -> Option<String> {
        self.runtime.prepare_for_first_key();
        let mut coordinator = self.runtime.lock_coordinator();
        let manager = coordinator.claim(CONTEXT);
        let mut document = NoDocument;
        let mut top = None;
        for key in raw.chars() {
            manager.append(key.encode_utf8(&mut [0; 4]), &mut document);
            top = match manager.fetch_candidates() {
                CandidateFetchOutcome::Found(candidates) => candidates.into_iter().next(),
                _ => None,
            };
        }
        manager.cancel_composition(&mut document);
        top.map(|candidate| candidate.hanji.unwrap_or(candidate.roman))
    }
}
