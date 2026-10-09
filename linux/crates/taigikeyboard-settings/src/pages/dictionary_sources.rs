//! The Dictionary Sources pane: which dictionaries the engine draws from, in three
//! groups — MOE, the others, the supplements — with the MOE dictionary's twelve
//! subcollections stepped in under it, always visible and greyed while the MOE dictionary
//! is off (the Mac's and Windows' shape; roadmap PR7; port of
//! `DictionaryTogglesView.swift` and the Windows `dictionary_sources.rs`).
//! Every toggle is read live by the engine bridge on the next fetch.

use super::PageContext;
use adw::prelude::*;
use taigi_desktop_core::settings::dictionary_sources::{
    KAUTIAN_SUBCOLLECTIONS, MOE_OTHERS, OTHERS, SUPPLEMENTS,
};
use taigi_desktop_core::settings::{keys, SettingsDocument};
use taigi_desktop_core::strings::StringKey;

/// How far an accent row steps in under the MOE dictionary (`DictionaryTogglesView.swift`'s
/// indent).
const SUBCOLLECTION_INDENT: i32 = 20;

pub fn build<'a>(mut context: PageContext<'a>, page: &adw::PreferencesPage) -> PageContext<'a> {
    let moe = adw::PreferencesGroup::builder()
        .title(
            context
                .strings
                .resolve(StringKey::DictionaryMoeSectionTitle),
        )
        .build();
    // The MOE dictionary first, its twelve subcollections stepped in under it: disabled, not
    // cleared, while the MOE dictionary is off — the choices come back with it
    // (`DictionaryTogglesView.swift` indents the same twelve under the same
    // master toggle; the Windows card greys them the same way).
    context.switch_row(&moe, StringKey::CommonMoeDict, keys::IS_KAUTIAN_ENABLED);
    let is_kautian_enabled = context.document.bool(&keys::IS_KAUTIAN_ENABLED);
    let mut subcollections = Vec::new();
    for (key, label) in KAUTIAN_SUBCOLLECTIONS {
        let row = context.switch_row_in(|row| moe.add(row), label, key);
        row.add_prefix(
            &gtk::Box::builder()
                .width_request(SUBCOLLECTION_INDENT)
                .build(),
        );
        row.set_sensitive(is_kautian_enabled);
        subcollections.push(row);
    }
    context.on_refresh(move |document: &SettingsDocument| {
        let is_on = document.bool(&keys::IS_KAUTIAN_ENABLED);
        for row in &subcollections {
            row.set_sensitive(is_on);
        }
    });
    for (key, label) in MOE_OTHERS {
        context.switch_row(&moe, label, key);
    }
    page.add(&moe);

    let others = adw::PreferencesGroup::builder()
        .title(
            context
                .strings
                .resolve(StringKey::DictionaryOtherSectionTitle),
        )
        .build();
    for (key, label) in OTHERS {
        context.switch_row(&others, label, key);
    }
    page.add(&others);

    let supplements = adw::PreferencesGroup::builder()
        .title(
            context
                .strings
                .resolve(StringKey::DictionarySupplementSectionTitle),
        )
        .build();
    for (key, label) in SUPPLEMENTS {
        context.switch_row(&supplements, label, key);
    }
    page.add(&supplements);

    context.reset_row(page, SettingsDocument::reset_dictionary_sources);
    context
}
