//! The framework-independent input session the Linux shells and the web
//! version share: a key goes in as a [`crate::keys::KeyEventSnapshot`], and
//! what comes back is a list of [`Emit`]s the shell renders with whatever
//! its framework offers. Nothing here depends on a bus, a panel or a
//! browser; the contract is written in IBus's terms because Linux came first
//! ([`Emit`] names the signal each one maps to).
//!
//! Design record: `docs/architecture/web-roadmap.md` § Phase 2.

mod executor;
mod global;
mod key;
mod list_selection;
mod selection;
mod state;
mod table;

pub use executor::{Emit, LookupTableContent};
pub(crate) use executor::{KeySurface, Recorder};
pub use global::{mode_label, perform_global};
pub use key::{
    begin_session, click_from_panel, commit_composition, end_session, is_composing,
    navigate_from_panel, process_key, refresh_composition, toggle_language_mode,
    tps_keyboard_press,
};
pub use list_selection::{ListLayout, ListSelection};
pub use selection::LookupSelection;
pub use state::{EngineState, KeyReply, SymbolPicker, TextBeforeCaret};
pub(crate) use table::{is_vertical_layout, present_table};

use crate::candidates::HorizontalPageLayout;

/// The most slot keys a page holds, the same on every desktop; the live set
/// may hold fewer (`CandidateSlotKeySet::page_size`).
pub const PAGE_SIZE: usize = HorizontalPageLayout::PAGE_SIZE;

#[cfg(test)]
pub(crate) mod test_support {
    use crate::candidates::{CollapsedLayout, ExpandableGeometryInput, ExpandableListModel};
    use crate::composing::SystemClock;
    use crate::platform::test_support::TEST_PLATFORM;
    use crate::runtime::{DesktopRuntime, RuntimeParts};
    use crate::settings::{
        SettingsDocument, SettingsProvider, SettingsStore, StaticSettingsProvider,
    };
    use std::sync::{Arc, Mutex};

    /// `count` cells 20 wide. trace: `row_budget(10)` = 90 packs four (80),
    /// the fifth would pass it: a collapsed row of four.
    pub(crate) fn row_model(count: usize) -> ExpandableListModel {
        ExpandableListModel::new(
            &vec![20.0; count],
            ExpandableGeometryInput {
                collapsed: CollapsedLayout::Row,
                base_width: 10.0,
                maximum_window_width: 640.0,
                chrome_width: 20.0,
                scroller_width: 0.0,
                item_height: 30.0,
                page_size: super::PAGE_SIZE,
                row_columns: 0,
            },
        )
    }

    /// A runtime over `document` with no store, no lexicon and no learning.
    pub(crate) fn runtime(document: SettingsDocument) -> DesktopRuntime {
        runtime_over(Arc::new(StaticSettingsProvider::new(document)), None)
    }

    /// One document the runtime both reads and writes, so a write is seen by
    /// the next read.
    struct SharedDocument(Mutex<Arc<SettingsDocument>>);

    impl SettingsProvider for SharedDocument {
        fn current(&self) -> Arc<SettingsDocument> {
            Arc::clone(&self.0.lock().unwrap())
        }
    }

    impl SettingsStore for Arc<SharedDocument> {
        fn update_document(
            &self,
            mutate: &mut dyn FnMut(&mut SettingsDocument),
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let mut document = self.0.lock().unwrap();
            let mut next = SettingsDocument::clone(&document);
            mutate(&mut next);
            *document = Arc::new(next);
            Ok(())
        }
    }

    /// [`runtime`] with a store whose writes the next read sees.
    pub(crate) fn runtime_with_store(document: SettingsDocument) -> DesktopRuntime {
        let shared = Arc::new(SharedDocument(Mutex::new(Arc::new(document))));
        runtime_over(Arc::clone(&shared) as _, Some(Box::new(shared)))
    }

    fn runtime_over(
        settings: Arc<dyn SettingsProvider + Send + Sync>,
        settings_store: Option<Box<dyn SettingsStore>>,
    ) -> DesktopRuntime {
        DesktopRuntime::new(RuntimeParts {
            settings,
            settings_store,
            data_directory: None,
            dictionaries: Box::new(|| None),
            dictionary_version: 1,
            system_locale: Box::new(|| "en-US".to_owned()),
            platform: TEST_PLATFORM,
            clock: Arc::new(SystemClock),
        })
    }
}
