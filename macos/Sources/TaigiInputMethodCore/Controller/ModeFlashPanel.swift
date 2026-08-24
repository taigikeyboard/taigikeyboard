// A brief on-screen flash naming the mode the keyboard just switched into.

import AppKit

/// The 英數/台語 toggle's only feedback: a small HUD that names the new mode
/// and fades. A mode with no indicator at all reads as the input method
/// breaking — composition stops starting, punctuation stops mapping — so the
/// switch announces itself once, the way McBopomofo's toggles do
/// (`references/McBopomofo/Source/NotifierController.swift`), and then gets
/// out of the way. Nothing persistent: no menu-bar variant, no status item.
@MainActor
final class ModeFlashPanel {
    static let shared = ModeFlashPanel()

    private var panel: NSPanel?
    private var hideTask: Task<Void, Never>?

    /// How long the flash holds before fading, and how long the fade takes.
    /// One reading, not an exposure: by the time these could feel wrong the
    /// user has long since learned what the tap does.
    private static let holdDuration: TimeInterval = 0.8
    private static let fadeDuration: TimeInterval = 0.25

    /// Shows `text` centred on the screen the user is working on, replacing
    /// any flash still up — a quick double tap must read as two states, not
    /// queue two panels.
    func flash(_ text: String) {
        hideTask?.cancel()
        panel?.orderOut(nil)

        let panel = Self.makePanel(text: text)
        self.panel = panel
        panel.alphaValue = 1
        panel.orderFrontRegardless()

        hideTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(Self.holdDuration))
            guard !Task.isCancelled else { return }
            self?.fadeOut(panel)
        }
    }

    private func fadeOut(_ panel: NSPanel) {
        NSAnimationContext.runAnimationGroup { context in
            context.duration = Self.fadeDuration
            panel.animator().alphaValue = 0
        } completionHandler: {
            Task { @MainActor [weak self] in
                panel.orderOut(nil)
                if self?.panel === panel { self?.panel = nil }
            }
        }
    }

    /// A borderless, non-activating rounded card. Rebuilt per flash: the text
    /// changes each time and the panel lives well under a second, so there is
    /// nothing worth keeping warm.
    private static func makePanel(text: String) -> NSPanel {
        let label = NSTextField(labelWithString: text)
        label.font = .systemFont(ofSize: 24, weight: .medium)
        label.alignment = .center
        label.sizeToFit()

        let padding = NSEdgeInsets(top: 14, left: 28, bottom: 14, right: 28)
        let contentSize = NSSize(
            width: label.frame.width + padding.left + padding.right,
            height: label.frame.height + padding.top + padding.bottom,
        )

        let background = NSVisualEffectView(frame: NSRect(origin: .zero, size: contentSize))
        background.material = .hudWindow
        background.state = .active
        background.wantsLayer = true
        background.layer?.cornerRadius = 12
        background.layer?.masksToBounds = true
        label.frame.origin = NSPoint(x: padding.left, y: padding.bottom)
        background.addSubview(label)

        let panel = NSPanel(
            contentRect: NSRect(origin: .zero, size: contentSize),
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false,
        )
        panel.contentView = background
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        panel.level = .floating
        panel.ignoresMouseEvents = true
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        panel.isReleasedWhenClosed = false

        // Centred a third of the way up, clear of both the caret line and the
        // dock. `NSScreen.main` is a best effort: an input-method agent holds
        // no key window, so on a multi-display setup the flash can land on
        // the primary display rather than the one being typed on — accepted
        // for a sub-second notice; anchoring to the caret would need a client
        // query this path does not have.
        let screen = NSScreen.main ?? NSScreen.screens.first
        if let frame = screen?.visibleFrame {
            panel.setFrameOrigin(NSPoint(
                x: frame.midX - contentSize.width / 2,
                y: frame.minY + frame.height / 3,
            ))
        }
        return panel
    }
}
