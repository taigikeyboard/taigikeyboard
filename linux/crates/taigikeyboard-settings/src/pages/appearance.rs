//! The Appearance pane, the Linux row set (roadmap L4): the candidate list's
//! layout and what each cell shows. The panel draws the list, so the
//! window's light / dark mode, the size and typeface rows of the other
//! desktops are not drawn, and the layout picker offers only Horizontal / Vertical.
//! Port of `AppearanceSettingsView.swift`.

use super::PageContext;
use adw::prelude::*;
use taigi_desktop_core::settings::{
    keys, CandidateDisplayMode, CandidateLayout, SettingChoice, SettingsDocument,
};
use taigi_desktop_core::strings::StringKey;

/// The layouts a panel can take: the expandable grid has none, and the
/// panel draws it vertical (`taigi-linux-core` `session.rs`).
const PANEL_LAYOUTS: &[CandidateLayout] = &[CandidateLayout::Horizontal, CandidateLayout::Vertical];

/// The stored layout as the panel draws it: expandable (the shared
/// default) reads as vertical. The key itself is never rewritten.
fn panel_layout(document: &SettingsDocument) -> CandidateLayout {
    match document.choice(&keys::CANDIDATE_LAYOUT) {
        CandidateLayout::Horizontal => CandidateLayout::Horizontal,
        CandidateLayout::Vertical | CandidateLayout::Expandable => CandidateLayout::Vertical,
    }
}

pub fn build<'a>(mut context: PageContext<'a>, page: &adw::PreferencesPage) -> PageContext<'a> {
    let group = adw::PreferencesGroup::new();
    let strings = context.strings;
    let labels = PANEL_LAYOUTS
        .iter()
        .map(|layout| strings.resolve(layout.label_key()).to_owned())
        .collect();
    let current = panel_layout(context.document);
    context.picker_row(
        &group,
        strings.resolve(StringKey::DesktopCandidateWindowLayout),
        labels,
        PANEL_LAYOUTS,
        current,
        |layout, document| document.set_choice(&keys::CANDIDATE_LAYOUT, layout),
        panel_layout,
    );
    context.choice_row(
        &group,
        StringKey::SettingsCandidateDisplayMode,
        CandidateDisplayMode::ALL,
        keys::CANDIDATE_DISPLAY_MODE,
        CandidateDisplayMode::label_key,
    );
    page.add(&group);
    context.reset_row(page, SettingsDocument::reset_appearance);
    context
}
