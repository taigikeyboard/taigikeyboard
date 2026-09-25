//! The About page: what the project is and where to find it. Port of
//! `AboutPage.swift`.
//!
//! What the tray menu's About row opens (USER 2026-09-20): two lines the
//! USER wrote; the sponsor and the four community links as rows of one card; the attribution line. In
//! the same cards as every other pane (USER 2026-09-20: "make it a page"). No app
//! icon, no name and no version — the update row on General already says which build
//! this is.

use crate::presentation::{DISCORD_URL, EMAIL_URL, GITHUB_URL, SPONSOR_URL, WEBSITE_URL};
use crate::winui::cards;
use crate::winui::font_awesome::FontAwesomeGlyph;
use crate::winui::window::{Message, SettingsWindow};
use taigi_desktop_core::strings::{StringKey, StringResolver};
use windows_reactor::*;

/// `CaptionTextBlockStyle`'s size, the attribution's fine print.
const CAPTION_FONT_SIZE: f64 = 12.0;
/// Between paragraphs of one text (`Metrics.paragraphSpacing`).
const PARAGRAPH_SPACING: f64 = 10.0;
/// A link card's mark, the size of a sidebar icon (`Metrics.rowGlyphSize`).
const MARK_SIZE: f64 = 16.0;
/// Between the last card and the attribution under it (`Metrics.footerGap`).
const FOOTER_GAP: f64 = 8.0;
/// The fine print's weight, as opacity — `PrimaryText` at less than full
/// is WinUI's secondary text, and it follows the theme.
const SECONDARY_OPACITY: f64 = 0.65;

pub fn view(
    _window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    // No heading (USER 2026-09-20: "no 'TaigiKeyboard' heading needed"): the window
    // title already names the page. Leading-aligned like the cards under
    // it (USER 2026-09-20: "left-align the copy").
    let introduction = cards::frame(StackPanel::new().spacing(PARAGRAPH_SPACING).children((
        paragraph(strings.resolve(StringKey::HomeAboutIntroProject)),
        paragraph(strings.resolve(StringKey::HomeAboutIntroMaintainer)),
    )));
    View::fragment((
        introduction,
        cards::section_gap(),
        // Every action a row, the sponsor link first (USER 2026-09-20: no
        // lone button on the page); one card, as on the Mac.
        cards::link_group([
            link_row(
                strings,
                context,
                FontAwesomeGlyph::Heart,
                StringKey::DesktopSponsorLink,
                SPONSOR_URL,
            ),
            link_row(
                strings,
                context,
                FontAwesomeGlyph::Globe,
                StringKey::HomeWebsiteLink,
                WEBSITE_URL,
            ),
            link_row(
                strings,
                context,
                FontAwesomeGlyph::Github,
                StringKey::HomeGithubLink,
                GITHUB_URL,
            ),
            link_row(
                strings,
                context,
                FontAwesomeGlyph::Discord,
                StringKey::HomeDiscordLink,
                DISCORD_URL,
            ),
            link_row(
                strings,
                context,
                FontAwesomeGlyph::Envelope,
                StringKey::HomeEmailLink,
                EMAIL_URL,
            ),
        ]),
        // Fine print on the ground under the last card: neither a setting
        // nor a link.
        TextBlock::new()
            .text(strings.resolve(StringKey::HomeCopyrightLine))
            .font_size(CAPTION_FONT_SIZE)
            .horizontal_alignment(HorizontalAlignment::Center)
            .margin(Thickness::new(0.0, FOOTER_GAP, 0.0, 0.0))
            .opacity(SECONDARY_OPACITY),
    ))
}

fn paragraph(text: &str) -> View {
    TextBlock::new()
        .text(text)
        .text_wrapping(TextWrapping::Wrap)
        .into()
}

/// A row that opens one of the links, its Font Awesome mark at the left
/// (`ExternalLinkButton.Style.row`).
fn link_row(
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
    glyph: FontAwesomeGlyph,
    title: StringKey,
    url: &'static str,
) -> View {
    // A `Viewbox` rather than a size on the icon: `PathIcon` draws its
    // geometry at the geometry's own size, and a smaller frame would only
    // clip it.
    let mark = Viewbox::new()
        .height(MARK_SIZE)
        .stretch(Stretch::Uniform)
        .opacity(SECONDARY_OPACITY)
        .slot(ViewboxSlot::Child, glyph.icon());
    cards::link_row(
        mark,
        strings.resolve(title),
        context.callback(move |()| Message::OpenUrl(url.to_owned())),
    )
}
