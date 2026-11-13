//! How both shells open an [`EngineState`] — with the popup measurer installed
//! so expandable layouts work (`list_layout.rs`).

use std::sync::Arc;

use taigi_desktop_core::session::{EngineState, ListLayout};

use crate::list_layout::shared_list_layout;

/// A fresh engine state wired for the Linux popup window.
pub fn new_engine_state() -> EngineState {
    let mut state = EngineState::default();
    let layout: Arc<dyn ListLayout> = shared_list_layout();
    state.list_layout = Some(layout);
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_desktop_core::settings::{keys, CandidateLayout, SettingsDocument};

    #[test]
    fn every_engine_gets_the_popup_measuring_layout() {
        let state = new_engine_state();
        assert!(state.list_layout.is_some());
        let mut settings = SettingsDocument::default();
        settings.set_choice(&keys::CANDIDATE_LAYOUT, CandidateLayout::Expandable);
        let cells = vec![taigi_desktop_core::composing::CandidateCellContent::new("a", None,); 12];
        let model = state
            .list_layout
            .as_ref()
            .unwrap()
            .layout(&cells, &settings, 9);
        assert!(model.is_some());
    }
}
