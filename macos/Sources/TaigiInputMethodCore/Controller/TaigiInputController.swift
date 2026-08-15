// Per-session IMKInputController: routes key events into the composing engine.

import InputMethodKit

/// One instance per client text session. Owns no composition of its own — it
/// claims the process-wide engine while its session is focused, translates key
/// events into composing intents, and writes the resulting effects into its own
/// client.
///
/// `@objc(TaigiInputController)` pins the Objective-C runtime name that
/// `InputMethodServerControllerClass` looks up in the bundle's Info.plist.
@objc(TaigiInputController)
public final class TaigiInputController: IMKInputController {
    /// `static`: IMK builds one controller per client text session, and the
    /// category never varies, so a stored property would create an `os_log`
    /// handle per session.
    private static let logger = DebugLogger(category: "InputController")

    /// This session's identity in the coordinator. Allocated at construction
    /// rather than read from the client, because identifying a client means
    /// asking it — see the activation rule on `activateServer` below.
    private let sessionToken = ComposingSessionToken()

    /// Whether this controller last left a marked region in its client. Kept
    /// here rather than read from the manager because the case that needs it is
    /// exactly the one where the manager no longer speaks for this session — see
    /// `finishComposition(into:)`.
    @MainActor
    private var isMarkedTextVisible = false

    /// The client this session belongs to, learned at activation — which always
    /// precedes any key event, because a session that never activated never
    /// claimed the engine. `inputControllerWillClose()` gets no sender, and this
    /// is the client holding whatever marked region has to be finished there.
    /// Weak because the client belongs to the host, not to us.
    @MainActor
    private weak var lastClient: (any IMKTextInput)?

    // MARK: - IMK entry points

    /// Keydown only, and deliberately nothing else. IMK sends
    /// `commitComposition:` when the user clicks outside an active composition
    /// ONLY for input methods whose mask is exactly the default keydown one
    /// (`IMKInputController.h:154-157`). Widening the mask — for the modifier
    /// chords a later slice may want — silently trades that behaviour away, and
    /// a composition left stranded by a click is a visible bug.
    override public func recognizedEvents(_ sender: Any!) -> Int {
        Int(NSEvent.EventTypeMask.keyDown.rawValue)
    }

    /// CHROMIUM DEADLOCK RULE — never query the client synchronously from here
    /// (`attributesForCharacterIndex:lineHeightRectangle:`, `selectedRange`,
    /// `markedRange`, `length`, `attributedSubstringFromRange:`, …). Chromium
    /// hosts deadlock on a synchronous round-trip during activation
    /// (Chromium issue 503787240, hit by azooKey-Desktop). Pinned by
    /// `ActivateServerClientQueryTests`.
    override public func activateServer(_ sender: Any!) {
        Self.logger.debug("activateServer")
        onMainActor(sender) { controller, client -> Void in
            controller.lastClient = client
            ComposingSessionCoordinator.shared.claim(controller.sessionToken)
        }
    }

    /// The session is losing focus. The composition is finished into the
    /// document rather than dropped — the user typed those characters — and
    /// ownership is given up in the same step: IMK sends no further key events
    /// to a deactivated session, so holding the engine after this point would
    /// only let a stray callback write into the session that takes over.
    override public func deactivateServer(_ sender: Any!) {
        Self.logger.debug("deactivateServer")
        onMainActor(sender) { controller, client in controller.endSession(client) }
    }

    /// Deliberately does not call `super`. `IMKInputController`'s implementation
    /// re-enters the controller to flush its own notion of the composition;
    /// the engine owns this one. Reached when the user clicks outside the
    /// marked region, which the keydown-only event mask is what earns us.
    ///
    /// Ownership is kept: the session is still focused, and the user carries on
    /// typing into it after the click.
    override public func commitComposition(_ sender: Any!) {
        Self.logger.debug("commitComposition")
        onMainActor(sender) { controller, client in controller.finishComposition(into: client) }
    }

    /// The only teardown hook every controller is guaranteed to receive, and
    /// the last chance to tidy up after a session that is closing without ever
    /// having been deactivated: the composition has to be finished into the
    /// client here, or the user's characters vanish with the session — and the
    /// engine has to be released, or every session after this one is mute.
    ///
    /// This hook gets no sender, so it finishes into the client the session last
    /// wrote to — the one whose marked region is at stake.
    override public func inputControllerWillClose() {
        Self.logger.debug("inputControllerWillClose")
        onMainActor(nil) { controller, _ in controller.endSession(controller.lastClient) }
    }

    override public func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown else { return false }
        // Snapshotted before the hop: `NSEvent` is a reference type that cannot
        // cross an isolation boundary.
        let key = KeyEventSnapshot(event)
        return onMainActor(sender) { controller, client in controller.handle(key, client: client) }
    }

    // MARK: - Main-actor work

    @MainActor
    private func handle(_ key: KeyEventSnapshot, client: IMKTextInput?) -> Bool {
        guard let manager = ComposingSessionCoordinator.shared.manager(ownedBy: sessionToken),
              let client
        else { return false }

        let intent = ComposingKeyIntent.intent(for: key, isComposing: manager.isComposing)
        Self.logger.debug("key intent \(String(describing: intent))")
        let executor = ClientEffectExecutor(client: client)
        defer { isMarkedTextVisible = manager.isComposing }
        switch intent {
        case let .input(text):
            manager.append(text, executing: executor)
        case .deleteBackward:
            manager.deleteBackward(executing: executor)
        case .commit:
            manager.commitComposition(executing: executor)
        case .cancel:
            manager.cancelComposition(executing: executor)
        case let .commitThenInsert(text):
            manager.commitComposition(thenInsert: text, executing: executor)
        case .commitThenPassThrough:
            manager.commitComposition(executing: executor)
            return false
        case .passThrough:
            return false
        }
        return true
    }

    /// Finishes the composition into `client` and gives up the engine, for a
    /// session that is going away.
    @MainActor
    private func endSession(_ client: IMKTextInput?) {
        finishComposition(into: client)
        ComposingSessionCoordinator.shared.release(sessionToken)
    }

    /// Writes whatever is composing into `client` and leaves it with no marked
    /// region.
    ///
    /// The two branches are not interchangeable. While this session owns the
    /// engine, committing is right: the user typed those characters and they
    /// belong in the document. Once another session has taken the engine, the
    /// composition is gone from Rust and only this client's marked region
    /// remains — committing is no longer possible, so the leftover is cleared
    /// instead. Clearing is skipped when nothing was marked: some clients
    /// mishandle an empty `setMarkedText` at teardown (McBopomofo issue #346,
    /// `references/McBopomofo/Source/InputMethodController.swift:485-489`).
    @MainActor
    private func finishComposition(into client: IMKTextInput?) {
        guard let client else { return }
        defer { isMarkedTextVisible = false }

        guard let manager = ComposingSessionCoordinator.shared.manager(ownedBy: sessionToken) else {
            guard isMarkedTextVisible else { return }
            ClientEffectExecutor(client: client).execute(.clearPreeditWithoutCommit)
            return
        }
        manager.commitComposition(executing: ClientEffectExecutor(client: client))
    }

    // MARK: - Main-actor assertion

    /// Runs `body` on the main actor, where the composing session lives.
    ///
    /// `assumeIsolated` does not hop — it checks that the current executor is
    /// already the main one and traps if it is not. That is deliberate. IMK
    /// delivers its callbacks on the main run loop in practice but declares
    /// none of them isolated, and an override cannot add isolation its
    /// superclass declaration lacks, so the assumption cannot be expressed in
    /// the signature. Asserting it here keeps the composing types genuinely
    /// main-actor-isolated and turns a violated platform assumption into an
    /// immediate crash rather than a silent data race. Hopping asynchronously
    /// is not an option either: `handle(_:client:)` has to answer IMK
    /// synchronously with whether it consumed the key.
    /// `body` takes the controller as a parameter rather than capturing `self`:
    /// a closure that captured it could not cross into the main-actor context
    /// without the compiler treating a non-`Sendable` controller as sent.
    private func onMainActor<T: Sendable>(
        _ sender: Any!,
        _ body: @MainActor @Sendable (TaigiInputController, IMKTextInput?) -> T,
    ) -> T {
        let arguments = CallbackArguments(controller: self, client: sender as? IMKTextInput)
        return MainActor.assumeIsolated { body(arguments.controller, arguments.client) }
    }

    /// What InputMethodKit hands a callback, carried into the assertion above.
    ///
    /// `@unchecked Sendable` because the compiler cannot check what holds here:
    /// these values are only ever read inside the enclosing callback, which the
    /// assertion has just established is running on the main actor, and neither
    /// is stored anywhere that outlives the call.
    private struct CallbackArguments: @unchecked Sendable {
        let controller: TaigiInputController
        let client: IMKTextInput?
    }
}
