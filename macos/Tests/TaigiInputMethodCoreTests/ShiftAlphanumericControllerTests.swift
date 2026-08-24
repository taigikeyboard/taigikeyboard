// The 英數 passthrough end to end: the tap toggles it, the session boundary
// resets it, and the widened event mask keeps the click-outside commit.

import InputMethodKit
@testable import TaigiInputMethodCore
import XCTest

/// Drives the real controller and engine through the modifier-event entry
/// point. Events go through `handleModifierEvent` directly — the public
/// `handle(_:client:)` override only extracts the same three fields from the
/// `NSEvent`, which tests cannot fabricate for `flagsChanged`.
@MainActor
final class ShiftAlphanumericControllerTests: XCTestCase {
    private static let leftShift: UInt16 = 56

    override func setUp() {
        super.setUp()
        InstalledLexicon.installOnce()
    }

    // MARK: - The toggle

    func testASoloTap_entersPassthrough_andASecondTapLeavesIt() throws {
        let session = try makeSession()

        XCTAssertTrue(tap(session, at: 0), "the completing release is consumed")
        let englishKey = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "t"), client: session.client,
        )
        XCTAssertFalse(englishKey, "passthrough hands every printable key to the host")
        XCTAssertEqual(session.flashes, ["英數"])

        XCTAssertTrue(tap(session, at: 1))
        let taigiKey = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "t"), client: session.client,
        )
        XCTAssertTrue(taigiKey, "back in Taigi, the letter starts a composition")
        XCTAssertEqual(session.flashes, ["英數", "台語"])
    }

    func testATapWithACompositionInFlight_commitsItFirst() throws {
        let session = try composedSession()

        XCTAssertTrue(tap(session, at: 0))

        XCTAssertEqual(session.client.insertedTexts.count, 1, "the composition lands in the document")
        XCTAssertFalse(
            try XCTUnwrap(session.client.insertedTexts.first).isEmpty,
            "got \(session.client.insertedTexts)",
        )
    }

    func testCapsLockLatched_aSoloTapStillToggles() throws {
        // Caps Lock is a latched state riding on every event while lit, not a
        // chord the user is holding — a Shift tap under it must still work.
        let session = try makeSession()

        _ = session.controller.handleModifierEvent(
            keyCode: Self.leftShift, modifiers: [.shift, .capsLock], timestamp: 0, client: session.client,
        )
        let handled = session.controller.handleModifierEvent(
            keyCode: Self.leftShift, modifiers: [.capsLock], timestamp: 0.1, client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertEqual(session.flashes, ["英數"])
    }

    func testAShiftedChord_neverToggles() throws {
        // ⇧ goes down, a letter is typed, ⇧ comes up — the capital's Shift.
        let session = try makeSession()

        _ = press(session, at: 0)
        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "T", modifiers: .shift), client: session.client,
        )
        XCTAssertFalse(release(session, at: 0.1))
        XCTAssertEqual(session.flashes, [])
    }

    // MARK: - Session boundaries

    func testDeactivation_resetsToTaigi() throws {
        let session = try makeSession()
        _ = tap(session, at: 0)

        session.controller.deactivateServer(session.client)
        session.controller.activateServer(session.client)

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "t"), client: session.client,
        )
        XCTAssertTrue(handled, "fresh focus types Taigi")
    }

    // MARK: - The widened event mask keeps its old duties

    func testTheMask_carriesKeyDownAndFlagsChangedOnly() throws {
        let session = try makeSession()

        let mask = session.controller.recognizedEvents(session.client)

        XCTAssertEqual(
            mask,
            Int(NSEvent.EventTypeMask([.keyDown, .flagsChanged]).rawValue),
            "wider ownership than the toggle needs trades away host behaviour",
        )
    }

    func testAClickOutside_stillCommitsTheComposition() throws {
        // Widening past the default keydown mask costs IMK's automatic
        // commit-on-click (`IMKInputController.h:154-157`); the override must
        // carry it. This is the regression the mask change would otherwise
        // hide until a real device shows a stranded marked region.
        let session = try composedSession()

        session.controller.commitComposition(session.client)

        XCTAssertEqual(session.client.insertedTexts.count, 1, "the marked region lands in the document")
    }

    // MARK: - Helpers

    private final class Session {
        let controller: TaigiInputController
        let client: RecordingTextInputClient
        let store: SettingsStore
        var flashes: [String] = []

        init(controller: TaigiInputController, client: RecordingTextInputClient, store: SettingsStore) {
            self.controller = controller
            self.client = client
            self.store = store
        }
    }

    private static let composition = "taigi"
    private static let caretIndex = composition.utf16.count - 1

    private func press(_ session: Session, at time: TimeInterval) -> Bool {
        session.controller.handleModifierEvent(
            keyCode: Self.leftShift, modifiers: .shift, timestamp: time, client: session.client,
        )
    }

    private func release(_ session: Session, at time: TimeInterval) -> Bool {
        session.controller.handleModifierEvent(
            keyCode: Self.leftShift, modifiers: [], timestamp: time, client: session.client,
        )
    }

    /// One complete solo tap; answers whether the completing release was
    /// consumed.
    private func tap(_ session: Session, at time: TimeInterval) -> Bool {
        _ = press(session, at: time)
        return release(session, at: time + 0.1)
    }

    private func makeSession(configure: ((SettingsStore) -> Void)? = nil) throws -> Session {
        let client = RecordingTextInputClient()
        client.caretRects = [Self.caretIndex: CGRect(x: 120, y: 400, width: 1, height: 18)]
        let controller = try TestFixtures.makeInputController()
        controller.candidatePresenter = RecordingCandidatePresenter()
        let store = try makeScratchSettingsStore()
        configure?(store)
        controller.settings = store
        // The flash texts are asserted verbatim, so the language is pinned to
        // hanji rather than left to the machine running the tests.
        let languageSuiteName = "ShiftAlphanumericControllerTests.\(UUID().uuidString)"
        let languageDefaults = try XCTUnwrap(UserDefaults(suiteName: languageSuiteName))
        addTeardownBlock { languageDefaults.removePersistentDomain(forName: languageSuiteName) }
        controller.displayLanguageOverride = TestFixtures.makeDisplayLanguageStore(
            .hanji, userDefaults: languageDefaults,
        )
        controller.activateServer(client)
        let session = Session(controller: controller, client: client, store: store)
        controller.modeFlashOverride = { [weak session] text in session?.flashes.append(text) }
        return session
    }

    private func composedSession(configure: ((SettingsStore) -> Void)? = nil) throws -> Session {
        let session = try makeSession(configure: configure)
        for character in Self.composition.map(String.init) {
            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: character), client: session.client,
            )
        }
        return session
    }
}
