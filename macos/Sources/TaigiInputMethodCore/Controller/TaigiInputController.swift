// Per-session IMKInputController: routes key events into the composing engine.

import InputMethodKit
import KeyboardShortcuts

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

    /// The candidates the last fetch returned, in the display order the window
    /// shows them — the absolute indices the `CandidatePresenter` seam answers
    /// with are positions in this array. The selection itself lives in the
    /// window, which owns the measured page geometry the selection moves
    /// through; this array is what maps an answered index back to the
    /// `ContinuousCandidate` a commit needs.
    ///
    /// Per controller rather than process-wide, even though the composition it
    /// describes is not: a session that is not focused cannot reach the engine
    /// (`ComposingSessionCoordinator.manager(ownedBy:)` answers nil), so its copy
    /// is never read again, and a shared one would need the same ownership guard
    /// the coordinator already provides.
    @MainActor
    private var fetchedCandidates: [ContinuousCandidate] = []

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

    /// Reads and writes the user's settings for the input-source menu. Its own
    /// instance rather than a shared one: the store holds no state, and the
    /// engine's copy is constructed at the composition root
    /// (`ComposingSessionCoordinator.shared`) — both read the same defaults
    /// domain, so a mode written here is what the next keystroke composes with.
    ///
    /// Settable so a test can point it at its own suite; the shipped one writes
    /// the settings of whoever is running the tests.
    var settings = SettingsStore()

    /// The display language the input-source menu renders in. Injectable for the same reason
    /// `settings` is: a test drives it from its own defaults suite rather than the machine's.
    ///
    /// `nil` means the process-wide store, which is what production runs with; only a test sets
    /// this. Optional rather than defaulted because the shared instance is main-actor-isolated and a
    /// stored default would have to be evaluated where this class is not.
    var displayLanguageOverride: DisplayLanguageStore?

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
            // After the claim, which just cleared the previous session's
            // endpoint: this session is the one the shortcut hotkeys should
            // now act through, and registering is what turns them on.
            ComposingSessionCoordinator.shared.registerShortcutTarget(
                controller, for: controller.sessionToken,
            )
            // Takes the bar down before this session starts typing, and takes
            // it away from the session that was showing it. IMK activates the
            // incoming session before it deactivates the outgoing one, so
            // without this the outgoing session's teardown is what would decide
            // whether this session's bar survives. Hiding our own window is not
            // a client query, so the activation rule above still holds.
            controller.candidatePresenter.hideForHandover()
            controller.fetchedCandidates = []
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

    // MARK: - Input-source menu

    /// The menu under the input-source icon in the menu bar.
    ///
    /// Built fresh on every call, which is what the contract asks for: the
    /// system calls this "whenever the menu needs to be drawn so that input
    /// methods can update the menu to reflect their current state"
    /// (`IMKInputController.h:307-310`). That is what keeps each row printing
    /// the key it currently answers to, with no refresh wiring of its own.
    ///
    /// Nothing here reads session state, so unlike the other entry points this
    /// one asserts no isolation: the mode comes from `UserDefaults`, which is
    /// thread-safe, and the rest is titles and selectors. It does build AppKit
    /// objects, and so still rests on the process-wide assumption that IMK
    /// calls its controllers on the main run loop — the assumption
    /// `onMainActor` exists to turn into a crash rather than a data race
    /// everywhere it can be checked.
    override public func menu() -> NSMenu! {
        // Each row's chord is read and assigned by hand rather than set through
        // the library's `NSMenuItem.setShortcut(for:)`: that helper registers a
        // `NotificationCenter` observer per item so a long-lived item can update
        // itself, and this menu is rebuilt from scratch every time the system
        // draws it (`IMKInputController.h:307-310`) — one observer would be
        // added per draw and never removed, since removal only happens by
        // re-binding the same item. Rebuilding IS the update mechanism the
        // observer exists to provide. Why only the global rows claim a key
        // equivalent is `InputSourceMenuRow`'s to say.
        //
        // The global chords fire through the Carbon hotkeys `ShortcutHotkeys`
        // registers, active only while a session holds the engine.
        //
        // `assumeIsolated` for the same reason the rest of this class hops
        // through `onMainActor`: IMK calls its controllers on the main run
        // loop, and the shortcut store is main-actor-isolated. Asserting turns
        // a broken assumption into a crash rather than a data race.
        //
        // The titles are resolved in the same hop, and the store is synced first: under Automatic
        // the OS language can change while the persisted tag stays `"system"`, so nothing writes the
        // key and no observation fires — a menu rebuilt per draw is exactly the right place to
        // notice.
        //
        // Read BEFORE the hop, so the closure captures a value rather than this
        // controller: `menu()` is nonisolated and the controller is not
        // Sendable, so sending `self` into a main-actor closure does not
        // compile. The store itself is `@unchecked Sendable`.
        let injectedLanguage = displayLanguageOverride
        let groups = MainActor.assumeIsolated { () -> [[InputSourceMenuRow]] in
            let language = injectedLanguage ?? DisplayLanguageStore.shared
            // Can rebuild the menu bar and relabel the settings window as a side effect: the sync
            // commits a language change, and committing one runs the chrome renderer.
            language.syncFromSettings()

            // Two rows and nothing else (USER 2026-08-21): the doorway into
            // the settings window, and the doorway into its shortcut pane.
            // Every action and every key lives behind those doors — the menu
            // stopped being the shortcut roster when the agent proved unable
            // to DISPLAY a composing key without also DISPATCHING it
            // (`InputSourceMenuRow`). The 開啟設定 chord stays: opening
            // settings is a real shortcut, and the agent's key column is its
            // display. The shortcut row has no chord — the pane is a place,
            // not an action.
            let openSettingsShortcut = KeyboardShortcuts.getShortcut(for: .openSettings)
            return [[
                InputSourceMenuRow(
                    label: ShortcutAction.openSettings.label(language),
                    keyEquivalent: openSettingsShortcut?.nsMenuItemKeyEquivalent ?? "",
                    modifiers: openSettingsShortcut?.modifiers ?? [],
                    action: #selector(showPreferences(_:)),
                ),
                InputSourceMenuRow(
                    label: language.string(.macosShortcutsTab),
                    action: #selector(openShortcutSettings(_:)),
                ),
            ]]
        }
        return InputSourceMenuRenderer.menu(groups)
    }

    /// Deliberately does not call `super`. The inherited implementation looks
    /// for a `preferences.nib` (`IMKInputController.h:165-170`); this package is
    /// built by SwiftPM and has no nib to find. The selector is kept because it
    /// is the one the system reserves for this command.
    override public func showPreferences(_: Any!) {
        Self.logger.debug("showPreferences")
        onMainActor(nil) { _, _ in SettingsWindowController.shared.show() }
    }

    /// Opens the settings window ON the shortcut pane. The pane is written
    /// before the window is asked to show, so an already-open window moves to
    /// it too.
    @objc
    private func openShortcutSettings(_: Any!) {
        onMainActor(nil) { controller, _ in
            controller.settings.selectedSettingsPane = .shortcuts
            SettingsWindowController.shared.show()
        }
    }

    private func switchInputMode(to mode: InputMode) {
        Self.logger.debug("switch input mode to \(mode.rawValue)")
        settings.inputMode = mode
        // The candidates on screen were fetched under the old romanization, and
        // the key contract lets Space commit whichever one is highlighted. They
        // go with the mode that produced them.
        onMainActor(nil) { controller, _ in controller.dismissCandidates() }
    }

    // MARK: - Shortcut actions

    /// What a recorded chord does while this session owns the engine. Reuses
    /// the menu handlers' paths so a setting has one behaviour regardless of
    /// which surface changed it; every case ends with the candidate bar down,
    /// because the candidates on screen were produced under the setting that
    /// just changed (same rule as `switchInputMode(to:)`).
    ///
    @MainActor
    func performShortcutAction(_ action: ShortcutAction) {
        switch action {
        case .openSettings:
            // Handled process-wide by `ShortcutHotkeys.perform` before any
            // session is consulted: opening a window needs no client, and a
            // user with no focused Taigi session still expects the chord to
            // work. Named rather than defaulted so a new action cannot fall
            // silently into "do nothing".
            break
        case .toggleRomanization:
            switchInputMode(to: settings.inputMode == .tl ? .poj : .tl)
        case .toggleTranslateSwapped:
            settings.isTranslateSwapped.toggle()
            dismissCandidates()
        }
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
            isShowingCandidates: !fetchedCandidates.isEmpty,
            bindings: settings.composingKeyBindings,
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
            // The host gets the key either way. Text going into the document
            // without passing through a composition is still context, though:
            // a full stop typed here is what ends the sentence the next-word
            // learning would otherwise carry across.
            if ComposingKeyIntent.isDocumentText(key), let characters = key.characters {
                manager.noteCharacterTypedOutsideComposition(characters)
            }
            return false
        case let .commitHighlightedCandidate(rendering):
            // The window answers which absolute index its selection is on. Nil
            // — a window that failed to reach a screen, or state torn down
            // between the fetch and the key — consumes the key without
            // committing: letting a Space through would drop a stray space into
            // a document whose composition is still running, and committing
            // would write a candidate the user cannot see.
            guard let selectedIndex = candidatePresenter.selectedCandidateIndex(ownedBy: sessionToken),
                  fetchedCandidates.indices.contains(selectedIndex)
            else { return true }
            let candidate = fetchedCandidates[selectedIndex]
            // A candidate that has not got the script the key asked for is left
            // alone, and the chord is consumed either way so it never reaches
            // the host (`CandidateDocumentText.Rendering.canRender`).
            guard rendering.canRender(candidate) else { return true }
            commit(candidate, rendering: rendering, from: manager, client: client, executing: executor)
        case let .selectCandidateSlot(slot):
            // A chord aimed at one of the empty slots the last page ends with.
            // Consumed rather than passed on: `⌃7` is a candidate chord while the
            // bar is up, and handing it to the host only when the page happens to
            // be short would make it fire a host shortcut at random.
            guard let selectedIndex = candidatePresenter.candidateIndex(forSlot: slot, ownedBy: sessionToken),
                  fetchedCandidates.indices.contains(selectedIndex)
            else { return true }
            commit(fetchedCandidates[selectedIndex], from: manager, client: client, executing: executor)
        case let .navigate(direction):
            // The window interprets the direction for its layout and repaints
            // itself — nothing comes back, because the window is authoritative
            // for the selection and the commit paths above ask it.
            candidatePresenter.navigate(direction, ownedBy: sessionToken)
        }
        return true
    }

    // MARK: - Candidates

    /// Commits one candidate and shows whatever the composition became.
    @MainActor
    private func commit(
        _ candidate: ContinuousCandidate,
        rendering: CandidateDocumentText.Rendering = .settings,
        from manager: ComposingManager,
        client: IMKTextInput,
        executing executor: ComposingEffectExecutor,
    ) {
        let outcome = manager.commitCandidate(candidate, rendering: rendering, executing: executor)
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
            fetchedCandidates = fetched
            if fetched.isEmpty {
                dismissCandidates()
            } else {
                presentCandidates(from: manager, client: client)
            }
        }
    }

    /// Puts the list on screen, anchored to the caret. The window selects its
    /// first candidate — a fresh keystroke re-ranks the whole list, so a held
    /// position would sit on an unrelated word.
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
            CandidateWindowContent(
                cells: fetchedCandidates.map(manager.cellContent(for:)),
            ),
            anchoredTo: caretRect,
            hostWindowLevel: client.windowLevel(),
            // Feeds the Multicolour accent resolution: when the system has no
            // fixed accent, the highlight takes the host app's own. Safe to ask
            // here — this runs inside a key event, like every client query.
            hostBundleIdentifier: client.bundleIdentifier(),
            ownedBy: sessionToken,
        )
    }

    @MainActor
    private func dismissCandidates() {
        fetchedCandidates = []
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

/// The method lives in the class body (it needs the private candidate state);
/// the conformance is stated here where it reads as the contract it is.
extension TaigiInputController: ShortcutActionTarget {}
