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

    /// Whether the user has said, with `↓`, that they are choosing a candidate
    /// rather than still typing — which lets a bare `1`…`9` pick one out of a
    /// buffer that could otherwise still take a tone digit.
    ///
    /// Session state, so it lives here rather than in `ComposingKeyBindings`
    /// (the user's settings) or in the engine (which knows the buffer, not what
    /// the bar is doing). `ComposingKeyIntent.selectionLatch(after:wasLatched:)`
    /// is the rule; this is only where the answer is kept between keystrokes.
    @MainActor
    private var isSelectionLatched = false

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

    /// How a menu doorway puts the settings window up, and the update checker
    /// the 檢查更新 row drives. Both injectable for the same reason
    /// `displayLanguageOverride` above is, and `nil` means production: a test
    /// that drove the shipped pair would order a real window in front of
    /// whoever is running it, and reach the network.
    @MainActor
    var settingsPresenterOverride: (@MainActor () -> Void)?

    @MainActor
    var updateCheckerOverride: UpdateChecker?

    /// Where the caret sat right after this controller's last auto-inserted
    /// trailing space — armed only when the client answered a collapsed
    /// selection there, and only until the very next key event, which either
    /// swaps an attaching punctuation with that space or invalidates it.
    /// Controller state rather than something read from the document because a
    /// committed document cannot be reliably read back under IMK; the stored
    /// caret is re-checked against the client before any rewrite, so a caret
    /// moved by a mouse click this keydown-only controller never saw degrades
    /// to no swap rather than to deleting a character that was not our space.
    @MainActor
    private var armedAutoSpace: ArmedAutoSpace?

    /// A trailing auto space this controller wrote, and what it wrote it for.
    ///
    /// The two travel together because the swap needs both: `caretLocation` to
    /// verify the space is still where it was measured, and `script` to re-read
    /// the gate the way the commit that armed it read it. Since the 漢羅 key
    /// those differ — a romanization committed in 漢字 mode earns a space that
    /// the OUTPUT MODE alone would say was never earned.
    private struct ArmedAutoSpace {
        let caretLocation: Int
        let script: CandidateScript
    }

    /// Where the mode flash goes. `nil` means the shared HUD panel; a test injects a
    /// recorder, for the same reason `candidatePresenter` is injectable — the
    /// shipped one puts a real window on screen.
    @MainActor
    var modeFlashOverride: ((String) -> Void)?

    /// The client this session belongs to, learned at activation — which always
    /// precedes any key event, because a session that never activated never
    /// claimed the engine. `inputControllerWillClose()` gets no sender, and this
    /// is the client holding whatever marked region has to be finished there.
    /// Weak because the client belongs to the host, not to us.
    @MainActor
    private weak var lastClient: (any IMKTextInput)?

    // MARK: - IMK entry points

    /// Keydown only — the default, restated rather than left implicit so a
    /// future edit meets the cost of widening it before paying it.
    ///
    /// It carried `flagsChanged` too until 2026-08-26, for a solo-Shift 英數
    /// toggle this input method no longer implements — it has no English mode
    /// at all now (USER): a Mac switches input sources with ⌘Space, and
    /// switching hands the user the real ABC source rather than a
    /// pass-through imitation of one. Nothing reads a modifier transition any
    /// more, and a mask that keeps asking for them is an input method owning
    /// events it does not act on.
    ///
    /// Widening past keydown costs IMK's automatic `commitComposition:` on a
    /// click outside the composition (`IMKInputController.h:154-157`), which is
    /// why the `commitComposition(_:)` override below force-commits — kept even
    /// now that the mask is narrow again, because it is also what the manual
    /// commit paths call. `TaigiInputControllerTests` pins the mask.
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
            // Fresh focus types Taigi — and no Shift half-tapped elsewhere may
            // decide here.
            // Takes the bar down before this session starts typing, and takes
            // it away from the session that was showing it. IMK activates the
            // incoming session before it deactivates the outgoing one, so
            // without this the outgoing session's teardown is what would decide
            // whether this session's bar survives. Hiding our own window is not
            // a client query, so the activation rule above still holds.
            controller.candidatePresenter.hideForHandover()
            controller.fetchedCandidates = []
            // Whatever space a previous focus left armed was measured against
            // a document this activation may no longer be looking at.
            controller.armedAutoSpace = nil
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
    /// state as far as the key contract is concerned — the arrows and the slot
    /// keys belong to a bar the user can see, and with the bar gone they go
    /// back to the host until the next keystroke fetches candidates again. The
    /// selection latch goes with them for the same reason (`dismissCandidates`):
    /// the digits were picking out of a list that is no longer on screen.
    override public func hidePalettes() {
        Self.logger.debug("hidePalettes")
        onMainActor(nil) { controller, _ in controller.dismissCandidates() }
        super.hidePalettes()
    }

    override public func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event else { return false }
        switch event.type {
        case .keyDown:
            // Snapshotted before the hop: `NSEvent` is a reference type that
            // cannot cross an isolation boundary.
            let key = KeyEventSnapshot(event)
            return onMainActor(sender) { controller, client in controller.handle(key, client: client) }
        default:
            return false
        }
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

            // One doorway (USER 2026-08-26). There was a row per settings pane
            // from 2026-08-21 until then: each carried its own ⌃⇧ chord, and
            // once those were retired the five rows were five names for one
            // window. What is left is the window itself, opening wherever the
            // user left it — naming the panes is the sidebar's job.
            //
            // This row may claim a key equivalent because it IS a shortcut: an
            // entry in the registry the 快捷鍵 pane records, whose chord
            // carries modifiers no composition types. Read live, so the row
            // prints whatever the user last recorded on it. (No composing key
            // can ever claim one — the agent proved unable to DISPLAY one
            // without also DISPATCHING it, `InputSourceMenuRow`.)
            let openSettingsShortcut = KeyboardShortcuts.getShortcut(for: .openLastSettingsPane)
            let settings = InputSourceMenuRow(
                label: language.string(.macosMenuSettings),
                keyEquivalent: openSettingsShortcut?.nsMenuItemKeyEquivalent ?? "",
                modifiers: openSettingsShortcut?.modifiers ?? [],
                action: #selector(showPreferences(_:)),
            )
            // No chord, by design: an on-demand check is a command a user
            // reaches for once in a while, and a key equivalent claimed here
            // is taken from the host application for as long as this input
            // source is selected.
            let checkForUpdates = InputSourceMenuRow(
                label: language.string(.macosUpdateCheckNow),
                action: #selector(checkForUpdates(_:)),
            )
            return [[settings], [checkForUpdates]]
        }
        return InputSourceMenuRenderer.menu(groups)
    }

    /// Deliberately does not call `super`. The inherited implementation looks
    /// for a `preferences.nib` (`IMKInputController.h:165-170`); this package is
    /// built by SwiftPM and has no nib to find.
    ///
    /// What the menu's 設定 row sends, since 2026-08-26: the header promises
    /// that a row whose action IS `showPreferences:` routes here
    /// (`IMKInputController.h:165-170`), which is the selector the system
    /// reserves for this exact command — so a second one of our own would be
    /// two names for one body. Opening no particular pane is what both callers
    /// want: the user returns where they left off, which is also where the
    /// ⌃⌘S chord the row prints lands.
    override public func showPreferences(_: Any!) {
        Self.logger.debug("showPreferences")
        openSettings(on: nil)
    }

    /// Brings the window up on this controller's own settings store, so a test
    /// drives the menu through its own defaults suite. `nil` leaves the stored
    /// pane alone, reopening the window where the user left it.
    private func openSettings(on pane: SettingsPane?) {
        Self.logger.debug("open settings on \(pane?.rawValue ?? "the last-used pane")")
        onMainActor(nil) { controller, _ in
            ShortcutHotkeys.openSettings(
                on: pane,
                in: controller.settings,
                show: controller.settingsPresenterOverride,
            )
        }
    }

    /// Checks for a new version, with the settings window already up.
    ///
    /// The window first, then the check, and never the other way round: a
    /// fetch can take seconds, and the answer is a sheet on that window
    /// (`UpdateAlertPresenter`). Opening it at the moment the user picks the
    /// command is what puts the answer somewhere they are already looking —
    /// and this process is an `LSUIElement` whose activation makes the focused
    /// client resign, committing whatever was composing into the user's
    /// document (`UpdateNotificationOffer`), so that cost is paid on their
    /// click rather than seconds later when the network happens to answer.
    ///
    /// 一般 because that is the pane the update state lives on
    /// (`GeneralSettingsView`), so the outcome has somewhere to land.
    @objc
    private func checkForUpdates(_: Any!) {
        Self.logger.debug("check for updates")
        openSettings(on: .general)
        onMainActor(nil) { controller, _ in
            let checker = controller.updateCheckerOverride ?? UpdateChecker.shared
            checker.checkManually()
        }
    }

    private func switchInputMode(to mode: InputMode) {
        Self.logger.debug("switch input mode to \(mode.rawValue)")
        settings.inputMode = mode
        onMainActor(nil) { controller, _ in
            // The candidates on screen were fetched under the old
            // romanization, and the key contract lets Space commit whichever
            // one is highlighted. They go with the mode that produced them.
            controller.dismissCandidates()
            // Then the HUD, because the chord fires from anywhere and a
            // romanization that changed with
            // no notice reads as the keyboard breaking — the next syllable
            // composes under rules nothing on screen said had changed
            // (USER 2026-08-26).
            controller.flash(mode == .tl ? .settingsTlMode : .settingsPojMode)
        }
    }

    // MARK: - Shortcut actions

    /// What a recorded chord does while this session owns the engine. A
    /// setting has one behaviour regardless of which surface changed it. What
    /// happens to the candidate bar follows what the setting invalidates: a
    /// romanization switch changes what a fetch would return, so its bar comes
    /// down (same rule as `switchInputMode(to:)`); the 漢羅 swap changes only
    /// how the same candidates display, so its bar stays and re-renders.
    @MainActor
    func performShortcutAction(_ action: ShortcutAction) {
        switch action {
        case .openLastSettingsPane:
            // Handled process-wide by `ShortcutHotkeys.perform` before any
            // session is consulted: opening a window needs no client, and a
            // user with no focused Taigi session still expects the chord to
            // work. Named rather than defaulted so a new action cannot fall
            // silently into "do nothing".
            break
        case .toggleRomanization:
            switchInputMode(to: settings.inputMode == .tl ? .poj : .tl)
        case .toggleTranslateSwapped:
            // The bar STAYS: the swap changes how a candidate displays and
            // commits, never which candidates exist, so the list on screen is
            // still the right one — re-rendered, selection kept. Dismissing
            // here read as the window vanishing (real device, 2026-08-21).
            settings.isTranslateSwapped.toggle()
            rerenderCandidatesForDisplayChange()
        }
    }

    /// Re-renders the candidates on screen after a display-only setting flip.
    ///
    /// Through `updateCells`, not `presentCandidates`: this runs from the
    /// Carbon hotkey path, which has no client to ask for a caret rectangle —
    /// and needs none, because the window is already anchored. The cells are
    /// rebuilt through the manager, whose `cellContent(for:)` reads the live
    /// settings the toggle just wrote.
    @MainActor
    private func rerenderCandidatesForDisplayChange() {
        guard !fetchedCandidates.isEmpty,
              let manager = ComposingSessionCoordinator.shared.manager(ownedBy: sessionToken)
        else { return }
        candidatePresenter.updateCells(
            fetchedCandidates.map(manager.cellContent(for:)),
            ownedBy: sessionToken,
        )
    }

    // MARK: - Main-actor work

    @MainActor
    private func handle(_ key: KeyEventSnapshot, client: IMKTextInput?) -> Bool {
        // Every key gets exactly one chance at the swap: the arm is consumed
        // here — before ANY early return, so an event this session cannot
        // handle still invalidates it — and only the auto-space paths in the
        // switch below re-arm it. A key that went anywhere else changed the
        // document or the caret, and a swap after that would be rewriting text
        // it never measured.
        let armedSwap = armedAutoSpace
        armedAutoSpace = nil

        guard let manager = ComposingSessionCoordinator.shared.manager(ownedBy: sessionToken),
              let client
        else { return false }

        let intent = ComposingKeyIntent.intent(
            for: key,
            isComposing: manager.isComposing,
            isShowingCandidates: !fetchedCandidates.isEmpty,
            bindings: settings.composingKeyBindings,
            // The raw buffer, not `displayText`: the question is whether the
            // last thing TYPED was a letter, and the display has already
            // turned `tai5` into `tâi`.
            rawInput: manager.rawInput,
            isSelectionLatched: isSelectionLatched,
        )
        Self.logger.debug("key intent \(String(describing: intent))")

        // Updated BEFORE the intent is carried out, so every path below that
        // re-shows the bar already draws the key the NEXT keystroke will use.
        // The classification above is unaffected — it read the latch as it
        // stood when this key was pressed, which is the only reading that can
        // be right for the key that flips it.
        isSelectionLatched = ComposingKeyIntent.selectionLatch(
            after: intent, wasLatched: isSelectionLatched,
        )

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
            let committedText = manager.commitComposition(executing: executor)
            dismissCandidates()
            appendAutoSpace(afterCommit: committedText, client: client)
        case .cancel:
            manager.cancelComposition(executing: executor)
            dismissCandidates()
        case let .commitThenInsert(text):
            // Mapped before the auto-space augmentation so the full-width
            // character rides the same single mutation as the commit. On THIS
            // path the two policies still cannot both fire — full-width serves
            // the swapped mode, and a `.primary` commit there earns no space —
            // so the order
            // only keeps the contract uniform, it never composes the rewrites.
            let documentText = fullWidthMapped(text) ?? text
            let insert = AutoSpacePolicy.augmentInsert(
                documentText,
                afterComposition: manager.displayText,
                isGateActive: isAutoSpaceGateActive(),
            )
            let committedText = manager.commitComposition(thenInsert: insert.text, executing: executor)
            dismissCandidates()
            // Armed only when the engine really wrote the mutation — a commit
            // it ignored left the document without the space to swap with.
            if insert.leavesTrailingAutoSpace, committedText != nil {
                armAutoSpaceSwap(client)
            }
        case .commitThenPassThrough:
            manager.commitComposition(executing: executor)
            dismissCandidates()
            return false
        case .passThrough:
            // Attaching punctuation typed right after an auto-inserted space
            // swaps with it (`guá ` + `?` → `guá? `) instead of reaching the
            // host — one of the two pass-through keys this input method
            // consumes.
            //
            // Read BEFORE the full-width map, and since the 漢羅 key that
            // ordering decides a real case rather than an impossible one: in
            // 漢字 mode Space writes a romanization and arms a space, and the
            // `?` that follows matches both rules. The swap wins, and should —
            // the word in front of the caret is romanization, which reads as
            // Latin text and takes Latin punctuation, whatever the mode would
            // say about a hanji word. Pinned by
            // `AutoSpaceControllerTests.testTheSwapFollowsASpaceTheAlternate…`.
            if let armedSwap,
               swapAutoSpace(with: key, armedAt: armedSwap, client: client, manager: manager)
            {
                return true
            }
            // Full-width punctuation typed outside a composition — the other
            // consumed pass-through key. The host cannot map a key it types
            // itself, so the mapped character is written here instead, and the
            // engine hears about it the same way it would have below.
            if ComposingKeyIntent.isDocumentText(key), let characters = key.characters,
               let mapped = fullWidthMapped(characters)
            {
                client.insertText(mapped, replacementRange: ClientEffectExecutor.atInsertionPoint)
                manager.noteCharacterTypedOutsideComposition(mapped)
                return true
            }
            // The host gets the key either way. Text going into the document
            // without passing through a composition is still context, though:
            // a full stop typed here is what ends the sentence the next-word
            // learning would otherwise carry across.
            if ComposingKeyIntent.isDocumentText(key), let characters = key.characters {
                manager.noteCharacterTypedOutsideComposition(characters)
            }
            return false
        case .commitHighlightedCandidate:
            // The window is authoritative for which absolute index its selection
            // is on.
            commitCandidate(
                at: candidatePresenter.selectedCandidateIndex(ownedBy: sessionToken),
                from: manager, client: client, executing: executor,
            )
        case .commitAlternateScript:
            // The 漢羅 key: same candidate the highlight is on, written in the
            // other script. A candidate that has only one answers `.ignored`
            // inside the commit, so the key is consumed and nothing happens —
            // the same answer `⌃7` gets on a page with no seventh slot.
            commitCandidate(
                at: candidatePresenter.selectedCandidateIndex(ownedBy: sessionToken),
                script: .alternate,
                from: manager, client: client, executing: executor,
            )
        case let .selectCandidateSlot(slot):
            // A chord aimed at one of the empty slots the last page ends with
            // resolves to no index, and is consumed all the same: `⌃7` is a
            // candidate chord while the bar is up, and handing it to the host
            // only when the page happens to be short would make it fire a host
            // shortcut at random.
            commitCandidate(
                at: candidatePresenter.candidateIndex(forSlot: slot, ownedBy: sessionToken),
                from: manager, client: client, executing: executor,
            )
        case let .navigate(direction):
            // Navigating is the one path that changes which key picks a
            // candidate without producing a new list, so it is the one path
            // that has to say so: `↓` latches, and nothing else here would
            // repaint the keys the cells are drawn with. Before the move, so
            // the repaint the move does is already in the new style.
            //
            // Not guarded on the latch having CHANGED — `applySlotKeyStyle`
            // already returns on an unchanged style, and a second copy of that
            // test here is a claim about the panel that can only ever fall out
            // of step with it.
            if direction == .down {
                candidatePresenter.updateSlotKeyStyle(
                    slotKeyStyle(after: manager.rawInput), ownedBy: sessionToken,
                )
            }
            // The window interprets the direction for its layout and repaints
            // itself — nothing comes back, because the window is authoritative
            // for the selection and the commit paths above ask it.
            candidatePresenter.navigate(direction, ownedBy: sessionToken)
        }
        return true
    }

    /// Announces a mode the user just switched into, through the injected
    /// recorder in tests and the shared HUD in production.
    ///
    /// One caller since 2026-08-26: the 英數 toggle raised this too, until
    /// this input method stopped having an English mode (USER). Switching
    /// input sources is the system's business and it draws its own indicator;
    /// a second one of ours over it would be announcing a mode nothing here
    /// owns.
    @MainActor
    private func flash(_ mode: StringKey) {
        let language = displayLanguageOverride ?? DisplayLanguageStore.shared
        let text = language.string(mode)
        if let modeFlashOverride {
            modeFlashOverride(text)
        } else {
            ModeFlashPanel.shared.flash(text)
        }
    }

    // MARK: - Candidates

    /// Commits the candidate at `index`, or nothing when there is none to
    /// commit.
    ///
    /// Nil — a window that failed to reach a screen, or state torn down between
    /// the fetch and the key — and an index past the list both mean "nothing to
    /// commit", and the key is consumed either way: letting a Space through
    /// would drop a stray space into a document whose composition is still
    /// running, and committing would write a candidate the user cannot see.
    @MainActor
    private func commitCandidate(
        at index: Int?,
        script: CandidateScript = .primary,
        from manager: ComposingManager,
        client: IMKTextInput,
        executing executor: ComposingEffectExecutor,
    ) {
        guard let index, fetchedCandidates.indices.contains(index) else { return }
        commit(
            fetchedCandidates[index], script: script,
            from: manager, client: client, executing: executor,
        )
    }

    /// Commits one candidate and shows whatever the composition became.
    @MainActor
    private func commit(
        _ candidate: ContinuousCandidate,
        script: CandidateScript = .primary,
        from manager: ComposingManager,
        client: IMKTextInput,
        executing executor: ComposingEffectExecutor,
    ) {
        let (outcome, committedText) = manager.commitCandidate(
            candidate, script: script, executing: executor,
        )
        Self.logger.debug("candidate commit \(String(describing: outcome))")
        switch outcome {
        case .finalized:
            dismissCandidates()
            // Final commit only, mirroring iOS (`ActionHandler+Suggestions.swift:129-133`):
            // a nailed segment keeps composing more syllables — and writes
            // nothing to the document under Model B anyway.
            //
            // `script` is carried into the gate rather than short-circuiting
            // it here: spacing is a property of ROMANIZATION, and the 漢羅 key
            // is the one commit whose script disagrees with the output mode the
            // gate used to read (`AutoSpacePolicy.isGateActive`). So the answer
            // follows the document — a romanization written in 漢字 mode is
            // spaced, a hanji written in 羅馬字 mode is not — and 自動空白 OFF
            // still means no space anywhere (USER 2026-08-25).
            appendAutoSpace(afterCommit: committedText, script: script, client: client)
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
                slotKeyStyle: slotKeyStyle(after: manager.rawInput),
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

    /// Which key picks a candidate for the buffer as it stands.
    ///
    /// The same two rules the key handler classifies against — the tone-digit
    /// grammar (`ComposingKeyIntent.canTypeToneDigit`) read from the same
    /// buffer, and the selection latch over it — so the window cannot draw a
    /// key that would do something else. The key set is the user's, since they
    /// can choose which keys the slots take.
    ///
    /// Snapshotted per show rather than live-read by the window: every
    /// keystroke that changes the buffer re-fetches and re-shows, so the hint
    /// is never older than the buffer it describes. `↓` is the exception — it
    /// changes the live key without producing a new list, which is why the
    /// handler pushes the style to the window itself
    /// (`CandidatePresenter.updateSlotKeyStyle`). A rebind cannot strand a
    /// stale hint either —
    /// reaching the shortcut pane moves focus off the client, and
    /// `finishComposition` takes the bar down with the session.
    @MainActor
    private func slotKeyStyle(after rawInput: String) -> CandidateSlotKeyStyle {
        // The latch outranks the grammar rule, and has to: it exists precisely
        // for the buffers the rule keeps answering "a digit could still be a
        // tone" about.
        guard !isSelectionLatched else { return .bare }
        return ComposingKeyIntent.canTypeToneDigit(after: rawInput)
            ? .keyed(settings.composingKeyBindings.slotKeySet)
            : .bare
    }

    @MainActor
    private func dismissCandidates() {
        fetchedCandidates = []
        // The bar going away ends selection mode, whether or not the
        // composition ends with it — `hidePalettes` takes the window down and
        // leaves the composition running, and a latch surviving that would let
        // the next digit commit from a bar the user can no longer see.
        isSelectionLatched = false
        candidatePresenter.hide(ownedBy: sessionToken)
    }

    // MARK: - Full-width punctuation

    /// The full-width form of the text a punctuation key just typed, or nil
    /// when the output is roman-first or the key is not one the policy maps —
    /// the mode is read live, like the auto-space gate below, so a swap
    /// applies to the very next key.
    @MainActor
    private func fullWidthMapped(_ text: String) -> String? {
        guard settings.isTranslateSwapped else { return nil }
        return FullWidthPunctuation.mapped(text)
    }

    // MARK: - Auto-space

    /// The gate every auto-space site reads — live, so a toggle flipped in the
    /// settings window applies to the very next commit.
    ///
    /// `script` says which of the candidate's two renderings the commit wrote,
    /// which is what the gate actually turns on. It defaults to `.primary`
    /// because every site but the 漢羅 key writes what the settings lead with —
    /// the mid-composition punctuation path included, which commits the preedit
    /// as rendered.
    @MainActor
    private func isAutoSpaceGateActive(for script: CandidateScript = .primary) -> Bool {
        AutoSpacePolicy.isGateActive(
            isAutoSpaceEnabled: settings.isAutoSpaceEnabled,
            wroteRomanization: AutoSpacePolicy.writesRomanization(
                script: script,
                isTranslateSwapped: settings.isTranslateSwapped,
                isOutputBothScripts: settings.isOutputBothScripts,
            ),
        )
    }

    /// Writes the trailing auto space after a commit that produced
    /// `committedText`, and arms the punctuation swap on it.
    ///
    /// A second document mutation rather than part of the commit's: whether
    /// the space is earned depends on the text the engine decided to write,
    /// which is only known once the commit has run. `nil` — a commit that
    /// never reached the engine, or wrote nothing — earns nothing.
    ///
    /// Called from the explicit commit paths only. The lifecycle commits
    /// (`finishComposition` on deactivate, close, or a click outside) leave
    /// the document alone: the user did not finish a word there, and a space
    /// appearing at the old caret after focus moved on reads as corruption.
    @MainActor
    private func appendAutoSpace(
        afterCommit committedText: String?,
        script: CandidateScript = .primary,
        client: IMKTextInput,
    ) {
        guard let committedText,
              isAutoSpaceGateActive(for: script),
              AutoSpacePolicy.shouldAppendSpace(afterCommitting: committedText)
        else { return }
        client.insertText(" ", replacementRange: ClientEffectExecutor.atInsertionPoint)
        armAutoSpaceSwap(client, script: script)
    }

    /// Remembers where the caret sits now that the auto space is in front of
    /// it — the position the swap re-checks before it rewrites anything.
    ///
    /// A client that cannot answer, answers mid-selection, or answers with the
    /// caret at the document start simply never arms: the swap degrades to
    /// pass-through (`guá ?`) rather than risk replacing a character that was
    /// not our space. Asking here is safe — this runs inside a key event, like
    /// every client query (see `caretRect`'s activation-only deadlock rule).
    @MainActor
    private func armAutoSpaceSwap(_ client: IMKTextInput, script: CandidateScript = .primary) {
        let caret = client.selectedRange()
        guard caret.location != NSNotFound, caret.length == 0, caret.location > 0 else { return }
        armedAutoSpace = ArmedAutoSpace(caretLocation: caret.location, script: script)
    }

    /// Replaces the auto space before the caret with `?` + space — the
    /// smart-punctuation swap (`guá ` + `?` → `guá? `), matching iOS
    /// (`ActionHandler+KeyActions.swift:132-147`). Answers whether the key was
    /// consumed.
    ///
    /// Three verifications before the rewrite, because `replacementRange` is a
    /// real edit of committed text: the caret must still be a collapsed
    /// selection exactly where the space left it, and the character under the
    /// range must still be a space. Any client that fails one — including one
    /// that cannot answer a substring query at all — gets the key passed
    /// through untouched. Re-armed on success, so `?!` chains keep swapping.
    @MainActor
    private func swapAutoSpace(
        with key: KeyEventSnapshot,
        armedAt armed: ArmedAutoSpace,
        client: IMKTextInput,
        manager: ComposingManager,
    ) -> Bool {
        // Re-read against the settings as they stand NOW, deliberately: a
        // toggle switched off or a 漢羅 flip since the commit invalidates the
        // space it left behind, and the key maps or passes through instead
        // (`AutoSpaceControllerTests.testTheToggleFlippedOffAfterTheCommit…`,
        // `FullWidthPunctuationControllerTests.testSwappingModesAfterAnArmed…`).
        // Read for the script that ARMED it, though, not for `.primary`: a
        // romanization committed in 漢字 mode by the 漢羅 key really did earn
        // its space, and asking under the output mode would refuse to swap a
        // space this controller had just written.
        guard ComposingKeyIntent.isDocumentText(key),
              let characters = key.characters,
              AutoSpacePunctuation.isAttaching(characters),
              isAutoSpaceGateActive(for: armed.script)
        else { return false }
        let caret = client.selectedRange()
        guard caret.length == 0, caret.location == armed.caretLocation else { return false }
        let spaceRange = NSRange(location: armed.caretLocation - 1, length: 1)
        guard let preceding = client.attributedSubstring(from: spaceRange),
              preceding.string == " "
        else { return false }
        client.insertText(characters + " ", replacementRange: spaceRange)
        // The character still ends the next-word context, exactly as it would
        // have on the pass-through path it was consumed from.
        manager.noteCharacterTypedOutsideComposition(characters)
        // Re-armed by arithmetic rather than another `selectedRange()` query:
        // the rewrite's end is fully determined by the range just replaced,
        // and the next swap re-verifies the position against the client
        // anyway — a client that moved the caret degrades to no swap.
        armedAutoSpace = ArmedAutoSpace(
            caretLocation: armed.caretLocation + (characters as NSString).length,
            script: armed.script,
        )
        return true
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
        // Focus is moving or the user clicked — either way the caret the swap
        // was measured against is gone. (No auto space is appended here
        // either: lifecycle commits are not a finished word.)
        armedAutoSpace = nil
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
