// The Telex guide end to end: the chord that raises it, the key that takes it
// down, and who is allowed to.

import InputMethodKit
@testable import TaigiInputMethodCore
import XCTest

/// Drives the real controller and the real engine against the shared guide
/// panel. What is pinned is the routing — when the card goes up and comes
/// down, which key is swallowed, whose teardown may close it — not how the
/// card is drawn.
@MainActor
final class TaigiInputControllerTelexGuideTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUpWithError() throws {
        try super.setUpWithError()
        InstalledLexicon.installOnce()
        suiteName = "TaigiInputControllerTelexGuideTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        // The guide is process-wide, like the bar: a card left up by one case
        // would be the card the next case finds.
        TelexGuidePanel.shared.hideNow()
    }

    override func tearDown() {
        TelexGuidePanel.shared.hideNow()
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    // MARK: - The chord

    func testTheGuideAction_showsTheGuide_ownedByTheSession() throws {
        let session = try makeSession()

        session.controller.performShortcutAction(.showTelexGuide)

        XCTAssertTrue(TelexGuidePanel.shared.isShowing)
        XCTAssertNotNil(TelexGuidePanel.shared.owner, "a guide on screen names the session that raised it")
    }

    func testTheGuideAction_again_hidesIt() throws {
        let session = try makeSession()
        session.controller.performShortcutAction(.showTelexGuide)

        session.controller.performShortcutAction(.showTelexGuide)

        XCTAssertFalse(TelexGuidePanel.shared.isShowing)
        XCTAssertNil(TelexGuidePanel.shared.owner)
    }

    /// The card is spelled for the romanization in use: `z` is `ts` under TL
    /// and `ch` under POJ, and tone 9 takes a different diacritic in each.
    func testTheGuide_isSpelledForTheRomanizationInUse() throws {
        let session = try makeSession()

        session.controller.performShortcutAction(.showTelexGuide)
        let tlExamples = TelexGuidePanel.examples(under: .tl)
        XCTAssertEqual(TelexGuidePanel.shared.shownInputMode, .tl)
        XCTAssertTrue(tlExamples.contains("zo → tso"))
        XCTAssertTrue(tlExamples.contains("tsangq → tsa̋ng"))

        session.controller.performShortcutAction(.showTelexGuide)
        session.controller.settings.inputMode = .poj
        session.controller.performShortcutAction(.showTelexGuide)

        let pojExamples = TelexGuidePanel.examples(under: .poj)
        XCTAssertEqual(TelexGuidePanel.shared.shownInputMode, .poj)
        XCTAssertTrue(pojExamples.contains("zit → chit"))
        XCTAssertTrue(pojExamples.contains("zangq → chăng"))
        XCTAssertFalse(pojExamples.contains("zo → tso"))
    }

    /// A switch that ran under an open guide would leave a table spelled for
    /// the romanization the user just left, so every other global action
    /// takes the guide down before it runs.
    func testAnotherGlobalAction_hidesTheGuideFirst() throws {
        let session = try makeSession()
        session.controller.performShortcutAction(.showTelexGuide)

        session.controller.performShortcutAction(.toggleRomanization)

        XCTAssertFalse(TelexGuidePanel.shared.isShowing)
        XCTAssertEqual(session.controller.settings.inputMode, .poj, "the switch itself still ran")
    }

    /// The settings doorway never reaches the session, so it takes the card
    /// down itself before the window comes up.
    func testOpeningSettings_hidesTheGuide() throws {
        let session = try makeSession()
        session.controller.performShortcutAction(.showTelexGuide)
        let store = try makeScratchSettingsStore()

        ShortcutHotkeys.openSettings(on: nil, in: store, show: {})

        XCTAssertFalse(TelexGuidePanel.shared.isShowing)
    }

    /// The system asking for the palettes takes the guide too — and only the
    /// guide: the composition stays where it was.
    func testHidePalettes_hidesTheGuide_andKeepsTheComposition() throws {
        let session = try makeSession()
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: session.client)
        session.controller.performShortcutAction(.showTelexGuide)

        session.controller.hidePalettes()

        XCTAssertFalse(TelexGuidePanel.shared.isShowing)
        XCTAssertEqual(session.client.writes.last, .setMarkedText("t", selectionLocation: 1))
    }

    // MARK: - The next key

    /// A letter takes the card down AND still types: the guide is something
    /// to glance at mid-word, not a mode the user has to leave first.
    func testAPlainLetter_hidesTheGuide_andStillTypes() throws {
        let session = try makeSession()
        try session.type("ta")
        session.controller.performShortcutAction(.showTelexGuide)
        session.client.clearWrites()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "i"), client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertFalse(TelexGuidePanel.shared.isShowing)
        guard case let .setMarkedText(marked, _) = try XCTUnwrap(session.client.writes.last) else {
            return XCTFail("the letter should have re-rendered the composition — got \(session.client.writes)")
        }
        XCTAssertEqual(marked, "tai")
    }

    /// Escape ends the guide and nothing else: the composition and its bar
    /// stay as they were, so a user who checked the table can go on typing
    /// the word they were in.
    func testEscape_hidesTheGuide_andIsSwallowed() throws {
        let session = try composedSession()
        XCTAssertTrue(session.presenter.isShowing, "a bar is up before the guide")
        session.controller.performShortcutAction(.showTelexGuide)
        session.client.clearWrites()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\u{1B}"), client: session.client,
        )

        XCTAssertTrue(handled, "the Escape that closed the guide must not reach the host")
        XCTAssertFalse(TelexGuidePanel.shared.isShowing)
        XCTAssertTrue(session.presenter.isShowing, "the bar stays up")
        XCTAssertEqual(session.client.writes, [], "the composition is untouched")
        XCTAssertEqual(session.client.insertedTexts, [])

        // The negative control: with no guide up the same Escape cancels the
        // composition, as it always has.
        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\u{1B}"), client: session.client,
        )
        XCTAssertFalse(session.presenter.isShowing)
    }

    /// `⌃3` arrives as Escape (`ComposingKeyIntent`): a chord the host owns
    /// takes the guide down like any key but is not swallowed for it.
    func testEscapeUnderAHostChord_hidesTheGuide_andFallsThrough() throws {
        let session = try makeSession()
        session.controller.performShortcutAction(.showTelexGuide)

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\u{1B}", modifiers: .control, charactersIgnoringModifiers: "3"),
            client: session.client,
        )

        XCTAssertFalse(TelexGuidePanel.shared.isShowing)
        XCTAssertFalse(handled, "nothing is composing, so the host keeps its chord")
    }

    // MARK: - Ownership

    /// IMK activates the incoming session before it deactivates the outgoing
    /// one, so the outgoing session's teardown must not close a guide the
    /// arriving session just raised — the candidate window's rule.
    func testDeactivate_ofAnotherSession_leavesTheLiveSessionsGuideAlone() throws {
        let leaving = try makeSession()
        let arriving = try makeSession()
        arriving.controller.performShortcutAction(.showTelexGuide)

        leaving.controller.deactivateServer(leaving.client)

        XCTAssertTrue(TelexGuidePanel.shared.isShowing, "a session that did not raise the guide took it down")

        arriving.controller.deactivateServer(arriving.client)

        XCTAssertFalse(TelexGuidePanel.shared.isShowing, "the guide goes with the focus of the session that raised it")
    }

    // MARK: - Harness

    private struct Session {
        let controller: TaigiInputController
        let client: RecordingTextInputClient
        let presenter: RecordingCandidatePresenter

        @MainActor
        func type(_ text: String) throws {
            for character in text.map(String.init) {
                _ = try controller.handle(TestFixtures.keyDownEvent(characters: character), client: client)
            }
        }
    }

    /// An activated session that has typed nothing yet, reading its settings
    /// from this case's own suite so no romanization switch here leaks into
    /// the machine's defaults.
    private func makeSession() throws -> Session {
        let client = RecordingTextInputClient()
        client.caretRects = [Self.caretIndex: CGRect(x: 120, y: 400, width: 1, height: 18)]
        let controller = try TestFixtures.makeInputController()
        controller.settings = SettingsStore(userDefaults: userDefaults)
        controller.displayLanguageOverride = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)
        let presenter = RecordingCandidatePresenter()
        controller.candidatePresenter = presenter
        controller.activateServer(client)
        return Session(controller: controller, client: client, presenter: presenter)
    }

    /// An activated session that has typed `taigi`, so a bar is up.
    private func composedSession() throws -> Session {
        let session = try makeSession()
        try session.type(Self.composition)
        return session
    }

    private static let composition = "taigi"
    private static let caretIndex = composition.utf16.count - 1
}
