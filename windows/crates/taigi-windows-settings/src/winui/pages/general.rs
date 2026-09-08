//! The 一般 pane: romanization system, tone keys, display language,
//! auto-space, the typed-text candidate, the update row, and the attribution
//! footer. Port of `GeneralSettingsView.swift`.

use super::choice_row;
use crate::presentation::{display_language_label, SPONSOR_URL};
use crate::updates::INSTALLED_VERSION;
use crate::winui::cards;
use crate::winui::window::{Message, SettingsWindow, SettingsWrite};
use taigi_windows_core::keys::ToneInputScheme;
use taigi_windows_core::settings::{keys, InputMode, SettingChoice, SettingsDocument};
use taigi_windows_core::strings::{DisplayLanguage, StringKey, StringResolver};
use taigi_windows_update::checker;
use windows_reactor::*;

/// The spinner beside the 檢查更新 button.
const SPINNER_SIZE: f64 = 20.0;
/// Above the footer, so it sits off the last card.
const FOOTER_TOP_MARGIN: f64 = 24.0;
/// The footer's own line spacing.
const FOOTER_SPACING: f64 = 4.0;
/// The fine print's weight, as opacity — `PrimaryText` at less than full
/// is WinUI's secondary text, and it follows the theme.
const FOOTER_OPACITY: f64 = 0.65;

pub fn view(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let document = window.document();
    View::fragment((
        // A pop-up like the row under it, not a radio group (System
        // Settings' shape for a small mutually-exclusive choice).
        choice_row(
            strings.resolve(StringKey::SettingsInputMode),
            InputMode::ALL,
            document.choice(&keys::INPUT_MODE),
            |mode: InputMode| strings.resolve(mode.label_key()).to_owned(),
            |mode| Message::set_choice(mode, &keys::INPUT_MODE),
            context,
        ),
        // Directly under the romanization it belongs to: which keys type a
        // tone is a fact about how the syllable is spelled, not a shortcut
        // (USER 2026-09-08), and the slot keys follow from it rather than
        // being chosen on the shortcut pane (`GeneralSettingsView.swift`).
        choice_row(
            strings.resolve(StringKey::SettingsToneInputScheme),
            ToneInputScheme::ALL,
            document.choice(&keys::TONE_INPUT_SCHEME),
            |scheme: ToneInputScheme| strings.resolve(scheme.label_key()).to_owned(),
            |scheme| Message::set_choice(scheme, &keys::TONE_INPUT_SCHEME),
            context,
        ),
        telex_legend(document, strings),
        choice_row(
            strings.resolve(StringKey::SettingsDisplayLanguage),
            &DisplayLanguage::PICKER,
            DisplayLanguage::from_tag(&document.string(&keys::DISPLAY_LANGUAGE)),
            |language: DisplayLanguage| display_language_label(language, strings),
            |language| Message::SetChoice(language.map(SettingsWrite::display_language)),
            context,
        ),
        cards::switch_row(
            strings.resolve(StringKey::SettingsAutoSpace),
            document.bool(&keys::IS_AUTO_SPACE_ENABLED),
            true,
            context.callback(|is_on| Message::SetSwitch(keys::IS_AUTO_SPACE_ENABLED, is_on)),
        ),
        // §34/S22, under 自動空白 where the USER placed it (2026-09-03). On
        // means candidate slot 0 is the preedit literal, so Enter writes the
        // typed romanization.
        cards::switch_row(
            strings.resolve(StringKey::SettingsLiteralRomanCandidate),
            document.bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED),
            true,
            context.callback(|is_on| {
                Message::SetSwitch(keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, is_on)
            }),
        ),
        cards::section_gap(),
        update_row(window, strings, context),
        footer(strings, context),
    ))
}

/// The Telex key table, spelled for the romanization in use: `z` is `ts`
/// under TL and `ch` under POJ. Shown only while Telex is on, because
/// Standard's keys are the ones every TL/POJ user already knows.
fn telex_legend(document: &SettingsDocument, strings: &StringResolver) -> View {
    if document.choice(&keys::TONE_INPUT_SCHEME) != ToneInputScheme::Telex {
        return View::empty();
    }
    let key = match document.choice(&keys::INPUT_MODE) {
        InputMode::Poj => StringKey::SettingsToneSchemeTelexLegendPoj,
        InputMode::Tl => StringKey::SettingsToneSchemeTelexLegendTl,
    };
    TextBlock::new()
        .text(strings.resolve(key))
        .text_wrapping(TextWrapping::Wrap)
        .opacity(FOOTER_OPACITY)
        .into()
}

/// One row, never two (`GeneralSettingsView.swift:89-117`): a known update
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
            .opacity(FOOTER_OPACITY)
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

/// Centred at the foot of the pane rather than inside the form: it is
/// neither a setting nor a note about one (`sponsorFooter`).
fn footer(strings: &StringResolver, context: &mut ViewContext<SettingsWindow>) -> View {
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(FOOTER_SPACING)
        .horizontal_alignment(HorizontalAlignment::Center)
        .margin(Thickness::new(0.0, FOOTER_TOP_MARGIN, 0.0, 0.0))
        .children((
            TextBlock::new()
                .text(strings.resolve(StringKey::DesktopCopyrightLine))
                .vertical_alignment(VerticalAlignment::Center)
                .opacity(FOOTER_OPACITY),
            TextBlock::new()
                .text("\u{00B7}")
                .vertical_alignment(VerticalAlignment::Center)
                .opacity(FOOTER_OPACITY),
            // Our own open, not the control's `navigate_uri`: a browser
            // that refuses must be reported, never swallowed
            // (`ExternalLinkButton.swift:740-744`).
            HyperlinkButton::new()
                .on_click(context.callback(|()| Message::OpenUrl(SPONSOR_URL.to_owned())))
                .content(strings.resolve(StringKey::DesktopSponsorLink)),
        ))
}
