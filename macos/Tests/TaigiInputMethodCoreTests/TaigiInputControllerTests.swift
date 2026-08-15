// End-to-end through IMK's entry points: key event in, client document out.

import InputMethodKit
import XCTest

@testable import TaigiInputMethodCore

/// Drives the real controller against a recording client, so the wiring between
/// IMK's callbacks, the session coordinator and the engine is exercised the way
/// the system exercises it.
@MainActor
final class TaigiInputControllerTests: XCTestCase {
    func testTypedRomanization_appearsAsMarkedTextInTheClient() throws {
        let client = RecordingTextInputClient()
        let controller = try makeActivatedController(client: client)

        let handled = controller.handle(try TestFixtures.keyDownEvent(characters: "t"), client: client)

        XCTAssertTrue(handled, "a romanization character is the input method's to consume")
        XCTAssertEqual(client.writes, [.setMarkedText("t", selectionLocation: 1)])
    }

    func testEscape_clearsTheCompositionWithoutWritingText() throws {
        let client = RecordingTextInputClient()
        let controller = try makeActivatedController(client: client)
        _ = controller.handle(try TestFixtures.keyDownEvent(characters: "t"), client: client)

        let handled = controller.handle(try TestFixtures.keyDownEvent(characters: "\u{1B}"), client: client)

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
        _ = controller.handle(try TestFixtures.keyDownEvent(characters: "t"), client: client)

        let handled = controller.handle(
            try TestFixtures.keyDownEvent(characters: "s", modifiers: .command),
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
        _ = controller.handle(try TestFixtures.keyDownEvent(characters: "t"), client: client)

        controller.inputControllerWillClose()

        XCTAssertFalse(
            client.insertedTexts.isEmpty,
            "a session torn down without a deactivate must not take the user's characters with it",
        )
    }

    func testASupersededSession_clearsItsOwnMarkedRegionInsteadOfCommitting() throws {
        let leavingClient = RecordingTextInputClient()
        let leaving = try makeActivatedController(client: leavingClient)
        _ = leaving.handle(try TestFixtures.keyDownEvent(characters: "t"), client: leavingClient)
        // The arriving session takes the engine before the leaving one is told
        // it lost focus; IMK does not order these callbacks across sessions.
        _ = try makeActivatedController(client: RecordingTextInputClient())
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
        _ = controller.handle(try TestFixtures.keyDownEvent(characters: "t"), client: client)

        controller.deactivateServer(client)

        XCTAssertFalse(
            client.insertedTexts.isEmpty,
            """
            this is the last moment the controller can reach the client holding the marked \
            region; dropping it would delete characters the user typed
            """,
        )
    }

    func testAClosedSessionStopsDrivingTheEngine() throws {
        let client = RecordingTextInputClient()
        let controller = try makeActivatedController(client: client)

        controller.inputControllerWillClose()
        let handled = controller.handle(try TestFixtures.keyDownEvent(characters: "t"), client: client)

        XCTAssertFalse(
            handled,
            "a controller that gave up ownership must not write into the next session's composition",
        )
        XCTAssertEqual(client.writes, [])
    }

    func testRecognizedEvents_isKeyDownOnly() throws {
        let controller = try TestFixtures.makeInputController()

        XCTAssertEqual(
            controller.recognizedEvents(nil),
            Int(NSEvent.EventTypeMask.keyDown.rawValue),
            """
            IMK only sends `commitComposition:` on a click outside the marked region for \
            input methods whose mask is exactly keyDown (IMKInputController.h:154-157)
            """,
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
