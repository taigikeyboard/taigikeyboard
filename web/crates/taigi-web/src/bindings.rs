//! The API the page calls (`docs/architecture/web-roadmap.md` § Phase 3).
//! Replies are JSON strings (`reply.rs`), parsed once on the page's side.

use std::path::Path;
use std::sync::Arc;

use sqlite_wasm_rs::{MemVfsUtil, WasmOsCallback};

use taigi_desktop_core::composing::Clock;
use taigi_desktop_core::dictionary_artifacts::DictionaryArtifacts;
use taigi_desktop_core::engine::{lexicon_install, user_data};
use taigi_desktop_core::platform::DesktopPlatform;
use wasm_bindgen::prelude::*;

use crate::key::BrowserKey;
use crate::layout::{Measured, TextRuler};
use crate::reply::tps_rows_json;
use crate::session::{WebSession, DICTIONARY_VERSION};

/// Where the dictionary bytes are registered. No file exists there; the
/// engine only needs a rooted name to ask `mmap_host` for.
const DICTIONARIES_ROOT: &str = "/dictionaries";

/// Where the user's stores live: SQLite's memory VFS, which keys its files by
/// path. The page keeps their bytes (`userDataFiles`).
const USER_DATA_ROOT: &str = "/taigi";

/// The memory VFS path of one of `userDataFiles`, or a refusal for any other
/// name.
fn user_data_path(name: &str) -> Result<String, JsError> {
    if userdata::USER_DATA_FILES.contains(&name) {
        Ok(format!("{USER_DATA_ROOT}/{name}"))
    } else {
        Err(JsError::new(&format!("{name} is not a user-data file")))
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Date)]
    fn now() -> f64;

    /// The page's ruler, set before a session starts. Answers
    /// `[maximumWindowWidth, ...widths]`. `catch`: an exception unwinding
    /// through here would skip the drop of `&mut self`, and every later call
    /// would be refused.
    #[wasm_bindgen(catch, js_name = taigiMeasureCells)]
    fn measure_cells(runs_json: &str) -> Result<Vec<f32>, JsValue>;
}

/// `SystemTime::now` panics on this target.
struct BrowserClock;

impl Clock for BrowserClock {
    fn now_ms(&self) -> i64 {
        now() as i64
    }
}

/// Measures in the page, which is what draws.
#[derive(Debug)]
struct PageRuler;

impl TextRuler for PageRuler {
    fn measure(&self, runs: &[(&str, f32)]) -> Option<Measured> {
        // Strings and numbers only: this cannot fail.
        let json = serde_json::to_string(runs).ok()?;
        let answer = measure_cells(&json).ok()?;
        let (maximum_window_width, widths) = answer.split_first()?;
        Some(Measured {
            maximum_window_width: *maximum_window_width,
            widths: widths.to_vec(),
        })
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
        let ruler = Arc::new(PageRuler);
        WebSession::new(
            settings_json,
            platform,
            locale,
            Arc::new(BrowserClock),
            ruler,
        )
        .map(|inner| Self { inner })
        .map_err(|error| JsError::new(&error.to_string()))
    }

    /// The files `addDictionary` takes.
    #[wasm_bindgen(js_name = dictionaryFiles)]
    pub fn dictionary_files(&self) -> Vec<String> {
        DictionaryArtifacts::FILE_NAMES.map(str::to_owned).to_vec()
    }

    /// Hands in one of `dictionaryFiles`.
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

    /// The files the page keeps the user's data in, by name.
    #[wasm_bindgen(js_name = userDataFiles)]
    pub fn user_data_files(&self) -> Vec<String> {
        userdata::USER_DATA_FILES.map(str::to_owned).to_vec()
    }

    /// Hands in one of `userDataFiles` as the page kept it. Before
    /// `openUserData` only: an open file is refused.
    #[wasm_bindgen(js_name = importUserData)]
    pub fn import_user_data(&self, name: &str, bytes: &[u8]) -> Result<(), JsError> {
        // The VFS is registered once per instance; `new` finds it.
        MemVfsUtil::<WasmOsCallback>::new()
            .import_db(&user_data_path(name)?, bytes)
            .map_err(|error| JsError::new(&format!("{name}: {error}")))
    }

    /// Opens the stores over what was imported, a new file for each one not,
    /// and finishes before it returns. From here on the engine learns. Once
    /// per instance; `false` when the engine did not acknowledge it.
    #[wasm_bindgen(js_name = openUserData)]
    pub fn open_user_data(&self) -> bool {
        user_data::open_on_this_thread(Path::new(USER_DATA_ROOT))
    }

    /// One of `userDataFiles` as it is now, for the page to keep. Every write
    /// has landed: the stores' queues drain on this thread (`database.rs`).
    #[wasm_bindgen(js_name = exportUserData)]
    pub fn export_user_data(&self, name: &str) -> Result<Vec<u8>, JsError> {
        MemVfsUtil::<WasmOsCallback>::new()
            .export_db(&user_data_path(name)?)
            .map_err(|error| JsError::new(&format!("{name}: {error}")))
    }

    /// The open composition's segmentation and ranking, JSON, or `null`.
    pub fn explain(&self) -> String {
        self.inner.explain()
    }

    #[wasm_bindgen(js_name = keyDown)]
    pub fn key_down(&mut self, key_json: &str) -> Result<String, JsError> {
        Ok(self.inner.key_down(&parse_key(key_json)?))
    }

    #[wasm_bindgen(js_name = keyUp)]
    pub fn key_up(&mut self, key_json: &str) -> Result<String, JsError> {
        Ok(self.inner.key_up(&parse_key(key_json)?))
    }

    /// A click on a cell: selects, never commits (`click_from_panel`).
    /// `position`: within the current page, or under a `layout` the cell's
    /// index in the list.
    #[wasm_bindgen(js_name = clickCandidate)]
    pub fn click_candidate(&mut self, position: usize) -> String {
        self.inner.click_candidate(position)
    }

    /// The TPS key panel's rows, JSON. They never change, so once is enough.
    #[wasm_bindgen(js_name = tpsKeyboardRows)]
    pub fn tps_keyboard_rows(&self) -> String {
        tps_rows_json()
    }

    /// The Telex guide's card, JSON: `title` and `rows` of `key` and
    /// `meaning`, in the display language.
    #[wasm_bindgen(js_name = telexGuide)]
    pub fn telex_guide(&self) -> String {
        self.inner.telex_guide()
    }

    /// The mode chip under the editor, JSON: `isEnglish`, `mode`, `script`
    /// and `other`, in the display language. Again on `modeChanged` and a settings change.
    #[wasm_bindgen(js_name = modeLine)]
    pub fn mode_line(&self) -> String {
        self.inner.mode_line()
    }

    /// A click on a TPS key cap: `glyph` is what the cap types.
    #[wasm_bindgen(js_name = tpsKeyPress)]
    pub fn tps_key_press(&mut self, glyph: &str) -> String {
        self.inner.tps_key_press(glyph)
    }

    /// A click on the panel's page arrows.
    #[wasm_bindgen(js_name = turnPage)]
    pub fn turn_page(&mut self, is_forward: bool) -> String {
        self.inner.turn_page(is_forward)
    }

    /// The wheel moved the unfolded grid: the keys read the same viewport.
    #[wasm_bindgen(js_name = viewportScrolled)]
    pub fn viewport_scrolled(&mut self, offset: f32) {
        self.inner.viewport_scrolled(offset);
    }

    #[wasm_bindgen(js_name = commitComposition)]
    pub fn commit_composition(&mut self) -> String {
        self.inner.commit_composition()
    }

    /// ChromeOS: the field confirmed the preedit itself (blur, reset).
    #[wasm_bindgen(js_name = endSession)]
    pub fn end_session(&mut self) -> String {
        self.inner.end_session()
    }

    /// ChromeOS: a text field gained focus.
    #[wasm_bindgen(js_name = beginSession)]
    pub fn begin_session(&mut self) -> String {
        self.inner.begin_session()
    }

    #[wasm_bindgen(js_name = caretMoved)]
    pub fn caret_moved(&mut self) {
        self.inner.caret_moved();
    }

    #[wasm_bindgen(js_name = focusLost)]
    pub fn focus_lost(&mut self) {
        self.inner.focus_lost();
    }

    /// A malformed document is refused and the previous one stays.
    /// The sidebar's Shortcuts section; JSON, `shortcuts.rs`.
    pub fn shortcuts(&self) -> String {
        self.inner.shortcuts()
    }

    #[wasm_bindgen(js_name = setSettings)]
    pub fn set_settings(&self, settings_json: &str) -> Result<(), JsError> {
        self.inner
            .set_settings(settings_json)
            .map_err(|error| JsError::new(&error.to_string()))
    }

    #[wasm_bindgen(js_name = setSortOrder)]
    pub fn set_sort_order(&mut self, json: &str) -> Result<String, JsError> {
        self.inner
            .set_sort_order(json)
            .map_err(|error| JsError::new(&error))
    }

    /// A one-word best path joins the twelve stages. Several words stay pinned.
    #[wasm_bindgen(js_name = setRankOneWord)]
    pub fn set_rank_one_word(&mut self, enabled: bool) -> String {
        self.inner.set_rank_one_word(enabled)
    }

    /// The custom dictionary as JSON `{ "entries": [...] }`.
    #[wasm_bindgen(js_name = listCustomEntries)]
    pub fn list_custom_entries(&self) -> Result<String, JsError> {
        self.inner
            .list_custom_entries()
            .map_err(|error| JsError::new(&error))
    }

    /// Adds a custom word. A refusal (no romanization, a full dictionary)
    /// is an error whose message is the engine's.
    #[wasm_bindgen(js_name = saveCustomEntry)]
    pub fn save_custom_entry(&mut self, roman: &str, hanji: &str) -> Result<String, JsError> {
        self.inner
            .save_custom_entry(roman, hanji)
            .map_err(|error| JsError::new(&error))
    }

    /// Removes the custom word with `id`.
    #[wasm_bindgen(js_name = deleteCustomEntry)]
    pub fn delete_custom_entry(&mut self, id: &str) -> Result<String, JsError> {
        self.inner
            .delete_custom_entry(id)
            .map_err(|error| JsError::new(&error))
    }

    /// Empties the three learning stores. Custom words stay.
    #[wasm_bindgen(js_name = clearLearningRecords)]
    pub fn clear_learning_records(&mut self) -> Result<String, JsError> {
        self.inner
            .clear_learning_records()
            .map_err(|error| JsError::new(&error))
    }
}
