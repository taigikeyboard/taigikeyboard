//! The settings model: every persisted key, its default, the typed choices,
//! and the engine-facing snapshot derived from them.
//!
//! Persistence itself (the `settings.json` file, its atomic replace and the
//! mtime/revision reload) lives in `taigi-windows-storage`; this module owns
//! the SHAPE of the document so the TIP and the settings window agree on it.
//! Key spellings are the iOS ones, as on macOS
//! (`macos/Sources/TaigiInputMethodCore/Settings/SettingsStore.swift:36-50`),
//! so `settings.json` speaks the same vocabulary as `defaults read` does on a
//! Mac and a future settings transfer has one name per setting.

mod choices;
mod document;
mod engine_settings;
mod font_selection;
pub mod keys;
pub mod launch;

pub use choices::{
    AppearanceMode, CandidateFontChoice, CandidateFontSelection, CandidateLayout,
    CandidateTextSizeChoice, CandidateWindowSizeChoice, CustomFontId, InstalledFontId, SettingChoice,
    SettingsPane,
};
pub use document::{SettingsDocument, SettingsKey};
pub use engine_settings::{
    CandidateDisplayMode, DictionarySourceToggles, EngineSettings, InputMode, KautianSubcollections,
};
pub use font_selection::{set_stored_font_selection, stored_font_selection, StoredFontSelection};

/// Live-read access to the current settings document.
///
/// Every consumer reads through this at the moment it needs a value and
/// caches nothing (behavioural invariant §11 — a snapshot taken at
/// construction is exactly what breaks mid-session TL↔POJ switching). One
/// user intent takes ONE snapshot and passes it down, so a compound operation
/// (an `Append` followed by an `EnterContinuous`) cannot straddle a change.
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
