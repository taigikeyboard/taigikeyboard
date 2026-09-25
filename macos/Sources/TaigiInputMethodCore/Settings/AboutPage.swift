// The About page: what the project is and where to find it.

import SwiftUI

/// What the input-source menu's About row opens (USER 2026-09-20): two lines the
/// USER wrote; the sponsor and the four community links as rows; the
/// attribution line.
///
/// A grouped `Form` like every other pane (USER 2026-09-20: "make it a page"), so the
/// page sits where the settings do and reads in the same cards. No app icon, no
/// name and no version — the update row on General already says which build this is.
struct AboutPage: View {
    @Environment(DisplayLanguageStore.self) private var language

    var body: some View {
        Form {
            // No heading (USER 2026-09-20: "no 'TaigiKeyboard' heading needed"): the window
            // title already names the page. Leading-aligned like the rows
            // under it (USER 2026-09-20: "left-align the copy").
            Section {
                VStack(alignment: .leading, spacing: Metrics.paragraphSpacing) {
                    Text(language.string(.desktopAboutIntroProject))
                    Text(language.string(.desktopAboutIntroMaintainer))
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.vertical, Metrics.cardInset)
            }

            // Every action a row, the sponsor link first (USER 2026-09-20: no
            // lone button on the page). The attribution as the card's
            // footer: fine print on the ground, where a form puts a note that
            // is neither a setting nor a link.
            Section {
                ExternalLinkButton(titleKey: .desktopSponsorLink, url: Self.sponsorURL, style: .row(.heart))
                ExternalLinkButton(titleKey: .desktopWebsiteLink, url: Self.websiteURL, style: .row(.globe))
                ExternalLinkButton(titleKey: .desktopGithubLink, url: Self.githubURL, style: .row(.github))
                ExternalLinkButton(titleKey: .desktopDiscordLink, url: Self.discordURL, style: .row(.discord))
                ExternalLinkButton(titleKey: .desktopEmailLink, url: Self.emailURL, style: .row(.envelope))
            } footer: {
                Text(language.string(.desktopCopyrightLine))
                    .frame(maxWidth: .infinity)
                    .padding(.top, Metrics.footerGap)
            }
        }
        .formStyle(.grouped)
    }

    private static let websiteURL = URL(string: "https://taigikeyboard.tw")
    private static let githubURL = URL(string: "https://github.com/taigikeyboard")
    private static let discordURL = URL(string: "https://discord.gg/kXhtQfWvK")
    private static let emailURL = URL(string: "mailto:info@taigikeyboard.tw")
    private static let sponsorURL = URL(string: "https://p.ecpay.com.tw/AA663DE")

    private enum Metrics {
        /// Between paragraphs of one text.
        static let paragraphSpacing: CGFloat = 10

        /// Air above and below a card of running text; a row of one line needs none.
        static let cardInset: CGFloat = 4

        /// Between the last card and the attribution under it.
        static let footerGap: CGFloat = 8
    }
}
