// The floating Telex key table a global chord toggles up and any key takes down.

import AppKit

/// One row of the guide: the key, what it does (a string key, so the meaning
/// follows the display language), and the example under each romanization.
///
/// The examples are romanization, not prose, so they are spelled here rather
/// than translated: `z` is `ts` under TL and `ch` under POJ, and tone 9 is a
/// double acute (U+030B) in TL but a breve (U+0306) in POJ — the row has to
/// show the spelling the user will actually see.
private struct TelexGuideRow {
    let key: String
    let meaning: Meaning
    let tlExample: String
    let pojExample: String

    enum Meaning {
        case tone(String)
        case initial
        case hyphen
        case pick
    }

    /// The same example under both romanizations — every tone key but 9.
    init(key: String, meaning: Meaning, example: String) {
        self.init(key: key, meaning: meaning, tlExample: example, pojExample: example)
    }

    init(key: String, meaning: Meaning, tlExample: String, pojExample: String) {
        self.key = key
        self.meaning = meaning
        self.tlExample = tlExample
        self.pojExample = pojExample
    }

    func example(under inputMode: InputMode) -> String {
        inputMode == .poj ? pojExample : tlExample
    }

    @MainActor
    func meaningText(_ language: DisplayLanguageStore) -> String {
        switch meaning {
        case let .tone(tone): language.resolver.desktopTelexGuideTone(tone: tone)
        case .initial: language.string(.desktopTelexGuideInitial)
        case .hyphen: language.string(.desktopTelexGuideHyphen)
        case .pick: language.string(.desktopTelexGuidePick)
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
        TelexGuideRow(key: "v", meaning: .tone("2"), example: "tev → té"),
        TelexGuideRow(key: "y", meaning: .tone("3"), example: "pay → pà"),
        TelexGuideRow(key: "d", meaning: .tone("5"), example: "langd → lâng"),
        TelexGuideRow(key: "w", meaning: .tone("7"), example: "kangw → kāng"),
        TelexGuideRow(key: "x", meaning: .tone("8"), example: "titx → ti̍t"),
        TelexGuideRow(key: "q", meaning: .tone("9"), tlExample: "tsangq → tsa̋ng", pojExample: "zangq → chăng"),
        TelexGuideRow(key: "z", meaning: .initial, tlExample: "zo → tso", pojExample: "zit → chit"),
        TelexGuideRow(key: "zh", meaning: .initial, tlExample: "zhi → tshi", pojExample: "zhit → chhit"),
        TelexGuideRow(key: "f", meaning: .hyphen, example: "taidfgiv → tâi-gí"),
        TelexGuideRow(key: "1–9", meaning: .pick, example: ""),
    ]

    /// The examples the guide shows under `inputMode`, in row order — what a
    /// test reads to pin the romanization without walking the view tree.
    static func examples(under inputMode: InputMode) -> [String] {
        rows.map { $0.example(under: inputMode) }
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

    /// The card: title, the three-column table, the dismiss hint, on the same
    /// HUD chrome as the mode flash.
    private static func makePanel(inputMode: InputMode, language: DisplayLanguageStore) -> NSPanel {
        let title = NSTextField(labelWithString: language.string(.desktopTelexGuideTitle))
        title.font = .systemFont(ofSize: NSFont.systemFontSize + 2, weight: .semibold)

        let grid = makeGrid(inputMode: inputMode, language: language)

        let hint = NSTextField(labelWithString: language.string(.desktopTelexGuideDismiss))
        hint.font = .systemFont(ofSize: NSFont.smallSystemFontSize)
        hint.textColor = .secondaryLabelColor

        let stack = NSStackView(views: [title, grid, hint])
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

    /// Key | meaning | example. The key and the example share a monospaced
    /// face because both are things the user types; the example is secondary
    /// so the key stays the thing the eye lands on.
    private static func makeGrid(inputMode: InputMode, language: DisplayLanguageStore) -> NSGridView {
        let keyFont = NSFont.monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .semibold)
        let exampleFont = NSFont.monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .regular)

        let cells = rows.map { row -> [NSView] in
            let key = NSTextField(labelWithString: row.key)
            key.font = keyFont
            let meaning = NSTextField(labelWithString: row.meaningText(language))
            let example = NSTextField(labelWithString: row.example(under: inputMode))
            example.font = exampleFont
            example.textColor = .secondaryLabelColor
            return [key, meaning, example]
        }
        let grid = NSGridView(views: cells)
        grid.rowSpacing = 4
        grid.columnSpacing = 18
        grid.translatesAutoresizingMaskIntoConstraints = false
        return grid
    }
}
