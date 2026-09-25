//! The About page: what the project is and where to find it. Port of
//! `AboutPage.swift` / the Windows `about.rs`: two paragraphs, the sponsor
//! and the four community links as rows of one group, the attribution
//! line. No app icon, no name and no version — the General pane's version row
//! already says which build this is.

use super::PageContext;
use crate::presentation::{DISCORD_URL, EMAIL_URL, GITHUB_URL, SPONSOR_URL, WEBSITE_URL};
use adw::prelude::*;
use taigi_desktop_core::strings::StringKey;

pub fn build<'a>(mut context: PageContext<'a>, page: &adw::PreferencesPage) -> PageContext<'a> {
    let introduction = adw::PreferencesGroup::new();
    let paragraphs = gtk::Box::new(gtk::Orientation::Vertical, 10);
    for key in [
        StringKey::DesktopAboutIntroProject,
        StringKey::DesktopAboutIntroMaintainer,
    ] {
        paragraphs.append(
            &gtk::Label::builder()
                .label(context.strings.resolve(key))
                .wrap(true)
                .xalign(0.0)
                .build(),
        );
    }
    introduction.add(&paragraphs);
    page.add(&introduction);

    // Every action a row, the sponsor link first (USER 2026-09-20).
    let links = adw::PreferencesGroup::new();
    for (key, url) in [
        (StringKey::DesktopSponsorLink, SPONSOR_URL),
        (StringKey::DesktopWebsiteLink, WEBSITE_URL),
        (StringKey::DesktopGithubLink, GITHUB_URL),
        (StringKey::DesktopDiscordLink, DISCORD_URL),
        (StringKey::DesktopEmailLink, EMAIL_URL),
    ] {
        let title = context.strings.resolve(key).to_owned();
        context.link_row(&links, &title, url);
    }
    page.add(&links);

    let footer = adw::PreferencesGroup::new();
    footer.add(
        &gtk::Label::builder()
            .label(context.strings.resolve(StringKey::DesktopCopyrightLine))
            .css_classes(["dim-label", "caption"])
            .build(),
    );
    page.add(&footer);
    context
}
