//! The settings as the page holds them: one JSON document it keeps in the
//! browser and hands in, with desktop's key spellings. Provider and store over
//! the same document (`RuntimeParts` takes one of each); a chord that writes
//! leaves the new document here for the reply, so the page persists it.

use std::sync::{Arc, Mutex, MutexGuard};

use taigi_desktop_core::settings::{SettingsDocument, SettingsProvider, SettingsStore};

#[derive(Clone, Default)]
pub(crate) struct PageSettings {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    document: Arc<SettingsDocument>,
    /// A write the page has not been told about.
    is_unsent: bool,
}

impl PageSettings {
    pub(crate) fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let document = Arc::new(SettingsDocument::from_json(json)?);
        let state = State {
            document,
            is_unsent: false,
        };
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
        })
    }

    /// The page's own change. A malformed document changes nothing.
    pub(crate) fn replace_from_json(&self, json: &str) -> Result<(), serde_json::Error> {
        let document = SettingsDocument::from_json(json)?;
        self.lock().document = Arc::new(document);
        Ok(())
    }

    /// The document as JSON, once per write from the key path.
    pub(crate) fn take_unsent(&self) -> Option<String> {
        let mut state = self.lock();
        std::mem::take(&mut state.is_unsent).then(|| state.document.to_json())
    }

    /// A poisoned lock is recovered: the value is a whole `Arc`.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl SettingsProvider for PageSettings {
    fn current(&self) -> Arc<SettingsDocument> {
        Arc::clone(&self.lock().document)
    }
}

impl SettingsStore for PageSettings {
    fn update_document(
        &self,
        mutate: &mut dyn FnMut(&mut SettingsDocument),
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut state = self.lock();
        let mut document = (*state.document).clone();
        let revision = document.revision;
        mutate(&mut document);
        if document.revision != revision {
            state.document = Arc::new(document);
            state.is_unsent = true;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_desktop_core::settings::keys;

    #[test]
    fn a_write_is_handed_over_once_and_a_no_op_not_at_all() {
        let settings = PageSettings::default();
        settings.update_document(&mut |_| {}).unwrap();
        assert_eq!(settings.take_unsent(), None);
        settings
            .update_document(&mut |document| document.set_bool(&keys::IS_HANJI_FIRST, false))
            .unwrap();
        let json = settings.take_unsent().expect("the written document");
        assert_eq!(
            SettingsDocument::from_json(&json).unwrap(),
            *settings.current()
        );
        assert_eq!(settings.take_unsent(), None);
    }

    #[test]
    fn a_malformed_document_changes_nothing() {
        let settings = PageSettings::from_json(r#"{"values":{"inputMode":"poj"}}"#).unwrap();
        let before = settings.current();
        assert!(settings.replace_from_json("{").is_err());
        assert_eq!(settings.current(), before);
    }
}
