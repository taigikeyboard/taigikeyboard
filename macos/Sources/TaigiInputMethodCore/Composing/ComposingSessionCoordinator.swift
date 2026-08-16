// Decides which input session is allowed to drive the one composing engine.

import Foundation

/// Identity of one input session.
///
/// Allocated rather than derived from the controller's address: a controller
/// that is torn down without `inputControllerWillClose` leaves its ownership
/// behind, and the allocator can hand the same address to the next controller.
/// An address-derived token would let that new session inherit the dead one's
/// ownership — and its half-typed composition — instead of starting clean.
struct ComposingSessionToken: Hashable, Sendable {
    /// Only ever compared, never read — which is why a `UUID` is enough and no
    /// counter, lock or mutable global is needed to hand these out.
    private let value = UUID()
}

/// Hands the single `ComposingManager` to whichever input session is focused.
///
/// IMK creates one controller per client text session, but the Rust composing
/// state is one per process (`engine/composing/src/handle.rs:21-56`). Without
/// an owner, a controller that is still alive in a background app would append
/// to the composition the user is typing in the foreground one.
///
/// Ownership is keyed by a token the controller supplies, never by anything
/// read from the client — see `ActivateServerClientQueryTests` for why the
/// client must not be asked anything during activation.
@MainActor
final class ComposingSessionCoordinator {
    /// Process-wide, because the engine state it guards is — and the one place
    /// the shipped composition is assembled, which is why the settings store is
    /// named here rather than defaulted into `ComposingManager`.
    static let shared = ComposingSessionCoordinator(
        composingManager: ComposingManager(settingsProvider: SettingsStore()),
    )

    private let composingManager: ComposingManager
    private var currentOwner: ComposingSessionToken?
    private static let logger = DebugLogger(category: "SessionCoordinator")

    init(composingManager: ComposingManager) {
        self.composingManager = composingManager
    }

    /// Makes `owner` the session that drives the engine, and returns the
    /// manager it should drive.
    ///
    /// Taking ownership from another session starts a fresh engine session:
    /// whatever the previous one was composing belongs to a document this one
    /// cannot write to. Re-claiming an ownership this session already holds
    /// leaves the composition alone — an app can be deactivated and reactivated
    /// (a menu opening, a palette taking focus) with the composition intact.
    @discardableResult
    func claim(_ owner: ComposingSessionToken) -> ComposingManager {
        guard currentOwner != owner else { return composingManager }
        Self.logger.debug("session ownership changed")
        composingManager.startNewSession()
        currentOwner = owner
        return composingManager
    }

    /// The manager, or `nil` when `owner` is not the focused session.
    ///
    /// `nil` is the answer that keeps a stale controller from writing into the
    /// live session's composition; callers treat it as "this key is not mine"
    /// and leave the event to the host.
    func manager(ownedBy owner: ComposingSessionToken) -> ComposingManager? {
        currentOwner == owner ? composingManager : nil
    }

    /// Gives up ownership when a session ends.
    ///
    /// Called from `inputControllerWillClose`, which is the only lifecycle hook
    /// every controller is guaranteed to receive — a controller that is torn
    /// down without ever being deactivated would otherwise hold ownership for
    /// the rest of the process's life and mute every session after it.
    ///
    /// A session that is no longer the owner has already been superseded by
    /// `claim`, so releasing it must not disturb the composition that took its
    /// place.
    func release(_ owner: ComposingSessionToken) {
        guard currentOwner == owner else { return }
        Self.logger.debug("session ownership released")
        composingManager.startNewSession()
        currentOwner = nil
    }
}
