//! The About page: what the project is and where to find it. Port of
//! `AboutPage.swift` / the Windows `about.rs`: two paragraphs, the three
//! sponsor and the seven community links as rows of one group, the attribution
//! line. No app icon, no name and no version — the General pane's version row
//! already says which build this is.

use super::PageContext;
use adw::prelude::*;
use taigi_desktop_core::settings::presentation::{
    DISCORD_URL, EMAIL_URL, FACEBOOK_URL, GITHUB_URL, INSTAGRAM_URL, SPONSOR_ECPAY_URL,
    SPONSOR_KOFI_URL, SPONSOR_PAYPAL_URL, THREADS_URL, WEBSITE_URL,
};
use taigi_desktop_core::strings::StringKey;

pub fn build<'a>(mut context: PageContext<'a>, page: &adw::PreferencesPage) -> PageContext<'a> {
    let introduction = adw::PreferencesGroup::new();
    let paragraphs = gtk::Box::new(gtk::Orientation::Vertical, 10);
    for key in [
        StringKey::HomeAboutIntroProject,
        StringKey::HomeAboutIntroMaintainer,
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

    // Every action a row, the sponsor links first (USER 2026-09-20).
    let links = adw::PreferencesGroup::new();
    for (key, url) in [
        (StringKey::DesktopSponsorEcpay, SPONSOR_ECPAY_URL),
        (StringKey::DesktopSponsorPaypal, SPONSOR_PAYPAL_URL),
        (StringKey::DesktopSponsorKofi, SPONSOR_KOFI_URL),
        (StringKey::HomeWebsiteLink, WEBSITE_URL),
        (StringKey::HomeGithubLink, GITHUB_URL),
        (StringKey::HomeDiscordLink, DISCORD_URL),
        (StringKey::HomeFacebookLink, FACEBOOK_URL),
        (StringKey::HomeInstagramLink, INSTAGRAM_URL),
        (StringKey::HomeThreadsLink, THREADS_URL),
        (StringKey::HomeEmailLink, EMAIL_URL),
    ] {
        let title = context.strings.resolve(key).to_owned();
        context.link_row(&links, &title, url);
    }
    page.add(&links);

    let footer = adw::PreferencesGroup::new();
    footer.add(
        &gtk::Label::builder()
            .label(context.strings.resolve(StringKey::HomeCopyrightLine))
            .css_classes(["dim-label", "caption"])
            .build(),
    );
    page.add(&footer);
    context
}
