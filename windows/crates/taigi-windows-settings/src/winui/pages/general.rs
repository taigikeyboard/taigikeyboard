//! The General pane: input script, keyboard layout, output script, tone
//! keys, auto-space, the candidate window's two switches, display language;
//! then the update row and the reset card. Port of `GeneralSettingsView.swift`.

use super::{choice_row, reset_row};
use crate::updates::INSTALLED_VERSION;
use crate::winui::cards;
use crate::winui::window::{Message, ResetScope, SettingsWindow, SettingsWrite};
use taigi_desktop_core::keys::ToneInputScheme;
use taigi_desktop_core::settings::presentation::{
    display_language_label, nasal_marker_style_label, output_script_label, NASAL_MARKER_STYLES,
    OUTPUT_SCRIPTS,
};
use taigi_desktop_core::settings::{
    keys, InputMode, KeyboardLayout, SettingChoice, SyllableSeparator,
};
use taigi_desktop_core::strings::{DisplayLanguage, StringKey, StringResolver};
use taigi_desktop_update::checker;
use windows_reactor::*;

/// The spinner beside the Check for Updates button.
const SPINNER_SIZE: f64 = 20.0;
/// The update note's weight, as opacity — `PrimaryText` at less than full
/// is WinUI's secondary text, and it follows the theme.
const NOTE_OPACITY: f64 = 0.65;

pub fn view(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let document = window.document();
    // One run of cards, no sub-groups (USER 2026-09-18: "no grouping"), in the
    // order the typing pipeline runs (USER 2026-09-21): what is typed and
    // how its tones are spelled, then the candidate window and its content,
    // then what a commit writes and its shape, then the app's language
    // (`GeneralSettingsView.swift`).
    View::fragment((
        // A pop-up like the Output Script row below, not a radio group (System
        // Settings' shape for a small mutually-exclusive choice). Input Script /
        // Output Script name the pair (USER 2026-09-18); mobile keeps Input Mode,
        // whose picker also holds TPS.
        choice_row(
            strings.resolve(StringKey::SettingsInputScript),
            InputMode::ALL,
            document.choice(&keys::INPUT_MODE),
            true,
            |mode: InputMode| strings.resolve(mode.label_key()).to_owned(),
            Message::SetInputMode,
            context,
        ),
        // Beside Input Script, as on the Mac; disabled under TPS.
        choice_row(
            strings.resolve(StringKey::SettingsKeyboardLayout),
            KeyboardLayout::ALL,
            document.key_reading_layout(),
            !document.is_typing_tps(),
            |layout: KeyboardLayout| layout.display_name().to_owned(),
            |layout| Message::set_choice(layout, &keys::KEYBOARD_LAYOUT),
            context,
        ),
        // Which keys type a tone is a fact about how the syllable is
        // spelled, not a shortcut (USER 2026-09-08), and the slot keys
        // follow from it rather than being chosen on the shortcut pane.
        // Under Input Script because both say what the user types.
        choice_row(
            strings.resolve(StringKey::SettingsToneInputScheme),
            ToneInputScheme::ALL,
            document.choice(&keys::TONE_INPUT_SCHEME),
            true,
            |scheme: ToneInputScheme| strings.resolve(scheme.label_key()).to_owned(),
            |scheme| Message::set_choice(scheme, &keys::TONE_INPUT_SCHEME),
            context,
        ),
        // Under Tone Keys, independent of it: both say what a letter types.
        // TPS has no `c` initial, so the row greys there, as the layout does.
        cards::switch_row(
            strings.resolve(StringKey::SettingsCAffricate),
            document.bool(&keys::IS_C_AFFRICATE_ENABLED),
            !document.is_typing_tps(),
            context.callback(|is_on| Message::SetSwitch(keys::IS_C_AFFRICATE_ENABLED, is_on)),
        ),
        // Beside it for the same reason; TPS has no romanization hyphen on
        // `f`, so the row greys there too.
        cards::switch_row(
            strings.resolve(StringKey::SettingsFHyphen),
            document.bool(&keys::IS_F_HYPHEN_ENABLED),
            !document.is_typing_tps(),
            context.callback(|is_on| Message::SetSwitch(keys::IS_F_HYPHEN_ENABLED, is_on)),
        ),
        // S33 (USER 2026-09-08): off means no window at all — the user types
        // romanization and Space / Enter write it as typed. Directly above
        // Show Typed Text First, which describes the window's content and so reads
        // as its sub-option; that row stays enabled with the window off (one
        // plain switch, no greyed-out state to explain).
        cards::switch_row(
            strings.resolve(StringKey::SettingsCandidateWindow),
            document.bool(&keys::IS_CANDIDATE_WINDOW_ENABLED),
            true,
            context.callback(|is_on| Message::SetSwitch(keys::IS_CANDIDATE_WINDOW_ENABLED, is_on)),
        ),
        // §34/S22. On means candidate slot 0 is the preedit literal, so
        // Enter writes the typed romanization.
        cards::switch_row(
            strings.resolve(StringKey::SettingsLiteralRomanCandidate),
            document.bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED),
            true,
            context.callback(|is_on| {
                Message::SetSwitch(keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, is_on)
            }),
        ),
        // Which script a commit writes (USER 2026-09-18): the same stored
        // swap the backtick shortcut toggles, so the two never disagree.
        // Disabled exactly where the shortcut is inert — Candidate Display = Romanization Only
        // shows no Hanji to lead with (`allows_swap_toggle`); under Hanji with Romanization
        // both scripts are on screen and this only picks the punctuation
        // width, as the shortcut does there. A cleared pop-up writes nothing,
        // the rule `Message::set_choice` states for every other picker.
        choice_row(
            strings.resolve(StringKey::SettingsOutputScript),
            OUTPUT_SCRIPTS,
            document.bool(&keys::IS_HANJI_FIRST),
            document
                .choice(&keys::CANDIDATE_DISPLAY_MODE)
                .allows_swap_toggle(),
            |is_hanji: bool| strings.resolve(output_script_label(is_hanji)).to_owned(),
            |is_hanji| match is_hanji {
                Some(is_hanji) => Message::SetSwitch(keys::IS_HANJI_FIRST, is_hanji),
                None => Message::SetChoice(None),
            },
            context,
        ),
        // Syllable Separator (§49), directly under Output Script — it describes that
        // output's shape.
        choice_row(
            strings.resolve(StringKey::SettingsSyllableSeparator),
            SyllableSeparator::ALL,
            document.choice(&keys::SYLLABLE_SEPARATOR),
            true,
            |choice: SyllableSeparator| strings.resolve(choice.label_key()).to_owned(),
            |choice| Message::set_choice(choice, &keys::SYLLABLE_SEPARATOR),
            context,
        ),
        // Nasal mark in POJ capitals (§53): the other setting that shapes the
        // output's romanization — a pop-up over the stored switch, ᴺ or ⁿ.
        choice_row(
            strings.resolve(StringKey::SettingsNasalMarkerUppercase),
            NASAL_MARKER_STYLES,
            document.bool(&keys::IS_NASAL_MARKER_UPPERCASE_ENABLED),
            true,
            |is_uppercase: bool| {
                strings
                    .resolve(nasal_marker_style_label(is_uppercase))
                    .to_owned()
            },
            |is_uppercase| match is_uppercase {
                Some(is_uppercase) => {
                    Message::SetSwitch(keys::IS_NASAL_MARKER_UPPERCASE_ENABLED, is_uppercase)
                }
                None => Message::SetChoice(None),
            },
            context,
        ),
        // The space after a commit: the last thing the output stage does.
        cards::switch_row(
            strings.resolve(StringKey::SettingsAutoSpace),
            document.bool(&keys::IS_AUTO_SPACE_ENABLED),
            true,
            context.callback(|is_on| Message::SetSwitch(keys::IS_AUTO_SPACE_ENABLED, is_on)),
        ),
        choice_row(
            strings.resolve(StringKey::SettingsDisplayLanguage),
            &DisplayLanguage::PICKER,
            document.display_language(),
            true,
            |language: DisplayLanguage| display_language_label(language, strings),
            |language| Message::SetChoice(language.map(SettingsWrite::display_language)),
            context,
        ),
        cards::section_gap(),
        update_row(window, strings, context),
        // Not on the update row: it holds no setting of the user's to
        // restore.
        reset_row(strings, ResetScope::General, context),
    ))
}

/// One row, never two (`GeneralSettingsView.swift`'s update rows): a known update
/// replaces the version-and-check row rather than sitting under it, and its
/// trailing control is whatever the user's next move is; the note appears
/// only when something went wrong.
fn update_row(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let updates = window.updates();
    let Some(manifest) = checker::pending_update(window.document(), INSTALLED_VERSION) else {
        let label = strings.format(
            StringKey::DesktopUpdateCurrentVersionLabel,
            &[&INSTALLED_VERSION],
        );
        let is_checking = updates.is_checking();
        return cards::row(
            &label,
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(cards::CARD_SPACING)
                .children((
                    Button::new()
                        .is_enabled(!is_checking)
                        .on_click(context.callback(|()| Message::CheckForUpdates))
                        .content(strings.resolve(StringKey::DesktopUpdateCheckNow)),
                    if is_checking {
                        spinner()
                    } else {
                        View::empty()
                    },
                )),
        );
    };
    let offer = updates.installation.offer(&manifest);
    let label = strings.format(
        StringKey::DesktopUpdatePendingVersionLabel,
        &[&manifest.version],
    );
    let control = match offer.action_key() {
        Some(key) => Button::new()
            .on_click(context.callback(|()| Message::ActOnOffer))
            .content(strings.resolve(key)),
        None => spinner(),
    };
    let note = match offer.note_key() {
        Some(key) => TextBlock::new()
            .text(strings.resolve(key))
            .text_wrapping(TextWrapping::Wrap)
            .opacity(NOTE_OPACITY)
            .into(),
        None => View::empty(),
    };
    View::fragment((cards::row(&label, control), note))
}

fn spinner() -> View {
    ProgressRing::new()
        .is_active(true)
        .width(SPINNER_SIZE)
        .height(SPINNER_SIZE)
        .into()
}
