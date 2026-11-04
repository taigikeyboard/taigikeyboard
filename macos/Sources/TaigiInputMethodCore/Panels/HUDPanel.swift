// The chrome the mode flash and the Telex guide share: a floating HUD card.

import AppKit

/// The look and the window class both on-screen notices are built from, so the
/// two read as one family rather than as two panels that drifted apart.
@MainActor
enum HUDPanel {
    private static let cornerRadius: CGFloat = 12

    /// The rounded, blurred card a notice's contents sit on.
    static func makeBackground(size: NSSize) -> NSVisualEffectView {
        let background = NSVisualEffectView(frame: NSRect(origin: .zero, size: size))
        background.material = .hudWindow
        background.state = .active
        background.wantsLayer = true
        background.layer?.cornerRadius = cornerRadius
        background.layer?.masksToBounds = true
        return background
    }

    /// A borderless, non-activating floating panel wrapped around `background`,
    /// sized to it. Takes no focus and no clicks: an input-method agent must
    /// not steal either from the app being typed into. `acceptsClicks` lets
    /// clicks through to the content — the TPS key panel's caps — and still
    /// takes no focus: the panel becomes key only if a view needs it, and
    /// none does.
    static func makePanel(background: NSVisualEffectView, acceptsClicks: Bool = false) -> NSPanel {
        let panel = NSPanel(
            contentRect: NSRect(origin: .zero, size: background.frame.size),
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false,
        )
        panel.contentView = background
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        panel.level = .floating
        panel.ignoresMouseEvents = !acceptsClicks
        panel.becomesKeyOnlyIfNeeded = true
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        panel.isReleasedWhenClosed = false
        return panel
    }

    /// The visible frame of the screen the mouse is on, else the main one —
    /// best effort for "the screen being typed on", for the panels that stay
    /// up while the user types (`TpsKeyboardPanel`, `TelexGuidePanel`).
    /// Activation may not ask the client for a caret, and `NSScreen.main` in
    /// an input-method agent can be the primary display whatever the user is
    /// looking at (`noticeFrame`).
    static var typingScreenFrame: NSRect? {
        let mouse = NSEvent.mouseLocation
        let screen = NSScreen.screens.first { $0.frame.contains(mouse) } ?? NSScreen.main
        return screen?.visibleFrame
    }

    /// Where a notice may be placed. `NSScreen.main` is a best effort: an
    /// input-method agent holds no key window, and these paths have no client
    /// to ask for a caret, so on a multi-display setup a notice can land on
    /// the primary display rather than the one being typed on — accepted for
    /// something this brief; anchoring to the caret would need a client query
    /// neither path has.
    static var noticeFrame: NSRect? {
        (NSScreen.main ?? NSScreen.screens.first)?.visibleFrame
    }
}
