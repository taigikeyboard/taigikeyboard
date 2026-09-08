// The floating Telex key table a global chord toggles up and any key takes down.

import AppKit

/// One row of the guide: the key and what it does.
///
/// Two columns, no examples and no dismiss hint (USER 2026-09-09: 「不要多餘的
/// 說明文字…不需要範例,不需要說明如何 exit」). What the affricate keys spell
/// still has to follow the romanization in use, so `z` carries its spelling —
/// `ts` under TL, `ch` under POJ — inside the meaning itself.
private struct TelexGuideRow {
    let key: String
    let meaning: Meaning

    enum Meaning {
        case tone(String)
        /// `z` / `zh`, with the initial each spells under TL and under POJ.
        case initial(tl: String, poj: String)
        case hyphen
    }

    @MainActor
    func meaningText(_ inputMode: InputMode, _ language: DisplayLanguageStore) -> String {
        switch meaning {
        case let .tone(tone): language.resolver.desktopTelexGuideTone(tone: tone)
        case let .initial(tl, poj):
            language.resolver.desktopTelexGuideInitial(initial: inputMode == .poj ? poj : tl)
        case .hyphen: language.string(.desktopTelexGuideHyphen)
        }
    }
}

/// The Telex key table as a card the user can glance at while typing
/// (USER 2026-09-09): the same legend used to sit under the 聲調拍法 picker,
/// in a settings pane the user is not looking at when they forget which
/// letter is tone 7. Raised by the `showTelexGuide` chord, which fires from
/// anywhere, and dismissed by the very next key — Escape is swallowed, every
/// other key goes on to do its job (`TaigiInputController.handle`).
///
/// The same HUD chrome as `ModeFlashPanel`, with no timer: a table is read at
/// the user's pace, not the flash's. Owned by the session that raised it, the
/// way the candidate window is (`CandidatePanel`), so a session on its way
/// out cannot take down a guide the incoming one just put up.
///
/// Always available, not gated on the Telex scheme: a user deciding whether
/// to switch schemes is exactly the user who wants to see the table.
@MainActor
final class TelexGuidePanel {
    static let shared = TelexGuidePanel()

    private var panel: NSPanel?

    /// The session the guide is showing for, or nil when nothing is showing.
    private(set) var owner: ComposingSessionToken?

    /// The romanization the card on screen is spelled for, or nil when
    /// nothing is showing — what a test reads instead of the view tree.
    private(set) var shownInputMode: InputMode?

    var isShowing: Bool {
        panel?.isVisible == true
    }

    /// The widest the card may grow; past this the meaning column wraps.
    private static let maxWidth: CGFloat = 520

    /// Row order is reading order: the tones by number, then the two
    /// consonant keys, then the hyphen, then the digits.
    private static let rows: [TelexGuideRow] = [
        TelexGuideRow(key: "v", meaning: .tone("2")),
        TelexGuideRow(key: "y", meaning: .tone("3")),
        TelexGuideRow(key: "d", meaning: .tone("5")),
        TelexGuideRow(key: "w", meaning: .tone("7")),
        TelexGuideRow(key: "x", meaning: .tone("8")),
        TelexGuideRow(key: "q", meaning: .tone("9")),
        TelexGuideRow(key: "z", meaning: .initial(tl: "ts", poj: "ch")),
        TelexGuideRow(key: "zh", meaning: .initial(tl: "tsh", poj: "chh")),
        TelexGuideRow(key: "f", meaning: .hyphen),
    ]

    /// The meanings the guide shows under `inputMode`, in row order — what a
    /// test reads to pin the romanization without walking the view tree.
    @MainActor
    static func meanings(under inputMode: InputMode, language: DisplayLanguageStore) -> [String] {
        rows.map { $0.meaningText(inputMode, language) }
    }

    func toggle(inputMode: InputMode, language: DisplayLanguageStore, ownedBy owner: ComposingSessionToken) {
        if isShowing {
            hideNow()
        } else {
            show(inputMode: inputMode, language: language, ownedBy: owner)
        }
    }

    /// Shows the table for `inputMode`, replacing any guide still up. Rebuilt
    /// each time: the content follows the mode and the display language, and
    /// a card shown a few times a day is not worth keeping warm.
    func show(inputMode: InputMode, language: DisplayLanguageStore, ownedBy owner: ComposingSessionToken) {
        panel?.orderOut(nil)
        let panel = Self.makePanel(inputMode: inputMode, language: language)
        self.panel = panel
        panel.orderFrontRegardless()
        self.owner = owner
        shownInputMode = inputMode
    }

    /// Takes the guide down only if `owner` raised it. IMK activates the
    /// incoming session before it deactivates the outgoing one
    /// (`ComposingSessionCoordinator.claim`), so an old session's teardown
    /// must not close a guide the new one just put up.
    func hide(ownedBy owner: ComposingSessionToken) {
        guard self.owner == owner else { return }
        hideNow()
    }

    /// Takes the guide down whatever session raised it — what the key path and
    /// the toggle want, both of which run on the session the user is in.
    func hideNow() {
        owner = nil
        shownInputMode = nil
        panel?.orderOut(nil)
        panel = nil
    }

    /// The card: the scheme's name and the two-column table, on the same HUD
    /// chrome as the mode flash.
    private static func makePanel(inputMode: InputMode, language: DisplayLanguageStore) -> NSPanel {
        let title = NSTextField(labelWithString: language.string(.desktopTelexGuideTitle))
        title.font = .systemFont(ofSize: NSFont.systemFontSize + 2, weight: .semibold)

        let grid = makeGrid(inputMode: inputMode, language: language)

        let stack = NSStackView(views: [title, grid])
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 10
        stack.edgeInsets = NSEdgeInsets(top: 16, left: 22, bottom: 14, right: 22)
        stack.translatesAutoresizingMaskIntoConstraints = false
        stack.widthAnchor.constraint(lessThanOrEqualToConstant: maxWidth).isActive = true
        stack.layoutSubtreeIfNeeded()
        let contentSize = stack.fittingSize

        let background = HUDPanel.makeBackground(size: contentSize)
        background.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: background.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: background.trailingAnchor),
            stack.topAnchor.constraint(equalTo: background.topAnchor),
            stack.bottomAnchor.constraint(equalTo: background.bottomAnchor),
        ])

        let panel = HUDPanel.makePanel(background: background)

        if let frame = HUDPanel.noticeFrame {
            panel.setFrameOrigin(NSPoint(
                x: frame.midX - contentSize.width / 2,
                y: frame.midY - contentSize.height / 2,
            ))
        }
        return panel
    }

    /// Key | meaning. The key is monospaced semibold because it is a thing
    /// the user types and the one the eye lands on.
    private static func makeGrid(inputMode: InputMode, language: DisplayLanguageStore) -> NSGridView {
        let keyFont = NSFont.monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .semibold)

        let cells = rows.map { row -> [NSView] in
            let key = NSTextField(labelWithString: row.key)
            key.font = keyFont
            let meaning = NSTextField(labelWithString: row.meaningText(inputMode, language))
            return [key, meaning]
        }
        let grid = NSGridView(views: cells)
        grid.rowSpacing = 4
        grid.columnSpacing = 18
        grid.translatesAutoresizingMaskIntoConstraints = false
        return grid
    }
}
