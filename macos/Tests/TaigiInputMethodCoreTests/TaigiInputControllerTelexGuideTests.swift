// The Telex guide end to end: the chord that raises and hides it, the keys that
// leave it up, and who is allowed to take it down.

import InputMethodKit
@testable import TaigiInputMethodCore
import XCTest

/// Drives the real controller and the real engine against the shared guide
/// panel. What is pinned is the routing — when the card goes up and comes
/// down, that no key is swallowed for it, whose teardown may close it — not how the
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
        XCTAssertFalse(session.controller.settings.isTelexGuideShown)
    }

    /// The card is spelled for the romanization in use: `c` spells `ts`
    /// under TL and `ch` under POJ, which the meaning column names now that
    /// the example column is gone (USER 2026-09-09). A switch respells the
    /// guide in place.
    func testTheGuide_isSpelledForTheRomanizationInUse() throws {
        let session = try makeSession()
        let language = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)

        session.controller.performShortcutAction(.showTelexGuide)
        let tl = TelexGuidePanel.meanings(under: .tl, language: language)
        XCTAssertEqual(TelexGuidePanel.shared.shownInputMode, .tl)
        XCTAssertTrue(tl.contains { $0.contains("ts") && !$0.contains("tsh") }, "\(tl)")
        XCTAssertTrue(tl.contains { $0.contains("tsh") }, "\(tl)")

        session.controller.performShortcutAction(.toggleRomanization)

        let poj = TelexGuidePanel.meanings(under: .poj, language: language)
        XCTAssertTrue(TelexGuidePanel.shared.isShowing, "the switch does not close it")
        XCTAssertEqual(TelexGuidePanel.shared.shownInputMode, .poj)
        XCTAssertTrue(poj.contains { $0.contains("ch") && !$0.contains("chh") }, "\(poj)")
        XCTAssertTrue(poj.contains { $0.contains("chh") }, "\(poj)")
        XCTAssertFalse(poj.contains { $0.contains("ts") }, "\(poj)")
    }

    /// Under TPS, which types no Telex keys, the guide goes; the wish stays,
    /// so it returns with the romanization.
    func testTheGuide_leavesWithTps_andComesBack() throws {
        let session = try makeSession()
        session.controller.performShortcutAction(.showTelexGuide)

        session.controller.performShortcutAction(.toggleTps)

        XCTAssertFalse(TelexGuidePanel.shared.isShowing)
        XCTAssertTrue(session.controller.settings.isTelexGuideShown, "kept for the way back")

        session.controller.performShortcutAction(.toggleTps)

        XCTAssertTrue(TelexGuidePanel.shared.isShowing)
    }

    /// The next session to take focus puts the guide up from the stored wish.
    func testActivation_withTheWishStored_showsTheGuide() throws {
        userDefaults.set(true, forKey: SettingsStore.Keys.isTelexGuideShown.name)

        _ = try makeSession()

        XCTAssertTrue(TelexGuidePanel.shared.isShowing)
    }

    /// The chord is the only way the wish changes: the settings doorway
    /// leaves the guide up, and another global action does too.
    func testOtherGlobalActions_leaveTheGuideUp() throws {
        let session = try makeSession()
        session.controller.performShortcutAction(.showTelexGuide)
        let store = try makeScratchSettingsStore()

        ShortcutHotkeys.openSettings(on: nil, in: store, show: {})
        session.controller.performShortcutAction(.cycleCandidateDisplayMode)

        XCTAssertTrue(TelexGuidePanel.shared.isShowing)
    }

    /// The system asking for the palettes hides the guide for the moment: the
    /// composition stays where it was, and so does the wish.
    func testHidePalettes_hidesTheGuide_andKeepsTheCompositionAndTheWish() throws {
        let session = try makeSession()
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: session.client)
        session.controller.performShortcutAction(.showTelexGuide)

        session.controller.hidePalettes()

        XCTAssertFalse(TelexGuidePanel.shared.isShowing)
        XCTAssertTrue(session.controller.settings.isTelexGuideShown, "the system's hide is not the user's")
        XCTAssertEqual(session.client.writes.last, .setMarkedText("t", selectionLocation: 1))
    }

    func testResetGeneral_takesTheWishAway() throws {
        let session = try makeSession()
        session.controller.performShortcutAction(.showTelexGuide)

        session.controller.settings.resetGeneralSettings()

        XCTAssertNil(userDefaults.object(forKey: SettingsStore.Keys.isTelexGuideShown.name))
        XCTAssertFalse(session.controller.settings.isTelexGuideWanted)
    }

    // MARK: - Keys

    /// A letter types with the guide up, and the guide stays: reading it while
    /// typing is what it is for (#684).
    func testAPlainLetter_types_andLeavesTheGuideUp() throws {
        let session = try makeSession()
        try session.type("ta")
        session.controller.performShortcutAction(.showTelexGuide)
        session.client.clearWrites()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "i"), client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertTrue(TelexGuidePanel.shared.isShowing)
        guard case let .setMarkedText(marked, _) = try XCTUnwrap(session.client.writes.last) else {
            return XCTFail("the letter should have re-rendered the composition — got \(session.client.writes)")
        }
        XCTAssertEqual(marked, "tai")
    }

    /// Escape does what it does with no guide up: it cancels the composition.
    func testEscape_cancelsTheComposition_andLeavesTheGuideUp() throws {
        let session = try composedSession()
        XCTAssertTrue(session.presenter.isShowing, "a bar is up before the guide")
        session.controller.performShortcutAction(.showTelexGuide)

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\u{1B}"), client: session.client,
        )

        XCTAssertTrue(handled, "the cancel consumes the event")
        XCTAssertFalse(session.presenter.isShowing, "the composition and its bar are gone")
        XCTAssertTrue(TelexGuidePanel.shared.isShowing)
    }

    /// With nothing composing, Escape reaches the host: the guide no longer
    /// claims it.
    func testEscape_whenIdle_fallsThrough_andLeavesTheGuideUp() throws {
        let session = try makeSession()
        session.controller.performShortcutAction(.showTelexGuide)

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\u{1B}"), client: session.client,
        )

        XCTAssertFalse(handled)
        XCTAssertTrue(TelexGuidePanel.shared.isShowing)
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
