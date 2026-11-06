//! The API the page calls (`docs/architecture/web-roadmap.md` § Phase 3).
//! Replies are JSON strings (`reply.rs`), parsed once on the page's side.

use std::path::Path;
use std::sync::Arc;

use taigi_desktop_core::composing::Clock;
use taigi_desktop_core::dictionary_artifacts::DictionaryArtifacts;
use taigi_desktop_core::engine::lexicon_install;
use taigi_desktop_core::platform::DesktopPlatform;
use wasm_bindgen::prelude::*;

use crate::key::BrowserKey;
use crate::session::{WebSession, DICTIONARY_VERSION};

/// Where the dictionary bytes are registered. No file exists there; the
/// engine only needs a rooted name to ask `mmap_host` for.
const DICTIONARIES_ROOT: &str = "/dictionaries";

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

fn parse_key(json: &str) -> Result<BrowserKey, JsError> {
    serde_json::from_str(json).map_err(|error| JsError::new(&error.to_string()))
}

#[wasm_bindgen]
pub struct Session {
    inner: WebSession,
}

#[wasm_bindgen]
impl Session {
    /// `settings_json`: the document the page keeps, `{}` for the defaults.
    /// `platform`: `"macos"`, `"windows"` or `"linux"`. `locale`:
    /// `navigator.language`. The page passes the platform, which sets the
    /// caret chord (`keys/intent.rs`) and how Option reads (`key.rs`).
    #[wasm_bindgen(constructor)]
    pub fn new(settings_json: &str, platform: &str, locale: String) -> Result<Session, JsError> {
        let platform = match platform {
            "macos" => DesktopPlatform::MacOS,
            "windows" => DesktopPlatform::Windows,
            "linux" => DesktopPlatform::Linux,
            other => return Err(JsError::new(&format!("unknown platform {other}"))),
        };
        WebSession::new(settings_json, platform, locale, Arc::new(BrowserClock))
            .map(|inner| Self { inner })
            .map_err(|error| JsError::new(&error.to_string()))
    }

    /// Hands in one of `DictionaryArtifacts::FILE_NAMES`.
    #[wasm_bindgen(js_name = addDictionary)]
    pub fn add_dictionary(&self, name: &str, bytes: Vec<u8>) {
        mmap_host::register(&Path::new(DICTIONARIES_ROOT).join(name), bytes);
    }

    /// Installs the dictionaries handed in and answers the record count.
    /// Keys before this type romanization (`runtime.rs` `install_lexicon`).
    pub fn install(&self) -> Result<u64, JsError> {
        let artifacts = DictionaryArtifacts::under(Path::new(DICTIONARIES_ROOT));
        lexicon_install(&artifacts, DICTIONARY_VERSION)
            .map(|stats| stats.dictionary_record_count)
            .ok_or_else(|| JsError::new("the engine refused the dictionaries"))
    }

    #[wasm_bindgen(js_name = keyDown)]
    pub fn key_down(&mut self, key_json: &str) -> Result<String, JsError> {
        Ok(self.inner.key_down(&parse_key(key_json)?))
    }

    #[wasm_bindgen(js_name = keyUp)]
    pub fn key_up(&mut self, key_json: &str) -> Result<String, JsError> {
        Ok(self.inner.key_up(&parse_key(key_json)?))
    }

    /// A click on the `position`-th cell of the current page: selects, never
    /// commits (`click_from_panel`).
    #[wasm_bindgen(js_name = clickCandidate)]
    pub fn click_candidate(&mut self, position: usize) -> String {
        self.inner.click_candidate(position)
    }

    #[wasm_bindgen(js_name = commitComposition)]
    pub fn commit_composition(&mut self) -> String {
        self.inner.commit_composition()
    }

    #[wasm_bindgen(js_name = caretMoved)]
    pub fn caret_moved(&mut self) {
        self.inner.caret_moved();
    }

    #[wasm_bindgen(js_name = focusLost)]
    pub fn focus_lost(&mut self) {
        self.inner.focus_lost();
    }

    /// The status control's text, at load and after `setSettings`; a key's
    /// reply carries the change itself (`modeChanged`).
    #[wasm_bindgen(js_name = modeSymbol)]
    pub fn mode_symbol(&self) -> String {
        self.inner.mode_symbol().to_owned()
    }

    /// A malformed document is refused and the previous one stays.
    #[wasm_bindgen(js_name = setSettings)]
    pub fn set_settings(&self, settings_json: &str) -> Result<(), JsError> {
        self.inner
            .set_settings(settings_json)
            .map_err(|error| JsError::new(&error.to_string()))
    }
}
