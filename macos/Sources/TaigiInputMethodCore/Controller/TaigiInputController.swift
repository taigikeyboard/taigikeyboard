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

    /// This session's view of the candidate list: which candidates the last
    /// fetch returned, which one is highlighted, and which page of them is up.
    ///
    /// Per controller rather than process-wide, even though the composition it
    /// describes is not: a session that is not focused cannot reach the engine
    /// (`ComposingSessionCoordinator.manager(ownedBy:)` answers nil), so its copy
    /// is never read again, and a shared one would need the same ownership guard
    /// the coordinator already provides.
    @MainActor
    private var candidates = CandidateListModel()

    /// Where the candidate bar is shown. Backed by an optional so a test can
    /// substitute a double before the first key event: the shipped bar is an
    /// `NSPanel`, and the default cannot be written as a stored property's
    /// initial value because that expression is evaluated outside the main actor.
    @MainActor
    private var injectedPresenter: (any CandidatePresenter)?

    @MainActor
    var candidatePresenter: any CandidatePresenter {
        get { injectedPresenter ?? CandidatePanel.shared }
        set { injectedPresenter = newValue }
    }

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
    override public func recognizedEvents(_: Any!) -> Int {
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
        onMainActor(sender) { controller, client in
            controller.lastClient = client
            ComposingSessionCoordinator.shared.claim(controller.sessionToken)
            // Takes the bar down before this session starts typing, and takes
            // it away from the session that was showing it. IMK activates the
            // incoming session before it deactivates the outgoing one, so
            // without this the outgoing session's teardown is what would decide
            // whether this session's bar survives. Hiding our own window is not
            // a client query, so the activation rule above still holds.
            controller.candidatePresenter.hideForHandover()
            controller.candidates.reset()
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

    /// Take down every window this input method is showing, and nothing else.
    ///
    /// Apple's contract is UI-only: the system sends this when its own interface
    /// needs the screen, not when the composition is over, so releasing the
    /// engine or committing here would throw away work the user is in the middle
    /// of. The candidate model is cleared with the window because the two are one
    /// state as far as the key contract is concerned — the arrows and `⌃n` belong
    /// to a bar the user can see, and with the bar gone they go back to the host
    /// until the next keystroke fetches candidates again.
    override public func hidePalettes() {
        Self.logger.debug("hidePalettes")
        onMainActor(nil) { controller, _ in controller.dismissCandidates() }
        super.hidePalettes()
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

        let intent = ComposingKeyIntent.intent(
            for: key,
            isComposing: manager.isComposing,
            isShowingCandidates: !candidates.isEmpty,
        )
        Self.logger.debug("key intent \(String(describing: intent))")
        let executor = ClientEffectExecutor(client: client)
        defer { isMarkedTextVisible = manager.isComposing }
        switch intent {
        case let .input(text):
            manager.append(text, executing: executor)
            refreshCandidates(from: manager, client: client)
        case .deleteBackward:
            manager.deleteBackward(executing: executor)
            refreshCandidates(from: manager, client: client)
        case .commit:
            manager.commitComposition(executing: executor)
            dismissCandidates()
        case .cancel:
            manager.cancelComposition(executing: executor)
            dismissCandidates()
        case let .commitThenInsert(text):
            manager.commitComposition(thenInsert: text, executing: executor)
            dismissCandidates()
        case .commitThenPassThrough:
            manager.commitComposition(executing: executor)
            dismissCandidates()
            return false
        case .passThrough:
            return false
        case .commitHighlightedCandidate:
            // Unreachable by construction — the intent is only produced when the
            // list is non-empty, and a non-empty list always has a highlight.
            // Consuming the key anyway is the safe half of the impossible case:
            // letting a Space through would drop a stray space into a document
            // whose composition is still running.
            guard let highlighted = candidates.highlighted else { return true }
            commit(highlighted, from: manager, client: client, executing: executor)
        case let .selectCandidateSlot(slot):
            // A chord aimed at one of the empty slots the last page ends with.
            // Consumed rather than passed on: `⌃7` is a candidate chord while the
            // bar is up, and handing it to the host only when the page happens to
            // be short would make it fire a host shortcut at random.
            guard let selected = candidates.selectSlotInPage(slot) else { return true }
            commit(selected, from: manager, client: client, executing: executor)
        case let .moveHighlight(direction):
            candidates.moveHighlight(direction)
            presentCandidates(from: manager, client: client)
        case let .pageCandidates(direction):
            candidates.page(direction)
            presentCandidates(from: manager, client: client)
        }
        return true
    }

    // MARK: - Candidates

    /// Commits one candidate and shows whatever the composition became.
    @MainActor
    private func commit(
        _ candidate: ContinuousCandidate,
        from manager: ComposingManager,
        client: IMKTextInput,
        executing executor: ComposingEffectExecutor,
    ) {
        let outcome = manager.commitCandidate(candidate, executing: executor)
        Self.logger.debug("candidate commit \(String(describing: outcome))")
        switch outcome {
        case .finalized:
            dismissCandidates()
        case .nailed, .ignored, .unavailable:
            // Anything short of a finished composition is answered by asking the
            // engine what it is holding NOW rather than by reading the outcome:
            // a commit the engine ignored may have been ignored because a
            // generation change had already reset it to Idle
            // (`CandidateOutcomes.swift`), and treating that as "nothing
            // changed" would leave a bar describing a composition that is gone.
            refreshCandidates(from: manager, client: client)
        }
    }

    /// Re-reads the candidates for the composition as it now stands, and shows
    /// them.
    @MainActor
    private func refreshCandidates(from manager: ComposingManager, client: IMKTextInput) {
        switch manager.fetchCandidates() {
        case .unavailable:
            // The QUERY left the engine as it was, but the keystroke before it
            // did not: the character is already in the buffer and already in the
            // marked region. Candidates fetched for the previous buffer would
            // offer spans measured against text that has since changed, and
            // `CommitContinuous` only checks that a span is consumable — not
            // that it came from the composition on screen.
            Self.logger.debug("candidate fetch unavailable — taking the bar down")
            dismissCandidates()
        case .notComposing:
            dismissCandidates()
        case let .found(fetched):
            candidates.replace(with: fetched)
            if fetched.isEmpty {
                dismissCandidates()
            } else {
                presentCandidates(from: manager, client: client)
            }
        }
    }

    /// Puts the current page on screen, anchored to the caret.
    @MainActor
    private func presentCandidates(from manager: ComposingManager, client: IMKTextInput) {
        guard let caretRect = caretRect(in: client, markedTextLength: manager.displayText.utf16.count)
        else {
            // A client that cannot say where its caret is cannot host a bar that
            // points at it, and one parked in the corner of the screen is worse
            // than none: it would claim to describe text somewhere else entirely.
            //
            // The list is dropped with the window, not merely hidden. The key
            // contract turns on `isShowingCandidates`, so a model kept alive
            // behind a hidden bar would swallow the arrows and let Space commit a
            // candidate the user cannot see.
            Self.logger.debug("no caret rectangle from the client — candidates stay hidden")
            dismissCandidates()
            return
        }

        candidatePresenter.show(
            CandidateBarContent(
                labels: candidates.visiblePage.map(manager.documentText(for:)),
                highlightedSlot: candidates.highlightedSlotInPage,
            ),
            anchoredTo: caretRect,
            hostWindowLevel: client.windowLevel(),
            ownedBy: sessionToken,
        )
    }

    @MainActor
    private func dismissCandidates() {
        candidates.reset()
        candidatePresenter.hide(ownedBy: sessionToken)
    }

    /// Where the composition's last character is drawn, in screen coordinates.
    ///
    /// Walks back from the end of the marked region until the client answers
    /// with a real rectangle, matching McBopomofo
    /// (`references/McBopomofo/Source/InputMethodController.swift:886-891`).
    /// Index 0 would be wrong twice over: it is the START of the marked region
    /// rather than the caret, so the bar would drift further from the insertion
    /// point the longer the composition got, and some clients answer for that
    /// index with a zero rectangle they will happily give a later one for.
    ///
    /// "The client did not answer" is read as a rectangle left entirely at zero,
    /// not merely one at the screen origin: a caret really drawn at `(0, 0)` —
    /// the bottom-left corner of the leftmost display — still reports its line
    /// height, and rejecting it would hide the bar for a client that answered
    /// perfectly well. McBopomofo tests the origin alone
    /// (`InputMethodController.swift:886`); this is the same walk with the
    /// narrower rejection.
    ///
    /// Safe to ask here and only here: the deadlock this call causes in Chromium
    /// hosts is specific to activation (see `activateServer`).
    @MainActor
    private func caretRect(in client: IMKTextInput, markedTextLength: Int) -> CGRect? {
        var index = max(markedTextLength - 1, 0)
        while index >= 0 {
            var lineHeightRect = CGRect.zero
            _ = client.attributes(forCharacterIndex: index, lineHeightRectangle: &lineHeightRect)
            if lineHeightRect != .zero {
                return lineHeightRect
            }
            index -= 1
        }
        return nil
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
        // Before the client check: the bar belongs to this session whether or not
        // it still has a client to write into, and a session on its way out that
        // leaves one on screen leaves it there for good.
        dismissCandidates()
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
