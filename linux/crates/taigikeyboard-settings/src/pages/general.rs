//! The General pane: input script, tone keys, the candidate window's two
//! switches, output script and its shape, auto-space, display language;
//! then the version row and the reset row. Port of `GeneralSettingsView.swift`
//! in the Windows pane's row order (USER 2026-09-21: the typing pipeline's).
//! No update check (roadmap L10): the version row links to the download page.

use super::PageContext;
use adw::prelude::*;
use taigi_desktop_core::keys::ToneInputScheme;
use taigi_desktop_core::settings::presentation::{
    display_language_label, nasal_marker_style_label, output_script_label, NASAL_MARKER_STYLES,
    OUTPUT_SCRIPTS, WEBSITE_URL,
};
use taigi_desktop_core::settings::{
    keys, InputMode, InputModeRequest, SettingChoice, SettingsDocument, SyllableSeparator,
};
use taigi_desktop_core::strings::{DisplayLanguage, StringKey};

pub fn build<'a>(mut context: PageContext<'a>, page: &adw::PreferencesPage) -> PageContext<'a> {
    // One run of rows, no sub-groups (USER 2026-09-18: "no grouping").
    let group = adw::PreferencesGroup::new();
    // Through the one mode writer, which remembers the romanization a pick
    // of TPS leaves (`SettingsDocument::switch_input_mode`).
    let input_mode_labels = InputMode::ALL
        .iter()
        .map(|mode| context.strings.resolve(mode.label_key()).to_owned())
        .collect();
    context.picker_row(
        &group,
        context.strings.resolve(StringKey::SettingsInputScript),
        input_mode_labels,
        InputMode::ALL,
        context.document.choice(&keys::INPUT_MODE),
        |mode, document| {
            document.switch_input_mode(InputModeRequest::Pick(mode));
        },
        |document| document.choice(&keys::INPUT_MODE),
    );
    context.choice_row(
        &group,
        StringKey::SettingsToneInputScheme,
        ToneInputScheme::ALL,
        keys::TONE_INPUT_SCHEME,
        ToneInputScheme::label_key,
    );
    // Under Tone Keys, independent of it: both say what a letter types.
    context.switch_row(
        &group,
        StringKey::SettingsCAffricate,
        keys::IS_C_AFFRICATE_ENABLED,
    );
    context.switch_row(
        &group,
        StringKey::SettingsCandidateWindow,
        keys::IS_CANDIDATE_WINDOW_ENABLED,
    );
    context.switch_row(
        &group,
        StringKey::SettingsLiteralRomanCandidate,
        keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED,
    );
    // Which script a commit writes: the same stored swap the backtick
    // shortcut toggles. Disabled exactly where the shortcut is inert
    // (`allows_swap_toggle`).
    let output_labels = OUTPUT_SCRIPTS
        .iter()
        .map(|is_hanji| {
            context
                .strings
                .resolve(output_script_label(*is_hanji))
                .to_owned()
        })
        .collect();
    let output_row = context.picker_row(
        &group,
        context.strings.resolve(StringKey::SettingsOutputScript),
        output_labels,
        OUTPUT_SCRIPTS,
        context.document.bool(&keys::IS_HANJI_FIRST),
        |is_hanji, document| document.set_bool(&keys::IS_HANJI_FIRST, is_hanji),
        |document| document.bool(&keys::IS_HANJI_FIRST),
    );
    output_row.set_sensitive(allows_swap(context.document));
    context.on_refresh(move |document| output_row.set_sensitive(allows_swap(document)));
    context.choice_row(
        &group,
        StringKey::SettingsSyllableSeparator,
        SyllableSeparator::ALL,
        keys::SYLLABLE_SEPARATOR,
        SyllableSeparator::label_key,
    );
    // Nasal mark in POJ capitals (§53): a pop-up over the stored switch, ᴺ or ⁿ.
    let nasal_marker_labels = NASAL_MARKER_STYLES
        .iter()
        .map(|is_uppercase| {
            context
                .strings
                .resolve(nasal_marker_style_label(*is_uppercase))
                .to_owned()
        })
        .collect();
    context.picker_row(
        &group,
        context
            .strings
            .resolve(StringKey::SettingsNasalMarkerUppercase),
        nasal_marker_labels,
        NASAL_MARKER_STYLES,
        context
            .document
            .bool(&keys::IS_NASAL_MARKER_UPPERCASE_ENABLED),
        |is_uppercase, document| {
            document.set_bool(&keys::IS_NASAL_MARKER_UPPERCASE_ENABLED, is_uppercase)
        },
        |document| document.bool(&keys::IS_NASAL_MARKER_UPPERCASE_ENABLED),
    );
    context.switch_row(
        &group,
        StringKey::SettingsAutoSpace,
        keys::IS_AUTO_SPACE_ENABLED,
    );
    let language_labels = DisplayLanguage::PICKER
        .iter()
        .map(|language| display_language_label(*language, context.strings))
        .collect();
    context.picker_row(
        &group,
        context.strings.resolve(StringKey::SettingsDisplayLanguage),
        language_labels,
        &DisplayLanguage::PICKER,
        context.document.display_language(),
        |language, document| document.set_string(&keys::DISPLAY_LANGUAGE, language.tag()),
        SettingsDocument::display_language,
    );
    page.add(&group);

    // The running version and the download page (roadmap L10): the
    // package manager updates the input method, so this row only says
    // which build this is and where the others are.
    let version_group = adw::PreferencesGroup::new();
    let version_row = adw::ActionRow::builder()
        .title(context.strings.format(
            StringKey::DesktopUpdateCurrentVersionLabel,
            &[&env!("CARGO_PKG_VERSION")],
        ))
        .build();
    let download = gtk::Button::builder()
        .label(
            context
                .strings
                .resolve(StringKey::DesktopUpdateDownloadAction),
        )
        .valign(gtk::Align::Center)
        .build();
    let shell = context.shell.clone();
    download.connect_clicked(move |_| shell.open_url(WEBSITE_URL));
    version_row.add_suffix(&download);
    version_row.set_activatable_widget(Some(&download));
    version_group.add(&version_row);
    page.add(&version_group);

    // Keeps the display language (#118: a reset must not switch the UI
    // language) — `reset_general`'s own rule.
    context.reset_row(page, SettingsDocument::reset_general);
    context
}

fn allows_swap(document: &SettingsDocument) -> bool {
    document
        .choice(&keys::CANDIDATE_DISPLAY_MODE)
        .allows_swap_toggle()
}
