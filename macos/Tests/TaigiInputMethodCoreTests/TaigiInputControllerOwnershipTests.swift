import InputMethodKit
@testable import TaigiInputMethodCore
import XCTest

@MainActor
final class TaigiInputControllerOwnershipTests: XCTestCase {
    /// Device trace: session 2 activates, session 3 activates, then the host
    /// delivers keys to session 2. The extra activation must not mute it.
    func testAnAdditionalActivation_doesNotRejectKeysFromTheTypingSession() throws {
        let typing = try makeSession()
        let auxiliary = try makeSession()
        defer {
            typing.controller.deactivateServer(typing.client)
            auxiliary.controller.deactivateServer(auxiliary.client)
        }

        let handled = try typing.controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: typing.client)

        XCTAssertTrue(handled)
        XCTAssertEqual(typing.client.writes, [.setMarkedText("t", selectionLocation: 1)])
        XCTAssertEqual(auxiliary.client.writes, [])
    }

    func testAnAdditionalActivation_keepsTheTypingSessionsComposition() throws {
        let typing = try makeSession()
        _ = try typing.controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: typing.client)
        let auxiliary = try makeSession()
        defer {
            typing.controller.deactivateServer(typing.client)
            auxiliary.controller.deactivateServer(auxiliary.client)
        }
        typing.client.clearWrites()

        let handled = try typing.controller.handle(TestFixtures.keyDownEvent(characters: "a"), client: typing.client)

        XCTAssertTrue(handled)
        XCTAssertEqual(typing.client.writes, [.setMarkedText("ta", selectionLocation: 2)])
    }

    func testAKeyDeliveredToAnotherActiveSession_handsOverWithoutSharingComposition() throws {
        let leaving = try makeSession()
        _ = try leaving.controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: leaving.client)
        let arriving = try makeSession()
        defer { arriving.controller.deactivateServer(arriving.client) }

        XCTAssertTrue(try arriving.controller.handle(TestFixtures.keyDownEvent(characters: "k"), client: arriving.client))
        leaving.controller.deactivateServer(leaving.client)
        XCTAssertTrue(try arriving.controller.handle(TestFixtures.keyDownEvent(characters: "a"), client: arriving.client))

        XCTAssertEqual(arriving.client.writes.last, .setMarkedText("ka", selectionLocation: 2))
        XCTAssertEqual(leaving.client.writes.last, .setMarkedText("", selectionLocation: 0))
        XCTAssertTrue(leaving.client.insertedTexts.isEmpty)
    }

    func testADeactivatedSession_cannotReclaimThroughALateKey() throws {
        let leaving = try makeSession()
        let arriving = try makeSession()
        defer { arriving.controller.deactivateServer(arriving.client) }
        leaving.controller.deactivateServer(leaving.client)
        leaving.client.clearWrites()

        XCTAssertFalse(try leaving.controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: leaving.client))
        XCTAssertEqual(leaving.client.writes, [])
        XCTAssertTrue(try arriving.controller.handle(TestFixtures.keyDownEvent(characters: "a"), client: arriving.client))
        XCTAssertEqual(arriving.client.writes.last, .setMarkedText("a", selectionLocation: 1))
    }

    func testASoleWaitingSession_receivesShortcutsWhenTheOwnerEnds() throws {
        let leaving = try makeSession()
        let arriving = try makeSession()
        defer { arriving.controller.deactivateServer(arriving.client) }

        leaving.controller.deactivateServer(leaving.client)
        ComposingSessionCoordinator.shared.performShortcutAction(.toggleRomanization)

        XCTAssertEqual(arriving.controller.settings.inputMode, .poj)
        XCTAssertEqual(leaving.controller.settings.inputMode, .tl)
    }

    func testSeveralWaitingSessions_needKeyDeliveryToSelectTheShortcutTarget() throws {
        let leaving = try makeSession()
        let auxiliary = try makeSession()
        let arriving = try makeSession()
        defer {
            auxiliary.controller.deactivateServer(auxiliary.client)
            arriving.controller.deactivateServer(arriving.client)
        }

        leaving.controller.deactivateServer(leaving.client)
        ComposingSessionCoordinator.shared.performShortcutAction(.toggleRomanization)
        XCTAssertEqual(auxiliary.controller.settings.inputMode, .tl)
        XCTAssertEqual(arriving.controller.settings.inputMode, .tl)

        XCTAssertTrue(try arriving.controller.handle(TestFixtures.keyDownEvent(characters: "k"), client: arriving.client))
        ComposingSessionCoordinator.shared.performShortcutAction(.toggleRomanization)

        XCTAssertEqual(arriving.controller.settings.inputMode, .poj)
        XCTAssertEqual(auxiliary.controller.settings.inputMode, .tl)
    }

    func testClosingAnAuxiliarySession_doesNotEndTheTypingComposition() throws {
        let typing = try makeSession()
        defer { typing.controller.deactivateServer(typing.client) }
        _ = try typing.controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: typing.client)
        let auxiliary = try makeSession()

        auxiliary.controller.inputControllerWillClose()
        XCTAssertTrue(try typing.controller.handle(TestFixtures.keyDownEvent(characters: "a"), client: typing.client))

        XCTAssertEqual(typing.client.writes.last, .setMarkedText("ta", selectionLocation: 2))
    }

    private func makeSession() throws -> (controller: TaigiInputController, client: RecordingTextInputClient) {
        let controller = try TestFixtures.makeInputController()
        controller.settings = try makeScratchSettingsStore()
        controller.candidatePresenter = RecordingCandidatePresenter()
        controller.modeFlashOverride = { _ in }
        let client = RecordingTextInputClient()
        controller.activateServer(client)
        return (controller, client)
    }
}
