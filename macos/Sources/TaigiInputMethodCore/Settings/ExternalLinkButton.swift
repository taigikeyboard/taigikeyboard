// Shared settings control for an action that leaves the app to a web page.

import AppKit
import SwiftUI

/// A link out to the web, with the failure shown rather than swallowed.
///
/// `NSWorkspace.open` answers `false` when nothing could handle the URL, and a
/// button that silently does nothing is indistinguishable from a broken one.
struct ExternalLinkButton: View {
    /// How loudly the link reads.
    ///
    /// `.standard` is the settings-row form: the project's `arrow.up.forward.square`
    /// leave-the-app affordance, drawn in the accent colour.
    ///
    /// `.footer` is the understated inline form for an attribution line. It carries no
    /// icon, inherits the surrounding footer's type, and draws in the same `.secondary`
    /// as the text beside it — an icon and an accent colour are what would make a footer
    /// read as a control rather than as fine print, and the pointer plus the hover lift
    /// carry the affordance instead.
    enum Style {
        case standard
        case footer
    }

    @Environment(DisplayLanguageStore.self) private var language

    let titleKey: StringKey
    let url: URL?
    var style: Style = .standard

    @State private var didFail = false
    @State private var isHovering = false

    var body: some View {
        styledButton
            .alert(language.string(.macosOpenURLFailed), isPresented: $didFail) {
                Button(language.string(.commonOk)) {}
            } message: {
                Text(url?.absoluteString ?? "")
            }
    }

    @ViewBuilder
    private var styledButton: some View {
        switch style {
        case .standard:
            Button(action: open) {
                Label(language.string(titleKey), systemImage: "arrow.up.forward.square")
            }
            .buttonStyle(.link)

        case .footer:
            Button(action: open) {
                Text(language.string(titleKey))
            }
            .buttonStyle(.plain)
            // The idle colour restates the `.secondary` the footer line already sets, rather than
            // inheriting it, because hovering needs a stated colour to lift away from.
            .foregroundStyle(isHovering ? .primary : .secondary)
            .onHover { setHovering($0) }
            // `onHover` promises a callback when the pointer enters or leaves the frame, not when
            // the view goes away under a still-hovering pointer — which would strand the cursor.
            .onDisappear { setHovering(false) }
            // `.plain` drops the link role that `.buttonStyle(.link)` carried implicitly, and this
            // is a link rather than a button: it navigates away instead of acting on the window.
            .accessibilityRemoveTraits(.isButton)
            .accessibilityAddTraits(.isLink)
        }
    }

    private func open() {
        guard let url, NSWorkspace.shared.open(url) else {
            didFail = true
            return
        }
    }

    /// Mirrors the pointing-hand cursor `.buttonStyle(.link)` gives for free. `.pointerStyle(.link)`
    /// would say this natively, but it needs macOS 15 and this target deploys to 14.
    ///
    /// Sole owner of this view's place on the shared cursor stack: the state guard keeps every
    /// `push` paired with exactly one `pop`, so a repeated or late call cannot pop someone else's
    /// cursor.
    private func setHovering(_ hovering: Bool) {
        guard hovering != isHovering else { return }
        isHovering = hovering
        if hovering {
            NSCursor.pointingHand.push()
        } else {
            NSCursor.pop()
        }
    }
}
