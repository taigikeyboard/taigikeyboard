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
    /// the shipped composition is assembled, which is why the settings store and
    /// the two learning stores are named here rather than defaulted into
    /// `ComposingManager`.
    static let shared = ComposingSessionCoordinator(
        composingManager: ComposingManager(
            settingsProvider: SettingsStore(),
            frequencyStore: shippedStores.frequency,
            customDictionaryStore: shippedStores.customDictionary,
            nextWordLearner: NextWordLearner(store: shippedStores.association),
        ),
        learningStores: shippedStores,
    )

    /// The stores the shipped composition writes to. Named once so `shared` can
    /// both hand them to the manager and expose them for `AppDelegate` to open.
    private static let shippedStores = UserDataStores(directory: UserDataDirectory.standard)

    private let composingManager: ComposingManager
    private let learningStores: UserDataStores
    private var currentOwner: ComposingSessionToken?
    private static let logger = DebugLogger(category: "SessionCoordinator")

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

    init(composingManager: ComposingManager, learningStores: UserDataStores) {
        self.composingManager = composingManager
        self.learningStores = learningStores
    }

    /// Opens the learning databases. Called once at launch: the first
    /// composition of a session would otherwise rank without the user's
    /// history while the files were still being opened.
    func openUserDataStores() {
        learningStores.open()
        // After the open, and deliberately not awaited: the seed is what a
        // brand-new install finds in 詞庫 管理, and a first keystroke typed
        // before it lands simply does not match the two seeded words yet.
        let customDictionary = learningStores.customDictionary
        Task {
            do {
                try await customDictionary.seedIfEmpty()
            } catch {
                Self.logger.error("custom dictionary seed failed: \(error)")
            }
        }
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
        // The outgoing session's endpoint must not receive shortcuts meant for
        // the incoming one. Cleared here rather than left to the new session's
        // registration, because IMK activates the incoming session before it
        // deactivates the outgoing one — same ordering hazard the candidate
        // panel's owner token exists for.
        clearShortcutTarget()
        return composingManager
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
        clearShortcutTarget()
    }
}
