import SwiftUI

/// About page: the desktop About copy (two paragraphs, community links,
/// attribution) without the desktop sponsor link — App Store 3.1.1 bars an
/// external payment link outside the US storefront (USER 2026-09-26).
struct AboutKeyboardView: View {
    @Environment(DisplayLanguageStore.self) private var lang

    var body: some View {
        Form {
            Section {
                VStack(alignment: .leading, spacing: 12) {
                    Text(lang.string(.homeAboutIntroProject))
                    Text(lang.string(.homeAboutIntroMaintainer))
                }
                .lineSpacing(6)
            }

            Section {
                link(.homeWebsiteLink, url: Self.websiteURL, icon: Image(systemName: "globe"))
                link(.homeGithubLink, url: Self.githubURL, icon: Image("brand_github"))
                link(.homeDiscordLink, url: Self.discordURL, icon: Image("brand_discord"))
                link(.homeEmailLink, url: Self.emailURL, icon: Image(systemName: "envelope"))
            } footer: {
                Text(lang.string(.homeCopyrightLine))
                    .frame(maxWidth: .infinity)
            }
        }
        .navigationTitle(lang.string(.homeAboutKeyboard))
        .navigationBarTitleDisplayMode(.large)
    }

    private func link(_ key: StringKey, url: URL, icon: Image) -> some View {
        Link(destination: url) {
            HStack {
                Label {
                    Text(lang.string(key))
                } icon: {
                    icon
                        .resizable()
                        .scaledToFit()
                        .frame(width: 20, height: 20)
                }
                Spacer()
                Image(systemName: "arrow.up.forward.square")
                    .foregroundStyle(.secondary)
            }
        }
    }

    private static let websiteURL = URL(string: "https://taigikeyboard.tw")!
    private static let githubURL = URL(string: "https://github.com/taigikeyboard")!
    private static let discordURL = URL(string: "https://discord.gg/kXhtQfWvK")!
    private static let emailURL = URL(string: "mailto:info@taigikeyboard.tw")!
}
