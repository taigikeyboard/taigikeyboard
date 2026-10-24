// End-to-end through IMK's entry points: key event in, client document out.

import InputMethodKit
@testable import TaigiInputMethodCore
import XCTest

/// Drives the real controller against a recording client, so the wiring between
/// IMK's callbacks, the session coordinator and the engine is exercised the way
/// the system exercises it.
@MainActor
final class TaigiInputControllerTests: XCTestCase {
    func testTypedRomanization_appearsAsMarkedTextInTheClient() throws {
        let client = RecordingTextInputClient()
        let controller = try makeActivatedController(client: client)

        let handled = try controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: client)

        XCTAssertTrue(handled, "a romanization character is the input method's to consume")
        XCTAssertEqual(client.writes, [.setMarkedText("t", selectionLocation: 1)])
    }

    func testEscape_clearsTheCompositionWithoutWritingText() throws {
        let client = RecordingTextInputClient()
        let controller = try makeActivatedController(client: client)
        _ = try controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: client)

        let handled = try controller.handle(TestFixtures.keyDownEvent(characters: "\u{1B}"), client: client)

        XCTAssertTrue(handled)
        XCTAssertEqual(client.writes.last, .setMarkedText("", selectionLocation: 0))
        XCTAssertTrue(
            client.insertedTexts.isEmpty,
            "an abort must leave the document as it was",
        )
    }

    func testHostShortcut_isNotConsumed_butFinishesTheCompositionFirst() throws {
        let client = RecordingTextInputClient()
        let controller = try makeActivatedController(client: client)
        _ = try controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: client)

        let handled = try controller.handle(
            TestFixtures.keyDownEvent(characters: "s", modifiers: .command),
            client: client,
        )

        XCTAssertFalse(handled, "⌘S belongs to the host — consuming it costs the user their save")
        XCTAssertFalse(
            client.insertedTexts.isEmpty,
            """
            the composition must be written before the host acts on the document — otherwise it \
            keeps running and the next keystroke re-renders it wherever the host left the caret
            """,
        )
    }

    func testCloseWithoutDeactivate_stillFinishesTheCompositionIntoTheClient() throws {
        let client = RecordingTextInputClient()
        let controller = try makeActivatedController(client: client)
        _ = try controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: client)

        controller.inputControllerWillClose()

        XCTAssertFalse(
            client.insertedTexts.isEmpty,
            "a session torn down without a deactivate must not take the user's characters with it",
        )
    }

    func testASupersededSession_clearsItsOwnMarkedRegionInsteadOfCommitting() throws {
        let leavingClient = RecordingTextInputClient()
        let leaving = try makeActivatedController(client: leavingClient)
        _ = try leaving.handle(TestFixtures.keyDownEvent(characters: "t"), client: leavingClient)
        // A key identifies the arriving session as the one being typed in,
        // before the leaving session receives its deactivation callback.
        let arrivingClient = RecordingTextInputClient()
        let arriving = try makeActivatedController(client: arrivingClient)
        defer { arriving.deactivateServer(arrivingClient) }
        _ = try arriving.handle(TestFixtures.keyDownEvent(characters: "a", modifiers: .command), client: arrivingClient)
        leavingClient.clearWrites()

        leaving.deactivateServer(leavingClient)

        XCTAssertEqual(
            leavingClient.writes,
            [.setMarkedText("", selectionLocation: 0)],
            """
            the composition now belongs to another session, so this client's leftover marked \
            region can only be cleared — committing it would write text the engine no longer has
            """,
        )
    }

    func testDeactivate_finishesTheCompositionIntoTheClientLosingFocus() throws {
        let client = RecordingTextInputClient()
        let controller = try makeActivatedController(client: client)
        _ = try controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: client)

        controller.deactivateServer(client)

        XCTAssertFalse(
            client.insertedTexts.isEmpty,
            """
            this is the last moment the controller can reach the client holding the marked \
            region; dropping it would delete characters the user typed
            """,
        )
    }

    /// The client's commit callback can reactivate the same controller before
    /// the old deactivation returns. Its teardown must leave that new tenure
    /// owning the engine, or every subsequent letter passes through as ABC.
    func testReactivationDuringDeactivationCommit_keepsAcceptingInput() throws {
        let client = RecordingTextInputClient()
        let controller = try makeActivatedController(client: client)
        controller.candidatePresenter = RecordingCandidatePresenter()
        _ = try controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: client)
        var didReactivate = false
        client.afterInsertText = { _ in
            client.afterInsertText = nil
            didReactivate = true
            controller.activateServer(client)
        }
        defer {
            client.afterInsertText = nil
            controller.deactivateServer(client)
        }

        controller.deactivateServer(client)

        XCTAssertTrue(didReactivate)
        client.clearWrites()
        XCTAssertTrue(try controller.handle(TestFixtures.keyDownEvent(characters: "a"), client: client))
        XCTAssertEqual(client.writes, [.setMarkedText("a", selectionLocation: 1)])
    }

    /// The new tenure may already have typed before the old commit returns.
    /// Its marked region still needs clearing if another session supersedes it.
    func testTypingDuringReactivation_preservesTheNewMarkedRegionUntilHandover() throws {
        let client = RecordingTextInputClient()
        let controller = try makeActivatedController(client: client)
        controller.candidatePresenter = RecordingCandidatePresenter()
        _ = try controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: client)
        let nextKey = try TestFixtures.keyDownEvent(characters: "a")
        client.afterInsertText = { _ in
            client.afterInsertText = nil
            controller.activateServer(client)
            XCTAssertTrue(controller.handle(nextKey, client: client))
        }
        defer { client.afterInsertText = nil }

        controller.deactivateServer(client)
        let arrivingClient = RecordingTextInputClient()
        let arriving = try makeActivatedController(client: arrivingClient)
        defer { arriving.deactivateServer(arrivingClient) }
        _ = try arriving.handle(TestFixtures.keyDownEvent(characters: "a", modifiers: .command), client: arrivingClient)
        client.clearWrites()

        controller.deactivateServer(client)

        XCTAssertEqual(client.writes, [.setMarkedText("", selectionLocation: 0)])
    }

    func testAClosedSessionStopsDrivingTheEngine() throws {
        let client = RecordingTextInputClient()
        let controller = try makeActivatedController(client: client)

        controller.inputControllerWillClose()
        let handled = try controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: client)

        XCTAssertFalse(
            handled,
            "a controller that gave up ownership must not write into the next session's composition",
        )
        XCTAssertEqual(client.writes, [])
    }

    /// An app deactivated and activated again (a menu opening, a palette
    /// taking focus) with no deactivation in between keeps what was typed:
    /// the session claims the engine it already holds.
    func testActivatingAgain_keepsTheCompositionRunning() throws {
        let client = RecordingTextInputClient()
        let controller = try makeActivatedController(client: client)
        _ = try controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: client)

        controller.activateServer(client)
        _ = try controller.handle(TestFixtures.keyDownEvent(characters: "a"), client: client)

        XCTAssertEqual(client.writes.last, .setMarkedText("ta", selectionLocation: 2))
        XCTAssertTrue(client.insertedTexts.isEmpty)
    }

    /// Every key reads the settings as the store holds them then — a value
    /// written, and a value removed, which reads as its default again (the
    /// core back end: a snapshot without the entry).
    func testASettingWrittenThenRemoved_isReadOnTheNextKey() throws {
        let client = RecordingTextInputClient()
        client.caretRects = [0: CGRect(x: 120, y: 400, width: 1, height: 18)]
        let controller = try TestFixtures.makeInputController()
        let presenter = RecordingCandidatePresenter()
        controller.candidatePresenter = presenter
        let store = try makeScratchSettingsStore()
        controller.settings = store
        controller.activateServer(client)
        let windowKey = SettingsStore.Keys.isCandidateWindowEnabled.name

        store.userDefaults.set(false, forKey: windowKey)
        _ = try controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: client)
        XCTAssertFalse(presenter.isShowing, "written: no window")

        store.userDefaults.removeObject(forKey: windowKey)
        _ = try controller.handle(TestFixtures.keyDownEvent(characters: "a"), client: client)
        XCTAssertTrue(presenter.isShowing, "removed: the default, a window")
    }

    /// Keydown only. The mask carried `flagsChanged` while a solo-Shift tap
    /// drove English (ABC); that feature went on 2026-08-26 — there is no English mode
    /// here now, the user switches input sources — and a mask still asking for
    /// modifier transitions would be this input method owning events nothing
    /// reads.
    ///
    /// Widening it also costs IMK's automatic `commitComposition:` on a click
    /// outside the composition (`IMKInputController.h:154-157`), so this is the
    /// pin that keeps that cost from being paid by accident.
    func testRecognizedEvents_isKeyDownOnly() throws {
        let controller = try TestFixtures.makeInputController()

        XCTAssertEqual(
            NSEvent.EventTypeMask(rawValue: UInt64(controller.recognizedEvents(nil))),
            .keyDown,
        )
    }

    // MARK: - Helpers

    /// Activation is what claims the process-wide engine, so every case that
    /// types has to go through it — and the claim resets whatever the previous
    /// case left composing.
    private func makeActivatedController(client: RecordingTextInputClient) throws -> TaigiInputController {
        let controller = try TestFixtures.makeInputController()
        controller.activateServer(client)
        return controller
    }
}
