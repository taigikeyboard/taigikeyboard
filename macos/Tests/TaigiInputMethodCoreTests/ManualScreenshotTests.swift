// Renders the user manual's screenshots (`media/manual/shots/`) from the real controller, engine and windows.

import AppKit
import Carbon.HIToolbox
import KeyboardShortcuts
@testable import TaigiInputMethodCore
import XCTest

/// Not a test of behaviour: a generator. It types into the real controller,
/// lets the real candidate window come up over a stand-in document window,
/// and saves what the screen shows — so the manual's pictures are the app,
/// not drawings of it, and re-running this after a UI change refreshes them.
///
/// Skipped unless `TAIGI_MANUAL_SHOTS` names the output directory, because
/// it puts windows on screen and captures a region of it:
///
///     TAIGI_MANUAL_SHOTS=$PWD/../media/manual/shots swift test --filter ManualScreenshotTests
///
/// The terminal running it needs Screen Recording permission. Images come out
/// at the main display's scale, so run it on a Retina display for 2x. The
/// settings window is drawn inactive (grey switches and traffic lights): a
/// test runner cannot become the active application.
@MainActor
final class ManualScreenshotTests: XCTestCase {
    private static let outputEnvironmentKey = "TAIGI_MANUAL_SHOTS"

    /// Every setting a scene may move; each is cleared before a scene so a
    /// scene starts from the factory defaults plus what it names.
    private static let sceneSettingNames = [
        SettingsStore.Keys.inputMode.name,
        SettingsStore.Keys.isHanjiFirst.name,
        SettingsStore.Keys.candidateDisplayMode.name,
        SettingsStore.Keys.isLiteralRomanCandidateEnabled.name,
        SettingsStore.Keys.syllableSeparator.name,
        SettingsStore.Keys.isAutoSpaceEnabled.name,
        SettingsStore.Keys.candidateLayout.name,
        SettingsStore.Keys.appearanceMode.name,
        SettingsStore.Keys.candidateSize.name,
        SettingsStore.Keys.fontType.name,
        SettingsStore.Keys.toneInputScheme.name,
        SettingsStore.Keys.selectedSettingsPane.name,
        SettingsStore.Keys.learningRecordsKind.name,
        SettingsStore.Keys.learningRecordsOrder.name,
        SettingsStore.Keys.displayLanguage.name,
        SettingsStore.Keys.isTpsKeyboardShown.name,
        SettingsStore.Keys.isTelexGuideShown.name,
    ]

    private var outputDirectory = URL(fileURLWithPath: NSTemporaryDirectory())

    override func setUpWithError() throws {
        try super.setUpWithError()
        guard let path = ProcessInfo.processInfo.environment[Self.outputEnvironmentKey], !path.isEmpty else {
            throw XCTSkip("set \(Self.outputEnvironmentKey) to an output directory to render the manual's screenshots")
        }
        outputDirectory = URL(fileURLWithPath: path)
        try FileManager.default.createDirectory(at: outputDirectory, withIntermediateDirectories: true)
        InstalledLexicon.installOnce()
        XCTAssertEqual(TestFixtures.unregisterableFontFiles, [])
        for name in Self.sceneSettingNames {
            clearSettingRestoredAtTeardown(name)
        }
    }

    // MARK: - Scenes

    /// One picture of the candidate window over the stand-in document.
    private struct Scene {
        let name: String
        var settings: [String: Any] = [:]
        /// Typed one character per key event.
        var keys = "taigi"
        /// Pressed after `keys`: navigation that changes what is highlighted.
        var navigation: [NavigationKey] = []
        var shortcut: ShortcutAction?
        /// Opens the symbol picker, which answers its recorded chord on the
        /// key path rather than `performShortcutAction`.
        var opensSymbolPicker = false
        /// Saves the floating panel the scene raised (the Telex guide, the
        /// TPS key panel) instead of the document.
        var capturesFloatingPanel = false
        var documentSize = ManualScreenshotTests.tall
    }

    private static let layout = SettingsStore.Keys.candidateLayout.name
    private static let displayMode = SettingsStore.Keys.candidateDisplayMode.name
    private static let hanjiFirst = SettingsStore.Keys.isHanjiFirst.name
    private static let literal = SettingsStore.Keys.isLiteralRomanCandidateEnabled.name
    private static let toneScheme = SettingsStore.Keys.toneInputScheme.name
    private static let appearance = SettingsStore.Keys.appearanceMode.name
    private static let size = SettingsStore.Keys.candidateSize.name
    private static let font = SettingsStore.Keys.fontType.name
    private static let inputMode = SettingsStore.Keys.inputMode.name

    /// The document sizes: just enough room for the window each layout
    /// raises, so a picture is the window rather than the page around it.
    private nonisolated static let tall = NSSize(width: 330, height: 350)
    private nonisolated static let wide = NSSize(width: 560, height: 130)

    private static let scenes: [Scene] = [
        // Candidate Window Layout
        Scene(name: "layout-vertical", settings: [layout: "vertical"]),
        Scene(name: "layout-horizontal", settings: [layout: "horizontal"], documentSize: wide),
        Scene(name: "layout-expandable", settings: [layout: "expandable"], documentSize: wide),
        Scene(
            name: "layout-expandable-open", settings: [layout: "expandable"], navigation: [.downArrow],
            documentSize: NSSize(width: 560, height: 330),
        ),
        // Candidate Display
        Scene(name: "display-pairing", settings: [layout: "horizontal", displayMode: "sideBySide"], documentSize: wide),
        Scene(name: "display-combined", settings: [layout: "horizontal", displayMode: "combined"], documentSize: wide),
        Scene(name: "display-roman", settings: [layout: "horizontal", displayMode: "romanOnly"], documentSize: wide),
        // Switch Hanji or Romanization
        Scene(name: "swap-hanji-first", settings: [hanjiFirst: true]),
        Scene(name: "swap-roman-first", settings: [hanjiFirst: false]),
        // Show Typed Text First
        Scene(name: "literal-on", settings: [literal: true], keys: "siong5"),
        Scene(name: "literal-off", settings: [literal: false], keys: "siong5"),
        // Tone Keys
        Scene(name: "tone-numeric", settings: [toneScheme: "standard"], keys: "tai5gi2"),
        Scene(name: "tone-telex", settings: [toneScheme: "telex"], keys: "taidgiv"),
        // Appearance
        Scene(name: "appearance-light", settings: [appearance: "light"]),
        Scene(name: "appearance-dark", settings: [appearance: "dark"]),
        Scene(name: "size-extra-small", settings: [size: "13"]),
        Scene(name: "size-extra-large", settings: [size: "23"], documentSize: NSSize(width: 330, height: 440)),
        Scene(name: "font-huninn", settings: [font: "openHuninn"]),
        Scene(name: "font-iansui", settings: [font: "iansui"]),
        Scene(name: "font-genyomin", settings: [font: "genYoMin"]),
        // Input Script
        Scene(name: "mode-poj", settings: [inputMode: "poj"], keys: "peh8oe7ji7", documentSize: NSSize(width: 380, height: 220)),
        Scene(name: "mode-tl", settings: [inputMode: "tl"], keys: "peh8ue7ji7", documentSize: NSSize(width: 380, height: 220)),
        Scene(name: "mode-tps", settings: [inputMode: "tps"], keys: "296Eu4", navigation: [.downArrow]),
        // Panels
        Scene(name: "panel-symbols", keys: "", opensSymbolPicker: true),
        Scene(name: "panel-telex-guide", settings: [toneScheme: "telex"], keys: "", shortcut: .showTelexGuide, capturesFloatingPanel: true),
        Scene(name: "panel-tps-keyboard", settings: [inputMode: "tps"], keys: "", shortcut: .showTpsKeyboard, capturesFloatingPanel: true),
    ]

    func testRenderCandidateScenes() throws {
        let savedPickerShortcut = KeyboardShortcuts.getShortcut(for: .showSymbolPicker)
        KeyboardShortcuts.setShortcut(ShortcutAction.showSymbolPicker.defaultShortcut, for: .showSymbolPicker)
        defer { KeyboardShortcuts.setShortcut(savedPickerShortcut, for: .showSymbolPicker) }
        for scene in Self.scenes {
            try render(scene)
        }
    }

    private func render(_ scene: Scene) throws {
        for name in Self.sceneSettingNames {
            UserDefaults.standard.removeObject(forKey: name)
        }
        // Light unless the scene is about the appearance: Automatic would
        // follow whatever the Mac rendering this happens to be set to.
        UserDefaults.standard.set("light", forKey: Self.appearance)
        for (name, value) in scene.settings {
            UserDefaults.standard.set(value, forKey: name)
        }
        let isDark = scene.settings[Self.appearance] as? String == "dark"

        let document = DocumentWindow(size: scene.documentSize, isDark: isDark)
        document.orderFrontRegardless()
        defer { document.orderOut(nil) }

        let client = RecordingTextInputClient()
        let controller = try TestFixtures.makeInputController()
        controller.displayLanguageOverride = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: .standard)
        controller.symbolTableOverride = try TestFixtures.shippedSymbolTable()
        controller.activateServer(client)
        defer {
            controller.candidatePresenter.hideForHandover()
            controller.symbolPickerPresenter.hideForHandover()
            TelexGuidePanel.shared.hideNow()
            controller.deactivateServer(client)
            TpsKeyboardPanel.shared.hideNow()
        }

        func syncDocument() {
            var marked = ""
            if case let .setMarkedText(text, _) = client.writes.last {
                marked = text
            }
            let caret = document.show(committed: client.insertedTexts.joined(), marked: marked)
            client.caretRects = Dictionary(uniqueKeysWithValues: (0 ... 64).map { ($0, caret) })
        }

        syncDocument()
        for character in scene.keys.map(String.init) {
            // A capital is its key with Shift held, which is how the TPS
            // layout tells its two layers apart.
            let isCapital = character != character.lowercased()
            _ = try controller.handle(
                TestFixtures.keyDownEvent(characters: character, modifiers: isCapital ? [.shift] : []),
                client: client,
            )
            // The caret the NEXT key's window anchors to follows what this key wrote.
            syncDocument()
        }
        for key in scene.navigation {
            _ = try controller.handle(TestFixtures.arrowKeyDownEvent(key), client: client)
        }
        if let shortcut = scene.shortcut {
            controller.performShortcutAction(shortcut)
        }
        if scene.opensSymbolPicker {
            _ = try controller.handle(
                TestFixtures.keyDownEvent(
                    characters: ",", modifiers: [.control, .command], keyCode: UInt16(kVK_ANSI_Comma),
                ),
                client: client,
            )
        }
        syncDocument()

        guard scene.capturesFloatingPanel else {
            return try capture(screenRect: document.frame, to: scene.name)
        }
        settle()
        let panel = try XCTUnwrap(
            NSApplication.shared.windows.first { $0.isVisible && $0 !== document },
            "\(scene.name) raised no panel",
        )
        // Over the document rather than whatever is on the desktop: the
        // panel's material shows what is behind it.
        document.setFrame(panel.frame.insetBy(dx: -16, dy: -16), display: true)
        try capture(screenRect: document.frame, to: scene.name)
    }

    // MARK: - Settings window

    func testRenderSettingsPanes() throws {
        let frameKey = "NSWindow Frame TaigiSettingsWindow"
        setSettingRestoredAtTeardown(frameKey, to: nil)
        UserDefaults.standard.set("light", forKey: SettingsStore.Keys.appearanceMode.name)
        UserDefaults.standard.set("hanji", forKey: SettingsStore.Keys.displayLanguage.name)
        // The panes that draw from settings alone. The custom-dictionary and
        // learning-records panes list user data, which a test runner has none
        // of — they come up as an error alert and an empty list.
        let panes: [SettingsPane] = [.general, .appearance, .shortcuts, .dictionarySources, .fontManagement]
        for pane in panes {
            // Tall enough to show the pane whole; General stops above the
            // version row, which here would name the test runner's version.
            let contentHeight: CGFloat = switch pane {
            case .general: 392
            case .appearance: 290
            case .shortcuts: 1034
            default: 620
            }
            UserDefaults.standard.set(pane.rawValue, forKey: SettingsStore.Keys.selectedSettingsPane.name)
            let language = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: .standard)
            let window = SettingsWindowController.makeWindow(language: language)
            window.setFrameAutosaveName("")
            window.contentMinSize.height = contentHeight
            window.setContentSize(NSSize(width: SettingsPaneLayout.contentWidth, height: contentHeight))
            window.center()
            window.orderFrontRegardless()
            defer { window.orderOut(nil) }
            try capture(window: window, to: "settings-\(pane.rawValue)")
        }
    }

    // MARK: - Capture

    /// Long enough for the window server to have composited what was just
    /// ordered front, glass and all.
    private static let settleInterval: TimeInterval = 0.6

    private func settle() {
        RunLoop.main.run(until: Date().addingTimeInterval(Self.settleInterval))
    }

    /// `rect` is in AppKit screen coordinates (origin bottom-left);
    /// `screencapture -R` wants the origin top-left.
    private func capture(screenRect rect: NSRect, to name: String) throws {
        settle()
        let screenHeight = try XCTUnwrap(NSScreen.screens.first).frame.height
        let region = [rect.minX, screenHeight - rect.maxY, rect.width, rect.height]
            .map { String(Int($0.rounded())) }
            .joined(separator: ",")
        try runScreencapture(["-R", region], to: name)
    }

    /// The window alone, corners transparent and without its shadow.
    private func capture(window: NSWindow, to name: String) throws {
        settle()
        try runScreencapture(["-o", "-l", String(window.windowNumber)], to: name)
    }

    /// Retried: a window ordered front a moment ago is sometimes not yet one
    /// the window server will hand over, and `screencapture` exits 1.
    private func runScreencapture(_ arguments: [String], to name: String) throws {
        let output = outputDirectory.appendingPathComponent("\(name).png")
        for _ in 0 ..< 4 {
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
            process.arguments = ["-x"] + arguments + [output.path]
            try process.run()
            process.waitUntilExit()
            if process.terminationStatus == 0, FileManager.default.fileExists(atPath: output.path) {
                return
            }
            settle()
        }
        XCTFail("screencapture wrote no picture for \(name)")
    }
}

/// The stand-in for whatever the user is typing into: one line of text with
/// the composition underlined, as a host application draws marked text.
@MainActor
private final class DocumentWindow: NSWindow {
    private static let textOrigin = NSPoint(x: 22, y: 20)
    private static let textFont = NSFont.systemFont(ofSize: 22)

    private let line = NSTextField(labelWithString: "")

    init(size: NSSize, isDark: Bool) {
        let screen = NSScreen.screens.first?.visibleFrame ?? .zero
        super.init(
            contentRect: NSRect(x: screen.midX - size.width / 2, y: screen.midY - size.height / 2, width: size.width, height: size.height),
            styleMask: [.borderless], backing: .buffered, defer: false,
        )
        appearance = NSAppearance(named: isDark ? .darkAqua : .aqua)
        backgroundColor = isDark ? NSColor(white: 0.12, alpha: 1) : .white
        hasShadow = false
        isReleasedWhenClosed = false
        line.font = Self.textFont
        line.frame = NSRect(
            x: Self.textOrigin.x, y: size.height - Self.textOrigin.y - 30, width: size.width - Self.textOrigin.x * 2, height: 30,
        )
        contentView?.addSubview(line)
    }

    /// Draws the line and answers the caret's rectangle on screen, at the end
    /// of the composition.
    func show(committed: String, marked: String) -> NSRect {
        let text = NSMutableAttributedString(string: committed, attributes: [.font: Self.textFont])
        text.append(NSAttributedString(string: marked, attributes: [
            .font: Self.textFont,
            .underlineStyle: NSUnderlineStyle.single.rawValue,
        ]))
        line.attributedStringValue = text
        let width = text.size().width
        let lineOnScreen = convertToScreen(line.frame)
        return NSRect(x: lineOnScreen.minX + width + 2, y: lineOnScreen.minY, width: 1, height: lineOnScreen.height)
    }
}
