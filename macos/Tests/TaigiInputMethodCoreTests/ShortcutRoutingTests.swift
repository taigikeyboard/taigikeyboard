// Where a recorded chord lands: which session receives it, and when the
// hotkeys are live at all.

@testable import TaigiInputMethodCore
import XCTest

/// Records what the coordinator routed to it, standing in for the controller
/// that owns the focused session.
@MainActor
private final class RecordingShortcutTarget: ShortcutActionTarget {
    private(set) var performed: [ShortcutAction] = []

    func performShortcutAction(_ action: ShortcutAction) {
        performed.append(action)
    }
}

/// Its own coordinator per case: `shared` is process-wide, and the shipped one
/// would arm the real Carbon hotkeys through `AppDelegate`'s callback.
@MainActor
final class ShortcutRoutingTests: XCTestCase {
    func testActionReachesTheRegisteredSession() throws {
        let coordinator = try TestFixtures.makeCoordinator()
        let owner = ComposingSessionToken()
        let target = RecordingShortcutTarget()
        coordinator.claim(owner)
        coordinator.registerShortcutTarget(target, for: owner)

        coordinator.performShortcutAction(.toggleRomanization)

        XCTAssertEqual(target.performed, [.toggleRomanization])
    }

    /// A controller whose session was superseded must not receive shortcuts:
    /// it would apply them against a client the user has left.
    func testRegisteringFromASupersededSessionIsIgnored() throws {
        let coordinator = try TestFixtures.makeCoordinator()
        let stale = ComposingSessionToken()
        let target = RecordingShortcutTarget()
        coordinator.claim(stale)
        coordinator.claim(ComposingSessionToken())

        coordinator.registerShortcutTarget(target, for: stale)
        coordinator.performShortcutAction(.toggleBothScripts)

        XCTAssertTrue(target.performed.isEmpty)
    }

    /// IMK activates the incoming session before it deactivates the outgoing
    /// one, so the handover itself has to drop the endpoint — waiting for the
    /// outgoing session's teardown would leave a window where a chord reaches
    /// the app the user just left.
    func testHandoverDropsThePreviousSessionsEndpoint() throws {
        let coordinator = try TestFixtures.makeCoordinator()
        let first = ComposingSessionToken()
        let target = RecordingShortcutTarget()
        coordinator.claim(first)
        coordinator.registerShortcutTarget(target, for: first)

        coordinator.claim(ComposingSessionToken())
        coordinator.performShortcutAction(.toggleRomanization)

        XCTAssertTrue(target.performed.isEmpty)
    }

    func testReleaseDropsTheEndpoint() throws {
        let coordinator = try TestFixtures.makeCoordinator()
        let owner = ComposingSessionToken()
        let target = RecordingShortcutTarget()
        coordinator.claim(owner)
        coordinator.registerShortcutTarget(target, for: owner)

        coordinator.release(owner)
        coordinator.performShortcutAction(.toggleRomanization)

        XCTAssertTrue(target.performed.isEmpty)
    }

    // MARK: - Hotkey availability

    /// The hotkeys are Carbon-global once registered, and this process outlives
    /// the user's choice of input source. Availability tracks whether a Taigi
    /// session is focused, which is the scope the PR5 menu key equivalent had.
    func testHotkeysAreArmedWithASessionAndDisarmedWithout() throws {
        let coordinator = try TestFixtures.makeCoordinator()
        var availability: [Bool] = []
        coordinator.shortcutAvailabilityDidChange = { availability.append($0) }
        let owner = ComposingSessionToken()
        // Held for the whole case: the coordinator's reference is weak, and a
        // target that deallocates mid-case would let "armed" pass with no
        // endpoint behind it.
        let target = RecordingShortcutTarget()
        coordinator.claim(owner)

        coordinator.registerShortcutTarget(target, for: owner)
        coordinator.release(owner)

        XCTAssertEqual(availability, [true, false])
        XCTAssertTrue(target.performed.isEmpty)
    }

    /// The whole app-switch sequence in IMK's real order: B activates before A
    /// deactivates, so A's late release must not disarm the session that has
    /// already taken over.
    func testAppSwitchSequence_leavesTheIncomingSessionArmed() throws {
        let coordinator = try TestFixtures.makeCoordinator()
        var availability: [Bool] = []
        let sessionA = ComposingSessionToken()
        let sessionB = ComposingSessionToken()
        let targetA = RecordingShortcutTarget()
        let targetB = RecordingShortcutTarget()
        coordinator.claim(sessionA)
        coordinator.shortcutAvailabilityDidChange = { availability.append($0) }

        coordinator.registerShortcutTarget(targetA, for: sessionA) // A armed
        coordinator.claim(sessionB) // B activates: A's endpoint goes
        coordinator.registerShortcutTarget(targetB, for: sessionB) // B armed
        coordinator.release(sessionA) // A's late deactivate
        coordinator.performShortcutAction(.toggleRomanization)

        XCTAssertEqual(availability, [true, false, true])
        XCTAssertEqual(targetB.performed, [.toggleRomanization])
        XCTAssertTrue(targetA.performed.isEmpty)

        coordinator.release(sessionB)

        XCTAssertEqual(availability, [true, false, true, false])
    }

    /// An app that deactivates and reactivates (a menu opening, a palette
    /// taking focus) re-claims an ownership it already holds. Nothing about
    /// the composition changes there, and nothing about the hotkeys should
    /// either — a disarm/rearm pair would be a window where the chord is dead.
    func testReclaimingTheSameSession_leavesTheHotkeysArmed() throws {
        let coordinator = try TestFixtures.makeCoordinator()
        var availability: [Bool] = []
        let owner = ComposingSessionToken()
        let target = RecordingShortcutTarget()
        coordinator.claim(owner)
        coordinator.registerShortcutTarget(target, for: owner)
        coordinator.shortcutAvailabilityDidChange = { availability.append($0) }

        coordinator.claim(owner)
        coordinator.performShortcutAction(.toggleBothScripts)

        XCTAssertTrue(availability.isEmpty)
        XCTAssertEqual(target.performed, [.toggleBothScripts])
    }

    /// A controller can be deallocated without its session being released.
    /// Armed with nothing behind it means the chord is taken from the host for
    /// a command that cannot run, so the next attempt disarms.
    func testATargetThatWentAway_disarmsOnTheNextAction() throws {
        let coordinator = try TestFixtures.makeCoordinator()
        var availability: [Bool] = []
        let owner = ComposingSessionToken()
        coordinator.claim(owner)
        do {
            let target = RecordingShortcutTarget()
            coordinator.registerShortcutTarget(target, for: owner)
        }
        coordinator.shortcutAvailabilityDidChange = { availability.append($0) }

        coordinator.performShortcutAction(.toggleRomanization)

        XCTAssertEqual(availability, [false])
    }

    /// Releasing a session that never registered must not announce a change
    /// the hotkeys would act on — every release would otherwise disarm what a
    /// newly activated session had just armed.
    func testReleasingWithoutAnEndpointAnnouncesNothing() throws {
        let coordinator = try TestFixtures.makeCoordinator()
        var availability: [Bool] = []
        coordinator.shortcutAvailabilityDidChange = { availability.append($0) }
        let owner = ComposingSessionToken()

        coordinator.claim(owner)
        coordinator.release(owner)

        XCTAssertTrue(availability.isEmpty)
    }
}
