//! The settings model: every persisted key, its default, the typed choices,
//! and the engine-facing snapshot derived from them.
//!
//! Persistence itself (the `settings.json` file, its atomic replace and the
//! mtime/revision reload) lives in `taigi-desktop-storage`; this module owns
//! the SHAPE of the document so the TIP and the settings window agree on it.
//! Key spellings are the iOS ones, so `settings.json` speaks the same
//! vocabulary as `defaults read` does on a Mac and a future settings transfer
//! has one name per setting. macOS keeps a Swift twin: `SettingsStore.Keys`
//! (`macos/Sources/TaigiInputMethodCore/Settings/SettingsStore.swift`).
//!
//! It also holds what both settings WINDOWS draw from, toolkit-neutral:
//! `presentation` (labels, links, page notices), `launch` (the command
//! line), `dictionary_sources` (that pane's roster), `dictionary_search`
//! (Manage Dictionaries' search: result limit and debounce), `custom_dictionary` and
//! `learning_records` (those panes' models over the engine's user-data ops)
//! and `listing` (the paged list both draw).

mod choices;
pub mod custom_dictionary;
pub mod dictionary_search;
pub mod dictionary_sources;
mod document;
mod engine_settings;
mod font_selection;
pub mod key_path;
pub mod keys;
pub mod launch;
pub mod learning_records;
pub mod listing;
pub mod presentation;
pub mod update_schedule;

pub use choices::{
    AppearanceMode, CandidateFontChoice, CandidateFontSelection, CandidateLayout,
    CandidateSizeChoice, CustomFontId, InstalledFontId, KeyboardLayout, SettingChoice,
    SettingsPane,
};
pub use document::{SettingsDocument, SettingsKey};
pub use engine_settings::{
    next_input_mode, CandidateDisplayMode, DictionarySourceToggles, EngineSettings, InputMode,
    InputModeRequest, KautianSubcollections, Romanization, SyllableSeparator,
};
pub use font_selection::{set_stored_font_selection, stored_font_selection, StoredFontSelection};

/// Where a write to the settings document goes: one locked load → mutate →
/// save, so two writers (an input method and its settings window) cannot
/// lose each other's change. Implemented over `settings.json` in
/// `taigi-desktop-storage`; this crate only names the contract.
pub trait SettingsStore: Send + Sync {
    /// Runs `mutate` on the document as it is now and saves it when it
    /// changed. `mutate` is not run when the store cannot be locked or read.
    fn update_document(
        &self,
        mutate: &mut dyn FnMut(&mut SettingsDocument),
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Live-read access to the current settings document.
///
/// Every consumer reads through this at the moment it needs a value and
/// caches nothing (behavioural invariant §11 — a snapshot taken at
/// construction is exactly what breaks mid-session TL↔POJ switching). One
/// user intent takes ONE snapshot and passes it down, so a compound operation
/// (a commit followed by its next-word handshake) cannot straddle a change.
pub trait SettingsProvider: Send + Sync {
    /// The document as of now. Cheap to call: implementations hand out a
    /// shared, already-parsed copy and only re-read the file when it changed.
    fn current(&self) -> std::sync::Arc<SettingsDocument>;
}

impl<T: SettingsProvider + ?Sized> SettingsProvider for std::sync::Arc<T> {
    fn current(&self) -> std::sync::Arc<SettingsDocument> {
        (**self).current()
    }
}

/// A provider that always answers with the same document — what tests and
/// the pure crates use when no file is involved.
#[derive(Clone, Debug, Default)]
pub struct StaticSettingsProvider {
    document: std::sync::Arc<SettingsDocument>,
}

impl StaticSettingsProvider {
    pub fn new(document: SettingsDocument) -> Self {
        Self {
            document: std::sync::Arc::new(document),
        }
    }
}

impl SettingsProvider for StaticSettingsProvider {
    fn current(&self) -> std::sync::Arc<SettingsDocument> {
        std::sync::Arc::clone(&self.document)
    }
}
