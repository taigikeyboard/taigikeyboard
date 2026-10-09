//! Which dictionaries the engine draws from, in three sections —
//! MOE, the others, the supplements — with the MOE dictionary's twelve subcollections
//! always visible under it, greyed while the MOE dictionary is off. Port of
//! `DictionaryTogglesView.swift`. Every toggle is read live by the engine
//! bridge on the next fetch.

use super::reset_row;
use crate::winui::cards;
use crate::winui::window::{Message, ResetScope, SettingsWindow};
use taigi_desktop_core::settings::dictionary_sources::{
    KAUTIAN_SUBCOLLECTIONS, MOE_OTHERS, OTHERS, SUPPLEMENTS,
};
use taigi_desktop_core::settings::{keys, SettingsKey};
use taigi_desktop_core::strings::{StringKey, StringResolver};
use windows_reactor::*;

pub fn view(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let is_kautian_enabled = window.document().bool(&keys::IS_KAUTIAN_ENABLED);
    View::fragment((
        cards::section_title(strings.resolve(StringKey::DictionaryMoeSectionTitle)),
        // The MOE dictionary and its twelve subcollections are ONE always-open card: the
        // parent source is the group's first line, the subcollections step
        // in under it and stay browsable — they are read far more often
        // than they are changed, and a collapsed group hides which accents the
        // engine is currently drawing from. Why this is a card and not an
        // `Expander` that starts open is written out on `cards::group`.
        // Disabled, not cleared, while the MOE dictionary is off: the choices come back
        // with it. `DictionaryTogglesView.swift` indents the same twelve
        // under the same master toggle.
        cards::group(
            cards::line(
                strings.resolve(StringKey::CommonMoeDict),
                cards::switch(
                    is_kautian_enabled,
                    true,
                    context.callback(|is_on| Message::SetSwitch(keys::IS_KAUTIAN_ENABLED, is_on)),
                ),
            ),
            KAUTIAN_SUBCOLLECTIONS.map(|(key, label)| {
                (
                    key.name,
                    cards::sub_row(
                        strings.resolve(label),
                        is_kautian_enabled,
                        cards::switch(
                            window.document().bool(&key),
                            is_kautian_enabled,
                            context.callback(move |is_on| Message::SetSwitch(key, is_on)),
                        ),
                    ),
                )
            }),
        ),
        source_rows(window, strings, context, &MOE_OTHERS),
        cards::section_title(strings.resolve(StringKey::DictionaryOtherSectionTitle)),
        source_rows(window, strings, context, &OTHERS),
        cards::section_title(strings.resolve(StringKey::DictionarySupplementSectionTitle)),
        source_rows(window, strings, context, &SUPPLEMENTS),
        reset_row(strings, ResetScope::DictionarySources, context),
    ))
}

/// One card per source, each bound to its own settings key.
fn source_rows(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
    sources: &'static [(SettingsKey<bool>, StringKey)],
) -> View {
    View::keyed_fragment(sources.iter().map(|(key, label)| {
        let key = *key;
        (
            key.name,
            cards::switch_row(
                strings.resolve(*label),
                window.document().bool(&key),
                true,
                context.callback(move |is_on| Message::SetSwitch(key, is_on)),
            ),
        )
    }))
}
