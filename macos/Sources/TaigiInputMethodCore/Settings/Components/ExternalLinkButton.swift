// Shared settings control for an action that leaves the app to a web page.

import AppKit
import SwiftUI

/// A link out to the web, with the failure shown rather than swallowed.
///
/// `NSWorkspace.open` answers `false` when nothing could handle the URL, and a
/// button that silently does nothing is indistinguishable from a broken one.
struct ExternalLinkButton: View {
    /// How the link reads.
    ///
    /// `.standard` is the settings-row form: the project's `arrow.up.forward.square`
    /// leave-the-app affordance, drawn in the accent colour.
    ///
    /// `.row(glyph)` is a whole form row: the glyph, the title, and the leave-the-app
    /// arrow at the trailing edge in the secondary colour — the shape System Settings
    /// gives a row that opens somewhere else.
    ///
    enum Style {
        case standard
        case row(FontAwesomeGlyph)
    }

    @Environment(DisplayLanguageStore.self) private var language

    let titleKey: StringKey
    let url: URL?
    var style: Style = .standard

    @State private var failedLink: String?

    var body: some View {
        styledButton
            .openURLFailureAlert($failedLink)
    }

    @ViewBuilder
    private var styledButton: some View {
        switch style {
        case .standard:
            Button(action: open) {
                Label(language.string(titleKey), systemImage: "arrow.up.forward.square")
            }
            .buttonStyle(.link)

        case let .row(glyph):
            Button(action: open) {
                HStack(spacing: Metrics.rowSpacing) {
                    Image(nsImage: glyph.image)
                        .renderingMode(.template)
                        .resizable()
                        .scaledToFit()
                        .frame(width: Metrics.rowGlyphSize, height: Metrics.rowGlyphSize)
                        .foregroundStyle(.secondary)
                    Text(language.string(titleKey))
                    Spacer()
                    Image(systemName: "arrow.up.forward.square")
                        .foregroundStyle(.secondary)
                }
                // The whole row, not only its text, takes the click.
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            // `.plain` drops the link role that `.buttonStyle(.link)` carried implicitly, and this
            // is a link rather than a button: it navigates away instead of acting on the window.
            .accessibilityRemoveTraits(.isButton)
            .accessibilityAddTraits(.isLink)
        }
    }

    private func open() {
        guard let url, NSWorkspace.shared.open(url) else {
            failedLink = url?.absoluteString ?? ""
            return
        }
    }

    private enum Metrics {
        /// Between the glyph and the title in a row: the gap a `Label` leaves.
        static let rowSpacing: CGFloat = 8

        /// A row's glyph, the size of a sidebar symbol — a mark, not fine print.
        static let rowGlyphSize: CGFloat = 16
    }
}

extension View {
    /// The alert a link that could not be opened raises, naming the link —
    /// shown while `failedLink` is set, cleared when dismissed.
    func openURLFailureAlert(_ failedLink: Binding<String?>) -> some View {
        modifier(OpenURLFailureAlert(failedLink: failedLink))
    }
}

private struct OpenURLFailureAlert: ViewModifier {
    @Environment(DisplayLanguageStore.self) private var language

    @Binding var failedLink: String?

    func body(content: Content) -> some View {
        content.alert(
            language.string(.desktopOpenURLFailed),
            isPresented: Binding(get: { failedLink != nil }, set: {
                if !$0 {
                    failedLink = nil
                }
            }),
        ) {
            Button(language.string(.commonOk)) {}
        } message: {
            Text(failedLink ?? "")
        }
    }
}
