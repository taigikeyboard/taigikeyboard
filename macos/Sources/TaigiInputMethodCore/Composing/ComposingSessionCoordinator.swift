// Decides which input session is allowed to drive the one composing engine.

import Foundation
import os

/// Identity of one input session.
///
/// Allocated rather than derived from the controller's address: a controller
/// that is torn down without `inputControllerWillClose` leaves its ownership
/// behind, and the allocator can hand the same address to the next controller.
/// An address-derived token would let that new session inherit the dead one's
/// ownership — and its half-typed composition — instead of starting clean.
struct ComposingSessionToken: Hashable, Sendable {
    /// What the desktop core knows the session by (`ActivateRequest.token`):
    /// counted from 1 for the process and never reused. 0 is what an unset
    /// proto field decodes to, which the core refuses, so it is never minted.
    let value: UInt64

    init() {
        value = Self.lastMinted.withLock { last in
            last += 1
            return last
        }
    }

    /// Minted from the controller's stored-property initializer, which runs
    /// off the main actor — hence a lock rather than an isolated counter.
    private static let lastMinted = OSAllocatedUnfairLock<UInt64>(initialState: 0)
}

/// A session-side endpoint for the user-configurable shortcuts.
///
/// The hotkey handlers land in the app delegate with no controller or client in
/// hand; the controller that owns the focused session is the only object that
/// can apply a setting change AND take down the candidate bar that change just
/// invalidated. Weakly held by the coordinator — the controller's lifetime
/// belongs to IMK.
@MainActor
protocol ShortcutActionTarget: AnyObject {
    func performShortcutAction(_ action: ShortcutAction)
}

/// A controller that IMK has activated and may still send keys to.
@MainActor
protocol InputSessionTarget: ShortcutActionTarget {
    func takeInputOwnership()
}

/// Which input session is focused — the one that drives the process's one
/// composing engine, and the one the shortcut hotkeys act through.
///
/// IMK creates one controller per client text session, but the Rust composing
/// state is one per process (`engine/composing/src/handle.rs:21-56`). Without
/// an owner, a controller that is still alive in a background app would append
/// to the composition the user is typing in the foreground one. The engine
/// itself is the core back end's (`ComposingBackend`): it claims here
/// beside its own `Activate` / `Release`, of which the core keeps the same
/// record.
///
/// Ownership is keyed by a token the controller supplies, never by anything
/// read from the client — see `ActivateServerClientQueryTests` for why the
/// client must not be asked anything during activation.
@MainActor
final class ComposingSessionCoordinator {
    /// Process-wide, because the engine state it guards is.
    static let shared = ComposingSessionCoordinator()

    private var currentOwner: ComposingSessionToken?
    private static let logger = DebugLogger(category: "SessionCoordinator")

    private struct ActiveSession {
        let token: ComposingSessionToken
        weak var target: (any InputSessionTarget)?
    }

    /// Activation is eligibility, not exclusive focus: a system client can
    /// activate after the text field that continues receiving key events.
    private var activeSessions: [ActiveSession] = []

    var hasActiveOwner: Bool {
        activeSessions.contains { $0.token == currentOwner && $0.target != nil }
    }

    func registerActiveSession(_ target: any InputSessionTarget, for token: ComposingSessionToken) {
        activeSessions.removeAll { $0.token == token || $0.target == nil }
        activeSessions.append(ActiveSession(token: token, target: target))
    }

    func unregisterActiveSession(_ token: ComposingSessionToken) {
        activeSessions.removeAll { $0.token == token || $0.target == nil }
    }

    /// Called after the outgoing owner has released the Rust session too.
    /// A sole waiting controller can resume its panels and shortcuts before
    /// the first key; with several, key delivery must resolve the ambiguity.
    func resumeWaitingSession() {
        guard !hasActiveOwner else { return }
        activeSessions.removeAll { $0.target == nil }
        guard activeSessions.count == 1 else { return }
        activeSessions[0].target?.takeInputOwnership()
    }

    /// The controller the shortcut hotkeys act through, valid only while its
    /// session owns the engine. Weak: IMK owns controller lifetime, and a
    /// coordinator keeping one alive would keep its client alive with it.
    private weak var shortcutTarget: (any ShortcutActionTarget)?

    /// Whether `shortcutAvailabilityDidChange` was last told `true`. Tracked
    /// separately from `shortcutTarget` because that reference is weak: a
    /// controller deallocated before its session is released would otherwise
    /// read as "never armed" and swallow the disarming call.
    private var areShortcutsArmed = false

    /// Told `true` while a target is registered, `false` when none is. The
    /// shipped closure is `ShortcutHotkeys.setEnabled` (assigned at launch by
    /// `AppDelegate`); left nil in tests so exercising the coordinator never
    /// registers Carbon hotkeys in the test runner.
    var shortcutAvailabilityDidChange: ((Bool) -> Void)?

    /// Makes `owner` the session that drives the engine; true when ownership
    /// moved to it.
    ///
    /// Taking ownership from another session is where the core back end's
    /// `Activate` starts a fresh engine session: whatever the previous one was composing belongs
    /// to a document this one cannot write to. Re-claiming an ownership this
    /// session already holds answers false and leaves the composition alone —
    /// an app can be deactivated and reactivated (a menu opening, a palette
    /// taking focus) with the composition intact.
    @discardableResult
    func claim(_ owner: ComposingSessionToken) -> Bool {
        guard currentOwner != owner else {
            Self.logger.notice("claim retained session=\(owner.value)")
            return false
        }
        Self.logger.notice("claim session=\(owner.value) previous=\(currentOwner?.value.description ?? "none")")
        currentOwner = owner
        // The outgoing session's endpoint must not receive shortcuts meant for
        // the incoming one. Cleared here rather than left to the new session's
        // registration, because IMK activates the incoming session before it
        // deactivates the outgoing one — same ordering hazard the candidate
        // panel's owner token exists for.
        clearShortcutTarget()
        return true
    }

    /// Makes `target` the endpoint the shortcut hotkeys act through, and turns
    /// the hotkeys on. Only the session that owns the engine may register —
    /// a stale controller registering late would route shortcuts into a
    /// session the user has left.
    func registerShortcutTarget(_ target: any ShortcutActionTarget, for owner: ComposingSessionToken) {
        guard currentOwner == owner else { return }
        shortcutTarget = target
        areShortcutsArmed = true
        shortcutAvailabilityDidChange?(true)
    }

    /// Routes one shortcut action to the focused session's endpoint.
    ///
    /// A target that has been deallocated without its session being released
    /// disarms the hotkeys here: the reference is weak, so it can go while
    /// `areShortcutsArmed` still says otherwise, and armed-with-nowhere-to-go
    /// means the chord is taken from the host for nothing.
    func performShortcutAction(_ action: ShortcutAction) {
        guard let shortcutTarget else {
            clearShortcutTarget()
            return
        }
        shortcutTarget.performShortcutAction(action)
    }

    /// Silent when nothing is armed: every release would otherwise disarm what
    /// a newly activated session had just armed.
    private func clearShortcutTarget() {
        guard areShortcutsArmed else { return }
        shortcutTarget = nil
        areShortcutsArmed = false
        shortcutAvailabilityDidChange?(false)
    }

    /// Whether `owner` is the focused session.
    ///
    /// False is the answer that keeps a stale controller from writing into the
    /// live session's composition; callers treat it as "this key is not mine"
    /// and leave the event to the host.
    func owns(_ owner: ComposingSessionToken) -> Bool {
        currentOwner == owner
    }

    /// Gives up ownership when a session ends; true when `owner` held it —
    /// where the core back end's `Release` gives the engine up too.
    ///
    /// Called from `inputControllerWillClose`, which is the only lifecycle hook
    /// every controller is guaranteed to receive — a controller that is torn
    /// down without ever being deactivated would otherwise hold ownership for
    /// the rest of the process's life and mute every session after it.
    ///
    /// A session that is no longer the owner has already been superseded by
    /// `claim`, so releasing it must not disturb the composition that took its
    /// place.
    @discardableResult
    func release(_ owner: ComposingSessionToken) -> Bool {
        guard currentOwner == owner else {
            Self.logger.notice("release ignored session=\(owner.value) owner=\(currentOwner?.value.description ?? "none")")
            return false
        }
        Self.logger.notice("release session=\(owner.value)")
        currentOwner = nil
        clearShortcutTarget()
        return true
    }
}
