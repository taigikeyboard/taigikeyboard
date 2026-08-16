// Who is allowed to drive the one composing engine, and what a handover costs.

@testable import TaigiInputMethodCore
import XCTest

/// Each case builds its own coordinator: `shared` is process-wide because the
/// engine state it guards is, and a test that mutated it would decide what the
/// next test starts from.
@MainActor
final class ComposingSessionCoordinatorTests: XCTestCase {
    private func makeCoordinator() -> ComposingSessionCoordinator {
        ComposingSessionCoordinator(
            composingManager: ComposingManager(startingGeneration: TestFixtures.generationCounter.next()),
        )
    }

    func testOnlyTheClaimingSessionCanDriveTheEngine() {
        let coordinator = makeCoordinator()
        let focused = ComposingSessionToken()
        let background = ComposingSessionToken()

        coordinator.claim(focused)

        XCTAssertNotNil(coordinator.manager(ownedBy: focused))
        XCTAssertNil(
            coordinator.manager(ownedBy: background),
            """
            a controller still alive in another app must not reach the engine — it would \
            append to the composition the user is typing in the focused one
            """,
        )
    }

    func testHandover_startsTheNextSessionFromAnIdleEngine() {
        let coordinator = makeCoordinator()
        let first = ComposingSessionToken()
        let second = ComposingSessionToken()
        let executor = RecordingEffectExecutor()
        coordinator.claim(first).append("t", executing: executor)

        let manager = coordinator.claim(second)

        XCTAssertFalse(
            manager.isComposing,
            "what the previous session was composing belongs to a document this one cannot write to",
        )
        XCTAssertEqual(manager.rawInput, "")
    }

    func testReclaimingAnOwnedSession_leavesTheCompositionRunning() {
        let coordinator = makeCoordinator()
        let owner = ComposingSessionToken()
        let executor = RecordingEffectExecutor()
        coordinator.claim(owner).append("t", executing: executor)

        let manager = coordinator.claim(owner)

        XCTAssertTrue(
            manager.isComposing,
            "a menu or palette taking focus and giving it back must not lose what was typed",
        )
        XCTAssertEqual(manager.rawInput, "t")
    }

    func testRelease_freesTheEngineForTheNextSession() {
        let coordinator = makeCoordinator()
        let closing = ComposingSessionToken()
        let next = ComposingSessionToken()
        coordinator.claim(closing)

        coordinator.release(closing)

        XCTAssertNil(
            coordinator.manager(ownedBy: closing),
            "a closed session must not keep ownership — every session after it would be mute",
        )
        coordinator.claim(next)
        XCTAssertNotNil(
            coordinator.manager(ownedBy: next),
            "the next session must be able to take the engine the closed one held",
        )
    }

    func testReleasingASupersededSession_leavesTheLiveOneAlone() {
        let coordinator = makeCoordinator()
        let superseded = ComposingSessionToken()
        let live = ComposingSessionToken()
        let executor = RecordingEffectExecutor()
        coordinator.claim(superseded)
        coordinator.claim(live).append("t", executing: executor)

        coordinator.release(superseded)

        let manager = coordinator.manager(ownedBy: live)
        XCTAssertNotNil(manager, "the live session must keep ownership when a dead one closes")
        XCTAssertEqual(
            manager?.rawInput,
            "t",
            "a late teardown callback must not wipe the composition that replaced it",
        )
    }
}
